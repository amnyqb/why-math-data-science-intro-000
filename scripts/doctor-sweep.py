#!/usr/bin/env python3
"""Run `dyn check FILE --gate GATE --json` on every model of core/tests/book_models and collect the reports in one JSON
file keyed by file name, with a table of counts per rule and severity.

    scripts/doctor-sweep.py DYN_BINARY OUT_DIR [GATE]     GATE is review (default) or publish
"""
import json, os, subprocess, sys, collections

dyn, out = sys.argv[1], sys.argv[2]
gate = sys.argv[3] if len(sys.argv) > 3 else "review"
here = os.path.dirname(os.path.abspath(__file__))
models = os.path.join(here, "..", "core", "tests", "book_models")
os.makedirs(out, exist_ok=True)
report, bad = {}, []
for name in sorted(os.listdir(models)):
    if not name.endswith((".mdl", ".flowmodel")):
        continue
    p = subprocess.run([dyn, "check", os.path.join(models, name), "--gate", gate, "--json"], capture_output=True, text=True)
    try:
        report[name] = json.loads(p.stdout)
    except Exception:
        bad.append(name)
        report[name] = {"error": (p.stdout + p.stderr)[-400:]}
json.dump(report, open(os.path.join(out, "doctor_book_models.json"), "w"), indent=1)
counts = collections.defaultdict(lambda: collections.Counter())
for r in report.values():
    for f in r.get("findings", []):
        counts[f["rule"]][("possible" if not f.get("confirmed", True) else f["severity"])] += 1
sev = ["error", "warning", "info", "possible"]
lines = ["| Rule | " + " | ".join(sev) + " | total |", "|---|" + "---|" * (len(sev) + 1)]
tot = collections.Counter()
for rule in sorted(counts, key=lambda r: (r[0], int(r[1:]) if r[1:].isdigit() else 0, r)):
    c = counts[rule]
    lines.append(f"| {rule} | " + " | ".join(str(c[s]) for s in sev) + f" | {sum(c.values())} |")
    tot.update(c)
lines.append("| all | " + " | ".join(str(tot[s]) for s in sev) + f" | {sum(tot.values())} |")
open(os.path.join(out, "counts.md"), "w").write("\n".join(lines) + "\n")
print(f"{len(report)} models, {len(bad)} unreadable: {bad}")
print("\n".join(lines))
