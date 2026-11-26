#!/usr/bin/env python3
"""Compare two arms of an sd-ai experiment (Dynamist tools off and on) from their full-results files.

    scripts/sdai-compare.py --off OFF1.json OFF2.json OFF3.json --on ON1.json ON2.json ON3.json [--sdai-commit SHA]

Each file is the `<id>_<experiment>_full_results.json` that `npm run evals` writes (plain or gzipped). One file is one pass; the
arms are the pooled passes. Prints Markdown: per category and group, the pass rate of each arm with a Wilson 95% interval, the
difference between the arms with a 95% interval (Newcombe's method), and the failures by type. Losses (the arm with the tools doing
worse) come first. Costs are summed from the files. Standard library only; the output depends on the files and nothing else.

A file that holds any `HeldOut` group is refused: this comparison is for development groups only.
"""
import argparse
import gzip
import json
import math
import sys
from collections import Counter, defaultdict

Z = 1.959963984540054


def load(path, drop_eval_errors=False):
    opener = gzip.open if path.endswith(".gz") else open
    with opener(path, "rt", encoding="utf-8") as f:
        data = json.load(f)
    rows = data["results"] if isinstance(data, dict) else data
    held = sorted({r.get("group", "") for r in rows if "HeldOut" in r.get("group", "")})
    if held:
        sys.exit(f"{path}: holds held-out groups ({', '.join(held)}); this comparison is for development groups only")
    if drop_eval_errors:
        rows = [r for r in rows if not any(f.get("type") == "Evaluation error" for f in r.get("failures") or [])]
    return rows


def wilson(k, n):
    if n == 0:
        return (float("nan"), float("nan"))
    p = k / n
    d = 1 + Z * Z / n
    c = (p + Z * Z / (2 * n)) / d
    h = Z * math.sqrt(p * (1 - p) / n + Z * Z / (4 * n * n)) / d
    return (max(0.0, c - h), min(1.0, c + h))


def newcombe(k1, n1, k2, n2):
    """95% interval for p2 - p1 (Newcombe's hybrid score method)."""
    if n1 == 0 or n2 == 0:
        return (float("nan"), float("nan"))
    p1, p2 = k1 / n1, k2 / n2
    l1, u1 = wilson(k1, n1)
    l2, u2 = wilson(k2, n2)
    d = p2 - p1
    return (d - math.sqrt((p2 - l2) ** 2 + (u1 - p1) ** 2), d + math.sqrt((u2 - p2) ** 2 + (p1 - l1) ** 2))


def pct(x):
    return "n/a" if x != x else f"{100 * x:.1f}%"


def pp(x):
    return "n/a" if x != x else f"{100 * x:+.1f}"


def tally(passes):
    """{(category, group): [k, n]} and failure counts and costs over a list of passes (each a list of rows)."""
    t = defaultdict(lambda: [0, 0])
    fails = defaultdict(Counter)
    cost = 0.0
    per_test = defaultdict(lambda: [0, 0])
    tools = Counter()
    rows_n = 0
    for rows in passes:
        for r in rows:
            key = (r["category"], r["group"])
            rows_n += 1
            for call in (r.get("generatedResponse") or {}).get("toolLog") or []:
                # Merlin's calls are logged as dynamist_check, Socrates's as client_dynamist_check
                name = str(call.get("toolName", "")).replace("client_", "", 1)
                if name.startswith("dynamist_"):
                    tools[name] += 1
            if any(str(c.get("toolName", "")).replace("client_", "", 1).startswith("dynamist_") for c in (r.get("generatedResponse") or {}).get("toolLog") or []):
                tools["rows with a call"] += 1
            t[key][1] += 1
            if r.get("pass"):
                t[key][0] += 1
            for f in r.get("failures") or []:
                fails[key][f.get("type", "unknown")] += 1
            c = r.get("cost") or {}
            if not c.get("reusedGeneration"):
                cost += c.get("total") or 0.0
            tk = (r.get("engineConfigName", ""), r["category"], r["group"], r.get("name", ""))
            per_test[tk][1] += 1
            per_test[tk][0] += 1 if r.get("pass") else 0
    tools["rows"] = rows_n
    return t, fails, cost, per_test, tools


def render(off_paths, on_paths, sdai_commit, drop_eval_errors=False):
    off = [load(p, drop_eval_errors) for p in off_paths]
    on = [load(p, drop_eval_errors) for p in on_paths]
    to, fo, co, po, uo = tally(off)
    tn, fn, cn, pn, un = tally(on)
    out = ["# With and without the Dynamist tools", ""]
    out.append(f"sd-ai commit: {sdai_commit or 'not given'}")
    out.append(f"Passes: {len(off)} without the tools, {len(on)} with them. Pass rates pool the passes; the intervals treat every pass of every test as an independent trial, which they are not (a test repeated three times is not three tests), so read them as optimistic.")
    if drop_eval_errors:
        out.append("Rows whose grading failed (failure type \"Evaluation error\": the judge model could not be reached) are left out of every table below, so the arms may have different numbers of rows.")
    out.append("")
    keys = sorted(set(to) | set(tn))
    rows = []
    for key in keys:
        (k1, n1), (k2, n2) = to.get(key, [0, 0]), tn.get(key, [0, 0])
        lo, hi = newcombe(k1, n1, k2, n2)
        d = (k2 / n2 if n2 else float("nan")) - (k1 / n1 if n1 else float("nan"))
        rows.append((d, key, (k1, n1), (k2, n2), (lo, hi)))
    rows.sort(key=lambda r: (r[0] != r[0], r[0], r[1]))  # the biggest losses first
    out.append("## Pass rate by category and group (losses first)")
    out.append("")
    out.append("| Category | Group | Without: passed | Without: rate (Wilson 95%) | With: passed | With: rate (Wilson 95%) | Difference, points (95%) |")
    out.append("|---|---|---|---|---|---|---|")
    for d, (cat, grp), (k1, n1), (k2, n2), (lo, hi) in rows:
        a, b = wilson(k1, n1), wilson(k2, n2)
        out.append(f"| {cat} | {grp} | {k1} of {n1} | {pct(k1 / n1 if n1 else float('nan'))} ({pct(a[0])} to {pct(a[1])}) | {k2} of {n2} | {pct(k2 / n2 if n2 else float('nan'))} ({pct(b[0])} to {pct(b[1])}) | {pp(d)} ({pp(lo)} to {pp(hi)}) |")
    out.append("")
    # tests that got worse with the tools
    worse, better = [], []
    for tk in sorted(set(po) | set(pn)):
        (a, na), (b, nb) = po.get(tk, [0, 0]), pn.get(tk, [0, 0])
        if na and nb:
            d = b / nb - a / na
            (worse if d < 0 else better if d > 0 else []).append((d, tk, f"{a}/{na}", f"{b}/{nb}"))
    out.append("## Tests that changed")
    out.append("")
    out.append(f"{len(worse)} test(s) did worse with the tools, {len(better)} did better.")
    out.append("")
    if worse or better:
        out.append("| Change | Engine config | Group | Test | Without | With |")
        out.append("|---|---|---|---|---|---|")
        for d, (cfg, cat, grp, name), x, y in sorted(worse) + sorted(better, reverse=True):
            out.append(f"| {d * 100:+.0f} points | {cfg} | {grp} | {name.replace('|', '/')} | {x} | {y} |")
        out.append("")
    out.append("## Failures by type")
    out.append("")
    out.append("| Category | Group | Failure type | Without | With |")
    out.append("|---|---|---|---|---|")
    any_fail = False
    for key in keys:
        for ftype in sorted(set(fo.get(key, {})) | set(fn.get(key, {}))):
            any_fail = True
            out.append(f"| {key[0]} | {key[1]} | {ftype.replace('|', '/')} | {fo.get(key, {}).get(ftype, 0)} | {fn.get(key, {}).get(ftype, 0)} |")
    if not any_fail:
        out.append("| none | | | | |")
    out.append("")
    out.append("## Use of the Dynamist tools")
    out.append("")
    out.append("| Arm | Result rows | Rows whose agent called a Dynamist tool | Calls to dynamist_check | Calls to dynamist_extreme_tests |")
    out.append("|---|---|---|---|---|")
    for label, u in (("Without", uo), ("With", un)):
        out.append(f"| {label} | {u['rows']} | {u['rows with a call']} | {u['dynamist_check']} | {u['dynamist_extreme_tests']} |")
    out.append("")
    out.append("Rows that share one generation (the tests of one case) share its tool calls, so a call counts once for each of those rows.")
    out.append("")
    out.append("## Cost")
    out.append("")
    out.append(f"Without the tools: ${co:.2f}. With the tools: ${cn:.2f}. (Rows that reused a sibling's generation are not counted twice.)")
    return "\n".join(out) + "\n"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--off", nargs="+", required=True, help="full-results files of the arm without the tools, one per pass")
    ap.add_argument("--on", nargs="+", required=True, help="full-results files of the arm with the tools, one per pass")
    ap.add_argument("--drop-eval-errors", action="store_true", help="leave out rows whose grading failed (failure type \"Evaluation error\")")
    ap.add_argument("--sdai-commit", help="the sd-ai commit both arms ran at (recorded in the output)")
    a = ap.parse_args()
    sys.stdout.write(render(a.off, a.on, a.sdai_commit, a.drop_eval_errors))


if __name__ == "__main__":
    main()
