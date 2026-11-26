#!/usr/bin/env python3
"""Run pysd_ref.py on many models in parallel, one process per model with a time limit.

    python3 pysd_all.py external/vensim_samples [--jobs 8] [--timeout 180]

Writes build/pysd/<stem>.csv for each model PySD runs, and results/b2_pysd_status.csv saying, per model, whether
PySD ran it, failed (and why), or ran out of time.
"""
import argparse
import csv
import os
import subprocess
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
PY = os.path.join(HERE, "build", "pysd-venv", "bin", "python")


def one(path, timeout):
    stem = os.path.basename(path).rsplit(".", 1)[0]
    try:
        p = subprocess.run([PY, os.path.join(HERE, "pysd_ref.py"), path], capture_output=True, text=True, timeout=timeout)
        line = next((l for l in p.stdout.splitlines() if l.startswith(stem + ":")), "")
        if "PySD failed" in line:
            return {"model": stem, "pysd": "fails", "detail": line.split("PySD failed:", 1)[1].strip()[:300]}
        if line:
            return {"model": stem, "pysd": "runs", "detail": line.split(":", 1)[1].strip()}
        return {"model": stem, "pysd": "fails", "detail": (p.stderr.strip().splitlines() or ["no output"])[-1][:300]}
    except subprocess.TimeoutExpired:
        return {"model": stem, "pysd": "timeout", "detail": f"more than {timeout} s"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("folder")
    ap.add_argument("--jobs", type=int, default=8)
    ap.add_argument("--timeout", type=int, default=180)
    a = ap.parse_args()
    # a folder of models, or a folder of folders each holding <folder name>.mdl with its data files beside it
    files = sorted(os.path.join(a.folder, f) for f in os.listdir(a.folder) if f.lower().endswith(".mdl"))
    files += sorted(os.path.join(a.folder, d, d + ".mdl") for d in os.listdir(a.folder) if os.path.exists(os.path.join(a.folder, d, d + ".mdl")))
    with ThreadPoolExecutor(a.jobs) as ex:
        rows = list(ex.map(lambda f: one(f, a.timeout), files))
    os.makedirs(os.path.join(HERE, "results"), exist_ok=True)
    out = os.path.join(HERE, "results", f"b2_pysd_status_{os.path.basename(os.path.normpath(a.folder))}.csv")
    with open(out, "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=["model", "pysd", "detail"])
        w.writeheader()
        w.writerows(rows)
    from collections import Counter
    print(len(rows), "models:", dict(Counter(r["pysd"] for r in rows)))


if __name__ == "__main__":
    main()
