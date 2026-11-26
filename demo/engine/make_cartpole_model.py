"""Writes cartpole.sdm: the cart-pole as a dynamist stock-and-flow model.
Four stocks (cart position, cart velocity, pole angle, pole rate), four flows,
and the feedback controller written as ordinary equations."""
import json

consts = {  # name: (value, comment)
    "CartMass": 1.0, "PoleMass": 0.1, "HalfLen": 0.5, "Grav": 9.81, "CartFric": 0.1, "PoleFric": 0.02,
    "kth": 50, "kw": 10, "kx": 2, "kv": 3, "ref": 0, "Fmax": 30,
    "Fuser": 0, "Auto": 1,
    "x0": 0, "v0": 0, "th0": 0.12, "w0": 0,
}
stocks = {"CartX": "x0", "CartV": "v0", "PoleTh": "th0", "PoleW": "w0"}
auxes = {
    "Fraw": "IF THEN ELSE(Auto > 0.5, kx*(CartX-ref) + kv*CartV + kth*PoleTh + kw*PoleW, 0) + Fuser",
    "Force": "MAX(0-Fmax, MIN(Fmax, Fraw))",
    "Tmp": "(Force - CartFric*CartV + PoleMass*HalfLen*PoleW*PoleW*SIN(PoleTh))/(CartMass+PoleMass)",
    "PoleAccel": "(Grav*SIN(PoleTh) - COS(PoleTh)*Tmp)/(HalfLen*(4/3 - PoleMass*COS(PoleTh)*COS(PoleTh)/(CartMass+PoleMass))) - PoleFric*PoleW",
    "CartAccel": "Tmp - PoleMass*HalfLen*PoleAccel*COS(PoleTh)/(CartMass+PoleMass)",
}
flows = {  # name: (equation, stock it feeds)
    "dX": ("CartV", "CartX"), "dV": ("CartAccel", "CartV"),
    "dTh": ("PoleW", "PoleTh"), "dW": ("PoleAccel", "PoleW"),
}

vars_, ids, nid, y = [], {}, 1, 40.0
def add(name, kind, eq, **kw):
    global nid, y
    v = {"id": nid, "name": name, "kind": kind, "equation": eq, "units": "", "comment": "",
         "pos": [300.0, y], "non_negative": False, "from": None, "to": None,
         "from_pt": [0.0, y], "to_pt": [0.0, y]}
    v.update(kw); ids[name] = nid; nid += 1; y += 70.0; vars_.append(v)
for n, val in consts.items(): add(n, "Constant", repr(val))
for n, eq in stocks.items(): add(n, "Stock", eq)
for n, eq in auxes.items(): add(n, "Aux", eq)
for n, (eq, st) in flows.items(): add(n, "Flow", eq, to=ids[st])

model = {"title": "Cart-Pole (inverted pendulum on a cart)",
         "vars": vars_, "links": [], "next_id": nid,
         "settings": {"initial_time": 0.0, "final_time": 0.1, "time_step": 0.005,
                      "time_units": "Second", "method": "RungeKutta4"}}
json.dump(model, open("cartpole.sdm", "w"), indent=1)
print("wrote cartpole.sdm with", len(vars_), "variables")
