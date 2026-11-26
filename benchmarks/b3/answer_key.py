#!/usr/bin/env python3
"""Check answer-key files against the rules of the brief (section 4).

Usage: answer_key.py [FILE.yaml ...]   (default: every answer_keys/*.yaml)
Exit status 1 if any entry breaks a rule. Run with the SALib venv's python (needs pyyaml).
"""
import glob
import os
import re
import sys

import yaml

TYPES = {"dominant_loop", "leverage_parameter", "core_structure", "policy", "reduction_size", "behaviour"}
REQUIRED = ["case", "finding", "type", "qualitative", "source", "version"]
# a source must say where in the paper: page, table, figure, section, or equation
LOCATOR = re.compile(r"\b(p\.|pp\.|page|table|tab\.|fig\.|figure|section|sec\.|eq\.|equation|abstract)\s*\S+", re.I)


def check_entry(e, where):
    errs = []
    if not isinstance(e, dict):
        return [f"{where}: entry is not a mapping"]
    for k in REQUIRED:
        if k not in e or e[k] in (None, ""):
            errs.append(f"{where}: missing '{k}'")
    if e.get("type") not in TYPES and e.get("type") not in (None, ""):
        errs.append(f"{where}: type '{e.get('type')}' not one of {sorted(TYPES)}")
    if "qualitative" in e and not isinstance(e["qualitative"], bool):
        errs.append(f"{where}: 'qualitative' must be true or false")
    src = str(e.get("source", ""))
    if src and not LOCATOR.search(src):
        errs.append(f"{where}: source '{src}' names no page, table, figure, section or equation")
    if e.get("qualitative") is True and e.get("value") not in (None, ""):
        errs.append(f"{where}: a qualitative finding must not carry a 'value'")
    if e.get("qualitative") is False and e.get("value") in (None, ""):
        errs.append(f"{where}: a quantitative finding needs a 'value'")
    if "digitised" in e and not isinstance(e["digitised"], bool):
        errs.append(f"{where}: 'digitised' must be true or false")
    if e.get("digitised") is True and e.get("value") in (None, ""):
        errs.append(f"{where}: 'digitised' set but there is no value")
    if re.search(r"fig(ure)?\.?\s*\d", src, re.I) and e.get("value") not in (None, "") and "digitised" not in e:
        errs.append(f"{where}: value cites a figure but does not say whether it was digitised (digitised: true if read off the plot, false if the text or caption states it)")
    return errs


def check_file(path):
    try:
        data = yaml.safe_load(open(path)) or []
    except Exception as ex:  # noqa: BLE001
        return [f"{path}: cannot read ({ex})"]
    if not isinstance(data, list):
        return [f"{path}: top level must be a list of entries"]
    errs = []
    for i, e in enumerate(data):
        errs += check_entry(e, f"{os.path.basename(path)}[{i}]")
    return errs


def main(argv):
    here = os.path.dirname(os.path.abspath(__file__))
    files = argv or sorted(glob.glob(os.path.join(here, "answer_keys", "*.yaml")))
    errs, n = [], 0
    for f in files:
        errs += check_file(f)
        n += 1
    for e in errs:
        print("ERROR", e)
    print(f"{n} file(s), {len(errs)} problem(s)")
    return 1 if errs else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
