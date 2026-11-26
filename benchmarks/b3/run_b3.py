#!/usr/bin/env python3
"""B3 runner. Freezes a case's configuration, then runs its steps through the `dyn` command.

  run_b3.py CASE [--step NAME ...] [--refreeze "reason"]

config/CASE.yaml names the model, and the settings of each step. The first run writes
results/CASE/CONFIG.sha256. Any later run with a different config is refused (rule: freeze before
scoring); --refreeze records the reason in results/CASE/FREEZE_LOG.md and replaces the hash, and
all results of the case must then be regenerated (they are deleted from the case folder).
Steps are commands of `dyn` with arguments taken from the config: screen, importance, reduce, ...
"""
import argparse
import datetime
import hashlib
import json
import os
import shlex
import subprocess
import sys

import yaml

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
FC = os.environ.get("FC", os.path.join(REPO, "target", "release", "dyn"))


def sha(path):
    return hashlib.sha256(open(path, "rb").read()).hexdigest()


def freeze(case, cfg_path, refreeze=None):
    out = os.path.join(HERE, "results", case)
    os.makedirs(out, exist_ok=True)
    hp = os.path.join(out, "CONFIG.sha256")
    now = sha(cfg_path)
    if os.path.exists(hp):
        old = open(hp).read().strip()
        if old != now:
            if not refreeze:
                sys.exit(f"REFUSED: config/{case}.yaml changed after it was frozen ({old[:12]} -> {now[:12]}). "
                         "Results must be regenerated; pass --refreeze \"reason\" to accept and log it.")
            with open(os.path.join(out, "FREEZE_LOG.md"), "a") as f:
                f.write(f"- {datetime.date.today()}: refrozen {old[:12]} -> {now[:12]}: {refreeze}\n")
            for fn in os.listdir(out):
                if fn.endswith((".csv", ".json")):
                    os.unlink(os.path.join(out, fn))
            open(hp, "w").write(now + "\n")
            print(f"refrozen ({refreeze}); earlier results removed")
    else:
        open(hp, "w").write(now + "\n")
        print(f"frozen config {now[:12]}")
    return out


def provenance(out, cfg_path):
    """Record what produced the results: commit, binary hash, config hash, key hashes (red-team finding A3)."""
    def sh(cmd):
        try:
            return subprocess.run(cmd, capture_output=True, text=True, cwd=REPO).stdout.strip()
        except Exception:
            return ""
    keys = {}
    kd = os.path.join(HERE, "keys")
    for root, _, files in os.walk(kd):
        for f in files:
            if f.endswith(".yaml"):
                keys[os.path.relpath(os.path.join(root, f), HERE)] = sha(os.path.join(root, f))[:16]
    info = {"date": str(datetime.datetime.now()), "git_commit": sh(["git", "rev-parse", "HEAD"]), "git_dirty": bool(sh(["git", "status", "--porcelain", "core", "cli"])),
            "fc": FC, "fc_sha256": sha(FC)[:16] if os.path.exists(FC) else "", "config_sha256": sha(cfg_path)[:16], "keys": keys}
    json.dump(info, open(os.path.join(out, "PROVENANCE.json"), "w"), indent=1)


def args_of(step_cfg):
    a = []
    for k, v in (step_cfg or {}).items():
        if k == "_args":            # extra positional arguments, e.g. the second model of `verify`
            a += [str(x) for x in v]
        elif isinstance(v, bool):
            if v:
                a.append(f"--{k.replace('_', '-')}")
        elif isinstance(v, list):
            for x in v:
                a += [f"--{k.replace('_', '-')}", str(x)]
        else:
            a += [f"--{k.replace('_', '-')}", str(v)]
    return a


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("case")
    ap.add_argument("--step", action="append")
    ap.add_argument("--refreeze")
    ap.add_argument("--dry", action="store_true", help="freeze and print the commands without running them")
    a = ap.parse_args()
    cfg_path = os.path.join(HERE, "config", a.case + ".yaml")
    cfg = yaml.safe_load(open(cfg_path))
    out = freeze(a.case, cfg_path, a.refreeze)
    if not a.dry:
        provenance(out, cfg_path)
    model = os.path.join(HERE, cfg["model"])
    steps = a.step or list(cfg.get("steps", {}).keys())
    for st in steps:
        if st == "scoring":
            continue
        sc = dict(cfg["steps"][st])
        cmd = [FC, st.split(":")[0], model] + args_of(sc)
        outfile = os.path.join(out, st.replace(":", "_") + ".json")
        print("$", " ".join(shlex.quote(c) for c in cmd), ">", os.path.relpath(outfile, HERE))
        if a.dry:
            continue
        r = subprocess.run(cmd, capture_output=True, text=True, cwd=HERE)
        if r.returncode != 0:
            sys.exit(f"step {st} failed:\n{r.stdout}\n{r.stderr}")
        open(outfile, "w").write(r.stdout)
        try:
            json.loads(r.stdout)
        except ValueError:
            pass


if __name__ == "__main__":
    main()
