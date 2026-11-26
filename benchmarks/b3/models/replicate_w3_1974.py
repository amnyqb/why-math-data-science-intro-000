#!/usr/bin/env python3
"""M1 replication: builds the World3-1974 Dynamist model (variants, three time steps), runs PyWorld3 (python -I in its venv),
compares, writes benchmarks/b3/results/b3-3b/replication.md. Needs benchmarks/b3/build/pyworld3 and pyworld3-venv (see README of this task)."""
import os, subprocess, sys

HERE = os.path.dirname(os.path.abspath(__file__))
B3 = os.path.join(HERE, "..")
BUILD = os.path.join(B3, "build")
RES = os.path.join(B3, "results", "b3-3b")
PY = os.path.join(BUILD, "pyworld3-venv", "bin", "python")
os.makedirs(RES, exist_ok=True)


def sh(*cmd):
    r = subprocess.run(list(cmd), capture_output=True, text=True)
    if r.returncode != 0:
        sys.exit(f"failed: {cmd}\n{r.stdout}\n{r.stderr}")
    return r.stdout


VARIANTS = [("default (book DELAY3 start)", [], "w3_1974"),
            ("PyWorld3 DELAY3 start", ["--delay3-init", "pw3"], "w3_1974_pw3delay"),
            ("book start values for LUFD/AI/PFR", ["--smooth-init", "book"], "w3_1974_bookinit"),
            ("both book", ["--smooth-init", "book"], "w3_1974_bookinit")]
DTS = ["0.5", "0.25", "1"]
main_csv = {}
for dt in DTS:
    p = os.path.join(RES, f"pyworld3_standard_dt{dt}.csv" if dt != "1" else "pyworld3_standard_dt1.csv")
    sh(PY, "-I", os.path.join(HERE, "run_pyworld3.py"), p, dt)
    main_csv[dt] = p

out = ["# B3-3b replication: Dynamist World3 (1974) against PyWorld3\n"]
out.append("Model: `benchmarks/b3/models/build_world3_1974.py` (equations and their PyWorld3 file:line in `W3_1974_SOURCES.md`). Reference: PyWorld3 standard run "
           "(`World3()` defaults, `run_world3(fast=False)`, year 1900-2100), exported by `run_pyworld3.py` to `pyworld3_standard_dt*.csv` in this folder. "
           "Both sides are explicit Euler at the same dt.\n")
out.append("**Tolerance, fixed before the comparison: 1% relative difference on population, industrial output, food, persistent pollution index and "
           "nonrenewable resources fraction remaining at 1950, 2000, 2050 and 2100.**\n")
out.append("Names: POP population, IO industrial output, F food, PPOLX persistent pollution index, NRFR nonrenewable resources fraction remaining.\n")

# main comparison: dt 0.5, default model
model_default = os.path.join(BUILD, "w3_1974.sdm")
sh(sys.executable, os.path.join(HERE, "build_world3_1974.py"))
out.append("## Main result (dt = 0.5, PyWorld3's own default step)\n")
out.append(sh(sys.executable, os.path.join(HERE, "compare_w3_1974.py"), model_default, main_csv["0.5"], "--label", "default model vs PyWorld3, dt 0.5"))

sh(sys.executable, os.path.join(HERE, "build_world3_1974.py"), "--delay3-init", "pw3", "--out", os.path.join(BUILD, "w3_1974_pw3delay.sdm"),
   "--sources", os.path.join(BUILD, "sources_tmp.md"))
out.append("## The same with PyWorld3's own DELAY3 start (variant, used to find the cause of the early pollution difference)\n")
out.append(sh(sys.executable, os.path.join(HERE, "compare_w3_1974.py"), os.path.join(BUILD, "w3_1974_pw3delay.sdm"), main_csv["0.5"],
              "--label", "variant with PyWorld3 DELAY3 start vs PyWorld3, dt 0.5", "--vars", "ppol,ppgr,ppapr,io,iopc,fpc,f,le,ai,pfr,cuf,ic"))

# sensitivity to the two start choices and to dt
out.append("## Summary over variants and time steps\n")
out.append("Worst relative difference at the 20 checkpoints, and worst difference over 1930-2100 for each main variable (1900-1930 holds the start-up transient described in the next section).\n")
out.append("| Dynamist variant, dt | worst at checkpoints | POP | IO | F | PPOLX | NRFR |\n|---|---|---|---|---|---|---|")
for dt in DTS:
    for label, flags, name in [("default", [], "a"), ("PyWorld3 DELAY3 start", ["--delay3-init", "pw3"], "b"),
                               ("book start LUFD/AI/PFR", ["--smooth-init", "book"], "c"),
                               ("book start + PyWorld3 DELAY3 start", ["--smooth-init", "book", "--delay3-init", "pw3"], "d")]:
        path = os.path.join(BUILD, f"w3var_{name}_{dt}.sdm")
        sh(sys.executable, os.path.join(HERE, "build_world3_1974.py"), "--dt", dt, "--out", path, "--sources", os.path.join(BUILD, "sources_tmp.md"), *flags)
        out.append(sh(sys.executable, os.path.join(HERE, "compare_w3_1974.py"), path, main_csv[dt], "--label", f"{label}, dt {dt}", "--brief").strip())
out.append("\n## Where the schemes differ, and the cause of every difference above\n")
out.append(open(os.path.join(HERE, "W3_1974_DIFFERENCES.md")).read())
sh(sys.executable, os.path.join(HERE, "build_world3_1974.py"))  # leave the default model and the sources table in place
open(os.path.join(RES, "replication.md"), "w").write("\n".join(out) + "\n")
print("written", os.path.join(RES, "replication.md"))
