#!/usr/bin/env python3
"""Run a Dynamist World3 model and compare it with a PyWorld3 CSV (run_pyworld3.py).
Usage: compare_w3_1974.py MODEL.sdm PYWORLD3.csv [--label TEXT] [--all] [--first N]
Prints a markdown table at 1950/2000/2050/2100 for the five main variables and the largest relative difference over the whole run."""
import argparse, csv, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", ".."))
ap = argparse.ArgumentParser()
ap.add_argument("model"); ap.add_argument("pycsv")
ap.add_argument("--label", default="")
ap.add_argument("--tol", type=float, default=0.01)
ap.add_argument("--vars", default="")
ap.add_argument("--fc", default=os.environ.get("FC"))
ap.add_argument("--keep", default="")
ap.add_argument("--brief", action="store_true")
a = ap.parse_args()
fc = a.fc
if not fc:
    for root in (REPO, os.path.abspath(os.path.join(REPO, "..", "..", ".."))):
        if os.path.exists(os.path.join(root, "target", "release", "dyn")):
            fc = os.path.join(root, "target", "release", "dyn"); break
out = a.keep or tempfile.mktemp(suffix=".csv")
r = subprocess.run([fc, "run", a.model, "--csv", out, "--every", "1"], capture_output=True, text=True)
if r.returncode != 0:
    sys.exit(r.stdout[:2000] + r.stderr[:2000])


def load(p):
    with open(p) as fh:
        rd = csv.reader(fh); head = [h.strip() for h in next(rd)]
        rows = [[float(x) for x in row] for row in rd]
    return head, rows


norm = lambda s: s.lower().replace(" ", "").replace("_", "")
fh, frows = load(out)
ph, prows = load(a.pycsv)
fi = {norm(h): i for i, h in enumerate(fh)}
pi = {norm(h): i for i, h in enumerate(ph)}
ft = {round(r_[0], 6): r_ for r_ in frows}
pt = {round(r_[0], 6): r_ for r_ in prows}
common = sorted(set(ft) & set(pt))
MAIN = [("population", "pop"), ("industrial output", "io"), ("food", "f"), ("persistent pollution index", "ppolx"),
        ("nonrenewable resources fraction remaining", "nrfr")]
YEARS = [1950, 2000, 2050, 2100]


def rel(x, y):
    return abs(x - y) / max(abs(y), 1e-300)


if a.brief:
    worst = 0.0; run_worst = {}
    for label, v in MAIN:
        for y in YEARS:
            worst = max(worst, rel(ft[float(y)][fi[v]], pt[float(y)][pi[v]]))
        run_worst[v] = max(rel(ft[t][fi[v]], pt[t][pi[v]]) for t in common if t >= 1930)
    print(f"| {a.label} | {worst:.3%} | " + " | ".join(f"{run_worst[v]:.3%}" for _, v in MAIN) + " |")
    if not a.keep:
        os.unlink(out)
    sys.exit(0)
print(f"### {a.label}\n")
print(f"steps compared: {len(common)} (Dynamist {len(frows)}, PyWorld3 {len(prows)}); tolerance {a.tol:.0%}\n")
print("| variable | year | PyWorld3 | Dynamist | relative difference |\n|---|---|---|---|---|")
worst_main = 0.0
for label, v in MAIN:
    for y in YEARS:
        pv, fv = pt[float(y)][pi[v]], ft[float(y)][fi[v]]
        d = rel(fv, pv); worst_main = max(worst_main, d)
        print(f"| {label} ({v.upper()}) | {y} | {pv:.6g} | {fv:.6g} | {d:.3%}{'' if d <= a.tol else ' **>tol**'} |")
print(f"\nlargest difference at the 20 checked points: {worst_main:.3%} ({'within' if worst_main <= a.tol else 'EXCEEDS'} tolerance)\n")
print("| variable | max relative difference over 1900-2100 | at year | last year with a difference above tolerance |\n|---|---|---|---|")
for label, v in MAIN:
    best = (0.0, 1900.0)
    last = None
    for t in common:
        d = rel(ft[t][fi[v]], pt[t][pi[v]])
        if d > best[0]:
            best = (d, t)
        if d > a.tol:
            last = t
    print(f"| {label} ({v.upper()}) | {best[0]:.3%} | {best[1]:.1f} | {'never' if last is None else f'{last:.1f}'} |")
if a.vars:
    print("\n| variable | max rel. difference | at year | value at 1900 PyWorld3 | Dynamist |\n|---|---|---|---|---|")
    for v in a.vars.split(","):
        if norm(v) not in fi or norm(v) not in pi:
            print(f"| {v} | missing | | | |"); continue
        best = (0.0, 1900.0)
        for t in common:
            d = rel(ft[t][fi[norm(v)]], pt[t][pi[norm(v)]])
            if d > best[0]:
                best = (d, t)
        print(f"| {v} | {best[0]:.3%} | {best[1]:.1f} | {pt[1900.0][pi[norm(v)]]:.6g} | {ft[1900.0][fi[norm(v)]]:.6g} |")
if not a.keep:
    os.unlink(out)
