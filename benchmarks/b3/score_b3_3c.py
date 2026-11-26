#!/usr/bin/env python3
"""Score B3-3c against keys/b3-3b/addendum_vermeulen_dejongh_sensitivity.yaml (entries 3, 5) and addendum_thissen1976 (13).
Criteria fixed before the results were opened:
 C1 FIOAC (FIOAC1 or FIOAC2), ICOR1 and ALIC1 are all among the 5 levers with the largest mean total Sobol index over the 4 goals.
 C2 the agriculture, resource and pollution levers (LYF1 ILF LFH ALAI1 LFERTI NRI PPGF1 PPTD1) have a median rank in the lower half (rank above 11.5 of 23).
Local one-at-a-time sensitivities (the papers) differ from global ones over +-30 percent: reported, not corrected."""
import csv, json, os, statistics
HERE = os.path.dirname(os.path.abspath(__file__)); R = os.path.join(HERE, "results", "b3-3c")
imp = json.load(open(os.path.join(R, "importance.json"))); scr = json.load(open(os.path.join(R, "screen.json")))
goals = imp["sobol"]["goals"]
names = [i["param"] for i in goals[0]["indices"]]
mean_st = {n: sum(next(i["st"] for i in g["indices"] if i["param"] == n) for g in goals) / len(goals) for n in names}
rank = {n: r + 1 for r, n in enumerate(sorted(names, key=lambda n: -mean_st[n]))}
top5 = [n for n in names if rank[n] <= 5]
ok1 = {"ICOR1", "ALIC1"} <= set(top5) and bool({"FIOAC1", "FIOAC2"} & set(top5))
low = ["LYF1", "ILF", "LFH", "ALAI1", "LFERTI", "NRI", "PPGF1", "PPTD1"]
med = statistics.median(rank[n] for n in low); ok2 = med > 11.5
rows = [("C1", "FIOAC, ICOR and ALIC are among the five most influential levers", ok1, f"top five by mean ST: {sorted(top5, key=lambda n: rank[n])}; ranks FIOAC1 {rank['FIOAC1']}, FIOAC2 {rank['FIOAC2']}, ICOR1 {rank['ICOR1']}, ALIC1 {rank['ALIC1']}"),
        ("C2", "agriculture, resource and pollution levers are in the lower half", ok2, f"median rank {med} of {len(names)}; ranks " + ", ".join(f"{n} {rank[n]}" for n in low))]
for r in rows: print(("PASS " if r[2] else "FAIL ") + r[0] + " " + r[1] + " | " + r[3])
print("convergence: Morris", [(g["goal"][:18], g["converged"], round(g["kendall_r_vs_2r"], 2)) for g in scr["goals"]], "Sobol", [(g["goal"][:18], g["converged"]) for g in goals])
print("mean ST ranking:", [(n, round(mean_st[n], 3)) for n in sorted(names, key=lambda n: rank[n])][:10])
w = csv.writer(open(os.path.join(R, "score.csv"), "w")); w.writerow(["id", "finding", "reproduced", "evidence"]); [w.writerow(r) for r in rows]
