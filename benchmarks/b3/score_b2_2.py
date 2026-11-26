#!/usr/bin/env python3
"""Score B2-2 (scenarios c and f) against answer_keys/b2-2-market-growth.yaml. Settings from config/b2-2{c,f}.yaml (frozen)."""
import csv, json, os, subprocess, sys, tempfile
import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC", os.path.join(HERE, "..", "..", "target", "release", "dyn"))


def ident(dom, spec, stocks):
    for l in dom["loops"]:
        v = l["variables"]
        if all(x in v for x in spec.get("contains", [])) and not any(x in v for x in spec.get("excludes", [])) and spec.get("polarity", l["polarity"]) == l["polarity"]:
            return l
    return None


def score(case):
    cfg = yaml.safe_load(open(os.path.join(HERE, "config", f"b2-2{case}.yaml")))
    sc = cfg["scoring"]
    R = os.path.join(HERE, "results", f"b2-2{case}")
    dom = json.load(open(os.path.join(R, "dominance.json")))
    model = os.path.join(HERE, cfg["model"])
    names = json.loads(subprocess.run([FC, "list", model, "--kind", "stock"], capture_output=True, text=True).stdout)
    stock = {v["name"] for v in (names if isinstance(names, list) else names.get("vars", []))}
    is_stock = lambda n: n in stock or " stage " in n
    run = list(csv.reader(open(os.path.join(R, "base_run.csv"))))
    t = [float(r[0]) for r in run[1:]]; y = [float(r[1]) for r in run[1:]]
    ext = [t[i] for i in range(1, len(y) - 1) if (y[i] - y[i - 1]) * (y[i + 1] - y[i]) < 0]
    ours = [b - a for a, b in zip(ext[:-1], ext[1:])]
    paper = sc["paper_half_cycles"]
    n = min(len(ours), len(paper))
    errs = [abs(o - p) / p for o, p in zip(ours[-n:] if case == "c" else ours[:n], paper[:n])] if n else [1]
    out = []
    out.append((f"M1 half-cycle durations of Backlog within {sc['half_cycle_tolerance']:.0%} of the paper", n == len(paper) and max(errs) <= sc["half_cycle_tolerance"],
                f"ours {[round(x, 1) for x in ours]} vs paper {paper}; largest relative difference {max(errs):.1%}"))
    L = {k: ident(dom, v, stock) for k, v in sc["loops"].items()}
    share = lambda l: l["share_of_time_dominant"] if l else float("nan")
    if case == "c":
        higher = [l for l in dom["loops"] if sum(1 for v in l["variables"] if is_stock(v)) >= 3]
        top = max(higher, key=lambda l: l["share_of_time_dominant"]) if higher else None
        dd = L["delivery_delay"]
        out.append(("M2 the third-order delivery delay loop has the largest share among loops with three or more stocks", bool(dd and top and dd["id"] == top["id"]),
                    f"delivery delay loop: id {dd['id'] if dd else None}, share {share(dd):.1%}; strongest of {len(higher)} higher-order loops: id {top['id'] if top else None} ({top['share_of_time_dominant']:.1%})" if top else "no higher-order loop"))
        first = [l for l in dom["loops"] if sum(1 for v in l["variables"] if is_stock(v)) == 1]
        fs = sum(l["share_of_time_dominant"] for l in first)
        out.append((f"M3 first-order loops are dominant at least {sc['first_order_share']:.0%} of the time", fs >= sc["first_order_share"], f"{len(first)} first-order loops, combined share {fs:.1%}"))
    else:
        sg = L["sales_growth"]; best = max(dom["loops"], key=lambda l: l["share_of_time_dominant"])
        out.append(("M5 the sales growth loop (reinforcing, Salesmen and Orders Booked) has the largest share", bool(sg and sg["id"] == best["id"]),
                    f"sales growth loop id {sg['id'] if sg else None} share {share(sg):.1%}; strongest loop id {best['id']} ({best['share_of_time_dominant']:.1%}, {best['polarity']})"))
        dd, ce = L["delivery_delay"], L["capacity_expansion"]
        ok = bool(dd and ce and dd["mean_relative_score"] and abs(dd["mean_relative_score"]) >= 0.01 and abs(ce["mean_relative_score"]) >= 0.01)
        out.append(("M6 delivery delay loop and capacity expansion loop both carry at least 1% of the mean loop score", ok,
                    f"delivery delay {abs(dd['mean_relative_score']) if dd else float('nan'):.3f}, capacity expansion {abs(ce['mean_relative_score']) if ce else float('nan'):.3f}"))
    # M7 (Phaff): fixing the capacity utilisation variable leaves the run virtually identical
    with tempfile.TemporaryDirectory() as d:
        fixed = os.path.join(d, "fixed.sdm")
        subprocess.run([FC, "convert", model, fixed], capture_output=True)
        ops = os.path.join(d, "o.json")
        base = json.loads(subprocess.run([FC, "run", model, "--series", "Production Capacity Fraction PCF9"], capture_output=True, text=True).stdout)
        v0 = base["series"][0]["first"]
        json.dump([{"op": "set_equation", "name": "Production Capacity Fraction PCF9", "equation": str(v0)}], open(ops, "w"))
        subprocess.run([FC, "ops", fixed, ops], capture_output=True)
        r = subprocess.run([FC, "verify", model, fixed, "--ensemble", "20", "--max-nrmse", "0.05"], capture_output=True, text=True)
        v = json.loads(r.stdout)
        worst = max(g["base"]["nrmse"] for g in v["goals"])
        out.append(("M7 fixing the capacity utilisation variable (Production Capacity Fraction) at its initial value leaves the run virtually identical", worst <= 0.05,
                    f"largest base-run NRMSE over the goals {worst:.3f} (utilisation moves from {v0:.3f} to {base['series'][0]['last']:.3f})"))
    # what F1 to F4 produced
    scr = json.load(open(os.path.join(R, "screen.json"))); imp = json.load(open(os.path.join(R, "importance.json")))
    red = json.load(open(os.path.join(R, "reduce.json"))); ver = json.load(open(os.path.join(R, "verify.json")))
    info = {
        "morris_converged": [g["converged"] for g in scr["goals"]],
        "sobol_converged": [g["converged"] for g in imp["sobol"]["goals"]],
        "unscored_loops": dom["unscored_loops"], "loops": dom["loop_count"],
        "reduction": {"variables": [red["before"]["variables"], red["after"]["variables"]], "levers": [red["before"]["levers"], red["after"]["levers"]], "fixed": [c["target"] for c in red["changes"] if c["step"] == "fix"], "pruned": [c["target"] for c in red["changes"] if c["step"] == "prune"]},
        "verification_pass": ver["pass"],
    }
    return out, info


if __name__ == "__main__":
    allrows = []
    for case in sys.argv[1:] or ["c", "f"]:
        rows, info = score(case)
        print(f"== scenario {case.upper()}")
        for name, ok, ev in rows:
            print(("PASS " if ok else "FAIL ") + name + " | " + ev)
            allrows.append((case, name, ok, ev))
        print("info:", json.dumps(info))
        w = csv.writer(open(os.path.join(HERE, "results", f"b2-2{case}", "score.csv"), "w"))
        w.writerow(["finding", "reproduced", "evidence"])
        for name, ok, ev in rows:
            w.writerow([name, ok, ev])
