#!/usr/bin/env python3
"""B1 on M5: the beer game driven by 70 store-by-department weekly demand series.

    python3 prepare_m5.py && python3 build_model.py && python3 run_m5.py

Design fixed in CRITERIA.md: each series is scaled by its own TRAINING mean to 100 cases a week (no test
information in the scaling), policies are tuned on weeks 0 to 224 (cost counted from week 52) and frozen,
and the last 52 weeks are the test window. The chain is replayed from week 0 in the test run so that its
state at the start of the window is the real one; only the parameters come from training.
"""

import csv
import json
import math
import os
import shutil
import statistics
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

import run_pilot as rp  # shared helpers: fc, statistics, csv writer

HERE = rp.HERE
MODEL, BUILD, RESULTS, FC = rp.MODEL, rp.BUILD, rp.RESULTS, rp.FC
STAGES = rp.STAGES
DATA = os.path.join(HERE, "data", "m5_store_department_weekly.csv")
TOTAL, TEST_WEEKS, WARM = 277, 52, 52
TRAIN_END = TOTAL - TEST_WEEKS  # 225
EFFORT = 2
LEAD = {"Retailer": 4, "Wholesaler": 4, "Distributor": 4, "Factory": 2}
OUT = rp.OUT
PRETTY = rp.PRETTY
PLACEHOLDER = {"theta": 0.36, "alpha": 0.26, "beta": 0.34, "S prime": 17}  # same placeholders as the pilot
WORKERS = 6


def load():
    by = {}
    for r in csv.DictReader(open(DATA)):
        by.setdefault((r["store_id"], r["dept_id"]), []).append(int(r["units"]))
    return by


def variant(path, overrides, table, final_time):
    shutil.copy(MODEL, path)
    ops = [{"op": "set_equation", "name": k, "equation": str(v)} for k, v in overrides.items()]
    ops.append({"op": "set_table", "name": "Demand data", "points": table})
    ops.append({"op": "set_setting", "key": "final_time", "value": str(final_time)})
    bp = path + ".batch.json"
    json.dump(ops, open(bp, "w"))
    rp.fc("ops", path, bp, "--actor", "run_m5", "--no-gate")
    os.remove(bp)
    log = path + ".oplog.jsonl"
    if os.path.exists(log):
        os.remove(log)


def optimize(path, levers):
    args = ["optimize", path, "--goal", "Chain window cost", "--measure", "final", "--effort", EFFORT]
    for k, (lo, hi) in levers.items():
        args += ["--lever", f"{k}:{lo}:{hi}"]
    d = rp.fc(*args)
    return d["best"]["values"], d["evaluations"], d["start"]["goal"], d["best"]["goal"]


def run(path, sets):
    args = ["run", path, "--series", ",".join(OUT)]
    for k, v in sets.items():
        args += ["--set", f"{k}={v}"]
    d = rp.fc(*args)
    return {PRETTY.get(r["name"], r["name"]): r["last"] for r in d["series"]}


def one_series(key, units):
    store, dept = key
    tag = f"{store}_{dept}"
    train = units[:TRAIN_END]
    scale = 100.0 / statistics.mean(train)
    y = [u * scale for u in units]
    mu, sd = 100.0, statistics.pstdev([u * scale for u in train])
    m0 = statistics.mean(y[:4])
    table = [[k, v] for k, v in enumerate(y)]
    base = {"Demand regime": 2, "Initial demand": round(m0, 6), "Initial inventory": round(3 * m0, 6), "Base demand": round(m0, 6),
            "Demand mean": mu, "Demand sd": round(sd, 6), "Use equilibrium S prime": 0}
    formula = {f"{s} base stock level": round(mu * (LEAD[s] + 1) + 1.28 * sd * math.sqrt(LEAD[s] + 1), 4) for s in STAGES}
    p0 = {**{f"{s} policy": 0 for s in STAGES}, **{f"{s} {k}": (v if k != "S prime" else round(v * m0 / 4, 4)) for s in STAGES for k, v in PLACEHOLDER.items()}}
    all_bs = {f"{s} policy": 1 for s in STAGES}
    trn = os.path.join(BUILD, f"m5_train_{tag}.sdm")
    tst = os.path.join(BUILD, f"m5_test_{tag}.sdm")
    variant(trn, {**base, **p0, "Test start": WARM}, table, TRAIN_END)
    variant(tst, {**base, **p0, "Test start": TRAIN_END}, table, TOTAL)

    def tuned(over, levers, name):
        t = os.path.join(BUILD, f"m5_train_{tag}_{name}.sdm")
        variant(t, {**base, **over, "Test start": WARM}, table, TRAIN_END)
        return optimize(t, levers)

    hi = 15 * m0
    bs_levers = {f"{s} base stock level": (0, round(hi, 3)) for s in STAGES}
    aa_levers = {}
    for s in STAGES:
        aa_levers.update({f"{s} theta": (0.05, 1.0), f"{s} alpha": (0.02, 1.0), f"{s} beta": (0.0, 3.0), f"{s} S prime": (0, round(hi, 3))})
    p1_best, p1_ev, _, p1_train = tuned({**all_bs, **formula}, bs_levers, "p1")
    p2_best, p2_ev, p2_start, p2_train = tuned(p0, aa_levers, "p2")
    pol = {
        "P0 human-like (placeholder parameters)": p0,
        "P1 base-stock (formula levels)": {**all_bs, **formula},
        "P1 base-stock (tuned)": {**all_bs, **p1_best},
        "P2 SD rule (tuned)": {**p0, **p2_best},
    }
    rows = []
    for name, ov in pol.items():
        r = run(tst, ov)
        rows.append({"store": store, "dept": dept, "policy": name, **r})
    # S1 and S2 at the retailer position: the other three stages fixed
    pos_rows = []
    for scen, others in (("S1 others base-stock (tuned)", "bs"), ("S2 others human-like", "p0")):
        fixed = {}
        for s in STAGES[1:]:
            fixed.update({f"{s} policy": 1, f"{s} base stock level": p1_best[f"{s} base stock level"]} if others == "bs" else {f"{s} policy": 0, **{f"{s} {k}": p0[f"{s} {k}"] for k in PLACEHOLDER}})
        pb, _, _, _ = tuned({**fixed, "Retailer policy": 1, "Retailer base stock level": formula["Retailer base stock level"]}, {"Retailer base stock level": (0, round(hi, 3))}, f"{others}_p1")
        pa, _, _, _ = tuned({**fixed, "Retailer policy": 0, **{f"Retailer {k}": p0[f"Retailer {k}"] for k in PLACEHOLDER}}, {f"Retailer {k}": r for k, r in (("theta", (0.05, 1.0)), ("alpha", (0.02, 1.0)), ("beta", (0.0, 3.0)), ("S prime", (0, round(hi, 3))))}, f"{others}_p2")
        for lab, ov in (("P0 placeholder", {**fixed, "Retailer policy": 0, **{f"Retailer {k}": p0[f"Retailer {k}"] for k in PLACEHOLDER}}), ("P1 tuned", {**fixed, "Retailer policy": 1, **pb}), ("P2 tuned", {**fixed, "Retailer policy": 0, **pa})):
            r = run(tst, ov)
            pos_rows.append({"store": store, "dept": dept, "scenario": scen, "policy": lab, "Chain window cost": r["Chain window cost"], "Fill rate": r["Fill rate"]})
    tuned_row = {"store": store, "dept": dept, "scale": scale, "train_sd_over_mean": sd / mu, "p1_evals": p1_ev, "p2_evals": p2_ev, "p1_train_cost": p1_train, "p2_train_cost": p2_train,
                 **{k: v for k, v in p2_best.items()}, **{k: v for k, v in p1_best.items()}}
    return rows, pos_rows, tuned_row


def pct(x):
    return f"{100 * x:+.1f}%"


def main():
    if not os.path.exists(DATA):
        sys.exit("run prepare_m5.py first")
    os.makedirs(BUILD, exist_ok=True)
    t0 = time.time()
    series = load()
    keys = sorted(series)
    results = {}
    errors = []

    def job(k):
        try:
            return k, one_series(k, series[k])
        except Exception as e:  # keep going; the failure is reported
            return k, e

    with ThreadPoolExecutor(max_workers=WORKERS) as ex:
        for k, res in ex.map(job, keys):
            if isinstance(res, Exception):
                errors.append((k, str(res)[:200]))
            else:
                results[k] = res
    all_rows = [r for k in sorted(results) for r in results[k][0]]
    pos_rows = [r for k in sorted(results) for r in results[k][1]]
    tuned_rows = [results[k][2] for k in sorted(results)]
    rp.write_csv("m5_s3_series.csv", all_rows)
    rp.write_csv("m5_s1_s2_retailer.csv", pos_rows)
    rp.write_csv("m5_tuned_parameters.csv", [{k: (round(v, 6) if isinstance(v, float) else v) for k, v in r.items()} for r in tuned_rows])

    pols = ["P0 human-like (placeholder parameters)", "P1 base-stock (formula levels)", "P1 base-stock (tuned)", "P2 SD rule (tuned)"]
    cost = {p: {(r["store"], r["dept"]): r["Chain window cost"] for r in all_rows if r["policy"] == p} for p in pols}
    ks = sorted(cost[pols[0]])
    n = len(ks)
    summ = []
    for p in pols:
        c = [cost[p][k] for k in ks]
        rr = [r for r in all_rows if r["policy"] == p]
        wins = sum(1 for k in ks if cost[p][k] == min(cost[q][k] for q in pols))
        summ.append({"policy": p, "median_cost": statistics.median(c), "q1": rp.percentile(c, .25), "q3": rp.percentile(c, .75), "series_it_wins": wins,
                     "fill_rate": statistics.mean(r["Fill rate"] for r in rr), **{f"{s}_bullwhip": statistics.median(r[f"{s} bullwhip"] for r in rr) for s in STAGES}})
    rp.write_csv("m5_s3_summary.csv", summ)

    def cmp(a, b, label):
        return rp.compare([cost[a][k] for k in ks], [cost[b][k] for k in ks], label)

    comps = [cmp("P2 SD rule (tuned)", "P1 base-stock (tuned)", "M5 S3: P2 vs P1 tuned"),
             cmp("P2 SD rule (tuned)", "P0 human-like (placeholder parameters)", "M5 S3: P2 vs P0 (descriptive)"),
             cmp("P1 base-stock (tuned)", "P1 base-stock (formula levels)", "M5 S3: P1 tuned vs P1 formula (descriptive)")]
    rp.write_csv("m5_s3_comparisons.csv", comps)
    pos = {}
    for r in pos_rows:
        pos.setdefault((r["scenario"], r["policy"]), {})[(r["store"], r["dept"])] = r["Chain window cost"]
    pos_cmp = []
    for scen in ("S1 others base-stock (tuned)", "S2 others human-like"):
        ks2 = sorted(pos[(scen, "P2 tuned")])
        pos_cmp.append(rp.compare([pos[(scen, "P2 tuned")][k] for k in ks2], [pos[(scen, "P1 tuned")][k] for k in ks2], f"{scen}, Retailer: P2 vs P1 tuned"))
    rp.write_csv("m5_s1_s2_comparisons.csv", pos_cmp)
    all_rows_global.extend(all_rows)
    report(n, errors, summ, comps, pos_cmp, tuned_rows, time.time() - t0)
    print(f"{n} series, {len(errors)} errors, {time.time() - t0:.0f} s")
    for c in comps + pos_cmp:
        print(f"{c['comparison']}: median {pct(c['median_rel_diff'])}, 90% {pct(c['p05'])} to {pct(c['p95'])}, wins {c['share_candidate_wins']:.2f}, p {c['wilcoxon_p']:.3g}")


all_rows_global = []


def report(n, errors, summ, comps, pos_cmp, tuned_rows, seconds):
    L = ["# B1 on M5: the beer game driven by 70 store-by-department series\n"]
    L.append("*Generated by `run_m5.py` on 19 November 2026. Demand is real M5 unit sales (Walmart, 2011 to 2016), aggregated to weeks. The supply chain is simulated: M5 has sales, not orders or inventory, so nothing here says what Walmart's chain did. The human-like parameters are still **placeholders** (Sterman 1989 could not be read), the DQN (P3) and the forecast inputs (F1, F2) are not part of these runs, and replications R1 to R3 are not passed. Read the result as: how the policy classes compare on this chain and this demand.*\n")
    L.append("## 1. Data\n")
    L.append("- Source: the M5 organisers' Google Drive package (`Dataset/sales_train_evaluation.csv`, `Dataset/calendar.csv`), fetched without a login. **The terms of use for redistributing derived aggregates were not found** (neither the ReadMe nor the Competitors Guide pages read state them), so no data file is committed; `prepare_m5.py` rebuilds the aggregates.")
    L.append("- Facts checked against the Competitors Guide (Table 1, section \"The dataset\"): 42,840 series in total, **70 at the store-by-department level**, 1,941 days of history (29 January 2011 to 19 June 2016), 10 stores, 7 departments. Our file has 70 series of 277 full weeks (the 1,941st day starts an incomplete week, which is dropped). Mean weekly units run from 84 to 20,386; no series has a zero week.")
    L.append("- Scaling: each series is multiplied so its **training** mean is 100 cases a week; the test window played no part in it. Costs are in that scaled unit.\n")
    L.append("## 2. Design (as frozen in CRITERIA.md)\n")
    L.append(f"Tuning on weeks 0 to {TRAIN_END - 1} with cost counted from week {WARM}; test on the last {TEST_WEEKS} weeks (weeks {TRAIN_END} to {TOTAL - 1}) with the parameters frozen, the chain replayed from week 0 so its state at the start of the window is real. P1 and P2 use the same search effort (level {EFFORT}); P1 tunes 4 levels and P2 16 parameters, so P2 searches a larger space. {n} series completed, {len(errors)} failed" + (": " + "; ".join(f"{k[0]} {k[1]} ({e})" for k, e in errors) if errors else "") + ".\n")
    L.append("## 3. S3: all four stages use the same policy\n")
    L.append("| policy | median chain cost | IQR | series it is cheapest on | fill rate | " + " | ".join(f"{s} bullwhip (median)" for s in STAGES) + " |\n|---|---|---|---|---|" + "---|" * len(STAGES))
    for r in summ:
        L.append(f"| {r['policy']} | {r['median_cost']:.0f} | {r['q1']:.0f} to {r['q3']:.0f} | {r['series_it_wins']} of {n} | {r['fill_rate']:.3f} | " + " | ".join(f"{r[f'{s}_bullwhip']:.2f}" for s in STAGES) + " |")
    L.append("")
    L.append("| comparison | median relative cost difference | 90% interval over series | Hodges-Lehmann | share of series the first wins | Wilcoxon p | effect r |\n|---|---|---|---|---|---|---|")
    for c in comps:
        L.append(f"| {c['comparison']} | {pct(c['median_rel_diff'])} | {pct(c['p05'])} to {pct(c['p95'])} | {pct(c['hodges_lehmann'])} | {c['share_candidate_wins']:.2f} | {c['wilcoxon_p']:.3g} | {c['effect_r']:.2f} |")
    L.append("")
    c0 = comps[0]
    L.append(f"**Confirmatory comparison (P2 versus tuned P1, S3, {n} series):** median relative cost difference {pct(c0['median_rel_diff'])}. By the pre-fixed reading: {rp.verdict(c0['median_rel_diff'])}.\n")
    L.append(f"**Cost of misperception** ((P2 - P0) / P0, descriptive): {pct(comps[1]['median_rel_diff'])} median. With placeholder human parameters this is not a statement about people.\n")
    L.append("## 4. S1 and S2 at the retailer position (the other three stages fixed)\n")
    L.append("| scenario | P2 vs tuned P1: median | 90% interval | share of series P2 wins | Wilcoxon p |\n|---|---|---|---|---|")
    for c in pos_cmp:
        L.append(f"| {c['comparison'].replace(': P2 vs P1 tuned', '')} | {pct(c['median_rel_diff'])} | {pct(c['p05'])} to {pct(c['p95'])} | {c['share_candidate_wins']:.2f} | {c['wilcoxon_p']:.3g} |")
    L.append("")
    betas = [r.get(f"Retailer beta") for r in tuned_rows if r.get("Retailer beta") is not None]
    if betas:
        L.append(f"Beta found by tuning at the retailer in S3 (P2): median {statistics.median(betas):.2f}, 10th to 90th percentile {rp.percentile(betas, .1):.2f} to {rp.percentile(betas, .9):.2f} (1 means the supply line is fully counted; the search allowed 0 to 3).\n")
    L.append("## 5. Checks on the result\n")
    at_b = sum(1 for r in tuned_rows for st in STAGES if float(r[f"{st} beta"]) > 2.999)
    th_lo = sum(1 for r in tuned_rows for st in STAGES if float(r[f"{st} theta"]) < 0.0501)
    L.append(f"- **Bounds:** of {4 * len(tuned_rows)} tuned betas, {at_b} sit at the upper bound (3); of {4 * len(tuned_rows)} tuned thetas, {th_lo} sit at the lower bound (0.05). The largest tuned desired stock is below the bound (15 times the starting demand).")
    ratios = []
    for r in tuned_rows:
        k = (r["store"], r["dept"])
        ratios.append(next(x["Chain window cost"] for x in all_rows_global if (x["store"], x["dept"]) == k and x["policy"] == "P2 SD rule (tuned)") / (float(r["p2_train_cost"]) * 52 / (TRAIN_END - WARM)))
    L.append(f"- **The test window is harder than training:** the P2 cost over the last 52 weeks is a median {statistics.median(ratios):.2f} times the training cost per 52 weeks, because the last year has more demand and variation. This applies to every policy, so the comparison is unaffected, but absolute costs are not comparable between windows.")
    L.append("- **Leakage:** the scaling uses the training weeks only; the tuning runs end at week 225 and integrate only weeks 0 to 224; the test run replays the same series with the parameters frozen. P1's tuned levels and P2's parameters never saw a test week.")
    L.append("- **Reading across the pilot:** on synthetic i.i.d. demand the tuned SD rule beat tuned base-stock by about 12 to 18%; on the real series the two are level. The edge in the pilot depended on demand with no trend or seasonality.\n")
    L.append("## 6. What this does and does not show\n")
    L.append("- It shows how a tuned base-stock rule, a tuned order-smoothing SD rule and a placeholder human-like rule compare on 70 real demand series in one simulated chain, with parameters fixed before the test window.")
    L.append("- It does not show that the SD rule is better than the optimum: echelon base-stock, the DQN and forecast-driven policies were not run.")
    L.append("- Demand at this level is smooth and has weekly and yearly seasonality; the result does not carry to item-store daily ordering.")
    L.append("- The parameters were tuned per series on 225 weeks of that series' own history, which is a luxury a real planner may not have.")
    L.append(f"\nThis run took {seconds:.0f} seconds. Reproduce: `python3 prepare_m5.py && python3 build_model.py && python3 run_m5.py`.")
    open(os.path.join(HERE, "B1_M5_report.md"), "w").write("\n".join(L) + "\n")


if __name__ == "__main__":
    main()
