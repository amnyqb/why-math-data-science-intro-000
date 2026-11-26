#!/usr/bin/env python3
"""Writes the B1 model in the spreadsheet template form (a folder of CSVs) so the app can import it
with a module per stage and the key levers as circles: `dynamist --import-sheet spreadsheet`.

The generated model (build_model.py) stays the source of truth; this is a view of it.
"""

import csv
import json
import os

HERE = os.path.dirname(os.path.abspath(__file__))
OPS = os.path.join(HERE, "build", "beer_game.ops.json")
OUT = os.path.join(HERE, "spreadsheet")
STAGES = ["Retailer", "Wholesaler", "Distributor", "Factory"]
RESULT_WORDS = ("cost", "order sum", "sumsq", "window", "peak", "bullwhip", "variance", "Fill rate", "Equilibrium gap", "Amplification", "Peaks later", "Window weeks", "Demand sum", "In window", "average backlog")
LEVERS = {"Demand regime", "Step demand", "Use equilibrium S prime"} | {f"{s} beta" for s in STAGES} | {"Retailer policy", "Factory policy"}


def module_of(name):
    for s in STAGES:
        if name.startswith(s + " "):
            rest = name[len(s) + 1:]
            if any(w in rest or w in name for w in RESULT_WORDS):
                return "Results and tests"
            return s
    if any(w in name for w in RESULT_WORDS):
        return "Results and tests"
    return "Demand and settings"


def write(name, rows):
    os.makedirs(OUT, exist_ok=True)
    with open(os.path.join(OUT, name + ".csv"), "w", newline="") as f:
        csv.writer(f).writerows(rows)


def main():
    ops = json.load(open(OPS))
    add = [o for o in ops if o["op"] == "add_var"]
    settings = {o["key"]: o["value"] for o in ops if o["op"] == "set_setting"}
    params, stocks, flows, variables, lookups = [["Name", "Value", "Units", "Min", "Max", "Description", "Per", "Combine", "Module", "Lever"]], \
        [["Name", "Initial value", "Units", "Never negative", "Description", "Per", "Combine", "Module"]], \
        [["Name", "Equation", "From stock", "To stock", "Units", "Description", "Per", "Combine", "Module"]], \
        [["Name", "Equation", "Units", "Description", "Per", "Combine", "Module"]], \
        [["Name", "Input", "Points", "Units", "Description", "Per", "Combine", "Module"]]
    table = next((o["points"] for o in ops if o["op"] == "set_table"), [[0, 4], [1000, 4]])
    for o in add:
        n, k, eq, u, d, m = o["name"], o["kind"], o["equation"], o["units"], o.get("doc", ""), module_of(o["name"])
        if k == "constant":
            val = float(eq)
            params.append([n, eq, u, 0, max(1, val * 2) if val else 1, d, "", "", m, "yes" if n in LEVERS else ""])
        elif k == "stock":
            stocks.append([n, eq, u, "yes" if o.get("non_negative") else "", d, "", "", m])
        elif k == "flow":
            flows.append([n, eq, o.get("from", ""), o.get("to", ""), u, d, "", "", m])
        elif k == "aux":
            variables.append([n, eq, u, d, "", "", m])
        elif k == "lookup":
            lookups.append([n, "Time", " ".join(f"({x},{y})" for x, y in table), u, d, "", "", m])
    write("Parameters", params)
    write("Stocks", stocks)
    write("Flows", flows)
    write("Variables", variables)
    write("Lookups", lookups)
    write("Settings", [["Setting", "Value"], ["Title", "Beer game on M5 (B1 pilot)"], ["Initial time", settings.get("initial_time", 0)], ["Final time", settings.get("final_time", 36)],
                       ["Time step", settings.get("time_step", 1)], ["Time units", settings.get("time_units", "Week")], ["Method", "Euler"]])
    checks = [["Name", "Variable", "Min", "Max", "When", "Kind", "Tolerance", "Reference", "Stocks", "Inflows", "Outflows"]]
    for o in ops:
        if o["op"] == "add_check":
            checks.append([o["name"], o.get("var", ""), o.get("min", ""), o.get("max", ""), o.get("when", "always"), o.get("kind", ""), o.get("tolerance", ""),
                           ";".join(f"{a}:{b}" for a, b in o.get("reference", [])), ";".join(o.get("stocks", [])), ";".join(o.get("inflows", [])), ";".join(o.get("outflows", []))])
    write("Checks", checks)
    write("Modules", [["Module", "Description"],
                      ["Demand and settings", "What customers ask for each week (a step, i.i.d. noise or a data series) and the cost and start values."],
                      ["Retailer", "Sells to customers. Holds stock and backlog, smooths the orders it receives, decides how much to order."],
                      ["Wholesaler", "Supplies the retailer, with shipment and order delays of two weeks each way."],
                      ["Distributor", "Supplies the wholesaler."],
                      ["Factory", "Produces for the distributor; its production takes two weeks."],
                      ["Results and tests", "Costs, order variance and bullwhip by stage, fill rate, and the quantities the in-model tests watch."]])
    print("wrote", OUT)


if __name__ == "__main__":
    main()
