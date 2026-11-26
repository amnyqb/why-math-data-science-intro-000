#!/usr/bin/env python3
"""Score the B2-1 results against answer_keys/b2-1-workforce-inventory.yaml. Settings come from config/b2-1.yaml (frozen)."""
import csv, json, os, sys
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
R = os.path.join(HERE, "results", "b2-1")
cfg = yaml.safe_load(open(os.path.join(HERE, "config", "b2-1.yaml")))
sc = cfg["scoring"]
dom = json.load(open(os.path.join(R, "dominance.json")))
imp = json.load(open(os.path.join(R, "importance.json")))


def ident(spec):
    for l in dom["loops"]:
        v = l["variables"]
        if all(x in v for x in spec.get("contains", [])) and not any(x in v for x in spec.get("excludes", [])):
            return l["id"]
    raise SystemExit(f"loop not found: {spec}")


b1, b2 = ident(cfg["loops"]["B1"]), ident(cfg["loops"]["B2"])
rows = list(csv.reader(open(os.path.join(R, "dominance.csv"))))
hdr, body = rows[0], rows[1:]
T = [float(r[0]) for r in body]
rel = {int(h[4:]): [abs(float(r[i + 1])) for r in body] for i, h in enumerate(hdr[1:])}
run = list(csv.reader(open(os.path.join(R, "base_run.csv"))))
rh = [h.lower() for h in run[0]]
wi = rh.index("workforce")
rt = [float(r[0]) for r in run[1:]]
W = [float(r[wi]) for r in run[1:]]
dt = rt[1] - rt[0]

# base-run facts
rate = [(W[i + 1] - W[i]) / dt for i in range(len(W) - 1)]
extrema = [i + 1 for i in range(1, len(rate)) if rate[i - 1] * rate[i] < 0 and rt[i] > 5]
ext_t = [rt[i] for i in extrema]
amp = [abs(W[i] - W[-1]) for i in extrema]


def dominant(t):
    k = min(range(len(T)), key=lambda j: abs(T[j] - t))
    a, b = rel[b1][k], rel[b2][k]
    return "B1" if a > b else "B2"


def share(pred_times, who):
    xs = [dominant(t) for t in pred_times]
    return sum(1 for x in xs if x == who) / len(xs) if xs else float("nan")


win = sc["turning_window"]
turn_times = [t for te in ext_t for t in [te + d * dt for d in range(-int(win / dt), int(win / dt) + 1)]]
slow_times = []
bounds = [5.0] + ext_t
for a, b in zip(bounds[:-1], bounds[1:]):
    seg = [i for i in range(len(rate)) if a <= rt[i] < b]
    if not seg:
        continue
    i_fast = max(seg, key=lambda i: abs(rate[i]))
    slow_times += [rt[i] for i in seg if rt[i] >= rt[i_fast] and rt[i] < b - win]
seq = [dominant(t) for t in T if t > 5.0]
switches = sum(1 for x, y in zip(seq[:-1], seq[1:]) if x != y)

# eigenvalue elasticities (damping = real part, frequency = imaginary part)
el = {r["param"]: r["elasticities"] for r in imp["eigenvalues"]["elasticity"]}
up = [i for i, e in enumerate(imp["eigenvalues"]["eigenvalues"]) if e["im"] > 0][0]
p1, p2 = cfg["loops"]["B1"]["parameter"], cfg["loops"]["B2"]["parameter"]
damp = {p: abs(el[p][up]["d_re_per_ln"]) for p in el}
freq = {p: abs(el[p][up]["d_im_per_ln"]) for p in el}

import subprocess
FC = os.environ.get("FC", os.path.join(HERE, "..", "..", "target", "release", "dyn"))
full = json.loads(subprocess.run([FC, "dominance", os.path.join(HERE, "models", "b2-1-workforce-inventory.sdm")], capture_output=True, text=True).stdout)
ed = [l for l in full["loops"] if "Expected Demand" in l["variables"]]
others = [l for l in full["loops"] if "Expected Demand" not in l["variables"]]
k2 = len(ed) == 1 and set(ed[0]["variables"]) <= {"Expected Demand", "Change in Expected Demand"} and len(others) == 2
out = [
    ("K1 damped oscillation, only balancing loops", len(extrema) >= 3 and all(a > b for a, b in zip(amp[:-1], amp[1:])) and all(l["polarity"] == "B" for l in dom["loops"]),
     f"{len(extrema)} extrema after the step, amplitudes {[round(a, 2) for a in amp[:4]]}…; polarities {[l['polarity'] for l in dom['loops']]}"),
    ("K2 Expected Demand outside the loops of the two-stock core", k2, f"three-stock model: {len(full['loops'])} loops; the Expected Demand loop has variables {ed[0]['variables'] if ed else None}, the other {len(others)} loops contain none of them"),
    ("K3 B2 dominates the turning points of Workforce", share(turn_times, "B2") >= 0.5, f"B2 dominant in {share(turn_times, 'B2'):.0%} of the time within ±{win} months of the {len(ext_t)} extrema"),
    ("K4 B1 dominates the slowing-down phases", share(slow_times, "B1") >= 0.5, f"B1 dominant in {share(slow_times, 'B1'):.0%} of the slowing-down intervals ({len(slow_times)} steps)"),
    ("K5 dominance alternates between B1 and B2", switches >= sc["min_switches"], f"{switches} switches after the step (need {sc['min_switches']})"),
    ("K6 damping explained by B1 only", damp[p1] > 0 and damp[p2] < sc["damping_ratio"] * damp[p1], f"|d Re λ / d ln p|: {p1} {damp[p1]:.4f}, {p2} {damp[p2]:.4f}"),
    ("K7 frequency affected by both, most by B2", freq[p1] > 0 and freq[p2] > freq[p1], f"|d Im λ / d ln p|: {p1} {freq[p1]:.4f}, {p2} {freq[p2]:.4f}"),
]
w = csv.writer(open(os.path.join(R, "score.csv"), "w"))
w.writerow(["finding", "reproduced", "evidence"])
for r in out:
    w.writerow(r)
    print(("PASS " if r[1] else "FAIL ") + r[0] + " | " + r[2])
print(f"{sum(1 for r in out if r[1])} of {len(out)} findings reproduced")
# internal consistency of the two screening methods (no ranking in the answer key, so no tau against a key)
scr = json.load(open(os.path.join(R, "screen.json")))
from itertools import combinations
for g, s in zip(scr["goals"], imp["sobol"]["goals"]):
    a = {e["param"]: e["mu_star"] for e in g["effects"]}
    b = {i["param"]: i["st"] for i in s["indices"]}
    ks = list(a)
    c = d = 0
    for x, y in combinations(ks, 2):
        da, db = a[x] - a[y], b[x] - b[y]
        if da * db > 0: c += 1
        elif da * db < 0: d += 1
    print(f"goal {g['goal']}: Morris mu* vs Sobol ST pairs agreeing {c}, disagreeing {d}; Morris converged {g['converged']}; Sobol converged {s['converged']}")
