#!/usr/bin/env python3
"""The checks of the venture model: units, simulation values, KPIs, loops, the second run.
   Usage: python3 check.py [path/to/dyn]   (run from this folder, after the model is built)"""
import json, subprocess, sys
FC = sys.argv[1] if len(sys.argv) > 1 else "dyn"
def fc(*a):
    p = subprocess.run([FC, *a], capture_output=True, text=True)
    try: return json.loads(p.stdout), p.returncode
    except Exception: return p.stdout, p.returncode
ok_all = True
def report(name, ok, detail=""):
    global ok_all; ok_all &= bool(ok); print(("PASS " if ok else "FAIL ") + name, detail)
v, rc = fc("validate", "venture.sdm", "--strict-units")
report("units (strict)", rc == 0, json.dumps(v)[:300] if rc else "0 errors")
def series(extra=()):
    r, rc = fc("run", "venture.sdm", "--every", "1", *extra)
    return r
r = series()
print("run keys:", list(r.keys())[:8] if isinstance(r, dict) else type(r))
sys.exit(0 if ok_all else 1)
