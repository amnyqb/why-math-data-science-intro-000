#!/usr/bin/env python3
"""Reference check for predator-prey.sdm against an independent plain-Python RK4 solution.
   Usage: FC_BIN=/path/to/dyn python3 check.py   (or: python3 check.py /path/to/dyn)"""
import json, math, os, pathlib, subprocess, sys

HERE = pathlib.Path(__file__).resolve().parent
FC = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("FC_BIN") or "dyn"
MODEL = str(HERE / "predator-prey.sdm")
a, b, c, d, K = 0.8, 0.04, 0.6, 0.01, 200.0   # prey growth, attack, predator death, conversion, capacity

def run(sets=(), every=10):
    cmd = [FC, "run", MODEL, "--series", "Prey,Predators", "--every", str(every)]
    for s in sets: cmd += ["--set", s]
    r = json.loads(subprocess.run(cmd, capture_output=True, text=True, check=True).stdout)
    return r["times"], r["series"]["prey"], r["series"]["predators"]

def rk4(x, y, T, dt, logistic=0.0):
    f = lambda x, y: (a * x * (1 - logistic * x / K) - b * x * y, d * x * y - c * y)
    out = [(0.0, x, y)]
    for i in range(1, int(round(T / dt)) + 1):
        k1 = f(x, y); k2 = f(x + dt / 2 * k1[0], y + dt / 2 * k1[1])
        k3 = f(x + dt / 2 * k2[0], y + dt / 2 * k2[1]); k4 = f(x + dt * k3[0], y + dt * k3[1])
        x += dt / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]); y += dt / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1])
        out.append((i * dt, x, y))
    return out

V = lambda x, y: d * x - c * math.log(x) + b * y - a * math.log(y)
def period(t, x, eq):
    ups = [t[i] + (t[i+1] - t[i]) * (eq - x[i]) / (x[i+1] - x[i]) for i in range(len(x) - 1) if x[i] < eq <= x[i+1]]
    return (ups[-1] - ups[0]) / (len(ups) - 1)

fails = []
def expect(name, ok, detail):
    print(("PASS " if ok else "FAIL ") + name + ": " + detail)
    if not ok: fails.append(name)

# 1. Euler (dt 0.001) against RK4 (dt 0.001) over the base run, 60 years
t, x, y = run()
ref = rk4(40.0, 15.0, 60.0, 0.001)[::10]
n = min(len(t), len(ref))
dx = max(abs(x[i] - ref[i][1]) / ref[i][1] for i in range(n))
dy = max(abs(y[i] - ref[i][2]) / ref[i][2] for i in range(n))
expect("trajectory vs RK4", max(dx, dy) < 0.10, f"max relative gap prey {dx:.4f}, predators {dy:.4f} (tolerance 0.10: Euler phase drift over 6 cycles)")

# 2. conserved quantity V
v0 = V(40.0, 15.0)
vs = [V(x[i], y[i]) for i in range(n)]
drift = max(abs(v - v0) for v in vs) / abs(v0)
rkv = max(abs(V(p[1], p[2]) - v0) for p in ref) / abs(v0)
expect("V conserved (RK4 reference)", rkv < 1e-6, f"RK4 relative drift {rkv:.2e} (tolerance 1e-6)")
expect("V conserved (dyn Euler)", drift < 0.03, f"V0 {v0:.4f}, Euler max relative drift {drift:.4f} over 60 years (tolerance 0.03; Euler spirals outward slowly)")

# 3. period for a small cycle: 2*pi/sqrt(a*c)
t2, x2, _ = run(["Prey=62", "Predators=20"], every=10)
p_small = period(t2[:], x2, c / d)
p_theory = 2 * math.pi / math.sqrt(a * c)
expect("small-cycle period", abs(p_small / p_theory - 1) < 0.01, f"measured {p_small:.3f} years, theory {p_theory:.3f} (tolerance 1%)")
p_big = period(t, x, c / d)
rk = [p[0] for p in ref]; p_ref = period(rk, [p[1] for p in ref], c / d)
expect("base-cycle period vs RK4", abs(p_big / p_ref - 1) < 0.03, f"dyn {p_big:.3f} years, RK4 {p_ref:.3f} (tolerance 3%); the big cycle is longer than the small-cycle formula")

# 4. logistic option: spiral in to x = c/d, y = (a/b)(1 - x/K)
t3, x3, y3 = run(["Logistic On=1"], every=100)
xe, ye = c / d, (a / b) * (1 - c / d / K)
expect("logistic equilibrium", abs(x3[-1] - xe) < 0.5 and abs(y3[-1] - ye) < 0.5, f"end prey {x3[-1]:.3f} (theory {xe:.1f}), predators {y3[-1]:.3f} (theory {ye:.1f}); tolerance 0.5")
ref3 = rk4(40.0, 15.0, 60.0, 0.001, 1.0)
expect("logistic vs RK4", abs(x3[-1] - ref3[-1][1]) < 0.05 and abs(y3[-1] - ref3[-1][2]) < 0.05, f"RK4 end {ref3[-1][1]:.3f}, {ref3[-1][2]:.3f} (tolerance 0.05)")
print("ALL PASS" if not fails else "FAILED: " + ", ".join(fails))
sys.exit(1 if fails else 0)
