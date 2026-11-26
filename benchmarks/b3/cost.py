#!/usr/bin/env python3
"""Wall-clock time and model runs of each step on each case (4 threads, release build). Writes results/COST.csv."""
import csv, json, os, subprocess, sys, time
import yaml
HERE = os.path.dirname(os.path.abspath(__file__))
FC = os.environ.get("FC", os.path.join(HERE, "..", "..", "target", "release", "dyn"))
rows = [("case", "step", "seconds", "model_runs")]
for case in ["b2-1", "b2-2c", "b2-2f", "b2-3"]:
    cfg = yaml.safe_load(open(os.path.join(HERE, "config", case + ".yaml")))
    model = os.path.join(HERE, cfg["model"])
    for st, sc in cfg["steps"].items():
        if st == "run":
            continue
        args = [FC, st, model]
        for k, v in sc.items():
            if k == "_args":
                args += [os.path.join(HERE, x) if not os.path.isabs(x) else x for x in v]
            elif k in ("csv", "out", "log"):
                args += [f"--{k}", "/tmp/b3_cost_" + os.path.basename(str(v))]
            elif isinstance(v, bool):
                args += [f"--{k.replace('_','-')}"] if v else []
            else:
                args += [f"--{k.replace('_','-')}", str(v)]
        t = time.time()
        r = subprocess.run(args, capture_output=True, text=True, cwd=HERE)
        dt = time.time() - t
        runs = ""
        try:
            d = json.loads(r.stdout)
            runs = d.get("runs") or (d.get("sobol", {}) or {}).get("runs") or ""
            if st == "importance":
                runs = d["sobol"]["runs"]
        except Exception:
            pass
        rows.append((case, st, f"{dt:.2f}", runs))
        print(rows[-1], flush=True)
csv.writer(open(os.path.join(HERE, "results", "COST.csv"), "w")).writerows(rows)
