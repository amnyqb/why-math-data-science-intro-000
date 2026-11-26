#!/usr/bin/env python3
"""Writes ops.json: an open-access fishery (Gordon 1954) with a quota and a fleet limit as policy levers.
   Usage: python3 make.py && dyn ops fishery-commons.sdm ops.json --no-gate"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "100"), ("time_step", "0.02"), ("time_units", "Year"), ("method", "euler"),
                 ("title", "Fishery commons: open access versus a managed fleet")):
    op(op="set_setting", key=key, value=val)
V = [
 ("Fish", "stock", "900", "tonne", None, None),
 ("Boats", "stock", "20", "boats", None, None),
 ("Fish Growth", "flow", "Growth Rate * Fish * (1 - Fish / Carrying Capacity)", "tonne/Year", None, "Fish"),
 ("Catch", "flow", "MIN(Catchability * Boats * Fish, Quota)", "tonne/Year", "Fish", None),
 ("Revenue", "aux", "Price * Catch", "kUSD/Year", None, None),
 ("Fleet Cost", "aux", "Cost Per Boat * Boats", "kUSD/Year", None, None),
 ("Profit", "aux", "Revenue - Fleet Cost", "kUSD/Year", None, None),
 ("Boat Purchases", "flow", "MIN(MAX(Profit, 0) * Fleet Response, MAX(Fleet Limit - Boats, 0) / TIME STEP)", "boats/Year", None, "Boats"),
 ("Boat Retirements", "flow", "MIN(MAX(-Profit, 0) * Fleet Response, Boats / TIME STEP)", "boats/Year", "Boats", None),
 ("Growth Rate", "constant", "0.5", "1/Year", None, None),
 ("Carrying Capacity", "constant", "1000", "tonne", None, None),
 ("Catchability", "constant", "0.001", "1/boats/Year", None, None),
 ("Price", "constant", "10", "kUSD/tonne", None, None),
 ("Cost Per Boat", "constant", "2", "kUSD/boats/Year", None, None),
 ("Fleet Response", "constant", "0.05", "boats/kUSD", None, None),
 ("Quota", "constant", "100000", "tonne/Year", None, None),
 ("Fleet Limit", "constant", "100000", "boats", None, None),
 ("Max Sustainable Yield", "aux", "Growth Rate * Carrying Capacity / 4", "tonne/Year", None, None),
]
for n, k, e, u, f, t in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u)
    if f: o["from"] = f
    if t: o["to"] = t
    ops.append(o)
for g, vs in (("Fish stock", ["Fish", "Fish Growth", "Catch", "Growth Rate", "Carrying Capacity", "Catchability", "Max Sustainable Yield"]),
              ("Fleet", ["Boats", "Boat Purchases", "Boat Retirements", "Fleet Response"]),
              ("Money", ["Revenue", "Fleet Cost", "Profit", "Price", "Cost Per Boat"]),
              ("Policy levers", ["Quota", "Fleet Limit"])):
    op(op="add_group", name=g, vars=vs)
for v, lo, hi in (("Quota", 0, 300), ("Fleet Limit", 0, 600)):
    op(op="set_lever", var=v, on=True); op(op="set_slider", var=v, min=lo, max=hi)
op(op="set_purpose", problem="Why does a fishery open to everyone end up with a small stock and a big idle fleet, and which rule keeps both the fish and the profit?", kind="design a policy", out_of_scope="several species, fish age classes, price changes, fuel and crew markets")
op(op="add_objective", title="Keep the stock healthy", kind="stay within a band", test="Fish stays above half of Carrying Capacity (500 tonne) at the end of the 100 years.", var="Fish")
op(op="add_objective", title="Keep the fleet profitable", kind="grow to a plateau", test="Profit settles at a positive level instead of falling to zero.", var="Profit")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
