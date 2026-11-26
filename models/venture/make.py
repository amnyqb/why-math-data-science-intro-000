#!/usr/bin/env python3
"""Writes ops.json: the layered venture model, built with the operations language.
   Usage: python3 make.py && dyn ops venture.sdm ops.json --no-gate && dyn layout venture.sdm --mode tidy"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "60"), ("time_step", "0.25"), ("time_units", "Month"), ("method", "euler"), ("title", "Venture: a young firm that builds capacity")):
    op(op="set_setting", key=key, value=val)
LAYERS = [("World", "Which outside climate do we live in?", None, "stub"),
          ("Capital", "How much money is around, and who gets it?", 24, "built"),
          ("Firm", "Can the firm stay alive?", 36, "built"),
          ("Operations", "How much can we make and sell?", 36, "built")]
for i, (n, q, h, st) in enumerate(LAYERS):
    o = dict(op="add_layer", name=n, order=i, question=q, status=st)
    if h: o["horizon"] = h
    ops.append(o)
# name, kind, equation, units, layer, from, to
V = [
 ("Capital Pool", "stock", "200", "kUSD", "Capital", None, None),
 ("Cash", "stock", "50", "kUSD", "Firm", None, None),
 ("Capacity", "stock", "10", "units/Month", "Operations", None, None),
 ("Capital Climate", "aux", "0.6 + 0.005 * TIME", "1", "World", None, None),
 ("Investment", "flow", "Capital Climate * Traction * Investment Scale", "kUSD/Month", "Capital", None, "Capital Pool"),
 ("Investment Scale", "constant", "5", "kUSD/Month", "Capital", None, None),
 ("Funding Rate", "constant", "0.02", "1/Month", "Capital", None, None),
 ("Funds Available", "aux", "Capital Pool * Funding Rate", "kUSD/Month", "Capital", None, None),
 ("Funding", "flow", "Funds Available", "kUSD/Month", "Capital", "Capital Pool", "Cash"),
 ("Revenue", "flow", "Units Sold * Price", "kUSD/Month", "Firm", None, "Cash"),
 ("Operating Spend", "flow", "Fixed Cost + Variable Cost * Units Sold", "kUSD/Month", "Firm", "Cash", None),
 ("Capacity Spending", "flow", "Capacity Spending Fraction * MAX(Cash, 0) * Capacity Gap", "kUSD/Month", "Firm", "Cash", None),
 ("Price", "constant", "2", "kUSD/units", "Firm", None, None),
 ("Fixed Cost", "constant", "10", "kUSD/Month", "Firm", None, None),
 ("Variable Cost", "constant", "1.2", "kUSD/units", "Firm", None, None),
 ("Capacity Spending Fraction", "constant", "0.3", "1/Month", "Firm", None, None),
 ("Traction Half Point", "constant", "100", "kUSD", "Firm", None, None),
 ("Traction", "aux", "MAX(Cash, 0) / (MAX(Cash, 0) + Traction Half Point)", "1", "Firm", None, None),
 ("Capacity Growth", "flow", "Capacity Spending / Capacity Unit Cost", "units/Month/Month", "Operations", None, "Capacity"),
 ("Capacity Decay", "flow", "Capacity / Capacity Life", "units/Month/Month", "Operations", "Capacity", None),
 ("Capacity Unit Cost", "constant", "4", "kUSD/(units/Month)", "Operations", None, None),
 ("Capacity Life", "constant", "36", "Month", "Operations", None, None),
 ("Market Size", "constant", "60", "units/Month", "Operations", None, None),
 ("Capacity Gap", "aux", "MAX(0, 1 - Capacity / Market Size)", "1", "Operations", None, None),
 ("Utilisation", "constant", "0.8", "1", "Operations", None, None),
 ("Units Sold", "aux", "Capacity * Utilisation", "units/Month", "Operations", None, None),
]
for n, k, e, u, layer, f, t in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u, layer=layer)
    if f: o["from"] = f
    if t: o["to"] = t
    ops.append(o)
GROUPS = [("Investment", ["Capital Pool", "Investment", "Investment Scale"]),
          ("Funding", ["Funds Available", "Funding Rate", "Funding"]),
          ("Cash account", ["Cash", "Revenue", "Operating Spend", "Capacity Spending"]),
          ("Health signal", ["Traction", "Traction Half Point"]),
          ("Constants", ["Price", "Fixed Cost", "Variable Cost", "Capacity Spending Fraction"]),
          ("Capacity build", ["Capacity", "Capacity Growth", "Capacity Decay", "Capacity Unit Cost", "Capacity Life"]),
          ("Market", ["Market Size", "Capacity Gap"]),
          ("Sales", ["Units Sold", "Utilisation"])]
for g, vs in GROUPS: op(op="add_group", name=g, vars=vs)
for v, lo, hi in (("Price", 1, 4), ("Capacity Spending Fraction", 0, 0.5)):
    op(op="set_lever", var=v, on=True); op(op="set_slider", var=v, min=lo, max=hi)
op(op="set_purpose", problem="Can a young firm stay alive while it builds capacity?", kind="design a policy", out_of_scope="competitors, regulation")
op(op="add_objective", title="Stay alive", kind="stay within a band", test="Cash stays above zero in every month of the 60.", var="Cash")
op(op="add_objective", title="Grow", kind="grow to a plateau", test="Capacity rises, then levels off. Plateau near Market Size (60 units/month).", var="Capacity")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
