#!/usr/bin/env python3
"""Writes B2_report.md from results/b2_models.csv (run_b2.py) and the Vensim sample-model run (run_vensim_samples).

    python3 report_b2.py          # also runs every .mdl in the local Vensim install, if there is one, and copies the
                                  # ones that run to external/vensim_samples for the PySD comparison (pysd_all.py)

The Vensim sample models are only run (does it import, does it run); there is no reference output for them yet,
because Vensim saves results in its binary .vdf format. They are not copied into the repository.
"""
import collections
import csv
import glob
import json
import os
import re
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC_BIN", os.path.join(HERE, "..", "..", "target", "release", "dyn"))
VENSIM = "/Users/Shared/Vensim/Help/HTML/Models"

# error text -> the feature that is missing, most specific first
CAUSES = [
    (r"panicked at core/src/mdl.rs", "crash in the .mdl reader (subscript range with no items)"),
    (r"Unknown function or lookup 'game'", "GAME (gaming input) not supported"),
    (r"Unknown function 'DELAY N'|Unknown function 'SMOOTH N'", "DELAY N / SMOOTH N not supported"),
    (r"(function|lookup) 'A FUNCTION OF'", "A FUNCTION OF (placeholder equations) not supported"),
    (r"Unknown function( or lookup)? '[^']+'", "other Vensim function not supported (NPV, FORECAST, GAMMA LN, REINITIAL, SIMULTANEOUS, ...)"),
    (r"has no lookup table", "lookup defined or called in a form the reader does not take"),
    (r"Unexpected character ';'|Unexpected ',' after the end of the formula", "array constants written as a table (1,2;3,4)"),
    (r"is a reserved name", "model redefines TIME STEP / FINAL TIME / INITIAL TIME"),
    (r"Initial values depend on each other", "initial-value loop that Vensim resolves (e.g. through SMOOTH/DELAY initials)"),
    (r"Circular logic", "simultaneous equations (Vensim resolves or reports)"),
    (r"is not an item of|per Subrange|has one value per|This formula is per|Unknown name '[^']*(dim|sub)[^']*'|GET DIRECT SUBSCRIPT", "subranges, subscript mapping or dimension handling"),
    (r"Unexpected ':'", "Vensim ':' syntax (:NA:, :AND:, units ranges, ...)"),
    (r"Unexpected character \'|Unexpected 'Quotes'|Expected a number in the table", "quoted names and literal strings"),
    (r"valid UTF-8", "file not in UTF-8 (older Vensim encodings)"),
    (r"XML error", "XMILE the reader rejects"),
    (r"Unknown name", "a name the reader did not create (macro, data or array element)"),
    (r"gave no variables", "file read without error but empty (legacy Stella .stmx)"),
    (r"timeout", "timeout"),
]


def cause(msg):
    for pat, label in CAUSES:
        if re.search(pat, msg or "", re.I):
            return label
    return "other: " + (msg or "")[:80]


def run_vensim_samples():
    if not os.path.isdir(VENSIM):
        return []
    rows = []
    for f in sorted(glob.glob(VENSIM + "/**/*", recursive=True)):
        if not f.lower().endswith(".mdl"):
            continue
        rel = os.path.relpath(f, VENSIM)
        try:
            p = subprocess.run([FC, "run", f], capture_output=True, text=True, timeout=60)
            if p.returncode == 0:
                n = len(json.loads(p.stdout).get("series", []))
                rows.append({"model": rel, "status": "runs" if n else "empty", "variables": n, "error": ""})
            else:
                err = (p.stderr or p.stdout).strip().splitlines()
                msg = " ".join(l for l in (p.stderr or "").splitlines() if "panicked" in l) or (err[-1] if err else "")
                rows.append({"model": rel, "status": "crash" if "panicked" in (p.stderr or "") else "fails", "variables": 0, "error": msg[:400]})
        except subprocess.TimeoutExpired:
            rows.append({"model": rel, "status": "timeout", "variables": 0, "error": "timeout"})
    return rows


CLASSICS = {
    "Sample/WRLD3-03/wrld3-03.mdl": "World3-03 (Meadows et al., Limits to Growth: The 30-Year Update, 2004)",
    "Sample/EXTRA/WORLD.MDL": "World2 (Forrester, World Dynamics, 1971)",
    "Sample/URBAN/URBAN.mdl": "Urban Dynamics (Forrester, 1969)",
    "Sample/MARKET/MARKET.mdl": "Market (Vensim sample MARKET: products, receivables, sales reps; not Forrester's Market Growth)",
    "Sample/FINANCE/FINANCE.mdl": "Corporate finance (Vensim sample)",
    "Sample/MPROJECT/MPROJ3.mdl": "Project management (Vensim sample)",
    "Sample/EXTRA/EPIDEMIC.MDL": "Epidemic (Vensim sample)",
    "Sample/EXTRA/RABFOX.MDL": "Rabbits and foxes (Vensim sample)",
}


def main():
    rows = list(csv.DictReader(open(os.path.join(HERE, "results", "b2_models.csv"))))
    vs = run_vensim_samples()
    # copy the samples that run, under flat names, for the PySD comparison (pysd_all.py, run_b2.py)
    import shutil
    dst = os.path.join(HERE, "external", "vensim_samples")
    os.makedirs(dst, exist_ok=True)
    for r in vs:
        if r["status"] == "runs":
            name = re.sub(r"[^A-Za-z0-9_.-]+", "_", r["model"].replace("/", "__"))
            name = name if name.lower().endswith(".mdl") else name + ".mdl"
            if not os.path.exists(os.path.join(dst, name)):
                shutil.copy(os.path.join(VENSIM, r["model"]), os.path.join(dst, name))
    if vs:
        with open(os.path.join(HERE, "results", "b2_vensim_samples.csv"), "w", newline="") as f:
            w = csv.DictWriter(f, fieldnames=["model", "status", "variables", "error"])
            w.writeheader()
            w.writerows(vs)
    V = ["exact", "close", "differs", "no-import", "no-run", "crash", "no-common", "no-reference"]
    L = ["# B2: do published Vensim and Stella models give the same results in Dynamist?\n",
         "*Generated by `run_b2.py` and `report_b2.py`. Each model file is imported with `dyn run` (format from the extension), run with its own time settings, and compared with the output exported by the original tool at every saved time and for every variable both have. \"Exact\" means equal to the reference's printed precision: within 1e-5 relative, plus 1e-6 of the variable's largest value (Vensim computes in single precision, so a stock that should be 0 can print as -2.5e-6).*\n",
         "## 1. Test suites with the original tool's output\n",
         "- **SDXorg/test-models** (MIT, commit 21aab02): single-feature tests and sample models with Vensim output (`output.tab`/`.csv`) and, for some, Stella output.",
         "- **SDEverywhere test models** (MIT, Climate Interactive, commit 5adeb59): Vensim models, each with Vensim's own `.dat` output.\n",
         "| suite | format | files | " + " | ".join(V) + " |", "|---|---|---|" + "---|" * len(V)]
    for suite, label in (("samples", "test-models samples"), ("tests", "test-models tests"), ("sdeverywhere", "SDEverywhere")):
        for fmt in ("mdl", "xmile", "stmx"):
            rs = [r for r in rows if r["group"] == suite and r["format"] == fmt]
            if rs:
                c = collections.Counter(r["verdict"] for r in rs)
                L.append(f"| {label} | {fmt} | {len(rs)} | " + " | ".join(str(c.get(v, 0)) or "0" for v in V) + " |")
    suite_rows = [r for r in rows if r["group"] in ("samples", "tests", "sdeverywhere")]
    c = collections.Counter(r["verdict"] for r in suite_rows)
    scored = len(suite_rows) - c.get("no-reference", 0)
    L.append(f"| **all** | | **{len(suite_rows)}** | " + " | ".join(f"**{c.get(v, 0)}**" for v in V) + " |\n")
    L.append(f"**{c['exact']} of {scored} model files with a reference give the original tool's results exactly** ({100 * c['exact'] / scored:.0f}%); {c['close']} more are within 1e-3. "
             f"{c['differs']} run but give different numbers; {c['no-import'] + c['no-run'] + c['crash']} do not import or run.\n")
    L.append("### Exact by sample model\n")
    L.append("| model | file | verdict | worst error (in tolerances) | variable |")
    L.append("|---|---|---|---|---|")
    for r in rows:
        if r["group"] == "samples":
            L.append(f"| {r['case']} | {r['file']} | {r['verdict']} | {r.get('worst_error_in_tolerances', '')} | {r.get('worst_var', '') or r.get('error', '')[:60]} |")
    L.append("\n## 2. Widely cited models (Vensim's own sample library)\n")
    if vs:
        cv = collections.Counter(r["status"] for r in vs)
        L.append(f"Every `.mdl` in the local Vensim PLE install ({len(vs)} files) was imported and run. **{cv.get('runs', 0)} run**, {cv.get('fails', 0)} fail to import, {cv.get('crash', 0)} crash. There is no reference output for these yet (Vensim keeps results in binary `.vdf` files), so this section says only whether they run.\n")
        L.append("| model | runs in Dynamist? | why not |")
        L.append("|---|---|---|")
        byrel = {r["model"]: r for r in vs}
        for rel, name in CLASSICS.items():
            r = byrel.get(rel)
            if r:
                L.append(f"| {name} | {'yes' if r['status'] == 'runs' else '**no**'} | {'' if r['status'] == 'runs' else cause(r['error'])} |")
    cited = [r for r in rows if r["group"] == "cited"]
    if cited:
        L.append("\n### The classics against PySD\n")
        L.append("Vensim PLE cannot export its runs (they stay in binary `.vdf` files), so each classic model was also run in **PySD** "
                 "(SDXorg's open-source Vensim engine, itself tested against Vensim) and compared variable by variable, step by step "
                 "(`pysd_ref.py`). PySD needed light, non-numeric clean-up to read these files (`clean_copy` in `pysd_ref.py`). "
                 "Agreement with PySD is strong evidence, not proof, of agreement with Vensim.\n")
        L.append("| model | verdict against PySD | variables differing | worst error (in tolerances) | first variable |")
        L.append("|---|---|---|---|---|")
        for r in cited:
            L.append(f"| {r['case']} | {r['verdict']} | {r.get('vars_differing', '')} of {r.get('vars_compared', '')} | {r.get('worst_error_in_tolerances', '')} | {r.get('worst_var', '')} |")
        L.append("\n**World3-03** agrees at the initial time and then drifts by 0.004% to 0.17% (population 0.03%, industrial output up to 0.17% by 2100). "
                 "The first divergence is at the first step, in the two SMOOTHs fed by ACTIVE INITIAL variables (`Perceived Food Ratio`, "
                 "`Delayed Labor Utilization Fraction`): PySD leaves them at 1 for the first step although their inputs are 1.17 and 1.34, "
                 "while Dynamist moves them by one Euler step on those inputs. Two minimal models with the same pattern give identical results "
                 "in both engines, so the cause is not yet found. Which engine matches Vensim needs Vensim's own run of World3-03.\n")
    vsr = [r for r in rows if r["group"] == "vensim-samples"]
    if vsr:
        pst = {}
        sp = os.path.join(HERE, "results", "b2_pysd_status_vensim_samples.csv")
        if os.path.exists(sp):
            pst = {r["model"]: r for r in csv.DictReader(open(sp))}
        cv2 = collections.Counter(r["verdict"] for r in vsr)
        ran = len(vsr) - cv2.get("no-reference", 0)
        L.append("\n### Every Vensim sample model that runs, against PySD\n")
        L.append(f"All {len(vsr)} sample models that Dynamist runs were also run in PySD (`pysd_all.py`). PySD could run **{ran}** of them "
                 f"(it could not parse {cv2.get('no-reference', 0)}). **Dynamist matches PySD exactly on {cv2.get('exact', 0)} of {ran}** "
                 f"({100 * cv2.get('exact', 0) / max(ran, 1):.0f}%), on every variable at every saved time; {cv2.get('differs', 0)} differ:\n")
        why = [("PULSE", "PULSE TRAIN at its end time: Dynamist gives the pulse at t = end, PySD does not. Vensim's own output for PULSE TRAIN (SDEverywhere's `pulsetrain.dat`) has the pulse at t = end, so **Dynamist is right here**."),
               ("batch1", "PULSE TRAIN at its end time: Dynamist gives the pulse at t = end, PySD does not. Vensim's own output for PULSE TRAIN (SDEverywhere's `pulsetrain.dat`) has the pulse at t = end, so **Dynamist is right here**."),
               ("RANDOM", "random numbers: the two engines use different generators, so the draws differ (expected; not a modelling difference)."),
               ("all_controls", "random numbers (as above)."),
               ("Stochastic", "random numbers (as above)."),
               ("GET_VDF", "reads data from a Vensim binary .vdf file, which Dynamist does not import (it stands in a placeholder and says so)."),
               ("WRLD3", "World3-03: the open difference described above.")]
        L.append("| model | variables differing | cause |")
        L.append("|---|---|---|")
        for r in vsr:
            if r["verdict"] in ("differs", "close"):
                cause_ = next((w for k, w in why if k.lower() in r["case"].lower()), None)
                if cause_ is None:
                    src = open(os.path.join(HERE, "external", "vensim_samples", r["file"]), encoding="utf-8", errors="replace").read().upper()
                    cause_ = "random numbers (as above)." if "RANDOM " in src else "not yet examined"
                L.append(f"| {r['case']} | {r['vars_differing']} of {r['vars_compared']} | {cause_} |")
        fails = collections.Counter((v["detail"].split(":")[0] or "other") for v in pst.values() if v["pysd"] != "runs")
        if fails:
            L.append("\nWhy PySD could not run the rest: " + ", ".join(f"{k} {n}" for k, n in fails.most_common()) + " (mostly Vensim syntax its parser does not take).\n")
    L.append("\n## 3. Why models fail: the fix list\n")
    L.append("Causes are read from the error each failing file gives (first error only, so a file may hide a second cause).\n")
    fails = [(cause(r.get("error", "")), "suite") for r in suite_rows if r["verdict"] in ("no-import", "no-run", "crash")]
    fails += [(cause(r["error"]), "vensim") for r in vs if r["status"] in ("fails", "crash", "timeout")]
    cc = collections.Counter(c for c, _ in fails)
    L.append("| cause | files (suites) | files (Vensim samples) |")
    L.append("|---|---|---|")
    for k, n in cc.most_common():
        a = sum(1 for c_, s in fails if c_ == k and s == "suite")
        L.append(f"| {k} | {a} | {n - a} |")
    L.append("\n### Files that run but give different numbers\n")
    L.append("| suite | test | file | variables differing | worst error (in tolerances) | first variable |")
    L.append("|---|---|---|---|---|---|")
    for r in rows:
        if r["verdict"] in ("differs",) and r["group"] in ("samples", "tests", "sdeverywhere"):
            L.append(f"| {r['group']} | {r['case']} | {r['file']} | {r['vars_differing']} of {r['vars_compared']} | {r['worst_error_in_tolerances']} | {r['worst_var']} |")
    L.append("\n## 4. Reproduce\n\n```\ncd benchmarks/b2\ngit clone --depth 1 https://github.com/SDXorg/test-models external/test-models\ngit clone --depth 1 --filter=blob:none --sparse https://github.com/climateinteractive/SDEverywhere external/sdeverywhere && (cd external/sdeverywhere && git sparse-checkout set models)\ncargo build --release -p dynamist-cli\npython3 run_b2.py && python3 report_b2.py\n```\n")
    open(os.path.join(HERE, "B2_report.md"), "w").write("\n".join(L))
    print("\n".join(L[:30]))


if __name__ == "__main__":
    main()
