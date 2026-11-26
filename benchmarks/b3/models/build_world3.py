#!/usr/bin/env python3
"""World3-03 (Meadows et al. 2004, the Vensim sample wrld3-03.mdl) with a manifest for B3. This is NOT the 1974 DYNAMO version.
Lever ranges (about +-50% or the usual scenario changes) and goals are this benchmark's choice, not the literature's."""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); B3 = os.path.join(HERE, "..")
sys.path.insert(0, os.path.join(B3, "tests"))
from fixtures import FC
SRC = os.path.join(B3, "..", "b2", "external", "vensim_samples", "Sample__WRLD3-03__wrld3-03.mdl")
LEVERS = [("initial nonrenewable resources", 5e11, 2e12), ("industrial capital output ratio 1", 2, 4), ("average life of industrial capital 1", 10, 20),
          ("desired completed family size normal", 2.5, 5), ("persistent pollution transmission delay", 10, 40), ("social adjustment delay", 10, 40),
          ("lifetime perception delay", 10, 40), ("health services impact delay", 10, 40), ("technology development delay", 10, 40),
          ("initial land fertility", 450, 750), ("desired food ratio", 1.5, 2.5), ("persistent pollution generation factor 1", 0.5, 1.5),
          ("service capital output ratio 1", 0.5, 1.5), ("average life of service capital 1", 10, 30), ("average life of land normal", 500, 2000),
          ("maximum total fertility normal", 10, 14), ("average life of agricultural inputs 1", 1, 4), ("income expectation averaging time", 1.5, 6)]
GOALS = [("Population peak", "population", "peak", "max"), ("Industrial output per capita peak", "industrial output per capita", "peak", "max"),
         ("Food per capita lowest", "food per capita", "lowest", "max"), ("Persistent pollution index peak", "persistent pollution index", "peak", "min")]
out = os.path.join(B3, "build", "w3.sdm")
os.makedirs(os.path.dirname(out), exist_ok=True)
r = subprocess.run([FC, "convert", SRC, out], capture_output=True, text=True); assert r.returncode == 0, r.stdout
ops = [{"op": "set_manifest", "manifest": {"title": "World3-03 (Meadows et al. 2004)", "levers": [{"name": n, "min": a, "max": b} for n, a, b in LEVERS],
       "goals": [{"name": g, "output": o, "measure": m, "direction": d} for g, o, m, d in GOALS]}}]
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump(ops, f)
r = subprocess.run([FC, "ops", out, f.name], capture_output=True, text=True); os.unlink(f.name)
print("ok" if '"ok": true' in r.stdout else r.stdout[:800])
