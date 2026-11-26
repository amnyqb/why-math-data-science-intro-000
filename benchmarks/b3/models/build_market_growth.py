#!/usr/bin/env python3
"""Build the B2-2 case models from the Stella rebuild of Forrester's (1968) Market Growth model found at
https://github.com/CBSDLab/market-growth (solution folder "Solution-Warren Farr"; no licence stated, so the file is kept
under papers/market_growth/, which is git-ignored, and the models built from it go to build/, also git-ignored).

Scenario C of that repository's Scenarios.xlsx (SW2 = 1, capacity fixed) is the damped oscillation of Mojtahedzadeh 2015 Fig. 3;
scenario F (SW1 = SW2 = SW3 = 1) is the growth with cycles of his Fig. 10. Lever ranges and goals are this benchmark's choice.
"""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); B3 = os.path.join(HERE, "..")
sys.path.insert(0, os.path.join(B3, "tests"))
from fixtures import FC

SRC = os.path.join(B3, "papers", "market_growth", "ForresterMarketGrowth_WarrenFarr.stmx")
LEVERS = [("Salesman Constant SCT", 30, 120), ("Salesman Adjustment Time SAT", 10, 40), ("Salesman Salary SS", 1000, 4000),
          ("Revenue to Sales RS", 8, 16), ("Delivery Rate Adjustment Time DRAT", 0.5, 2), ("Production Capacity Receiving Delay PCRD", 6, 24),
          ("Delivery Delay Bias DDB", 0, 0.6), ("Time for Delivery Delay Recognition by Company TDDRC", 2, 8),
          ("Time for Delivery Delay Recognition by Market TDDRM", 3, 12), ("Time for Delivery Delay Traditional TDDT", 6, 24),
          ("Delivery Delay Management Goal DDMG", 1, 4), ("Sales Effectiveness Maximum SEM", 200, 600),
          ("Production Capacity Initial PCI", 6000, 24000)]
GOALS = [("Backlog peak", "Backlog BL7", "peak", "min"), ("Backlog at the end", "Backlog BL7", "final", "min"),
         ("Delivery delay (company) peak", "Delivery Delay Recognized by Company DDRC13", "peak", "min")]
CASES = {"C": {"SW2": 1}, "F": {"SW1": 1, "SW2": 1, "SW3": 1}}


def run(args):
    r = subprocess.run([FC, *args], capture_output=True, text=True)
    return r


if not os.path.exists(SRC):
    sys.exit(f"missing {SRC}: download the model from the repository named above first")
os.makedirs(os.path.join(B3, "build"), exist_ok=True)
for name, sw in CASES.items():
    out = os.path.join(B3, "build", f"mg_{name}.sdm")
    r = run(["convert", SRC, out]); assert r.returncode == 0, r.stdout
    ops = [{"op": "set_equation", "name": k, "equation": str(v)} for k, v in sw.items()]
    ops.append({"op": "set_manifest", "manifest": {
        "title": f"Market Growth (Forrester 1968), scenario {name}",
        "levers": [{"name": n, "min": a, "max": b} for n, a, b in LEVERS],
        "goals": [{"name": g, "output": o, "measure": m, "direction": d} for g, o, m, d in GOALS]}})
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(ops, f)
    r = run(["ops", out, f.name]); os.unlink(f.name)
    print(name, "ok" if '"ok": true' in r.stdout else r.stdout[:600])
