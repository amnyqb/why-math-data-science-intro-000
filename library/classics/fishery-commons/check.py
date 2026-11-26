#!/usr/bin/env python3
"""Reference check for fishery-commons.sdm against independent plain-Python RK4 and closed-form equilibria.
   Usage: FC_BIN=/path/to/dyn python3 check.py   (or: python3 check.py /path/to/dyn)"""
import json, os, pathlib, subprocess, sys

HERE = pathlib.Path(__file__).resolve().parent
FC = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("FC_BIN") or "dyn"
MODEL = str(HERE / "fishery-commons.sdm")
r, K, q, p, c, k = 0.5, 1000.0, 0.001, 10.0, 2.0, 0.05   # growth, capacity, catchability, price, cost per boat, fleet response

def run(sets=(), every=500):
    cmd = [FC, "run", MODEL, "--series", "Fish,Boats,Catch,Profit", "--every", str(every)]
    for s in sets: cmd += ["--set", s]
    return json.loads(subprocess.run(cmd, capture_output=True, text=True, check=True).stdout)["series"]

def ref(quota=1e9, limit=1e9, T=100.0, dt=0.01):
    S, E = 900.0, 20.0
    def f(S, E):
        catch = min(q * E * S, quota); profit = p * catch - c * E
        dE = k * profit
        if dE > 0 and E >= limit: dE = 0.0          # fleet limit: no purchases beyond the limit
        return r * S * (1 - S / K) - catch, dE
    for _ in range(int(round(T / dt))):
        k1 = f(S, E); k2 = f(S + dt/2*k1[0], E + dt/2*k1[1]); k3 = f(S + dt/2*k2[0], E + dt/2*k2[1]); k4 = f(S + dt*k3[0], E + dt*k3[1])
        S += dt/6*(k1[0] + 2*k2[0] + 2*k3[0] + k4[0]); E += dt/6*(k1[1] + 2*k2[1] + 2*k3[1] + k4[1])
    return S, E

fails = []
def expect(name, ok, detail):
    print(("PASS " if ok else "FAIL ") + name + ": " + detail)
    if not ok: fails.append(name)

# 1. open access equilibrium (closed form)
base = run()
S_oa = c / (p * q); E_oa = (r / q) * (1 - S_oa / K); Y_oa = q * E_oa * S_oa
expect("open-access stock", abs(base["fish"][-1] - S_oa) < 2.0, f"end {base['fish'][-1]:.2f}, theory c/(p q) = {S_oa:.1f} (tolerance 2 tonne)")
expect("open-access fleet", abs(base["boats"][-1] - E_oa) < 8.0, f"end {base['boats'][-1]:.1f}, theory (r/q)(1-S/K) = {E_oa:.1f} (tolerance 8 boats)")
expect("open-access rent gone", abs(base["profit"][-1]) < 10.0, f"end profit {base['profit'][-1]:.2f} kUSD/Year, against revenue 800 (tolerance 10, a 1.3% residual: the fleet is still settling)")
expect("open-access yield", abs(base["catch"][-1] - Y_oa) < 1.0, f"end catch {base['catch'][-1]:.2f}, theory {Y_oa:.1f}")
MSY = r * K / 4
expect("open access below MSY", base["catch"][-1] < MSY and base["fish"][-1] < K / 2, f"catch {base['catch'][-1]:.1f} < MSY {MSY:.0f}; stock {base['fish'][-1]:.0f} < K/2 = {K/2:.0f}")

# 2. dyn (Euler dt 0.02) against RK4 (dt 0.01), base run
S_ref, E_ref = ref()
expect("base vs RK4", abs(base["fish"][-1] - S_ref) < 2.0 and abs(base["boats"][-1] - E_ref) < 5.0, f"RK4 end stock {S_ref:.2f}, boats {E_ref:.2f}; dyn {base['fish'][-1]:.2f}, {base['boats'][-1]:.2f} (tolerance 2 tonne, 5 boats)")

# 3. quota policy: quota 100 < MSY. Boats rise to p*Q/c, quota binds, stock settles on the upper root of r S (1 - S/K) = Q
Q = 100.0
qs = run([f"Quota={Q}"])
S_q = K / 2 * (1 + (1 - 4 * Q / (r * K)) ** 0.5)
expect("quota stock", abs(qs["fish"][-1] - S_q) < 3.0, f"end {qs['fish'][-1]:.1f}, theory {S_q:.1f} (tolerance 3 tonne)")
expect("quota catch", abs(qs["catch"][-1] - Q) < 0.5, f"end catch {qs['catch'][-1]:.2f}, quota {Q}; open access {base['catch'][-1]:.1f}")
expect("quota beats open access", qs["catch"][-1] > base["catch"][-1] and qs["fish"][-1] > 3 * base["fish"][-1], f"catch {qs['catch'][-1]:.1f} vs {base['catch'][-1]:.1f}; stock {qs['fish'][-1]:.0f} vs {base['fish'][-1]:.0f}")
S_ref, E_ref = ref(quota=Q)
expect("quota vs RK4", abs(qs["fish"][-1] - S_ref) < 3.0, f"RK4 end stock {S_ref:.1f}, boats {E_ref:.1f}; dyn {qs['fish'][-1]:.1f}, {qs['boats'][-1]:.1f}")

# 4. fleet limit 200 boats: stock on 0.2 S = 0.5 S (1 - S/K) -> S = K (1 - q L / r); profit stays positive
L = 200.0
fl = run([f"Fleet Limit={L}"])
S_l = K * (1 - q * L / r); Y_l = q * L * S_l
expect("fleet-limit stock", abs(fl["fish"][-1] - S_l) < 3.0, f"end {fl['fish'][-1]:.1f}, theory {S_l:.1f} (tolerance 3 tonne)")
expect("fleet-limit profit", abs(fl["profit"][-1] - (p * Y_l - c * L)) < 10.0 and fl["profit"][-1] > 500, f"end profit {fl['profit'][-1]:.0f}, theory {p*Y_l - c*L:.0f} kUSD/Year (tolerance 10); open access {base['profit'][-1]:.1f}")
expect("fleet-limit near MSY", abs(fl["catch"][-1] - Y_l) < 1.0 and Y_l > 0.9 * MSY, f"catch {fl['catch'][-1]:.1f} = {fl['catch'][-1]/MSY:.0%} of MSY {MSY:.0f}")
print("ALL PASS" if not fails else "FAILED: " + ", ".join(fails))
sys.exit(1 if fails else 0)
