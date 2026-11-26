#!/usr/bin/env python3
"""Writes ops.json: predator and prey (Lotka 1925, Volterra 1926) with an optional logistic prey limit.
   Usage: python3 make.py && dyn ops predator-prey.sdm ops.json --no-gate"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "60"), ("time_step", "0.001"), ("time_units", "Year"), ("method", "euler"),
                 ("title", "Predator and prey: Lotka-Volterra cycles")):
    op(op="set_setting", key=key, value=val)
V = [
 ("Prey", "stock", "40", "prey", None, None),
 ("Predators", "stock", "15", "predators", None, None),
 ("Prey Births", "flow", "Prey Growth Rate * Prey * (1 - Logistic On * Prey / Prey Capacity)", "prey/Year", None, "Prey"),
 ("Prey Eaten", "flow", "Attack Rate * Prey * Predators", "prey/Year", "Prey", None),
 ("Predator Births", "flow", "Conversion Rate * Prey * Predators", "predators/Year", None, "Predators"),
 ("Predator Deaths", "flow", "Predator Death Rate * Predators", "predators/Year", "Predators", None),
 ("Prey Growth Rate", "constant", "0.8", "1/Year", None, None),
 ("Attack Rate", "constant", "0.04", "1/predators/Year", None, None),
 ("Conversion Rate", "constant", "0.01", "1/prey/Year", None, None),
 ("Predator Death Rate", "constant", "0.6", "1/Year", None, None),
 ("Logistic On", "constant", "0", "1", None, None),
 ("Prey Capacity", "constant", "200", "prey", None, None),
]
for n, k, e, u, f, t in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u)
    if f: o["from"] = f
    if t: o["to"] = t
    ops.append(o)
for g, vs in (("Prey", ["Prey", "Prey Births", "Prey Eaten", "Prey Growth Rate", "Attack Rate"]),
              ("Predators", ["Predators", "Predator Births", "Predator Deaths", "Conversion Rate", "Predator Death Rate"]),
              ("Logistic option", ["Logistic On", "Prey Capacity"])):
    op(op="add_group", name=g, vars=vs)
for v, lo, hi in (("Logistic On", 0, 1), ("Prey Capacity", 100, 1000), ("Attack Rate", 0.02, 0.08)):
    op(op="set_lever", var=v, on=True); op(op="set_slider", var=v, min=lo, max=hi)
op(op="set_purpose", problem="Why do predator and prey numbers rise and fall in endless cycles, and what changes when the prey also crowd themselves?", kind="understand a mechanism", out_of_scope="age structure, space, several prey species")
op(op="add_objective", title="Keep both populations alive", kind="stay within a band", test="Prey and Predators stay above zero in the 60 years (they cycle, they do not die out).", var="Predators")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
