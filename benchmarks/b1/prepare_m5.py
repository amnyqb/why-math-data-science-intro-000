#!/usr/bin/env python3
"""M5 daily unit sales -> weekly demand for the 70 store-by-department series (level 9 of the Guide's Table 1).

Reads data/raw/calendar.csv and data/raw/sales_train_evaluation.csv (not committed: the licence for
redistributing derived aggregates is unchecked) and writes data/m5_store_department_weekly.csv with one
row per series and week. Weeks are the Walmart weeks of the calendar (wm_yr_wk); a week with fewer than
seven days in the history (the last one) is dropped.
"""

import csv
import os
from collections import defaultdict

HERE = os.path.dirname(os.path.abspath(__file__))
RAW = os.path.join(HERE, "data", "raw")
OUT = os.path.join(HERE, "data", "m5_store_department_weekly.csv")


def main():
    week_of_day, order = {}, []
    with open(os.path.join(RAW, "calendar.csv")) as f:
        for k, r in enumerate(csv.DictReader(f), start=1):
            # this copy of calendar.csv has no "d" column: the rows are in date order, so day d_k is row k
            week_of_day[f"d_{k}"] = r["wm_yr_wk"]
            if r["wm_yr_wk"] not in order:
                order.append(r["wm_yr_wk"])
    days_in = defaultdict(int)
    with open(os.path.join(RAW, "sales_train_evaluation.csv")) as f:
        rd = csv.reader(f)
        head = next(rd)
        di = [i for i, h in enumerate(head) if h.startswith("d_")]
        store_i, dept_i = head.index("store_id"), head.index("dept_id")
        dnames = [head[i] for i in di]
        for d in dnames:
            days_in[week_of_day[d]] += 1
        weeks = [w for w in order if days_in.get(w, 0) == 7]
        col_week = [week_of_day[d] for d in dnames]
        totals = defaultdict(lambda: defaultdict(int))
        n = 0
        for row in rd:
            t = totals[(row[store_i], row[dept_i])]
            for i, w in zip(di, col_week):
                v = int(row[i])
                if v:
                    t[w] += v
            n += 1
    with open(OUT, "w", newline="") as f:
        w = csv.writer(f)
        w.writerow(["store_id", "dept_id", "week", "wm_yr_wk", "units"])
        for key in sorted(totals):
            for k, wk in enumerate(weeks):
                w.writerow([key[0], key[1], k, wk, totals[key].get(wk, 0)])
    print(f"{n} item-store rows, {len(totals)} store-department series, {len(weeks)} full weeks (days in history: {len(dnames)}) -> {OUT}")


if __name__ == "__main__":
    main()
