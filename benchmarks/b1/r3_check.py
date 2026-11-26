#!/usr/bin/env python3
"""R3 (version 2): an independent recomputation of the seasonal-naive RMSSE in results/forecast_scores.csv.

It does not import run_pilot, run_forecasts or prepare_m5. It reads the raw daily M5 files, builds weekly
totals itself, applies the Competitors Guide's RMSSE (page 6, including its rule that the denominator
counts only the periods after the first non-zero demand), and compares with F1_RMSSE in the CSV.

    python3 r3_check.py        # prints the comparison, writes results/r3_check.csv

What this checks: the weekly aggregation, the seasonal-naive forecast, the rolling-origin bookkeeping and
the RMSSE formula. What it does not check: F2 (AutoETS), the pinball loss, or the official daily WRMSSE
(that needs sell_prices.csv and the organisers' code, neither of which is available).
"""

import csv
import math
import os
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "data", "raw")
FIRST, STEP, H, SEASON = 225, 4, 4, 52
TOL = 1e-5  # the CSV is rounded to 6 decimals


def weekly():
    day_week = []
    with open(os.path.join(RAW, "calendar.csv")) as f:
        for r in csv.DictReader(f):
            day_week.append(r["wm_yr_wk"])
    out = defaultdict(lambda: defaultdict(int))
    ndays = defaultdict(int)
    with open(os.path.join(RAW, "sales_train_evaluation.csv")) as f:
        rd = csv.reader(f)
        head = next(rd)
        di = [i for i, h in enumerate(head) if h.startswith("d_")]
        si, pi = head.index("store_id"), head.index("dept_id")
        first = True
        for row in rd:
            key = (row[si], row[pi])
            for k, i in enumerate(di):
                w = day_week[k]
                out[key][w] += int(row[i])
                if first:
                    ndays[w] += 1
            first = False
    weeks = sorted({w for w in ndays if ndays[w] == 7})  # complete weeks only, in calendar order
    return {k: [v[w] for w in weeks] for k, v in out.items()}


def rmsse(y, n, origins):
    """Guide formula: sum over forecast errors / h  divided by  sum of squared one-step naive errors / (n-1),
    both over periods from the first non-zero demand; the forecast is the value SEASON weeks earlier."""
    start = next(i for i, v in enumerate(y[:n]) if v != 0)
    hist = y[start:n]
    den = sum((hist[i] - hist[i - 1]) ** 2 for i in range(1, len(hist))) / (len(hist) - 1)
    errs = []
    for t in origins:
        for h in range(H):
            errs.append((y[t + h] - y[t + h - SEASON]) ** 2)
    return math.sqrt(sum(errs) / len(errs) / den)


def main():
    ser = weekly()
    n_weeks = len(next(iter(ser.values())))
    origins = list(range(FIRST, n_weeks - H + 1, STEP))
    ref = {(r["store"], r["dept"]): float(r["F1_RMSSE"]) for r in csv.DictReader(open(os.path.join(HERE, "results", "forecast_scores.csv")))}
    rows, worst, ndiff_start = [], 0.0, 0
    for key in sorted(ser):
        y = ser[key]
        mine = rmsse(y, FIRST, origins)
        # the sibling run's way of scaling (whole history, no first-non-zero rule), to see if the rule matters here
        d = sum((y[i] - y[i - 1]) ** 2 for i in range(1, FIRST)) / (FIRST - 1)
        errs = [(y[t + h] - y[t + h - SEASON]) ** 2 for t in origins for h in range(H)]
        alt = math.sqrt(sum(errs) / len(errs) / d)
        if any(v == 0 for v in y[:1]):
            ndiff_start += 1
        rel = abs(mine - ref[key]) / ref[key]
        worst = max(worst, rel)
        rows.append({"store": key[0], "dept": key[1], "points": len(errs), "independent": round(mine, 6),
                     "csv_F1_RMSSE": ref[key], "rel_diff": rel, "no_first_nonzero_rule": round(alt, 6)})
    with open(os.path.join(HERE, "results", "r3_check.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)
    print(f"series {len(rows)}, weeks {n_weeks}, origins {len(origins)}, forecast points per series {rows[0]['points']}")
    print(f"largest relative difference to F1_RMSSE in forecast_scores.csv: {worst:.2e} (tolerance {TOL:.0e})")
    print(f"series whose first week has zero demand: {ndiff_start}")
    print("R3 (RMSSE recomputation):", "PASS" if worst <= TOL else "FAIL")


if __name__ == "__main__":
    main()
