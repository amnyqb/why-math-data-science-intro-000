#!/usr/bin/env python3
"""Writes ops.json for the Bass (1969) diffusion model. Then: dyn ops bass-diffusion.sdm ops.json --no-gate"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "20"), ("time_step", "0.0009765625"), ("time_units", "Year"),
                 ("method", "euler"), ("title", "Bass diffusion: how a new product spreads")):
    op(op="set_setting", key=key, value=val)
# name, kind, equation, units, from, to
V = [
 ("Potential Adopters", "stock", "Market Size", "household", None, None),
 ("Adopters", "stock", "0", "household", None, None),
 ("Adoption", "flow", "Innovator Adoption + Imitator Adoption", "household/Year", "Potential Adopters", "Adopters"),
 ("Innovator Adoption", "aux", "Advertising Effect * Potential Adopters", "household/Year", None, None),
 ("Imitator Adoption", "aux", "Word of Mouth Effect * Adopters * Potential Adopters / Market Size", "household/Year", None, None),
 ("Advertising Effect", "constant", "0.02", "1/Year", None, None),
 ("Word of Mouth Effect", "constant", "0.45", "1/Year", None, None),
 ("Market Size", "constant", "500000", "household", None, None),
 ("Adoption Fraction", "aux", "Adopters / Market Size", "1", None, None),
 ("Imitation Share", "aux", "Imitator Adoption / MAX(Adoption, 0.000001)", "1", None, None),
]
for n, k, e, u, f, t in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u)
    if f: o["from"] = f
    if t: o["to"] = t
    ops.append(o)
for g, vs in (("Adoption engine", ["Potential Adopters", "Adopters", "Adoption"]),
              ("Two sources of adoption", ["Innovator Adoption", "Imitator Adoption", "Imitation Share"]),
              ("Behaviour parameters", ["Advertising Effect", "Word of Mouth Effect"]),
              ("Market", ["Market Size", "Adoption Fraction"])):
    op(op="add_group", name=g, vars=vs)
for v, lo, hi in (("Advertising Effect", 0, 0.1), ("Word of Mouth Effect", 0, 1)):
    op(op="set_lever", var=v, on=True); op(op="set_slider", var=v, min=lo, max=hi)
op(op="set_purpose", problem="How fast does a new household product reach its market, and what shapes the S-curve?",
   kind="understand behaviour", out_of_scope="price, competitors, repeat purchase, supply limits")
op(op="add_objective", title="Reach the market", kind="grow to a plateau",
   test="Adopters follow an S-curve and level off at Market Size (500000 households) within 20 years.", var="Adopters")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
