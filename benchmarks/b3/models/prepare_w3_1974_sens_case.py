#!/usr/bin/env python3
"""B3-3c: the World3 1974 model with a lever set chosen to test the sensitivity findings of Vermeulen and de Jongh and Thissen 1976.
Rule fixed before running: every lever varies over nominal x [0.7, 1.3] (the same rule for all, not tuned). Levers include the parameters
the two papers name as influential (FIOAC, ICOR, ALIC, SCOR, ALSC, DCFSN, LEN, RLT) and as uninfluential (agriculture, resources, pollution)."""
import json, os, subprocess, sys, tempfile
HERE = os.path.dirname(os.path.abspath(__file__)); B3 = os.path.join(HERE, "..")
FC = os.environ.get("FC", os.path.join(B3, "..", "..", "target", "release", "dyn"))
SRC = os.path.join(B3, "build", "w3_1974.sdm"); OUT = os.path.join(B3, "build", "w3_1974_c.sdm")
NAMES = ["FIOAC1", "FIOAC2", "ICOR1", "ALIC1", "SCOR1", "ALSC1", "DCFSN", "LEN", "RLT", "LYF1", "ILF", "LFH", "ALAI1", "NRI", "PPGF1", "PPTD1", "AHL70", "LFERTI", "MTFN", "SAD", "HSID", "LPD", "FSPD"]
GOALS = [("Population peak", "POP", "peak", "max", False), ("Industrial output per capita peak", "IOPC", "peak", "max", True),
         ("Food per capita lowest", "FPC", "lowest", "max", False), ("Persistent pollution index peak", "PPOLX", "peak", "min", True)]
lst = json.loads(subprocess.run([FC, "list", SRC, "--kind", "constant"], capture_output=True, text=True).stdout)
vs = lst if isinstance(lst, list) else lst["vars"]
nom = {v["name"]: float(v["equation"]) for v in vs}
levers = [{"name": n, "min": nom[n] * 0.7, "max": nom[n] * 1.3} for n in NAMES]
subprocess.run([FC, "convert", SRC, OUT], capture_output=True)
ops = [{"op": "set_manifest", "manifest": {"title": "World3 1974, sensitivity lever set (B3-3c)", "levers": levers,
       "goals": [{"name": g, "output": o, "measure": m, "direction": d, **({"transform": "log"} if lg else {})} for g, o, m, d, lg in GOALS]}}]
with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
    json.dump(ops, f)
r = subprocess.run([FC, "ops", OUT, f.name], capture_output=True, text=True); os.unlink(f.name)
print("ok" if '"ok": true' in r.stdout else r.stdout[:500])
