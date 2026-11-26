#!/usr/bin/env python3
"""Draws the B1 pilot charts from results/*.csv (and two runs of the model for the step response).

    uv run --with matplotlib python3 make_charts.py        # writes charts/*.png
"""

import csv
import json
import os
import subprocess

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402

HERE = os.path.dirname(os.path.abspath(__file__))
RES, OUT = os.path.join(HERE, "results"), os.path.join(HERE, "charts")
FC = os.environ.get("FC_BIN", os.path.join(HERE, "..", "..", "target", "debug", "dyn"))
MODEL = os.path.join(HERE, "beer_game_m5.sdm")
STAGES = ["Retailer", "Wholesaler", "Distributor", "Factory"]
COL = {"P0 human-like (placeholder parameters)": "#d9534f", "P1 base-stock (formula levels)": "#9aa5b1", "P1 base-stock (tuned)": "#2f6db5", "P2 SD rule (tuned)": "#1e9e6a"}
SHORT = {"P0 human-like (placeholder parameters)": "P0 human-like\n(placeholder)", "P1 base-stock (formula levels)": "P1 base-stock\n(formula)", "P1 base-stock (tuned)": "P1 base-stock\n(tuned)", "P2 SD rule (tuned)": "P2 SD rule\n(tuned)"}
plt.rcParams.update({"font.size": 11, "axes.spines.top": False, "axes.spines.right": False, "figure.dpi": 130})


def rows(name):
    return list(csv.DictReader(open(os.path.join(RES, name))))


def save(fig, name):
    os.makedirs(OUT, exist_ok=True)
    fig.tight_layout()
    fig.savefig(os.path.join(OUT, name))
    plt.close(fig)


def main():
    s3 = rows("s3_all_stages.csv")
    pols = list(COL)
    cost = {p: [float(r["Chain window cost"]) for r in s3 if r["policy"] == p] for p in pols}

    # 1. cost distribution
    fig, ax = plt.subplots(figsize=(8, 4.6))
    bp = ax.boxplot([cost[p] for p in pols], tick_labels=[SHORT[p] for p in pols], patch_artist=True, showfliers=True, widths=0.55)
    for patch, p in zip(bp["boxes"], pols):
        patch.set_facecolor(COL[p])
        patch.set_alpha(0.75)
    for med in bp["medians"]:
        med.set_color("black")
    ax.set_ylabel("Chain cost over the 52-week window (currency)")
    ax.set_title("Chain cost, 100 demand seeds (S3: all stages use the policy)")
    ax.set_yscale("log")
    save(fig, "cost_s3.png")

    # 2. bullwhip by stage
    fig, ax = plt.subplots(figsize=(8, 4.6))
    w = 0.2
    for i, p in enumerate(pols):
        vals = [sum(float(r[f"{s} bullwhip"]) for r in s3 if r["policy"] == p) / len([r for r in s3 if r["policy"] == p]) for s in STAGES]
        ax.bar([x + (i - 1.5) * w for x in range(4)], vals, w, label=SHORT[p].replace("\n", " "), color=COL[p])
    ax.axhline(1, color="black", lw=0.8, ls="--")
    ax.set_xticks(range(4), STAGES)
    ax.set_ylabel("Variance of orders / variance of demand")
    ax.set_title("Bullwhip by stage: 1 means orders are no more variable than demand")
    ax.set_yscale("log")
    ax.legend(fontsize=9, frameon=False)
    save(fig, "bullwhip_s3.png")

    # 3. paired differences P2 vs tuned P1
    by = {p: {int(r["seed"]): float(r["Chain window cost"]) for r in s3 if r["policy"] == p} for p in pols}
    rel = [100 * (by["P2 SD rule (tuned)"][s] - by["P1 base-stock (tuned)"][s]) / by["P1 base-stock (tuned)"][s] for s in by["P1 base-stock (tuned)"]]
    fig, ax = plt.subplots(figsize=(8, 4.2))
    ax.hist(rel, bins=24, color="#1e9e6a", alpha=0.8)
    ax.axvline(0, color="black", lw=1)
    med = sorted(rel)[len(rel) // 2]
    ax.axvline(med, color="#d9534f", lw=2, label=f"median {med:+.1f}%")
    ax.set_xlabel("P2 cost relative to tuned P1, per seed (%; negative = P2 cheaper)")
    ax.set_ylabel("seeds")
    ax.set_title("Paired comparison over 100 seeds: P2 is cheaper on {:.0f}% of them".format(100 * sum(1 for x in rel if x < 0) / len(rel)))
    ax.legend(frameon=False)
    save(fig, "paired_s3.png")

    # 4. S1 and S2 by position
    cmp = rows("s1_s2_comparisons.csv")
    fig, ax = plt.subplots(figsize=(9, 4.6))
    labels, mid, lo, hi, col = [], [], [], [], []
    for r in cmp:
        scen, pos = r["comparison"].split(": ")[0].split(", ")[0][:2], r["comparison"].split(": ")[0].split(", ")[1]
        labels.append(f"{scen} {pos}")
        m, a, b = 100 * float(r["median_rel_diff"]), 100 * float(r["p05"]), 100 * float(r["p95"])
        mid.append(m); lo.append(m - a); hi.append(b - m)
        col.append("#2f6db5" if scen == "S1" else "#d9534f")
    ax.barh(range(len(labels)), mid, color=col, alpha=0.8, xerr=[lo, hi], error_kw={"lw": 1.2, "capsize": 3})
    ax.axvline(0, color="black", lw=1)
    ax.set_yticks(range(len(labels)), labels)
    ax.invert_yaxis()
    ax.set_xlabel("Median cost of P2 relative to tuned P1 (%), bars show the 5th to 95th percentile of seeds")
    ax.set_title("One stage tunes, the others are fixed: S1 others base-stock (blue), S2 others human-like (red)")
    save(fig, "positions_s1_s2.png")

    # 5. the classic step: orders by stage, human-like (placeholder) vs the tuned SD rule
    scen = {s["name"]: s["values"] for s in json.load(open(MODEL))["manifest"]["scenarios"]}
    fig, axes = plt.subplots(1, 2, figsize=(11, 4.2), sharey=True)
    for ax, name in zip(axes, ["P0 human-like (placeholder parameters)", "P2 SD rule (tuned)"]):
        args = [FC, "run", MODEL, "--every", "1", "--series", ",".join(f"{s} orders placed" for s in STAGES)]
        for k, v in scen[name].items():
            args += ["--set", f"{k}={v}"]
        d = json.loads(subprocess.run(args, capture_output=True, text=True).stdout)
        for s, c in zip(STAGES, ["#2f6db5", "#1e9e6a", "#e08a1e", "#d9534f"]):
            ax.plot(d["times"], d["series"][f"{s.lower()} orders placed"], label=s, color=c, lw=2)
        ax.set_title(name.replace(" (placeholder parameters)", " (placeholder)"))
        ax.set_xlabel("week")
    axes[0].set_ylabel("orders placed (cases/week)")
    axes[0].legend(frameon=False)
    fig.suptitle("Classic step, demand 4 to 8 at week 5")
    save(fig, "step_orders.png")
    m5_charts()
    print("wrote", OUT)


def m5_charts():
    path = os.path.join(RES, "m5_s3_series.csv")
    if not os.path.exists(path):
        return
    rows_ = rows("m5_s3_series.csv")
    pols = list(COL)
    cost = {p: {(r["store"], r["dept"]): float(r["Chain window cost"]) for r in rows_ if r["policy"] == p} for p in pols}
    ks = sorted(cost[pols[0]])
    fig, ax = plt.subplots(figsize=(8, 4.6))
    bp = ax.boxplot([[cost[p][k] for k in ks] for p in pols], tick_labels=[SHORT[p] for p in pols], patch_artist=True, widths=0.55)
    for patch, p in zip(bp["boxes"], pols):
        patch.set_facecolor(COL[p])
        patch.set_alpha(0.75)
    for med in bp["medians"]:
        med.set_color("black")
    ax.set_yscale("log")
    ax.set_ylabel("Chain cost, last 52 weeks (scaled currency)")
    ax.set_title("M5: cost over 70 store-department series (S3)")
    save(fig, "m5_cost_s3.png")
    rel = sorted(100 * (cost["P2 SD rule (tuned)"][k] - cost["P1 base-stock (tuned)"][k]) / cost["P1 base-stock (tuned)"][k] for k in ks)
    fig, ax = plt.subplots(figsize=(9, 4.2))
    ax.bar(range(len(rel)), rel, color=["#1e9e6a" if x < 0 else "#2f6db5" for x in rel])
    ax.axhline(0, color="black", lw=1)
    ax.axhline(rel[len(rel) // 2], color="#d9534f", lw=1.5, ls="--", label=f"median {rel[len(rel) // 2]:+.1f}%")
    ax.set_xlabel("series, sorted")
    ax.set_ylabel("P2 cost vs tuned P1 (%)")
    ax.set_title("M5: P2 against tuned P1, one bar per series (green: P2 cheaper)")
    ax.legend(frameon=False)
    save(fig, "m5_paired.png")
    fig, ax = plt.subplots(figsize=(8, 4.6))
    w = 0.2
    for i, p in enumerate(pols):
        vals = []
        for s in STAGES:
            xs = sorted(float(r[f"{s} bullwhip"]) for r in rows_ if r["policy"] == p)
            vals.append(xs[len(xs) // 2])
        ax.bar([x + (i - 1.5) * w for x in range(4)], vals, w, label=SHORT[p].replace("\n", " "), color=COL[p])
    ax.axhline(1, color="black", lw=0.8, ls="--")
    ax.set_xticks(range(4), STAGES)
    ax.set_yscale("log")
    ax.set_ylabel("Median bullwhip")
    ax.set_title("M5: bullwhip by stage (median over 70 series)")
    ax.legend(fontsize=9, frameon=False)
    save(fig, "m5_bullwhip.png")
    key = ("CA_3", "FOODS_3")
    tst = os.path.join(HERE, "build", "m5_test_" + "_".join(key) + ".sdm")
    tuned = {(r["store"], r["dept"]): r for r in rows("m5_tuned_parameters.csv")}[key]
    if os.path.exists(tst):
        fig, axes = plt.subplots(1, 3, figsize=(13, 3.8), sharey=True)
        p2 = {k: v for k, v in tuned.items() if k.split(" ")[0] in STAGES and any(k.endswith(x) for x in ("theta", "alpha", "beta", "S prime"))}
        p1 = {**{f"{s} policy": 1 for s in STAGES}, **{f"{s} base stock level": tuned[f"{s} base stock level"] for s in STAGES}}
        for ax, (name, ov) in zip(axes, [("P0 human-like (placeholder)", {}), ("P1 base-stock (tuned)", p1), ("P2 SD rule (tuned)", p2)]):
            args = [FC, "run", tst, "--every", "1", "--series", "Customer demand," + ",".join(f"{s} orders placed" for s in STAGES)]
            for k, v in ov.items():
                args += ["--set", f"{k}={v}"]
            d = json.loads(subprocess.run(args, capture_output=True, text=True).stdout)
            t = d["times"]
            i0 = t.index(225.0)
            ax.plot(t[i0:], d["series"]["customer demand"][i0:], color="black", lw=1.5, label="customer demand")
            for s, col in zip(STAGES, ["#2f6db5", "#1e9e6a", "#e08a1e", "#d9534f"]):
                ax.plot(t[i0:], d["series"][f"{s.lower()} orders placed"][i0:], color=col, lw=1.4, label=s)
            ax.set_title(name)
            ax.set_xlabel("week")
        axes[0].set_ylabel("cases per week (scaled)")
        axes[0].legend(frameon=False, fontsize=8)
        fig.suptitle("M5 series CA_3 FOODS_3: the 52-week test window")
        save(fig, "m5_one_series.png")


if __name__ == "__main__":
    main()
