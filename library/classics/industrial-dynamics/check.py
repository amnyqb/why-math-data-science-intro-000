#!/usr/bin/env python3
"""Checks of the industrial-dynamics model. None depends on a reference file.
Usage: FC_BIN=/path/to/dyn python3 check.py     (run in this folder, after the model is built)"""
import json, os, shutil, subprocess, sys, tempfile
FC = os.environ.get("FC_BIN") or (sys.argv[1] if len(sys.argv) > 1 else "dyn")
MODEL = "industrial-dynamics.sdm"
def fc(*a):
    p = subprocess.run([FC, *a], capture_output=True, text=True)
    try: return json.loads(p.stdout), p.returncode
    except Exception: return p.stdout, p.returncode
ok_all = True
def report(name, ok, detail=""):
    global ok_all; ok_all &= bool(ok); print(("PASS " if ok else "FAIL ") + name, detail)
def run(model, series, every=1, sets=()):
    args = ["run", model, "--series", ",".join(series), "--every", str(every)]
    for s in sets: args += ["--set", s]
    r, rc = fc(*args)
    assert rc == 0, r
    return r["series"]
def key(n): return n.lower()

# 1. strict units
v, rc = fc("validate", MODEL, "--strict-units")
report("units (strict): 0 errors", rc == 0 and v.get("errors") == 0, f"errors={v.get('errors')}")

# 2. steady state: no step, every stock constant to 1e-9
lst, _ = fc("list", MODEL, "--kind", "stock")
stocks = [x["name"] if isinstance(x, dict) else x for x in (lst.get("items", lst.get("variables", lst)) if isinstance(lst, dict) else lst)]
s = run(MODEL, stocks, 8, ["Step Size=0"])
drift = {n: max(s[key(n)]) - min(s[key(n)]) for n in stocks if n not in ("Cumulative Delivered", "Cumulative Starts")}
report("steady state: stocks constant with no step", max(drift.values()) < 1e-9, f"max drift {max(drift.values()):.2e} over {len(drift)} stocks")
c = run(MODEL, ["Cumulative Delivered", "Cumulative Starts"], 8, ["Step Size=0"])
report("steady state: tallies grow at exactly the demand", abs(c["cumulative delivered"][-1] - 100 * 100) < 1e-6 and abs(c["cumulative starts"][-1] - 100 * 100) < 1e-6,
       f"delivered {c['cumulative delivered'][-1]:.6f}, started {c['cumulative starts'][-1]:.6f} (expect 10000)")

# 3. amplification up the chain, at three levels of supply-line weight
ORD = ["Customer Orders", "Retailer Orders", "Distributor Orders", "Production Starts"]
peaks = {}
for w in (0, 0.5, 1):
    s = run(MODEL, ORD, 1, [f"Supply Line Weight={w}"])
    p = [max(s[key(n)]) for n in ORD]; peaks[w] = p
    report(f"amplification, supply-line weight {w}: peak orders rise up the chain", p[0] < p[1] < p[2] < p[3],
           "peaks " + ", ".join(f"{n}={x:.2f}" for n, x in zip(ORD, p)))
report("a larger supply-line weight gives a smaller factory peak", peaks[0][3] > peaks[0.5][3] > peaks[1][3],
       f"{peaks[0][3]:.1f} > {peaks[0.5][3]:.1f} > {peaks[1][3]:.1f}")

# 4. the new equilibrium: everything settles at the new demand, inventories at cover x new demand
s = run(MODEL, ORD + ["Retailer Inventory", "Distributor Inventory", "Factory Inventory"], 8)
fin = {n: s[key(n)][-1] for n in s}
ok = all(abs(fin[key(n)] - 110) < 1e-3 for n in ORD) and all(abs(fin[key(n)] - 330) < 1e-3 for n in ("Retailer Inventory", "Distributor Inventory", "Factory Inventory"))
report("new equilibrium reached by week 100", ok, "orders " + ", ".join(f"{fin[key(n)]:.5f}" for n in ORD) + "; inventories " + ", ".join(f"{fin[key(n)]:.4f}" for n in ("Retailer Inventory", "Distributor Inventory", "Factory Inventory")))

# 5. accounting identity: goods in the system change by (started - delivered), at every step
s = run(MODEL, ["Material in System", "Cumulative Starts", "Cumulative Delivered"], 1)
m, cs, cd = s["material in system"], s["cumulative starts"], s["cumulative delivered"]
res = max(abs((m[i] - m[0]) - (cs[i] - cd[i])) for i in range(len(m)))
report("conservation: change in material = starts - deliveries", res < 1e-9, f"max residual {res:.2e} over {len(m)} steps")

# 6. halved step: peaks and final values converge as the time step is halved
def with_step(dt):
    d = tempfile.mkdtemp(); f = os.path.join(d, "m.sdm"); shutil.copy(MODEL, f)
    b = os.path.join(d, "b.json"); json.dump([{"op": "set_setting", "key": "time_step", "value": str(dt)}], open(b, "w"))
    r, rc = fc("ops", f, b, "--no-gate"); assert rc == 0, r
    return f
pk = {}
for dt, ev in ((0.125, 8), (0.0625, 16), (0.03125, 32)):
    s = run(with_step(dt), ["Production Starts"], ev)
    pk[dt] = max(s["production starts"])
d1, d2 = abs(pk[0.125] - pk[0.0625]), abs(pk[0.0625] - pk[0.03125])
report("halved step: factory peak converges", d2 < d1 and d1 / pk[0.0625] < 0.01, f"peaks {pk[0.125]:.4f}, {pk[0.0625]:.4f}, {pk[0.03125]:.4f}; successive differences {d1:.4f} then {d2:.4f}")
sys.exit(0 if ok_all else 1)
