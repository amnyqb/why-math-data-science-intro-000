#!/usr/bin/env python3
"""B2: import published Vensim and Stella models into Dynamist, run them, and compare with the original tool's output.

    python3 run_b2.py [--only NAME] [--jobs 8]      # writes results/b2_models.csv and B2_report.md

Sources (downloaded into external/, not committed):
  - SDXorg/test-models (MIT): ~150 single-feature tests and 9 sample models, each with the canonical output
    exported from Vensim (output.tab / output.csv) and, for some, from Stella (output_stella*.csv).
  - climateinteractive/SDEverywhere (MIT): its test models, each with Vensim's own output as a .dat file.
  - widely cited models listed in models.json (copies of Vensim's sample library in external/vensim_classics),
    compared with PySD's run of the same file (pysd_ref.py), because Vensim keeps its runs in binary .vdf files.

For every model file (.mdl, .xmile, .stmx) the runner: imports it with `dyn run` (which reads the format from the
extension), runs it with the file's own time settings, and compares every variable the reference has, at every
time the reference has. Reference files print about 6 significant digits, so "exact" means equal to that
precision: |ours - ref| <= 1e-5 * |ref| + 1e-6 * max(1, largest |ref| of that variable) at every point (the second
term allows for Vensim's single-precision arithmetic near zero).

Verdicts: exact (every point within the tolerance) | close (within 100 tolerances, about 1e-3 relative) | differs | no-import | no-run | crash (fc panicked) | no-common (no variable names in
common) | no-reference.
"""
import argparse
import csv
import json
import math
import os
import re
import subprocess
import sys
from concurrent.futures import ThreadPoolExecutor

HERE = os.path.dirname(os.path.abspath(__file__))
EXT = os.path.join(HERE, "external")
BUILD = os.path.join(HERE, "build")
RES = os.path.join(HERE, "results")
FC = os.environ.get("FC_BIN", os.path.join(HERE, "..", "..", "target", "release", "dyn"))
EXACT_REL, EXACT_ABS, CLOSE_REL = 1e-5, 1e-6, 1e-3
SKIP = {"time", "initial time", "final time", "time step", "saveper", "dt", "starttime", "stoptime"}


def norm(name):
    n = name.strip().strip('"').lower()
    n = n.replace("_", " ").replace("\\n", " ")
    n = re.sub(r"\s+", " ", n)
    n = re.sub(r"\s*([\[\],])\s*", r"\1", n)
    return n


def read_table(path):
    """Columnar output: first row names, first column time. Tab or comma separated; any line endings."""
    raw = open(path, "rb").read().decode("utf-8", "replace").replace("\r\n", "\n").replace("\r", "\n")
    lines = [l for l in raw.split("\n") if l.strip()]
    sep = "\t" if "\t" in lines[0] else ","
    rows = list(csv.reader(lines, delimiter=sep))
    head = rows[0]
    # Vensim sometimes exports transposed (one row per variable, times across): detect a numeric header
    def num(x):
        try:
            float(x)
            return True
        except ValueError:
            return False
    if len(head) > 2 and all(num(h) for h in head[1:]) and not num(head[0]):
        times = [float(h) for h in head[1:]]
        cols = {}
        for r in rows[1:]:
            if r and r[0].strip():
                cols[r[0]] = [float(x) if num(x) else math.nan for x in r[1:len(times) + 1]]
        return times, cols
    times, cols = [], {h: [] for h in head[1:]}
    for r in rows[1:]:
        if not r or not num(r[0]):
            continue
        times.append(float(r[0]))
        for h, x in zip(head[1:], r[1:]):
            cols[h].append(float(x) if num(x) else math.nan)
    return times, cols


def read_dat(path):
    """Vensim DAT export: a line with the variable name, then 'time<TAB>value' lines. Constants have one line."""
    series, cur = {}, None
    for line in open(path, encoding="utf-8", errors="replace").read().replace("\r", "\n").split("\n"):
        if not line.strip():
            continue
        parts = line.split("\t")
        if len(parts) == 2:
            try:
                series[cur].append((float(parts[0]), float(parts[1])))
                continue
            except (ValueError, KeyError):
                pass
        cur = line.strip()
        series[cur] = []
    return series


def read_ref(path):
    """Any reference file as {variable: [(time, value), ...]}."""
    if path.lower().endswith(".dat"):
        return read_dat(path)
    times, cols = read_table(path)
    return {k: list(zip(times, v)) for k, v in cols.items()}


def references(folder):
    files = os.listdir(folder)
    vensim = [f for f in ("output.tab", "output.csv") if f in files] + sorted(f for f in files if f.startswith("output_vensim")) + sorted(f for f in files if f.endswith(".dat"))
    stella = sorted(f for f in files if f.startswith("output_stella"))
    return vensim, stella


def compare(ref_path, our_csv):
    ref = read_ref(ref_path)
    ot, oc = read_table(our_csv)
    ours = {norm(k): v for k, v in oc.items()}
    # match each reference time to our nearest saved time; references print times with few digits (0.0313)
    import bisect
    step = min((b - a for a, b in zip(ot, ot[1:]) if b > a), default=1.0)
    def at(t):
        j = bisect.bisect_left(ot, t)
        best = min((k for k in (j - 1, j) if 0 <= k < len(ot)), key=lambda k: abs(ot[k] - t), default=None)
        return best if best is not None and abs(ot[best] - t) <= max(1e-6, 0.49 * step) else None
    worst, worst_var, worst_t, n_vars, n_pts, bad_vars, missing = 0.0, "", None, 0, 0, [], []
    ref_times, matched = set(), set()
    for name, pts in ref.items():
        k = norm(name)
        if k in SKIP or not pts:
            continue
        if k not in ours:
            missing.append(name)
            continue
        n_vars += 1
        var_bad = False
        # Vensim runs in single precision by default: near zero, a stock that should be 0 can print as -2.5e-6.
        # So the absolute part of the tolerance scales with the variable's own size over the run.
        scale = max((abs(x) for _, x in pts if not math.isnan(x)), default=0.0)
        for t, rv in pts:
            ref_times.add(t)
            i = at(t)
            if i is None or math.isnan(rv):
                continue
            matched.add(t)
            v = ours[k][i]
            n_pts += 1
            err = abs(v - rv)
            tol = EXACT_REL * abs(rv) + EXACT_ABS * max(scale, 1.0)
            if not (err <= tol):
                var_bad = True
            rel = err / tol if not math.isnan(v) else math.inf  # in units of the tolerance: <= 1 is exact
            if rel > worst:
                worst, worst_var, worst_t = rel, name, t
        if var_bad:
            bad_vars.append(name)
    return {"vars": n_vars, "points": n_pts, "missing": missing, "bad_vars": bad_vars, "worst_rel": worst,
            "worst_var": worst_var, "worst_t": worst_t, "ref_times": len(ref_times), "times_matched": len(matched)}


def run_one(job):
    group, case, folder, model = job
    tag = re.sub(r"[^A-Za-z0-9_.-]+", "_", f"{group}__{case}__{os.path.basename(model)}")
    out_csv = os.path.join(BUILD, tag + ".csv")
    vensim, stella = references(folder)
    ext = model.rsplit(".", 1)[-1].lower()
    if group in ("cited", "vensim-samples"):
        # classic models: Vensim's own runs are binary .vdf files, so the reference is PySD's run (pysd_ref.py)
        pysd_csv = os.path.join(BUILD, "pysd", os.path.basename(model).rsplit(".", 1)[0] + ".csv")
        vensim, stella = ([os.path.relpath(pysd_csv, folder)] if os.path.exists(pysd_csv) else []), []
    refs = (vensim or stella) if ext == "mdl" else (stella or vensim)
    stem_dat = os.path.basename(model).rsplit(".", 1)[0] + ".dat"
    if group == "sdeverywhere" and os.path.exists(os.path.join(folder, stem_dat)):
        refs = [stem_dat]
    row = {"group": group, "case": case, "file": os.path.basename(model), "format": ext, "reference": refs[0] if refs else ""}
    if not refs:
        return {**row, "verdict": "no-reference"}
    try:
        p = subprocess.run([FC, "run", model, "--csv", out_csv], capture_output=True, text=True, timeout=120)
    except subprocess.TimeoutExpired:
        return {**row, "verdict": "no-run", "error": "timeout after 120 s"}
    if p.returncode != 0 or not os.path.exists(out_csv):
        err = (p.stderr or p.stdout).strip().splitlines()
        msg = err[-1] if err else f"exit {p.returncode}"
        if "panicked" in (p.stderr or "") or "BACKTRACE" in msg:
            panic = [l for l in (p.stderr or "").splitlines() if "panicked" in l or l.startswith("index") or "overflow" in l]
            return {**row, "verdict": "crash", "error": (" ".join(panic) or msg)[:300]}
        verdict = "no-import" if any(w in msg.lower() for w in ("parse", "unknown", "unsupported", "read", "import", "not supported", "expected")) else "no-run"
        return {**row, "verdict": verdict, "error": msg[:300]}
    with open(out_csv) as f:
        header = f.readline().strip().split(",")
    if len(header) <= 1:
        return {**row, "verdict": "no-import", "error": "the file was read without error but gave no variables"}
    try:
        c = compare(os.path.join(folder, refs[0]), out_csv)
    except Exception as e:  # a malformed reference is reported, not hidden
        return {**row, "verdict": "no-reference", "error": f"reference unreadable: {e}"[:300]}
    if c["vars"] == 0:
        verdict = "no-common"
    elif not c["bad_vars"] and c["times_matched"] == c["ref_times"]:
        verdict = "exact"
    elif c["worst_rel"] <= CLOSE_REL / EXACT_REL and c["times_matched"] == c["ref_times"]:
        verdict = "close"
    else:
        verdict = "differs"
    return {**row, "verdict": verdict, "vars_compared": c["vars"], "points": c["points"], "vars_missing": len(c["missing"]),
            "vars_differing": len(c["bad_vars"]), "worst_error_in_tolerances": f"{c['worst_rel']:.3g}", "worst_var": c["worst_var"],
            "worst_time": c["worst_t"], "ref_times": c["ref_times"], "times_matched": c["times_matched"],
            "missing_names": "; ".join(c["missing"][:8]), "differing_names": "; ".join(c["bad_vars"][:8])}


def jobs_suite():
    root = os.path.join(EXT, "test-models")
    out = []
    for group in ("samples", "tests"):
        gdir = os.path.join(root, group)
        for case in sorted(os.listdir(gdir)):
            folder = os.path.join(gdir, case)
            if not os.path.isdir(folder):
                continue
            for f in sorted(os.listdir(folder)):
                if f.lower().rsplit(".", 1)[-1] in ("mdl", "xmile", "stmx", "itmx"):
                    out.append((group, case, folder, os.path.join(folder, f)))
    return out


def jobs_sde():
    root = os.path.join(EXT, "sdeverywhere", "models")
    out = []
    if not os.path.isdir(root):
        return out
    for case in sorted(os.listdir(root)):
        folder = os.path.join(root, case)
        if os.path.isdir(folder):
            for f in sorted(os.listdir(folder)):
                if f.lower().rsplit(".", 1)[-1] in ("mdl", "xmile", "stmx") and os.path.exists(os.path.join(folder, f.rsplit(".", 1)[0] + ".dat")):
                    out.append(("sdeverywhere", case, folder, os.path.join(folder, f)))
    return out


def jobs_vensim_samples():
    """Copies of every model of a local Vensim install that Dynamist runs (report_b2.py makes the list); the
    reference is PySD's run (pysd_all.py), as for the classics."""
    folder = os.path.join(EXT, "vensim_samples")
    if not os.path.isdir(folder):
        return []
    return [("vensim-samples", f.rsplit(".", 1)[0], folder, os.path.join(folder, f)) for f in sorted(os.listdir(folder)) if f.lower().endswith(".mdl")]


def jobs_cited():
    spec = os.path.join(HERE, "models.json")
    if not os.path.exists(spec):
        return []
    out = []
    for m in json.load(open(spec)):
        folder = os.path.join(EXT, m["folder"])
        if os.path.isdir(folder):
            for f in m["files"]:
                out.append(("cited", m["name"], folder, os.path.join(folder, f)))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--only")
    ap.add_argument("--jobs", type=int, default=8)
    a = ap.parse_args()
    os.makedirs(BUILD, exist_ok=True)
    os.makedirs(RES, exist_ok=True)
    jobs = jobs_suite() + jobs_sde() + jobs_cited() + jobs_vensim_samples()
    if a.only:
        jobs = [j for j in jobs if a.only in j[1]]
    with ThreadPoolExecutor(a.jobs) as ex:
        rows = list(ex.map(run_one, jobs))
    keys = []
    for r in rows:
        for k in r:
            if k not in keys:
                keys.append(k)
    with open(os.path.join(RES, "b2_models.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=keys)
        w.writeheader()
        w.writerows(rows)
    from collections import Counter
    tot = Counter(r["verdict"] for r in rows)
    print(f"{len(rows)} model files: " + ", ".join(f"{k} {v}" for k, v in tot.most_common()))
    for fmt in ("mdl", "xmile", "stmx"):
        c = Counter(r["verdict"] for r in rows if r["format"] == fmt)
        print(f"  {fmt:6s} " + ", ".join(f"{k} {v}" for k, v in c.most_common()))


if __name__ == "__main__":
    main()
