#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-only
# This file is a derivative work of Edali & Yasarcan's R code (CoMSES codebase 4163, GPL-3.0) and is therefore
# licensed under GPL-3.0 (https://www.gnu.org/licenses/gpl-3.0.html), independently of the rest of this repository.
"""R1 on an exact board-game replica: a line-by-line Python port of Edali & Yasarcan (2014).

Source: "A Mathematical Model of The Beer Game Coded in R for Verification", Mert Edali and Hakan Yasarcan,
CoMSES codebase 4163 v1.0.0 (GPL-3.0), the R code for JASSS 17(4) 2, doi:10.18564/jasss.2555. Equation
numbers in the comments are theirs. Their paper says this code gives "the exact same benchmark cost values
reported by Sterman (1989)". The port is checked against that claim first, then the 11 teams' Table 4
parameters are run through it and compared with Table 2, the same comparison r1_sterman.py makes on the
Dynamist model.

    python3 r1_board.py          # prints the comparison, writes results/r1_board_teams.csv
"""
import csv
import math
import os
import statistics

import ast

HERE = os.path.dirname(os.path.abspath(__file__))

# Take the Table 2 and 4 constants from r1_sterman.py without running it (it runs the Dynamist model at import).
_src = ast.parse(open(os.path.join(HERE, "r1_sterman.py")).read())
_ns = {}
for _node in _src.body:
    if isinstance(_node, ast.Assign) and any(getattr(t, "id", None) in ("T4", "T2", "BENCH", "ST") for t in _node.targets):
        exec(compile(ast.Module([_node], []), "r1_sterman.py", "exec"), _ns)
T4, T2, BENCH, ST = _ns["T4"], _ns["T2"], _ns["BENCH"], _ns["ST"]
WEEKS = 36


def rnd(x):
    return math.floor(max(x, 0) + 0.5)  # round half away from zero on non-negative values, as in their code


def game(p, weeks=WEEKS):
    """p: four (theta, alpha, beta, S') tuples for R, W, D, F. Returns per-stage costs and order series."""
    th = [q[0] for q in p]
    al = [q[1] for q in p]
    be = [q[2] for q in p]
    sp = [q[3] for q in p]
    endcd = lambda t: 4 if t <= 4 else 8  # eq. 7
    I = [12, 12, 12, 12]  # 16
    B = [0, 0, 0, 0]  # 15
    iti1 = [4, 4, 4]  # 17 (R, W, D)
    iti2 = [4, 4, 4]  # 19
    wip1, wip2 = 4, 4  # 18, 20
    ex = [4, 4, 4, 4]  # 8, 9: eecd, eo_r, eo_w, eo_d
    o = [4, 4, 4]  # 21: orders placed for week 1 (R, W, D)
    psr = 4  # 22
    io = [None, 4, 4, 4]  # 23: incoming orders of W, D, F for week 1
    tc = [0.0] * 4  # 24
    orders = [[] for _ in range(4)]
    for t in range(1, weeks + 1):
        # Step 1: receive
        I[0] += iti2[0]; I[1] += iti2[1]; I[2] += iti2[2]; I[3] += wip2  # 27, 28
        iti2 = list(iti1); wip2 = wip1  # 29, 30
        iti1 = [0, 0, 0]; wip1 = 0  # 31, 32
        # Step 2: ship
        dem = [endcd(t), io[1], io[2], io[3]]
        s = [min(I[k], B[k] + dem[k]) for k in range(4)]  # 33-36
        iti1 = [s[1], s[2], s[3]]  # 37: W ships to R, D to W, F to D
        wip1 = psr  # 38
        # Step 3: backlog and inventory
        for k in range(4):
            B[k] = B[k] + dem[k] - s[k]  # 39-42
            I[k] = I[k] - s[k]  # 43-46
        # expectation formation
        for k in range(4):
            ex[k] = ex[k] + th[k] * (dem[k] - ex[k])  # 47-50
        # Step 4: pass orders
        io = [None, o[0], o[1], o[2]]  # 51-53, these are incoming orders for week t+1
        # Step 5: place orders for week t+1
        ei = [I[k] - B[k] for k in range(4)]  # 58
        sl = [io[1] + B[1] + iti1[0] + iti2[0],  # 59
              io[2] + B[2] + iti1[1] + iti2[1],  # 60
              io[3] + B[3] + iti1[2] + iti2[2],  # 61
              wip1 + wip2]  # 62
        new = []
        for k in range(4):
            if t <= 4:
                new.append(4)  # 72-75, first four weeks
            else:
                new.append(rnd(ex[k] + al[k] * (sp[k] - ei[k] - be[k] * sl[k])))  # 72-75, sat = 1/alpha, wsl = beta
        o, psr = new[:3], new[3]
        for k in range(4):
            orders[k].append(new[k])
            tc[k] += B[k] + 0.5 * I[k]  # 69
    return tc, orders


def main():
    opt = [(0, 1, 1, 28), (0, 1, 1, 28), (0, 1, 1, 28), (0, 1, 1, 20)]
    tc, _ = game(opt)
    print("Benchmark (Sterman's optimal parameters):")
    for k, st in enumerate(ST):
        print(f"  {st:12s} {tc[k]:8.1f}   Sterman {BENCH[st]}")
    print(f"  {'team':12s} {sum(tc):8.1f}   Sterman {BENCH['team']}")

    rows = []
    for team, p in T4.items():
        c, o = game(p)
        rows.append({"team": team, **{st: c[k] for k, st in enumerate(ST)}, "total": sum(c),
                     **{f"{st} peak": max(o[k]) for k, st in enumerate(ST)}})
    with open(os.path.join(HERE, "results", "r1_board_teams.csv"), "w", newline="") as f:
        w = csv.DictWriter(f, fieldnames=list(rows[0]))
        w.writeheader()
        w.writerows(rows)
    print("\nThe 11 teams' Table 4 parameters (mean over teams):")
    for st in ST + ["total"]:
        ref = T2["team"] if st == "total" else T2[st]
        print(f"  {st:12s} {statistics.mean(r[st] for r in rows):8.1f}   Sterman {ref}")
    print("  median team cost", round(statistics.median(r["total"] for r in rows), 1))
    for st in ST:
        print(f"  mean peak order {st:12s} {statistics.mean(r[st + ' peak'] for r in rows):5.1f}")


if __name__ == "__main__":
    main()
