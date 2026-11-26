#!/usr/bin/env python3
"""Run models in PySD and save every variable at every saved time, as a second, independent reference.

    build/pysd-venv/bin/python pysd_ref.py MODEL.mdl [MODEL.mdl ...]     # writes build/pysd/<stem>.csv

PySD (SDXorg, MIT) translates Vensim models to Python and is itself tested against Vensim's output on the SDXorg
test suite. Agreement with PySD is strong evidence, not proof, that Dynamist matches Vensim: it is used for the
classic models whose Vensim output is not available as text (Vensim keeps runs in binary .vdf files, and Vensim PLE
cannot export them).
"""
import os
import sys
import traceback

import pysd

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(HERE, "build", "pysd")


def clean_copy(path):
    """A copy PySD can parse. Only edits that cannot change a number: one {UTF-8} header instead of two, and
    text-only entries (`name :is: 'text'`, Vensim 10 documentation strings) left out. Old DYNAMO-style tables
    `name ( x1..xn, y1..yn )` are rewritten as `name ( (x1,y1), ... )`, which is the same table."""
    import re
    src = open(path, encoding="utf-8", errors="replace").read().replace("\r\n", "\n").replace("\r", "\n")
    if "{UTF-8}" in src:
        src = "{UTF-8}\n" + src.replace("{UTF-8}", "").lstrip()
    head, sep, sketch = src.partition("\\\\\\---///")
    entries = head.split("|")
    out = []
    for e in entries:
        if re.search(r":is:", e) and "~" in e:
            continue
        m = re.match(r"(\s*[^=(~]+?)\s*\(\s*([-+0-9.eE,\s]+)\)\s*(~[\s\S]*)$", e)
        if m and "=" not in e.split("~")[0]:
            nums = [x for x in re.split(r"[,\s]+", m.group(2).strip()) if x]
            if len(nums) >= 2 and len(nums) % 2 == 0:
                n = len(nums) // 2
                pairs = ",".join(f"({nums[k]},{nums[n + k]})" for k in range(n))
                e = f"{m.group(1)}(\n    {pairs})\n    {m.group(3)}"
        out.append(e)
    dst = os.path.join(OUT, "src", os.path.basename(path))
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    open(dst, "w").write("|".join(out) + sep + sketch)
    return dst


def main():
    os.makedirs(OUT, exist_ok=True)
    for path in sys.argv[1:]:
        stem = os.path.basename(path).rsplit(".", 1)[0]
        try:
            m = pysd.read_vensim(clean_copy(path))
            df = m.run()
            # PySD names columns by the model's own names; keep the time index as the first column
            df.index.name = "Time"
            df = df[[c for c in df.columns if df[c].dtype.kind in "fi"]]
            df.to_csv(os.path.join(OUT, stem + ".csv"))
            print(f"{stem}: {df.shape[1]} variables, {df.shape[0]} times")
        except Exception as e:
            print(f"{stem}: PySD failed: {type(e).__name__}: {str(e).splitlines()[0][:200] if str(e) else ''}")
            traceback.print_exc(limit=1, file=open(os.path.join(OUT, stem + ".error.txt"), "w"))


if __name__ == "__main__":
    main()
