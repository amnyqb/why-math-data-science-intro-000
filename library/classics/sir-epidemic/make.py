#!/usr/bin/env python3
"""Writes ops.json for the SIR epidemic (Kermack and McKendrick 1927). Then: dyn ops sir-epidemic.sdm ops.json --no-gate"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "300"), ("time_step", "0.01"), ("time_units", "Day"),
                 ("method", "euler"), ("title", "SIR epidemic: one outbreak in a closed town")):
    op(op="set_setting", key=key, value=val)
V = [
 ("Susceptible", "stock", "Population - Initial Infectious", "person", None, None),
 ("Infectious", "stock", "Initial Infectious", "person", None, None),
 ("Recovered", "stock", "0", "person", None, None),
 ("Infection", "flow", "Contact Rate * Infection Chance * Susceptible * Infectious / Population", "person/Day", "Susceptible", "Infectious"),
 ("Recovery", "flow", "Infectious / Infectious Period", "person/Day", "Infectious", "Recovered"),
 ("Population", "constant", "1000000", "person", None, None),
 ("Initial Infectious", "constant", "10", "person", None, None),
 ("Contact Rate", "constant", "10", "1/Day", None, None),
 ("Infection Chance", "constant", "0.05", "1", None, None),
 ("Infectious Period", "constant", "5", "Day", None, None),
 ("Transmission Rate", "aux", "Contact Rate * Infection Chance", "1/Day", None, None),
 ("Basic Reproduction Number", "aux", "Transmission Rate * Infectious Period", "1", None, None),
 ("Effective Reproduction Number", "aux", "Basic Reproduction Number * Susceptible / Population", "1", None, None),
 ("Herd Immunity Threshold", "aux", "MAX(0, 1 - 1 / Basic Reproduction Number)", "1", None, None),
 ("Immune Fraction", "aux", "Recovered / Population", "1", None, None),
 ("Total People", "aux", "Susceptible + Infectious + Recovered", "person", None, None),
]
for n, k, e, u, f, t in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u)
    if f: o["from"] = f
    if t: o["to"] = t
    ops.append(o)
for g, vs in (("Disease states", ["Susceptible", "Infectious", "Recovered"]),
              ("Transitions", ["Infection", "Recovery"]),
              ("Contact lever", ["Contact Rate", "Infection Chance", "Transmission Rate"]),
              ("Herd immunity check", ["Basic Reproduction Number", "Effective Reproduction Number", "Herd Immunity Threshold", "Immune Fraction"]),
              ("Town", ["Population", "Initial Infectious", "Total People"])):
    op(op="add_group", name=g, vars=vs)
for v, lo, hi in (("Contact Rate", 1, 20), ("Infection Chance", 0.01, 0.1)):
    op(op="set_lever", var=v, on=True); op(op="set_slider", var=v, min=lo, max=hi)
op(op="set_purpose", problem="How big does one outbreak get, when does it peak, and how far must contacts fall to stop it?",
   kind="understand behaviour", out_of_scope="births, deaths, waning immunity, vaccination, spatial spread")
op(op="add_objective", title="Keep the peak small", kind="stay within a band",
   test="Infectious stays below 50000 people at every day.", var="Infectious")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
