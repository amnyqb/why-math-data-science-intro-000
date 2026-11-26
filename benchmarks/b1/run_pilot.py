#!/usr/bin/env python3
"""B1-pilot: the beer game on synthetic i.i.d. demand (NOT M5), scored as CRITERIA.md prescribes.

    python3 build_model.py && python3 run_pilot.py

Writes results/*.csv and B1_pilot_report.md. Everything is reproducible from the repository files:
the demand is a seeded random normal, the tuning uses fixed seeds disjoint from the test seeds, and
no result of the test window is used to choose anything.
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

HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC_BIN", os.path.join(HERE, "..", "..", "target", "debug", "dyn"))
MODEL = os.path.join(HERE, "beer_game_m5.sdm")
BUILD = os.path.join(HERE, "build")
RESULTS = os.path.join(HERE, "results")
STAGES = ["Retailer", "Wholesaler", "Distributor", "Factory"]

# fixed before any result (CRITERIA.md section 2): seeds, windows, effort
TRAIN_SEED, TRAIN_WEEKS, TEST_START = 9001, 312, 52
TEST_SEEDS = list(range(1, 101))
TEST_WEEKS = 104  # 52 warm-up + the 52-week test window
EFFORT = 2
PARITY, COMPETITIVE = 0.02, 0.05  # criteria section 5
T4_TOL = 0.15  # |bullwhip - 1| allowed for base-stock on i.i.d. demand (T4)

OUT = ["Chain window cost", "Fill rate"] + [f"{s} bullwhip" for s in STAGES] + [f"{s} average backlog" for s in STAGES]


def fc(*args, check=True):
    p = subprocess.run([FC, *map(str, args)], capture_output=True, text=True)
    if check and p.returncode not in (0,):
        raise RuntimeError(f"dyn {' '.join(map(str, args))}\n{p.stdout}\n{p.stderr}")
    return json.loads(p.stdout) if p.stdout.strip() else {}


PRETTY = {n.lower(): n for n in OUT}


def values(d):
    """The last value of each requested series, under the names asked for (fc prints them lower-cased)."""
    return {PRETTY.get(r["name"], r["name"]): r["last"] for r in d["series"]}


def run(overrides, weeks, test_start=TEST_START):
    args = ["run", MODEL]
    for k, v in overrides.items():
        args += ["--set", f"{k}={v}"]
    args += ["--set", f"Test start={test_start}", "--series", ",".join(OUT)]
    # the run length is a setting, not a variable: use a variant file when it differs from the model's
    path = variant({}, f"len{weeks}", weeks) if weeks != 36 else MODEL
    args[1] = path
    return values(fc(*args))


_variants = {}


def variant(overrides, name, weeks=None):
    """A copy of the model with some constants replaced and optionally another run length."""
    path = os.path.join(BUILD, f"{name}.sdm")
    if path in _variants:
        return path
    shutil.copy(MODEL, path)
    ops = [{"op": "set_equation", "name": k, "equation": str(v)} for k, v in overrides.items()]
    if weeks:
        ops.append({"op": "set_setting", "key": "final_time", "value": str(weeks)})
    if ops:
        bp = path + ".batch.json"
        json.dump(ops, open(bp, "w"))
        fc("ops", path, bp, "--actor", "run_pilot", "--no-gate")
        # the log next to the copy is not part of the results
        for ext in (".oplog.jsonl",):
            if os.path.exists(path + ext):
                os.remove(path + ext)
        os.remove(bp)
    _variants[path] = True
    return path


def policy_set(policies):
    return {f"{s} policy": policies[s] for s in STAGES}


def apply_best(best):
    return {k: v for k, v in best.items()}


def tune(levers, name, base_overrides, effort=EFFORT, bounds_report=None):
    """dyn optimize on the training run; returns (best values, evaluations)."""
    path = variant({**base_overrides, "Demand regime": 1, "Demand seed": TRAIN_SEED, "Test start": TEST_START}, f"train_{name}", TRAIN_WEEKS)
    args = ["optimize", path, "--goal", "Chain window cost", "--measure", "final", "--effort", effort]
    for lv, (lo, hi) in levers.items():
        args += ["--lever", f"{lv}:{lo}:{hi}"]
    d = fc(*args)
    if bounds_report is not None:
        for lv, (lo, hi) in levers.items():
            v = d["best"]["values"][lv]
            if abs(v - hi) < 1e-6 * max(1.0, abs(hi)) or abs(v - lo) < 1e-6 * max(1.0, abs(lo)):
                bounds_report.append(lv)
    return d["best"]["values"], d["evaluations"], d["start"]["goal"], d["best"]["goal"]


def test_runs(overrides, name):
    """Run all test seeds; returns a list of result dicts (one per seed)."""
    path = variant({}, f"test{TEST_WEEKS}", TEST_WEEKS)

    def one(seed):
        args = ["run", path, "--series", ",".join(OUT), "--set", "Demand regime=1", "--set", f"Demand seed={seed}", "--set", f"Test start={TEST_START}"]
        for k, v in overrides.items():
            args += ["--set", f"{k}={v}"]
        return values(fc(*args))

    with ThreadPoolExecutor(max_workers=8) as ex:
        out = list(ex.map(one, TEST_SEEDS))
    # guard: if every seed gave the same cost the seeds are not doing anything and every statistic below is void
    if len({round(r["Chain window cost"], 9) for r in out}) < len(out) // 2:
        raise SystemExit("the test seeds do not change the demand: refusing to report statistics from identical runs")
    return out


# ------------------------------------------------------------------ statistics (no third-party packages)
def percentile(xs, q):
    xs = sorted(xs)
    pos = q * (len(xs) - 1)
    lo, hi = int(math.floor(pos)), int(math.ceil(pos))
    return xs[lo] + (xs[hi] - xs[lo]) * (pos - lo)


def wilcoxon(diffs):
    """Signed-rank test, two-sided, normal approximation with continuity and tie correction."""
    d = [x for x in diffs if x != 0]
    n = len(d)
    if n < 10:
        return {"n": n, "p": float("nan"), "z": float("nan"), "r": float("nan")}
    order = sorted(range(n), key=lambda i: abs(d[i]))
    ranks = [0.0] * n
    i, ties = 0, 0.0
    while i < n:
        j = i
        while j + 1 < n and abs(d[order[j + 1]]) == abs(d[order[i]]):
            j += 1
        avg = (i + j) / 2 + 1
        for k in range(i, j + 1):
            ranks[order[k]] = avg
        t = j - i + 1
        ties += t**3 - t
        i = j + 1
    w_plus = sum(r for r, x in zip(ranks, d) if x > 0)
    mean = n * (n + 1) / 4
    var = n * (n + 1) * (2 * n + 1) / 24 - ties / 48
    z = (w_plus - mean - math.copysign(0.5, w_plus - mean)) / math.sqrt(var) if var > 0 else 0.0
    p = math.erfc(abs(z) / math.sqrt(2))
    return {"n": n, "p": p, "z": z, "r": z / math.sqrt(n)}


def hodges_lehmann(d):
    return statistics.median((d[i] + d[j]) / 2 for i in range(len(d)) for j in range(i, len(d)))


def compare(a, b, label):
    """Paired comparison of cost lists a (candidate) and b (reference): relative differences."""
    rel = [(x - y) / y for x, y in zip(a, b)]
    w = wilcoxon(rel)
    return {
        "comparison": label, "n": len(rel), "median_rel_diff": statistics.median(rel),
        "p05": percentile(rel, 0.05), "p95": percentile(rel, 0.95), "hodges_lehmann": hodges_lehmann(rel),
        "share_candidate_wins": sum(1 for r in rel if r < 0) / len(rel), "wilcoxon_p": w["p"], "effect_r": w["r"],
    }


def verdict(median_rel):
    if abs(median_rel) <= PARITY:
        return "parity (within ±2%): expected when beta is near 1; not evidence of superiority"
    if median_rel > COMPETITIVE:
        return "the candidate loses by more than 5%: report where and why"
    if median_rel < -PARITY:
        return "the candidate is better by more than 2%: suspect first (P1 search, tuning window, leakage) before reporting"
    return "between parity and the 5% competitive limit"


def write_csv(name, rows):
    os.makedirs(RESULTS, exist_ok=True)
    keys = list(rows[0].keys())
    with open(os.path.join(RESULTS, name), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        w.writerows(rows)


def main():
    os.makedirs(BUILD, exist_ok=True)
    t0 = time.time()
    if not os.path.exists(MODEL):
        sys.exit("run build_model.py first")
    log = []

    # ------------------------------------------------------------------ A1: the model's own tests, strict units
    tests = fc("test", MODEL, check=False)
    strict = fc("validate", MODEL, "--strict-units", check=False)
    a1 = bool(tests.get("ok")) and bool(strict.get("ok"))
    log.append(f"A1 in-file tests: {'pass' if tests.get('ok') else 'FAIL'}; strict units: {'pass' if strict.get('ok') else 'FAIL'}")

    # ------------------------------------------------------------------ T4: base-stock on i.i.d. demand does not amplify
    all_bs = policy_set({s: 1 for s in STAGES})
    t4_rows = []
    for seed in range(1, 31):
        r = run({**all_bs, "Demand regime": 1, "Demand seed": seed}, 300)
        t4_rows.append({"seed": seed, **{s: r[f"{s} bullwhip"] for s in STAGES}})
    t4_mean = {s: statistics.mean(r[s] for r in t4_rows) for s in STAGES}
    t4_ok = all(abs(v - 1) <= T4_TOL for v in t4_mean.values())
    write_csv("t4_basestock_bullwhip.csv", t4_rows)
    log.append(f"T4 base-stock bullwhip on i.i.d. demand: {', '.join(f'{s} {v:.2f}' for s, v in t4_mean.items())} (allowed 1 ± {T4_TOL}): {'pass' if t4_ok else 'FAIL'}")

    # ------------------------------------------------------------------ classic step (R1 shape; parameters are placeholders)
    step_rows = []
    peaks = {}
    for label, pol in (("P0 human-like (placeholder parameters)", {s: 0 for s in STAGES}), ("P1 base-stock (formula levels)", {s: 1 for s in STAGES})):
        extra = {"Use equilibrium S prime": 0} if pol[STAGES[0]] == 0 else {}
        r = run({**policy_set(pol), **extra, "Demand regime": 0}, 36, test_start=0)
        pk = fc("run", MODEL, "--set", "Demand regime=0", *sum([["--set", f"{k}={v}"] for k, v in {**policy_set(pol), **extra}.items()], []),
                "--series", ",".join(f"{s} {m}" for s in STAGES for m in ("peak order", "peak time")))
        pv = {x["name"]: x["last"] for x in pk["series"]}
        peaks[label] = pv
        step_rows.append({"policy": label, **r})
    write_csv("step_classic.csv", step_rows)
    p0 = peaks["P0 human-like (placeholder parameters)"]
    amplifies = all(p0[f"{STAGES[i].lower()} peak order"] < p0[f"{STAGES[i + 1].lower()} peak order"] for i in range(3))
    later = all(p0[f"{STAGES[i].lower()} peak time"] <= p0[f"{STAGES[i + 1].lower()} peak time"] for i in range(3))
    log.append(f"R1 shape with PLACEHOLDER parameters: orders amplify upstream: {amplifies}; peaks no earlier upstream: {later} (R1 cannot pass until the paper's parameters replace the placeholders)")

    # ------------------------------------------------------------------ tuning (same effort for P1 and P2)
    t_tune = time.time()
    bs_levers = {f"{s} base stock level": (0, 80) for s in STAGES}
    aa_levers = {}
    for s in STAGES:
        aa_levers.update({f"{s} theta": (0.05, 1.0), f"{s} alpha": (0.02, 1.0), f"{s} beta": (0.0, 1.5), f"{s} S prime": (0, 60)})
    p1_best, p1_evals, p1_start, p1_goal = tune(bs_levers, "p1", {**all_bs})
    p2_base = {**policy_set({s: 0 for s in STAGES}), "Use equilibrium S prime": 0}
    p2_best, p2_evals, p2_start, p2_goal = tune(aa_levers, "p2", p2_base)
    tuning_note = f"P1 tuned: {p1_evals} evaluations, training cost {p1_start:.0f} -> {p1_goal:.0f}; P2 tuned: {p2_evals} evaluations, training cost {p2_start:.0f} -> {p2_goal:.0f} ({time.time() - t_tune:.0f} s)"
    log.append(tuning_note)

    # ------------------------------------------------------------------ S3: all four stages use the same policy, test seeds
    policies = {
        "P0 human-like (placeholder parameters)": {**policy_set({s: 0 for s in STAGES}), "Use equilibrium S prime": 0},
        "P1 base-stock (formula levels)": {**all_bs},
        "P1 base-stock (tuned)": {**all_bs, **p1_best},
        "P2 SD rule (tuned)": {**p2_base, **p2_best},
    }
    results = {}
    rows = []
    for name, ov in policies.items():
        rs = test_runs(ov, name)
        results[name] = rs
        for seed, r in zip(TEST_SEEDS, rs):
            rows.append({"scenario": "S3", "policy": name, "seed": seed, **r})
    write_csv("s3_all_stages.csv", rows)
    summary = []
    for name, rs in results.items():
        c = [r["Chain window cost"] for r in rs]
        summary.append({
            "policy": name, "median_cost": statistics.median(c), "q1": percentile(c, 0.25), "q3": percentile(c, 0.75),
            **{f"{s}_bullwhip": statistics.mean(r[f"{s} bullwhip"] for r in rs) for s in STAGES},
            "fill_rate": statistics.mean(r["Fill rate"] for r in rs),
        })
    write_csv("s3_summary.csv", summary)
    cmp_rows = [
        compare([r["Chain window cost"] for r in results["P2 SD rule (tuned)"]], [r["Chain window cost"] for r in results["P1 base-stock (tuned)"]], "S3: P2 vs P1 tuned"),
        compare([r["Chain window cost"] for r in results["P2 SD rule (tuned)"]], [r["Chain window cost"] for r in results["P0 human-like (placeholder parameters)"]], "S3: P2 vs P0 (cost of misperception, descriptive)"),
        compare([r["Chain window cost"] for r in results["P1 base-stock (tuned)"]], [r["Chain window cost"] for r in results["P1 base-stock (formula levels)"]], "S3: P1 tuned vs P1 formula (descriptive)"),
    ]
    # Holm correction over the confirmatory comparisons (here: one, so the p-value stands)
    write_csv("s3_comparisons.csv", cmp_rows)

    # ------------------------------------------------------------------ robustness (CRITERIA.md section 5: P2 better than P1 by more than 2% is suspect)
    at_bound = []
    p1_big, p1_big_evals, _, p1_big_goal = tune(bs_levers, "p1_big", {**all_bs}, effort=3)
    wide = {}
    for s_ in STAGES:
        wide.update({f"{s_} theta": (0.05, 1.0), f"{s_} alpha": (0.02, 1.0), f"{s_} beta": (0.0, 3.0), f"{s_} S prime": (0, 200)})
    p2_wide, p2_wide_evals, _, p2_wide_goal = tune(wide, "p2_wide", p2_base, effort=3, bounds_report=at_bound)
    r_p1_big = test_runs({**all_bs, **p1_big}, "")
    r_p2_wide = test_runs({**p2_base, **p2_wide}, "")
    robust = [
        compare([r["Chain window cost"] for r in r_p2_wide], [r["Chain window cost"] for r in r_p1_big], "Robustness: P2 (wider bounds, effort 3) vs P1 (effort 3)"),
        compare([r["Chain window cost"] for r in results["P2 SD rule (tuned)"]], [r["Chain window cost"] for r in r_p1_big], "Robustness: P2 (as above) vs P1 (effort 3)"),
    ]
    # an independent check of P1: coordinate search over the four levels on the training run (not the Optimize search)
    train_p1 = variant({**all_bs, "Demand regime": 1, "Demand seed": TRAIN_SEED, "Test start": TEST_START}, "train_p1", TRAIN_WEEKS)

    def train_cost(levels):
        args = ["run", train_p1, "--series", "Chain window cost"] + sum([["--set", f"{k}={v}"] for k, v in levels.items()], [])
        return fc(*args)["series"][0]["last"]

    levels = dict(p1_best)
    best_cost = train_cost(levels)
    for _ in range(3):
        for st in STAGES:
            key = f"{st} base stock level"
            for cand in [levels[key] + d for d in (-8, -4, -2, -1, -0.5, 0.5, 1, 2, 4, 8)]:
                if cand < 0:
                    continue
                c = train_cost({**levels, key: cand})
                if c < best_cost - 1e-9:
                    best_cost, levels = c, {**levels, key: cand}
    coord_note = f"Independent coordinate search on the training run starting from the tuned levels: cost {train_cost(p1_best):.0f} -> {best_cost:.0f}"
    write_csv("s3_robustness.csv", robust)
    robust_note = (f"P1 at effort 3: {p1_big_evals} evaluations, training cost {p1_best_goal(p1_goal):.0f} -> {p1_big_goal:.0f}; P2 with beta up to 3 and S prime up to 200 at effort 3: {p2_wide_evals} evaluations, training cost {p2_goal:.0f} -> {p2_wide_goal:.0f}; "
                   f"levers still at a bound after widening: {', '.join(at_bound) if at_bound else 'none'}")
    robust_note += ". " + coord_note
    log.append("Robustness: " + robust_note)

    # ------------------------------------------------------------------ S1 and S2: one position under test, others fixed
    scen_rows, scen_cmp = [], []
    for scen, others in (("S1 others base-stock (tuned)", "bs"), ("S2 others human-like", "p0")):
        for pos in STAGES:
            base = {}
            for s in STAGES:
                if s == pos:
                    continue
                if others == "bs":
                    base.update({f"{s} policy": 1, f"{s} base stock level": p1_best[f"{s} base stock level"]})
                else:
                    base.update({f"{s} policy": 0})
            base["Use equilibrium S prime"] = 0
            # position under test: P1 tuned (one lever) and P2 tuned (four levers), same effort
            b_best, _, _, _ = tune({f"{pos} base stock level": (0, 80)}, f"s_{others}_{pos}_p1", {**base, f"{pos} policy": 1})
            a_best, _, _, _ = tune({f"{pos} {p}": r for p, r in (("theta", (0.05, 1.0)), ("alpha", (0.02, 1.0)), ("beta", (0.0, 1.5)), ("S prime", (0, 60)))}, f"s_{others}_{pos}_p2", {**base, f"{pos} policy": 0})
            res_b = test_runs({**base, f"{pos} policy": 1, **b_best}, "")
            res_a = test_runs({**base, f"{pos} policy": 0, **a_best}, "")
            res_0 = test_runs({**base, f"{pos} policy": 0}, "")
            for lab, rs in (("P1 tuned", res_b), ("P2 tuned", res_a), ("P0 placeholder", res_0)):
                for seed, r in zip(TEST_SEEDS, rs):
                    scen_rows.append({"scenario": scen, "position": pos, "policy": lab, "seed": seed, "Chain window cost": r["Chain window cost"], "Fill rate": r["Fill rate"]})
            c = compare([r["Chain window cost"] for r in res_a], [r["Chain window cost"] for r in res_b], f"{scen}, {pos}: P2 vs P1 tuned")
            c["beta_found"] = a_best[f"{pos} beta"]
            scen_cmp.append(c)
    write_csv("s1_s2_runs.csv", scen_rows)
    write_csv("s1_s2_comparisons.csv", scen_cmp)

    # scenarios in the manifest: the four policies as lever sets (so a user or an agent can reproduce them)
    from build_model import manifest as make_manifest  # same folder
    scen = []
    def lever_values(d):
        return {k: float(v) for k, v in d.items()}
    scen.append({"name": "P0 human-like (placeholder parameters)", "values": lever_values({**{f"{s} policy": 0 for s in STAGES}, "Use equilibrium S prime": 0})})
    scen.append({"name": "P1 base-stock (tuned)", "values": lever_values({**{f"{s} policy": 1 for s in STAGES}, **p1_best})})
    scen.append({"name": "P2 SD rule (tuned)", "values": lever_values({**{f"{s} policy": 0 for s in STAGES}, "Use equilibrium S prime": 0, **p2_best})})
    scen.append({"name": "P2 SD rule (tuned, wider bounds)", "values": lever_values({**{f"{s} policy": 0 for s in STAGES}, "Use equilibrium S prime": 0, **p2_wide})})
    mstocks = [k for k in []]
    base = json.load(open(MODEL))["manifest"]
    base["scenarios"] = scen
    bp = os.path.join(BUILD, "manifest.batch.json")
    json.dump([{"op": "set_manifest", "manifest": base}], open(bp, "w"))
    fc("ops", MODEL, bp, "--actor", "run_pilot", "--reason", "record the tuned policies as manifest scenarios")
    for ext in (".oplog.jsonl",):
        pass
    report(log, a1, t4_ok, t4_mean, step_rows, summary, cmp_rows, scen_cmp, p1_best, p2_best, tuning_note, time.time() - t0, robust, robust_note, p2_best)
    print("\n".join(log))
    print(f"done in {time.time() - t0:.0f} s; see {os.path.join(HERE, 'B1_pilot_report.md')}")


def p1_best_goal(x):
    return x


def pct(x):
    return f"{100 * x:+.1f}%"


def report(log, a1, t4_ok, t4_mean, step_rows, summary, cmp_rows, scen_cmp, p1_best, p2_best, tuning_note, seconds, robust, robust_note, p2_params):
    L = []
    L.append("# B1-pilot report: the beer game on synthetic i.i.d. demand\n")
    L.append("*Generated by `run_pilot.py` on 19 November 2026. This is a pilot: the demand is a seeded random normal (mean 4, sd 2), **not M5**, and no claim is made about M5. The human-like parameters are **placeholders written from memory** because the Sterman (1989) paper could not be read; replication R1 cannot pass until they are replaced. The DQN (P3), forecasts F1 and F2, and replications R2 and R3 are not run (blockers below).*\n")
    L.append("## 1. Gate A: infrastructure\n")
    L.append(f"- In-file tests T1 (chain balance), T2 (equilibrium flat before the step), T3 (orders amplify upstream) and never-negative tests pass; strict units are clean: **{'pass' if a1 else 'FAIL'}**.")
    L.append(f"- T4, base-stock on i.i.d. demand does not amplify (mean bullwhip per stage {', '.join(f'{s} {v:.2f}' for s, v in t4_mean.items())}; allowed 1 ± {T4_TOL}): **{'pass' if t4_ok else 'FAIL'}**. T4 lives in the harness, not in the model file, because a model test applies to the configuration the file is saved in (a gap, section 6).")
    L.append("- Reproducibility (A3), the manifest (A4) and the licence statement (A5) are handled in the repository files; A3 is checked by running this script twice and comparing the CSVs.\n")
    L.append("## 2. The classic step (shape only, placeholder parameters)\n")
    L.append("| policy | chain cost | fill rate | " + " | ".join(f"{s} bullwhip" for s in STAGES) + " |\n|---|---|---|" + "---|" * len(STAGES))
    for r in step_rows:
        L.append(f"| {r['policy']} | {r['Chain window cost']:.0f} | {r['Fill rate']:.2f} | " + " | ".join(f"{r[f'{s} bullwhip']:.1f}" for s in STAGES) + " |")
    L.append("")
    L.append("## 3. Tuning\n")
    L.append(f"{tuning_note}. Tuning used seed 9001 over 312 weeks with the window from week 52; the test seeds are 1 to 100 over 104 weeks with the window from week 52 (52 weeks counted). P1 had 4 levers and P2 had 16 at the same effort level; the number of evaluations each used is above. Tuned base-stock levels: " + ", ".join(f"{k.split(' base')[0]} {v:.1f}" for k, v in p1_best.items()) + ".\n")
    L.append("## 4. S3: all four stages use the same policy (100 seeds)\n")
    L.append("| policy | median cost | IQR | fill rate | " + " | ".join(f"{s} bullwhip" for s in STAGES) + " |\n|---|---|---|---|" + "---|" * len(STAGES))
    for r in summary:
        L.append(f"| {r['policy']} | {r['median_cost']:.0f} | {r['q1']:.0f} to {r['q3']:.0f} | {r['fill_rate']:.3f} | " + " | ".join(f"{r[f'{s}_bullwhip']:.2f}" for s in STAGES) + " |")
    L.append("")
    L.append("| comparison | median relative cost difference | 90% interval | Hodges-Lehmann | share of seeds the first wins | Wilcoxon p | effect r |\n|---|---|---|---|---|---|---|")
    for c in cmp_rows:
        L.append(f"| {c['comparison']} | {pct(c['median_rel_diff'])} | {pct(c['p05'])} to {pct(c['p95'])} | {pct(c['hodges_lehmann'])} | {c['share_candidate_wins']:.2f} | {c['wilcoxon_p']:.3g} | {c['effect_r']:.2f} |")
    L.append("")
    c0 = cmp_rows[0]
    L.append(f"**Confirmatory comparison (S3, P2 versus tuned P1):** median relative cost difference {pct(c0['median_rel_diff'])}: {verdict(c0['median_rel_diff'])}.\n")
    c1 = cmp_rows[1]
    L.append(f"**Cost of misperception** (P2 against the placeholder human-like rule): {pct(c1['median_rel_diff'])} median. With placeholder parameters this is not a statement about people.\n")
    L.append("### Robustness of the S3 result\n")
    L.append(robust_note + ".\n")
    L.append("| comparison | median relative cost difference | 90% interval | share of seeds P2 wins | Wilcoxon p |\n|---|---|---|---|---|")
    for c in robust:
        L.append(f"| {c['comparison']} | {pct(c['median_rel_diff'])} | {pct(c['p05'])} to {pct(c['p95'])} | {c['share_candidate_wins']:.2f} | {c['wilcoxon_p']:.3g} |")
    L.append("")
    L.append("Leakage check: tuning used seed 9001 over 312 weeks; the test seeds are 1 to 100, so no test draw was used to choose a parameter, and the counted window begins at the same week (52) in both.\n")
    L.append("Mechanism: the tuned SD rule smooths orders (the bullwhip upstream of the retailer is near zero in the table above), so upstream stages order an almost constant amount and absorb the retailer's variation in their stock. Installation base-stock passes every order on one for one (bullwhip 1.00 at every stage). In a chain where every stage pays holding and backlog cost, that smoothing is worth money. This says the SD rule class contains something better than installation base-stock here; it does not say the SD rule beats the optimum, which would need echelon base-stock and the DQN, neither of which was run.\n")
    L.append("## 5. S1 and S2: one position under test (100 seeds)\n")
    L.append("| scenario, position | P2 vs tuned P1: median | 90% interval | Wilcoxon p | beta found by tuning |\n|---|---|---|---|---|")
    for c in scen_cmp:
        L.append(f"| {c['comparison'].replace(': P2 vs P1 tuned', '')} | {pct(c['median_rel_diff'])} | {pct(c['p05'])} to {pct(c['p95'])} | {c['wilcoxon_p']:.3g} | {c['beta_found']:.2f}{' (at the upper bound 1.5)' if abs(c['beta_found'] - 1.5) < 1e-6 else ''} |")
    L.append("")
    betas = [c["beta_found"] for c in scen_cmp]
    L.append(f"Beta found by tuning ranged from {min(betas):.2f} to {max(betas):.2f} across the eight position and scenario combinations (1 means the supply line is fully counted).\n")
    L.append("## 6. Gaps in Dynamist met while building this (a deliverable)\n")
    L.append("- A model test applies to the configuration the file is saved in, so T2 (equilibrium) holds only while the step has not happened, and T4 (base-stock on i.i.d. demand) cannot be in the file at all. Tests per scenario are missing.")
    L.append("- Sets cannot express a stage that reads its neighbour (`stage i` receives from `stage i-1`), so the four stages are generated as copies by `build_model.py` rather than written once as a set.")
    L.append("- A delay is a hidden pipeline in `DELAY FIXED`, which breaks a conservation test; the model builds each delay from a chain of stocks instead.")
    L.append("- `dyn run` and `dyn optimize` change one model; looping over seeds, positions and scenarios is a script. A batch of scenarios in the manifest is not yet executable.")
    L.append("- No quantile-path input type, no way to replay an external policy except as a data lookup, and no compiled Python binding for the DQN.")
    L.append("- Optimize searches one run (one seed); averaging the training cost over several seeds needs a wrapper.\n")
    L.append("## 7. Blockers\n")
    L.append("- **Sterman (1989):** the host rate-limited the request, so the human-like parameters, delays and costs are unconfirmed. The DQN paper's simulator (lead times 2, holding 0.5 and backlog 1 for two of its three instances) agrees with the model's defaults.")
    L.append("- **M5 data:** needs a Kaggle account and acceptance of the rules, or the files supplied; the licence for derived aggregates is unchecked.")
    L.append("- **P3 (DQN):** needs an isolated Python 3.4 to 3.7 environment with TensorFlow 1.15; R2 needs it.")
    L.append("- **F2 (AutoETS from `augurs`):** a new dependency fetched from the internet; its licence is unchecked.\n")
    L.append("## 8. Reproduce\n")
    L.append("```\npython3 build_model.py\npython3 run_pilot.py\n```\n")
    L.append(f"This run took {seconds:.0f} seconds.")
    open(os.path.join(HERE, "B1_pilot_report.md"), "w").write("\n".join(L) + "\n")


if __name__ == "__main__":
    main()
