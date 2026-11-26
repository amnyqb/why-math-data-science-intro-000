#!/usr/bin/env python3
"""Writes ops.json: a firm whose sales force, order backlog, delivery delay and production capacity interact.
Our own formulation of the structure described in Forrester (1968), "Market growth as influenced by capital investment",
Industrial Management Review 9(2), 83-105. Three switches turn the feedback loops on and off.
Usage: python3 make.py && dyn ops market-growth.sdm ops.json --no-gate && dyn validate market-growth.sdm --strict-units"""
import json
ops = []
def op(**k): ops.append(k)
for key, val in (("initial_time", "0"), ("final_time", "120"), ("time_step", "0.125"), ("time_units", "Month"), ("method", "euler"),
                 ("title", "Market growth and capital investment")):
    op(op="set_setting", key=key, value=val)
V = []
def v(name, kind, eq, units, frm=None, to=None, doc=None): V.append((name, kind, eq, units, frm, to, doc))

# constants
v("Initial Capacity", "constant", "100", "units/Month", doc="Production capacity at the start (equals orders at the start)")
v("Target Delivery Delay", "constant", "3", "Month", doc="Delivery delay the firm and its customers regard as normal")
v("Minimum Delivery Time", "constant", "1", "Month", doc="Shortest time from order to shipment when capacity is ample")
v("Initial Sales Force", "constant", "20", "Person", doc="Salesmen at the start")
v("Orders per Salesman", "constant", "5", "units/Month/Person", doc="Orders one salesman brings in when the delivery delay is normal")
v("Price", "constant", "10", "kUSD/units", doc="Revenue per unit shipped")
v("Salary Cost", "constant", "2.5", "kUSD/Month/Person", doc="Cost of one salesman")
v("Budget Fraction", "constant", "0.07", "1", doc="Share of revenue spent on the sales force (0.05 keeps the start in balance)")
v("Hiring Time", "constant", "6", "Month", doc="Time to bring the sales force to its budget")
v("Delay Perception Time", "constant", "3", "Month", doc="Time customers and the firm take to notice a change in delivery delay")
v("Capacity Acquisition Time", "constant", "18", "Month", doc="Time from ordering new capacity until it works")
v("Delay Effect Switch", "constant", "1", "1", doc="1 = a long delivery delay discourages orders; 0 = orders ignore the delay")
v("Capacity Switch", "constant", "1", "1", doc="1 = the firm orders capacity when delivery delay is long; 0 = capacity stays as it is")
v("Sales Budget Switch", "constant", "1", "1", doc="1 = the sales force follows revenue; 0 = the sales force stays at its starting size")

# stocks
v("Sales Force", "stock", "Initial Sales Force", "Person", doc="Salesmen")
v("Order Backlog", "stock", "Target Delivery Delay * Initial Capacity", "units", doc="Orders taken and not yet shipped")
v("Capacity", "stock", "Initial Capacity", "units/Month", doc="Units per month the plant can ship")
v("Capacity on Order", "stock", "0", "units/Month", doc="Capacity ordered and not yet working")
# lookups
v("Delay Effect on Orders", "lookup", "", "1", doc="Multiplier on orders as a function of the delay ratio")
v("Capacity Order Fraction", "lookup", "", "1/Month", doc="Fraction of capacity ordered per month, as a function of the delay ratio")
# flows and auxes
v("Order Rate", "flow", "Sales Force * Orders per Salesman * Delay Effect Multiplier", "units/Month", None, "Order Backlog", "New orders")
v("Shipments", "aux", "MIN(Capacity, Order Backlog / Minimum Delivery Time)", "units/Month", doc="Shipments: capacity, unless the backlog is nearly empty")
v("Shipping", "flow", "Shipments", "units/Month", "Order Backlog", None, "Orders leaving the backlog as shipments")
v("Delivery Delay", "aux", "Order Backlog / Shipments", "Month", doc="Time an order waits: backlog over shipment rate")
v("Perceived Delay", "aux", "SMOOTH(Delivery Delay, Delay Perception Time)", "Month", doc="Delivery delay as customers and the firm see it")
v("Delay Ratio", "aux", "Perceived Delay / Target Delivery Delay", "1")
v("Delay Effect Multiplier", "aux", "1 + Delay Effect Switch * (Delay Effect on Orders(Delay Ratio) - 1)", "1", doc="Effect of delivery delay on orders, after the switch")
v("Revenue", "aux", "Shipments * Price", "kUSD/Month")
v("Target Sales Force", "aux", "IF THEN ELSE(Sales Budget Switch = 1, Budget Fraction * Revenue / Salary Cost, Initial Sales Force)", "Person", doc="Sales force the budget pays for")
v("Sales Force Change", "flow", "(Target Sales Force - Sales Force) / Hiring Time", "Person/Month", None, "Sales Force", "Net hiring (negative = leaving)")
v("Capacity Orders", "flow", "Capacity Switch * Capacity * Capacity Order Fraction(Delay Ratio)", "units/Month/Month", None, "Capacity on Order", "New capacity ordered")
v("Capacity Completions", "flow", "Capacity on Order / Capacity Acquisition Time", "units/Month/Month", "Capacity on Order", "Capacity", "New capacity coming into use")
v("Order to Capacity Ratio", "aux", "Order Rate / Capacity", "1", doc="Above 1 the backlog is growing")

V.sort(key=lambda x: 0 if x[1] == "stock" else 1)
for n, k, e, u, f, t, d in V:
    o = dict(op="add_var", name=n, kind=k, units=u)
    if e != "": o["equation"] = e
    if f: o["from"] = f
    if t: o["to"] = t
    if d: o["doc"] = d
    ops.append(o)
op(op="set_table", name="Delay Effect on Orders", points=[[0, 1.2], [1, 1], [1.5, 0.85], [2, 0.65], [3, 0.4], [4, 0.25], [6, 0.12]])
op(op="set_table", name="Capacity Order Fraction", points=[[0, 0], [1, 0], [1.5, 0.05], [2, 0.1], [3, 0.15], [5, 0.2]])
GROUPS = [("Sales force", ["Sales Force", "Sales Force Change", "Target Sales Force", "Budget Fraction", "Salary Cost", "Hiring Time", "Sales Budget Switch", "Initial Sales Force"]),
          ("Orders and backlog", ["Order Rate", "Order Backlog", "Shipping", "Orders per Salesman"]),
          ("Delivery delay", ["Delivery Delay", "Perceived Delay", "Delay Ratio", "Delay Perception Time", "Target Delivery Delay", "Minimum Delivery Time"]),
          ("Delay effect on orders", ["Delay Effect on Orders", "Delay Effect Multiplier", "Delay Effect Switch"]),
          ("Production capacity", ["Capacity", "Capacity on Order", "Capacity Orders", "Capacity Completions", "Capacity Order Fraction", "Capacity Acquisition Time", "Capacity Switch", "Initial Capacity"]),
          ("Money and shipments", ["Shipments", "Revenue", "Price", "Order to Capacity Ratio"])]
for g, vs in GROUPS: op(op="add_group", name=g, vars=vs)
for var, lo, hi in (("Budget Fraction", 0.04, 0.08), ("Capacity Acquisition Time", 3, 24), ("Delay Perception Time", 1, 12), ("Hiring Time", 2, 18),
                    ("Capacity Switch", 0, 1), ("Delay Effect Switch", 0, 1), ("Sales Budget Switch", 0, 1)):
    op(op="set_lever", var=var, on=True); op(op="set_slider", var=var, min=lo, max=hi)
op(op="set_purpose", problem="Does a growing firm stall because it cannot deliver fast enough, and does adding capacity in response to long delivery delays restore growth or only add swings?",
   kind="understand behaviour", out_of_scope="price changes, competitors, product quality, finance beyond a sales budget")
op(op="add_objective", title="Keep delivery near normal", kind="stay within a band", test="Delivery delay stays within 1.5 times the target of 3 months for the 120 months.", var="Delivery Delay")
op(op="add_objective", title="Keep growing", kind="grow", test="Capacity at month 120 is at least double the starting 100 units per month.", var="Capacity")
json.dump(ops, open("ops.json", "w"), indent=1)
print(len(ops), "operations")
