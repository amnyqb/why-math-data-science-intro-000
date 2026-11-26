#!/usr/bin/env python3
"""Score B3-3b (World3, 1974 version, our PyWorld3-based model) against keys/b3-3b/answer_key.yaml (Thissen 1978).
Only the entries that can be tested on our model are scored; the others are listed as unscored. Criteria were written from the key,
before scoring, after the owner-visible reduction results had been seen (disclosed in the report)."""
import csv, json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC", os.path.join(HERE, "..", "..", "target", "release", "dyn"))
M = os.path.join(HERE, "build", "w3_1974.sdm")
R = os.path.join(HERE, "results", "b3-3b")


def fc(*a):
    r = subprocess.run([FC, *map(str, a)], capture_output=True, text=True)
    try:
        return json.loads(r.stdout)
    except ValueError:
        return {"raw": r.stdout[:300], "rc": r.returncode}


def series(model, names, sets=()):
    out = os.path.join(tempfile.mkdtemp(), "r.csv")
    args = ["run", model, "--series", ",".join(names), "--csv", out]
    for s in sets:
        args += ["--set", s]
    fc(*args)
    rows = list(csv.reader(open(out)))
    h = [x.lower() for x in rows[0]]
    t = [float(r[0]) for r in rows[1:]]
    return t, {n: [float(r[h.index(n.lower())]) for r in rows[1:]] for n in names}


rows = []
def add(entry, name, ok, ev):
    rows.append((entry, name, ok, ev)); print(("PASS " if ok else "FAIL ") + f"[{entry}] {name} | {ev}")


t, s = series(M, ["POP", "IO", "F", "PPOLX", "NRFR", "IC"])
# entry 0: growth followed by decline in every central variable except NR, decline setting in during the 21st century
res = {}
for n in ("POP", "IO", "F", "PPOLX"):
    v = s[n]; i = max(range(len(v)), key=lambda k: v[k]); res[n] = (t[i], v[i], v[-1])
ok0 = all(1990 <= tp <= 2100 and last < 0.9 * pk for tp, pk, last in res.values())
add(0, "growth then decline of POP, IO, F, PPOLX with the decline in the 21st century", ok0, "; ".join(f"{n}: peak {tp:.0f}, end/peak {last/pk:.2f}" for n, (tp, pk, last) in res.items()))
# entry 9: PPOLX stays below 12 (pollution sector inactive)
add(9, "PPOLX stays below 12 over the standard run", max(s["PPOLX"]) < 12, f"maximum {max(s['PPOLX']):.2f}")
# entry 4/5: capital declines after a large share of the resources is used
nrfr = s["NRFR"]; ic = s["IC"]; ip = max(range(len(ic)), key=lambda k: ic[k]); used = 1 - nrfr[ip]
add(4, "capital begins to decline when roughly half to 60 percent of the resources are used (accept 40 to 70 percent)", 0.4 <= used <= 0.7, f"capital peaks in {t[ip]:.0f} with {used:.0%} of the resources used")
# entries 15/16: dominant growth eigenvalue of the 1970 linearisation about +0.027
imp = fc("importance", M, "--samples", 8, "--ensemble", 0, "--eigen-at", 1970, "--workers", 2)
ev = [e["re"] for e in (imp.get("eigenvalues") or {}).get("eigenvalues", [])]
top = max(ev) if ev else float("nan")
add(16, "dominant eigenvalue of the 1970 linearisation near +0.027 (within 30 percent)", ev != [] and abs(top - 0.027) <= 0.3 * 0.027, f"largest real part {top:.4f} ({len(ev)} eigenvalues; Jacobian includes hidden delay states)")
# entry 10: job sector can be ignored in the standard run: fix CUF at its base-run range -> run virtually identical?
with tempfile.TemporaryDirectory() as d:
    fixed = os.path.join(d, "cuf.sdm"); fc("convert", M, fixed)
    t2, c = series(M, ["CUF"]); lo, hi = min(c["CUF"]), max(c["CUF"])
    json.dump([{"op": "set_equation", "name": "CUF", "equation": "1"}], open(os.path.join(d, "o.json"), "w")); fc("ops", fixed, os.path.join(d, "o.json"))
    v = fc("verify", M, fixed, "--ensemble", 1, "--max-nrmse", 0.05)
    worst = max(g["base"]["nrmse"] for g in v.get("goals", [{"base": {"nrmse": 9}}]))
    add(10, "job sector can be ignored: CUF fixed at 1 changes the standard run by under 5 percent", worst <= 0.05, f"CUF ranges {lo:.3f} to {hi:.3f} in the standard run; largest NRMSE of a goal output {worst:.3f}")
# entries 33 to 37: Thissen's simple policy replicated on our model
with tempfile.TemporaryDirectory() as d:
    pol = os.path.join(d, "policy.sdm"); fc("convert", M, pol)
    ops = [
        {"op": "add_var", "name": "FIOARC", "kind": "aux", "equation": "MIN(0.11, MAX(0, TIME - 1975) * 0.11 / 25)"},
        {"op": "set_equation", "name": "FIOAI", "equation": "1 - FIOAA - FIOAS - FIOAC - FIOARC"},
        {"op": "set_equation", "name": "NRUR", "equation": "POP * PCRUM * NRUF * EXP(-10 * FIOARC)"},
        {"op": "set_equation", "name": "DCFSN", "equation": "4 - 0.7 * MIN(1, MAX(0, (TIME - 1975) / 25))"},
    ]
    json.dump(ops, open(os.path.join(d, "o.json"), "w")); r = fc("ops", pol, os.path.join(d, "o.json"))
    if not r.get("ok"):
        add(37, "Thissen's two-part policy applied to our model", False, f"could not apply: {str(r)[:200]}")
    else:
        t3, p = series(pol, ["POP", "IO", "F", "NRFR"])
        late = [i for i in range(len(t3)) if t3[i] >= 2000]
        io_ok = p["IO"][-1] >= 0.9 * max(p["IO"][i] for i in late); pop_ok = p["POP"][-1] >= 0.9 * max(p["POP"])
        add(37, "with the policy neither IO nor POP declines before 2100 (end value at least 90 percent of the maximum)", io_ok and pop_ok,
            f"IO end {p['IO'][-1]:.3g} (max {max(p['IO']):.3g}); POP end {p['POP'][-1]:.3g} (max {max(p['POP']):.3g}); the paper: IO about 1.5e12, POP slightly below 6e9. Standard run: POP end/peak {res['POP'][2]/res['POP'][1]:.2f}")
w = csv.writer(open(os.path.join(R, "score.csv"), "w")); w.writerow(["entry", "finding", "reproduced", "evidence"])
for r in rows: w.writerow(r)
print(f"{sum(1 for r in rows if r[2])} of {len(rows)} scored entries reproduced (of 41 in the key; the rest are not testable on this model)")
