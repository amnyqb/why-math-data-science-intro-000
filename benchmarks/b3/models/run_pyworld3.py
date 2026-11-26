#!/usr/bin/env python3
"""Run the PyWorld3 standard run and export CSV. Usage: run_pyworld3.py OUT.csv [dt] [fast]
Run it as `python -I`; the untrusted clone is put on the path explicitly below."""
import os, sys, csv
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "build", "pyworld3"))
import matplotlib; matplotlib.use("Agg")
from pyworld3 import World3

out = sys.argv[1]
dt = float(sys.argv[2]) if len(sys.argv) > 2 else 0.5
fast = len(sys.argv) > 3 and sys.argv[3] == "fast"
w = World3(year_min=1900, year_max=2100, dt=dt)
w.init_world3_constants(); w.init_world3_variables(); w.set_world3_table_functions(); w.set_world3_delay_functions()
w.run_world3(fast=fast)
VARS = ["pop","p1","p2","p3","p4","io","iopc","f","fpc","ppolx","ppol","nrfr","nr","ic","sc","so","sopc","al","pal","uil","lfert",
        "ai","aiph","pfr","fr","le","ly","tai","fioaa","fioas","fioai","fioac","fcaor","cuf","lufd","luf","j","lf","lmf","lmhs","lmp","lmc",
        "ehspc","hsapc","ple","diopc","aiopc","sfsn","frsn","fie","dcfs","dtf","mtf","tf","fce","fcfpc","fcapc","nfc","fsafc","b",
        "ppgr","ppapr","ppasr","ppgio","ppgao","lfdr","lfrt","falm","ldr","ler","lrui","lfd","lfr","nrur","pcrum","mpld","mpai","fiald","cai","dcph",
        "ifpc","lymc","lymap","llmy","all","uilr","uilpc"]
with open(out, "w", newline="") as fh:
    wr = csv.writer(fh); wr.writerow(["time"] + VARS)
    for k in range(w.n):
        wr.writerow([repr(float(w.time[k]))] + [repr(float(getattr(w, v)[k])) for v in VARS])
print("rows", w.n, "pop2100", w.pop[-1])
