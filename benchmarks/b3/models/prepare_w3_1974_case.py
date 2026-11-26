#!/usr/bin/env python3
"""Adds the B3-3b manifest (levers, goals) to build/w3_1974.sdm. Lever ranges and goals are this benchmark's choice."""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); B3 = os.path.join(HERE, "..")
FC = os.environ.get("FC", os.path.join(B3, "..", "..", "target", "release", "dyn"))
M = os.path.join(B3, "build", "w3_1974.sdm")
LEVERS = [("NRI", 5e11, 2e12), ("ICOR1", 2, 4), ("ALIC1", 10, 20), ("DCFSN", 2.5, 5), ("PPTD1", 10, 40), ("SAD", 10, 40), ("LPD", 10, 40),
          ("HSID", 10, 40), ("LFERTI", 450, 750), ("FSPD", 1.5, 2.5), ("PPGF1", 0.5, 1.5), ("SCOR1", 0.5, 1.5), ("ALSC1", 10, 30),
          ("ALLN", 3000, 12000), ("MTFN", 10, 14), ("ALAI1", 1, 4), ("IEAT", 1.5, 6)]
GOALS = [("Population peak", "POP", "peak", "max"), ("Industrial output per capita peak", "IOPC", "peak", "max"),
         ("Food per capita lowest", "FPC", "lowest", "max"), ("Persistent pollution index peak", "PPOLX", "peak", "min")]
# declared after the first reduce refused these two goals as heavy tailed (logged in results/b3-3b/FREEZE_LOG.md)
LOG = {"Industrial output per capita peak", "Persistent pollution index peak"}
ops = [{"op": "set_manifest", "manifest": {"title": "World3 1974 (from PyWorld3)", "levers": [{"name": n, "min": a, "max": b} for n, a, b in LEVERS],
       "goals": [{"name": g, "output": o, "measure": m, "direction": d, **({"transform": "log"} if g in LOG else {})} for g, o, m, d in GOALS]}}]
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump(ops, f)
r = subprocess.run([FC, "ops", M, f.name], capture_output=True, text=True); os.unlink(f.name)
print("ok" if '"ok": true' in r.stdout else r.stdout[:600])
