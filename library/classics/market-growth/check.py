#!/usr/bin/env python3
"""Checks of the market-growth model. None depends on a reference file.
Usage: FC_BIN=/path/to/dyn python3 check.py     (run in this folder, after the model is built)"""
import json, os, shutil, subprocess, sys, tempfile
FC = os.environ.get("FC_BIN") or (sys.argv[1] if len(sys.argv) > 1 else "dyn")
MODEL = "market-growth.sdm"
DT = 0.125
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
    return {k: v for k, v in r["series"].items()}
def at(series, month, every=1, dt=DT): return series[round(month / (dt * every))]
STOCKS = ["Sales Force", "Order Backlog", "Capacity", "Capacity on Order"]
K = lambda n: n.lower()

# 1. strict units
v, rc = fc("validate", MODEL, "--strict-units")
report("units (strict): 0 errors", rc == 0 and v.get("errors") == 0, f"errors={v.get('errors')}")

# 2. steady state: with Budget Fraction 0.05 the start is in balance, whatever the switches say
for sw in ([], ["Capacity Switch=0"], ["Delay Effect Switch=0"], ["Sales Budget Switch=0"], ["Capacity Switch=0", "Delay Effect Switch=0"]):
    s = run(MODEL, STOCKS, 8, ["Budget Fraction=0.05"] + sw)
    drift = max(max(s[K(n)]) - min(s[K(n)]) for n in STOCKS)
    report(f"steady state (Budget Fraction 0.05) {' '.join(sw) or 'all switches on'}", drift < 1e-9, f"max drift {drift:.2e}")

# 3. the base run, all feedbacks on
B = ["Order Rate", "Capacity", "Delivery Delay", "Sales Force", "Capacity on Order"]
s = run(MODEL, B, 8)
orders, cap, dd = s["order rate"], s["capacity"], s["delivery delay"]
report("all loops on: capacity more than doubles in 120 months", cap[-1] > 200, f"capacity {cap[0]:.1f} -> {cap[-1]:.1f}")
i12 = round(12 / (DT * 8)); i18 = round(18 / (DT * 8)); i6 = round(6 / (DT * 8)); i30 = round(30 / (DT * 8))
pk = max(range(i12 + 1), key=lambda i: orders[i]); tr = min(range(pk, i30 + 1), key=lambda i: orders[i])
report("all loops on: growth stalls after the first spurt", orders[tr] < 0.98 * orders[pk] and cap[-1] > 5 * cap[0],
       f"orders peak {orders[pk]:.2f} at month {pk*DT*8:.0f}, trough {orders[tr]:.2f} at month {tr*DT*8:.0f}, then orders reach {orders[-1]:.0f}")
report("all loops on: delivery delay overshoots then settles near the target",
       max(dd) > 1.5 * 3 and abs(dd[-1] - dd[-2]) < 0.01 and 3 < dd[-1] < 1.4 * 3,
       f"delay peak {max(dd):.2f}, final {dd[-1]:.3f} (target 3)")

# 4. capacity loop off: growth is limited by delivery delay
s = run(MODEL, ["Order Rate", "Capacity", "Delivery Delay", "Sales Force"], 8, ["Capacity Switch=0"])
o, c, d, sf = s["order rate"], s["capacity"], s["delivery delay"], s["sales force"]
report("capacity loop off: capacity does not move", max(c) - min(c) < 1e-12, f"capacity {c[0]:.1f}")
report("capacity loop off: orders peak near capacity, then settle on it", max(o) < 1.25 * c[0] and abs(o[-1] - c[-1]) < 0.01 * c[-1],
       f"orders peak {max(o):.1f}, final {o[-1]:.3f}, capacity {c[-1]:.1f}")
report("capacity loop off: delivery delay stays high and the sales force stops growing", d[-1] > 1.5 * 3 and abs(sf[-1] - sf[-40]) < 1e-3,
       f"delay final {d[-1]:.2f} (target 3), sales force {sf[0]:.1f} -> {sf[-1]:.2f}")
report("the capacity loop matters: with it, the delay is lower and orders far higher", dd[-1] < d[-1] and orders[-1] > 5 * o[-1], f"delay {dd[-1]:.2f} vs {d[-1]:.2f}; orders {orders[-1]:.0f} vs {o[-1]:.0f}")

# 5. capacity loop off and delay effect off: nothing limits orders, so the backlog grows without bound
s = run(MODEL, ["Delivery Delay", "Order Backlog"], 8, ["Capacity Switch=0", "Delay Effect Switch=0"])
d, bl = s["delivery delay"], s["order backlog"]
report("both limits off: delivery delay and backlog grow without bound", d[-1] > 10 * 3 and all(bl[i + 1] > bl[i] for i in range(len(bl) // 2, len(bl) - 1)),
       f"delay final {d[-1]:.1f}, backlog final {bl[-1]:.0f}")
s = run(MODEL, ["Delivery Delay"], 8, ["Delay Effect Switch=0"])
report("delay effect off, capacity on: growth continues, delay stays bounded", max(s["delivery delay"]) < 12, f"delay peak {max(s['delivery delay']):.2f}")

# 6. accounting identities (Euler, so sums are exact up to rounding)
s = run(MODEL, ["Order Backlog", "Order Rate", "Shipments", "Capacity", "Capacity on Order", "Capacity Orders", "Sales Force", "Sales Force Change"], 1)
n = len(s["order backlog"]); r1 = r2 = r3 = 0.0; cb = cc = cs = 0.0
for i in range(n):
    r1 = max(r1, abs(s["order backlog"][i] - 300 - cb)); r2 = max(r2, abs(s["capacity"][i] + s["capacity on order"][i] - 100 - cc)); r3 = max(r3, abs(s["sales force"][i] - 20 - cs))
    cb += DT * (s["order rate"][i] - s["shipments"][i]); cc += DT * s["capacity orders"][i]; cs += DT * s["sales force change"][i]
report("accounting: backlog = start + orders - shipments", r1 < 1e-8, f"max residual {r1:.2e}")
report("accounting: capacity + capacity on order = start + capacity ordered", r2 < 1e-8, f"max residual {r2:.2e}")
report("accounting: sales force = start + net hiring", r3 < 1e-8, f"max residual {r3:.2e}")

# 7. halved step
def with_step(dt):
    d = tempfile.mkdtemp(); f = os.path.join(d, "m.sdm"); shutil.copy(MODEL, f)
    b = os.path.join(d, "b.json"); json.dump([{"op": "set_setting", "key": "time_step", "value": str(dt)}], open(b, "w"))
    r, rc = fc("ops", f, b, "--no-gate"); assert rc == 0, r
    return f
res = {}
for dt, ev in ((0.125, 8), (0.0625, 16), (0.03125, 32)):
    s = run(with_step(dt), ["Capacity", "Delivery Delay"], ev)
    res[dt] = (s["capacity"][-1], max(s["delivery delay"]))
dc1 = abs(res[0.125][0] - res[0.0625][0]); dc2 = abs(res[0.0625][0] - res[0.03125][0])
dd1 = abs(res[0.125][1] - res[0.0625][1]); dd2 = abs(res[0.0625][1] - res[0.03125][1])
report("halved step: capacity at month 120 and peak delay converge", dc2 < dc1 and dd2 < dd1 and dc1 / res[0.0625][0] < 0.02 and dd1 / res[0.0625][1] < 0.02,
       f"capacity {res[0.125][0]:.2f}, {res[0.0625][0]:.2f}, {res[0.03125][0]:.2f}; peak delay {res[0.125][1]:.4f}, {res[0.0625][1]:.4f}, {res[0.03125][1]:.4f}")
sys.exit(0 if ok_all else 1)
