#!/usr/bin/env python3
"""Reference check for sir-epidemic against the Kermack-McKendrick (1927) results.
   Final size: S_inf = S0 exp(-R0 (N - S_inf)/N), solved by bisection (independent of dyn).
   Peak: dI/dt = 0 when S = N/R0; Imax = S0 + I0 - (N/R0)(1 + ln(R0 S0/N)).
   Usage: FC_BIN=/path/to/dyn python3 check.py"""
import json, math, os, subprocess, sys
HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("FC_BIN", "dyn")
MODEL = os.path.join(HERE, "sir-epidemic.sdm")
N, I0 = 1_000_000.0, 10.0
S0 = N - I0
fails = 0
def check(name, ok, detail):
    global fails
    print(("PASS " if ok else "FAIL ") + name + ": " + detail); fails += 0 if ok else 1
def sinf(R0):
    lo, hi = 0.0, N / R0 if R0 > 1 else S0          # root below N/R0 when R0 > 1
    g = lambda s: s - S0 * math.exp(-R0 * (N - s) / N)
    if R0 <= 1: lo, hi = 0.0, S0
    for _ in range(200):
        m = (lo + hi) / 2
        if g(lo) * g(m) <= 0: hi = m
        else: lo = m
    return (lo + hi) / 2
for c in (10, 6, 15):                                  # contact rate lever; R0 = c * 0.05 * 5 / 1
    R0 = c * 0.05 * 5
    r = json.loads(subprocess.check_output([BIN, "run", MODEL, "--series", "Susceptible,Infectious,Recovered,Total People,Basic Reproduction Number", "--every", "10", "--set", f"Contact Rate={c}"]))
    s = r["series"]; S, I, R, T = s["susceptible"], s["infectious"], s["recovered"], s["total people"]
    check(f"c={c} R0 reported", abs(s["basic reproduction number"][0] - R0) < 1e-12, f"model {s['basic reproduction number'][0]} theory {R0}")
    check(f"c={c} conservation", max(abs(t - N) for t in T) < 1e-6 * N, f"max |S+I+R-N| = {max(abs(t - N) for t in T):.2e} person")
    Sinf = sinf(R0)
    check(f"c={c} final size", abs(S[-1] - Sinf) <= 5e-4 * N and I[-1] < 1,
          f"model S_end {S[-1]:.0f}, theory S_inf {Sinf:.0f}, diff/N {abs(S[-1]-Sinf)/N:.2e} (tol 5e-4), I_end {I[-1]:.3g}")
    # fine peak search
    rp = json.loads(subprocess.check_output([BIN, "run", MODEL, "--series", "Susceptible,Infectious", "--every", "1", "--set", f"Contact Rate={c}"]))
    Ip = rp["series"]["infectious"]; Sp = rp["series"]["susceptible"]; k = max(range(len(Ip)), key=lambda i: Ip[i])
    Imax = S0 + I0 - (N / R0) * (1 + math.log(R0 * S0 / N))
    check(f"c={c} peak at S=N/R0", abs(Sp[k] - N / R0) <= 1e-3 * N, f"S at peak {Sp[k]:.0f}, N/R0 {N/R0:.0f} (tol 1e-3 N)")
    check(f"c={c} peak size", abs(Ip[k] - Imax) <= 5e-3 * Imax, f"model {Ip[k]:.0f}, theory {Imax:.0f}, rel {abs(Ip[k]-Imax)/Imax:.2e} (tol 5e-3)")
# subcritical: R0 = 0.8, no outbreak
r = json.loads(subprocess.check_output([BIN, "run", MODEL, "--series", "Infectious", "--every", "10", "--set", "Contact Rate=3.2"]))
check("c=3.2 (R0=0.8) dies out", max(r["series"]["infectious"]) <= I0 + 1e-9, f"max Infectious {max(r['series']['infectious']):.2f} <= I0 {I0}")
sys.exit(1 if fails else 0)
