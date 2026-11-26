#!/usr/bin/env python3
"""Reference check for bass-diffusion against the closed form of Bass (1969).
   F(t) = (1 - exp(-(p+q)t)) / (1 + (q/p) exp(-(p+q)t));  Adopters(t) = N * F(t).
   Usage: FC_BIN=/path/to/dyn python3 check.py   (default binary: dyn on PATH)"""
import json, math, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("FC_BIN", "dyn")
MODEL = os.path.join(HERE, "bass-diffusion.sdm")
TOL_REL = 1e-3          # relative to adopters, wherever F(t) >= 0.01
TOL_N = 1e-3            # relative to market size, everywhere
def run(*extra):
    out = subprocess.check_output([BIN, "run", MODEL, "--series", "Adopters,Potential Adopters", "--every", "128", *extra])
    return json.loads(out)
def F(t, p, q): 
    e = math.exp(-(p + q) * t); return (1 - e) / (1 + (q / p) * e)
fails = 0
def check(name, ok, detail):
    global fails
    print(("PASS " if ok else "FAIL ") + name + ": " + detail); fails += 0 if ok else 1
for p, q in ((0.02, 0.45), (0.05, 0.2), (0.01, 0.8)):
    r = run("--set", f"Advertising Effect={p}", "--set", f"Word of Mouth Effect={q}")
    N = 500000.0
    worst_rel = worst_n = 0.0
    for t, a, pa in zip(r["times"], r["series"]["adopters"], r["series"]["potential adopters"]):
        f = F(t, p, q)
        worst_n = max(worst_n, abs(a - N * f) / N)
        if f >= 0.01: worst_rel = max(worst_rel, abs(a - N * f) / (N * f))
        assert abs(a + pa - N) < 1e-6 * N, "conservation broken"
    check(f"p={p} q={q} vs closed form", worst_rel <= TOL_REL and worst_n <= TOL_N,
          f"worst relative error {worst_rel:.2e} (F>=0.01, tol {TOL_REL}), worst error/N {worst_n:.2e} (tol {TOL_N}), Adopters+Potential=N held")
    # peak of adoption flow is at t* = ln(q/p)/(p+q) when q>p
    if q > p:
        rr = json.loads(subprocess.check_output([BIN, "run", MODEL, "--series", "Adoption", "--every", "8", "--set", f"Advertising Effect={p}", "--set", f"Word of Mouth Effect={q}"]))
        ad = rr["series"]["adoption"]; tm = rr["times"]
        tp = tm[max(range(len(ad)), key=lambda i: ad[i])]; ts = math.log(q / p) / (p + q)
        check(f"p={p} q={q} peak time", abs(tp - ts) <= 0.02, f"model {tp:.3f} y, theory {ts:.3f} y (tol 0.02 y)")
sys.exit(1 if fails else 0)
