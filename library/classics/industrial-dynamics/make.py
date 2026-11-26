#!/usr/bin/env python3
"""Writes ops.json: a three-echelon production-distribution chain (retailer, distributor, factory),
our own formulation of the structure described in Forrester (1961), Industrial Dynamics, ch. 2 and 15-17.
Usage: python3 make.py && dyn ops industrial-dynamics.sdm ops.json --no-gate && dyn validate industrial-dynamics.sdm --strict-units"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "100"), ("time_step", "0.125"), ("time_units", "Week"), ("method", "euler"),
                 ("title", "Industrial dynamics: a production-distribution chain")):
    op(op="set_setting", key=key, value=val)
V = []   # name, kind, equation, units, from, to, doc
def v(name, kind, eq, units, frm=None, to=None, doc=None): V.append((name, kind, eq, units, frm, to, doc))

# shared constants
v("Initial Demand", "constant", "100", "units/Week", doc="Customer demand before the step")
v("Step Size", "constant", "10", "units/Week", doc="Size of the step in customer demand")
v("Step Time", "constant", "5", "Week", doc="When the step happens")
v("Demand Averaging Time", "constant", "4", "Week", doc="Information delay: time over which each stage smooths the orders it receives")
v("Cover Time", "constant", "3", "Week", doc="Desired inventory, in weeks of expected demand")
v("Adjustment Time", "constant", "4", "Week", doc="Time over which a stage closes gaps in inventory and supply line")
v("Supply Line Weight", "constant", "0.5", "1", doc="How much of the supply-line gap a stage corrects (0 = ignores goods on the way, 1 = full)")
v("Order Fill Time", "constant", "1", "Week", doc="Time to work through the backlog when stock is on hand")
v("Retailer Transit Delay", "constant", "2", "Week", doc="Material delay, distributor to retailer")
v("Distributor Transit Delay", "constant", "3", "Week", doc="Material delay, factory to distributor")
v("Production Delay", "constant", "4", "Week", doc="Material delay, production start to finished goods")
v("Customer Orders", "flow", "Initial Demand + STEP(Step Size, Step Time)", "units/Week", None, "Retailer Backlog", "Customer demand with a step")

ECH = [("Retailer", "Customer Orders", "Retailer Transit Delay", "Customer Shipments", "Cumulative Delivered", "Retailer Orders"),
       ("Distributor", "Retailer Orders", "Distributor Transit Delay", "Distributor Dispatch", "Retailer Pipeline", "Distributor Orders"),
       ("Factory", "Distributor Orders", "Production Delay", "Factory Dispatch", "Distributor Pipeline", "Production Starts")]
for X, inc, delay, dispatch, dest, orders in ECH:
    v(f"{X} Inventory", "stock", "Cover Time * Initial Demand", "units", doc="Goods on hand")
    v(f"{X} Backlog", "stock", "Order Fill Time * Initial Demand", "units", doc="Orders received and not yet shipped")
    v(f"{X} Pipeline", "stock", f"{delay} * Initial Demand", "units", doc="Goods ordered and on the way (in production at the factory)")
    v(f"{X} Expected Demand", "aux", f"SMOOTH({inc}, Demand Averaging Time)", "units/Week", doc="Orders received, smoothed (information delay)")
    v(f"{X} Desired Inventory", "aux", f"Cover Time * {X} Expected Demand", "units")
    v(f"{X} Desired Pipeline", "aux", f"{delay} * {X} Expected Demand", "units")
    v(f"{X} Shipments", "aux", f"MIN({X} Backlog, {X} Inventory) / Order Fill Time", "units/Week", doc="Shipments the stage can make now")
    v(f"{X} Receipts", "flow", f"{X} Pipeline / {delay}", "units/Week", f"{X} Pipeline", f"{X} Inventory", "Goods arriving (first-order material delay)")
    v(dispatch, "flow", f"{X} Shipments", "units/Week", f"{X} Inventory", dest, "Goods leaving the stock")
    v(f"{X} Orders Filled", "flow", f"{X} Shipments", "units/Week", f"{X} Backlog", None, "Backlog cleared by the shipment")
    v(orders, "flow",
      f"MAX(0, {X} Expected Demand + ({X} Desired Inventory - {X} Inventory) / Adjustment Time + Supply Line Weight * ({X} Desired Pipeline - {X} Pipeline) / Adjustment Time)",
      "units/Week", None, ("Distributor Backlog" if X == "Retailer" else "Factory Backlog" if X == "Distributor" else "Factory Pipeline"),
      "Order placed on the next stage up (production starts at the factory)")
# the factory's pipeline is its own work in process; the destination of its receipts is its inventory (set above)
v("Cumulative Delivered", "stock", "0", "units", doc="Goods delivered to customers so far")
v("Cumulative Starts", "stock", "0", "units", doc="Production started so far")
v("Starts Tally", "flow", "Production Starts", "units/Week", None, "Cumulative Starts", "Accounting copy of production starts")
v("Amplification", "aux", "Production Starts / Customer Orders", "1", doc="Factory production starts relative to customer demand")
v("Material in System", "aux", "Retailer Inventory + Retailer Pipeline + Distributor Inventory + Distributor Pipeline + Factory Inventory + Factory Pipeline", "units", doc="All goods held or moving")

V.sort(key=lambda x: 0 if x[1] == "stock" else 1)   # stocks first, so flows can name them
for n, k, e, u, f, t, d in V:
    o = dict(op="add_var", name=n, kind=k, equation=e, units=u)
    if f: o["from"] = f
    if t: o["to"] = t
    if d: o["doc"] = d
    ops.append(o)
GROUPS = [("Customer", ["Customer Orders", "Initial Demand", "Step Size", "Step Time"]),
          ("Retailer stock", ["Retailer Inventory", "Retailer Backlog", "Retailer Pipeline", "Retailer Receipts", "Customer Shipments", "Retailer Orders Filled", "Retailer Shipments"]),
          ("Retailer decision", ["Retailer Orders", "Retailer Expected Demand", "Retailer Desired Inventory", "Retailer Desired Pipeline"]),
          ("Distributor stock", ["Distributor Inventory", "Distributor Backlog", "Distributor Pipeline", "Distributor Receipts", "Distributor Dispatch", "Distributor Orders Filled", "Distributor Shipments"]),
          ("Distributor decision", ["Distributor Orders", "Distributor Expected Demand", "Distributor Desired Inventory", "Distributor Desired Pipeline"]),
          ("Factory stock", ["Factory Inventory", "Factory Backlog", "Factory Pipeline", "Factory Receipts", "Factory Dispatch", "Factory Orders Filled", "Factory Shipments"]),
          ("Factory decision", ["Production Starts", "Factory Expected Demand", "Factory Desired Inventory", "Factory Desired Pipeline"]),
          ("Policy", ["Demand Averaging Time", "Cover Time", "Adjustment Time", "Supply Line Weight", "Order Fill Time"]),
          ("Delays", ["Retailer Transit Delay", "Distributor Transit Delay", "Production Delay"]),
          ("Accounting", ["Cumulative Delivered", "Cumulative Starts", "Starts Tally", "Material in System", "Amplification"])]
for g, vs in GROUPS: op(op="add_group", name=g, vars=vs)
for var, lo, hi in (("Supply Line Weight", 0, 1), ("Adjustment Time", 1, 12), ("Demand Averaging Time", 1, 12), ("Cover Time", 1, 6), ("Step Size", 0, 30)):
    op(op="set_lever", var=var, on=True); op(op="set_slider", var=var, min=lo, max=hi)
op(op="set_purpose", problem="Why does a small step in customer demand swing production far more than it swings sales, and what makes the swing smaller?",
   kind="understand behaviour", out_of_scope="prices, capacity limits, shortages that lose customers, real product mix")
op(op="add_objective", title="Calm factory", kind="stay within a band", test="Production starts peak below 125 units/week, about 1.15 times the new customer demand of 110.", var="Production Starts")
op(op="add_objective", title="Customers served", kind="stay within a band", test="Retailer backlog stays near one week of demand.", var="Retailer Backlog")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
