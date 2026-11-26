#!/usr/bin/env python3
"""R1: replicate Sterman (1989 working paper D-3919, "Misperceptions of Feedback in Dynamic Decision Making").

Board (Figure 2, p. 36): order delay 1 week, shipping delay 2 weeks, factory production delay 3 weeks,
12 cases on hand and 4 per week in every pipe at the start, customer demand 4 -> 8 in week 5, 36 weeks.
Rule (eq. 17 as in the text): O = MAX(0, Le + alpha*(S' - S - beta*SL)), adaptive expectation with theta.
Each of the 11 teams is simulated with its own estimated parameters (Table 4); N/D entries are
alpha = 0 positions, so beta and S' do not matter there.
Targets: Table 2 (team cost mean 2028, benchmark 204) and Table 3 (peaks, variance order, phase lag).
"""
import csv, json, os, statistics, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC_BIN", os.path.join(HERE, "..", "..", "target", "debug", "dyn"))
MODEL = os.path.join(HERE, "build", "r1.sdm")
ST = ["Retailer", "Wholesaler", "Distributor", "Factory"]
# Table 4: (theta, alpha, beta, S') for R, W, D, F; N/D -> (beta 0, S' 0), alpha is 0 there
T4 = {
 "Bassbeer": [(.90,.10,.65,20),(0,.25,.50,27),(.15,.05,.35,14),(1.0,.65,.40,15)],
 "Budweiser": [(0,.40,.10,7),(0,.40,.75,30),(0,.30,.10,10),(.25,.25,.10,9)],
 "Coors": [(0,.20,0,25),(0,.15,.50,38),(.90,.30,.20,10),(.25,.30,0,18)],
 "Freebeer": [(.40,.35,.45,15),(.30,.05,0,30),(.05,.35,1.00,18),(.25,.25,0,19)],
 "Grin & Beer It": [(.10,.35,.65,13),(.95,.15,.55,14),(.20,.20,.30,19),(.25,.35,.55,24)],
 "Grizzly": [(.05,.30,.65,31),(.30,.20,.35,27),(.15,.05,.25,15),(.55,.65,0,9)],
 "Heineken1": [(.95,.15,0,9),(.50,0,0,0),(.20,.30,.05,8),(.80,0,0,0)],
 "Heineken2": [(.50,.05,.60,6),(.40,.10,.30,16),(1.0,.15,.80,14),(.55,.80,0,9)],
 "Heineken3": [(.05,.30,.45,5),(.20,0,0,0),(.30,.10,.90,12),(0,.30,.15,17)],
 "Suds": [(1.0,0,0,0),(.05,.30,.20,20),(.15,.60,.35,0),(.40,.35,1.05,32)],
 "Twoborg": [(.75,.35,0,4),(0,.25,.05,18),(.05,.50,0,15),(.95,.30,.20,26)],
}
# Table 2 / 3 targets
T2 = {"team": 2028, "Retailer": 383, "Wholesaler": 635, "Distributor": 630, "Factory": 380}
BENCH = {"team": 204, "Retailer": 46, "Wholesaler": 50, "Distributor": 54, "Factory": 54}
T3_PEAK = {"Retailer": 15, "Wholesaler": 19, "Distributor": 27, "Factory": 32}

def run_full(sets, series):
    a = [FC, "run", MODEL]
    for k, v in sets.items(): a += ["--set", f"{k}={v}"]
    a += ["--series", ",".join(series)]
    d = json.loads(subprocess.run(a, capture_output=True, text=True, check=True).stdout)
    return {r["name"]: r for r in d["series"]}

def run(sets, series):
    a = [FC, "run", MODEL]
    for k, v in sets.items(): a += ["--set", f"{k}={v}"]
    a += ["--series", ",".join(series)]
    d = json.loads(subprocess.run(a, capture_output=True, text=True, check=True).stdout)
    return {r["name"]: r["last"] for r in d["series"]}

def params(p):
    s = {"Use equilibrium S prime": 0, "Demand regime": 0, "Test start": 0}
    for st, (th, al, be, sp) in zip(ST, p):
        s.update({f"{st} policy": 0, f"{st} theta": th, f"{st} alpha": al, f"{st} beta": be, f"{st} S prime": sp})
    return s

SER = [f"{s} cost" for s in ST] + [f"{s} peak order" for s in ST] + [f"{s} peak time" for s in ST]
def one(p):
    r = run(params(p), SER)
    return {k: r[k.lower()] for k in SER} if all(k.lower() in r for k in SER) else r

T3_VAR = {"Retailer": 13, "Wholesaler": 23, "Distributor": 45, "Factory": 72}
T3_MIN = {"Retailer": -25, "Wholesaler": -46, "Distributor": -45, "Factory": -23}
T3_MAXINV = {"Retailer": 20, "Wholesaler": 41, "Distributor": 49, "Factory": 50}

def shape():
    """Qualitative R1 clauses over the 11 teams, with Table 3 for comparison."""
    ser = [f"{s} order variance" for s in ST] + [f"{s} inventory" for s in ST]
    var = {s: [] for s in ST}; mn = {s: [] for s in ST}; mx = {s: [] for s in ST}
    up_var = up_peak = below0 = 0
    for name, p in T4.items():
        r = run_full(params(p), ser + [f"{s} peak order" for s in ST])
        v = [r[f"{s} order variance".lower()]["last"] for s in ST]
        pk = [r[f"{s} peak order".lower()]["last"] for s in ST]
        for s, x in zip(ST, v): var[s].append(x)
        for s in ST:
            mn[s].append(r[f"{s} inventory".lower()]["min"]); mx[s].append(r[f"{s} inventory".lower()]["max"])
        up_var += all(v[i] < v[i + 1] for i in range(3))
        up_peak += all(pk[i] < pk[i + 1] for i in range(3))
        below0 += all(mn[s][-1] < 0 for s in ST)
    out = {}
    for s in ST:
        out[s] = (statistics.mean(var[s]), statistics.mean(mn[s]), statistics.mean(mx[s]))
    return out, up_var, up_peak, below0

def main():
    rows = []
    opt = [(0,1,1,28),(0,1,1,28),(0,1,1,28),(0,1,1,20)]
    r = one(opt); tot = sum(r[f"{s} cost"] for s in ST)
    print("benchmark (theta 0, alpha 1, beta 1, S' 28/20):", round(tot), {s: round(r[f'{s} cost']) for s in ST}, "paper 204", BENCH)
    rows.append({"trial": "BENCHMARK", **r, "team": tot})
    res = {}
    for name, p in T4.items():
        r = one(p); r["team"] = sum(r[f"{s} cost"] for s in ST); rows.append({"trial": name, **r}); res[name] = r
    mean = {k: statistics.mean(r[k] for r in res.values()) for k in rows[1] if k != "trial"}
    print("mean of 11 teams:", {k: round(v, 1) for k, v in mean.items()})
    print("paper table 2   :", T2, "peaks", T3_PEAK)
    with open(os.path.join(HERE, "results", "r1_sterman_teams.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0])); w.writeheader(); w.writerows(rows)
    for k in ["team"] + ST:
        key = "team" if k == "team" else f"{k} cost"
        print(f"{k:12s} ours {mean[key]:8.0f}  paper {T2[k]:6d}  ratio {mean[key]/T2[k]:.2f}")
main()
sh, up_var, up_peak, below0 = shape()
print("\nQUALITATIVE (11 teams)  variance rises upstream in", up_var, "teams; peak order rises upstream in", up_peak, "; every stage goes into backlog in", below0)
for k, (v, mn, mx) in sh.items():
    print(f"{k:12s} order variance ours {v:6.1f} paper {T3_VAR[k]:3d} | min inventory ours {mn:6.1f} paper {T3_MIN[k]:4d} | max inventory ours {mx:6.1f} paper {T3_MAXINV[k]:3d}")
