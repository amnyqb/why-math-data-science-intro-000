#!/usr/bin/env python3
"""Build the Workforce-Inventory model as Hayward and Roach (2023) describe it (their Fig. 5 and text, pp. 4-5).

Stocks Inventory, Workforce, Expected Demand; loops B1 workforce adjustment, B2 inventory adjustment, B3 demand
adjustment. Starts in equilibrium; demand steps from 100 to 110 at month 5. Parameters of their Fig. 6: productivity
per worker 2, desired inventory 100, hiring time 4, inventory correction time 2, demand adjustment time 2.
The paper gives the diagram, not the formulas; the equations are the standard ones for this structure (a risk
to be stated in the report: the model is a reconstruction of the diagram). Lever ranges are this benchmark's choice.

Two files: the full model (three stocks) and the core (two stocks, Expected Demand as an exogenous input equal to the
first-order response of demand), which is the system the paper analyses (p. 4: "we can split the system into two
subsystems and concentrate on the two stock Workforce-Inventory model, with Expected Demand E1 as another exogenous input").
"""
import json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tests"))
from fixtures import FC

HERE = os.path.dirname(os.path.abspath(__file__))


def V(n, k, eq, units="", **kw):
    return {"op": "add_var", "name": n, "kind": k, "equation": eq, "units": units, **kw}


def build(core):
    ops = [
        {"op": "set_setting", "key": "initial_time", "value": "0"},
        {"op": "set_setting", "key": "final_time", "value": "60"},
        {"op": "set_setting", "key": "time_step", "value": "0.0625"},
        {"op": "set_setting", "key": "time_units", "value": "Month"},
        {"op": "set_setting", "key": "method", "value": "euler"},
        V("Productivity per Worker", "constant", "2", "Unit/Person/Month"),
        V("Desired Inventory", "constant", "100", "Unit"),
        V("Hiring Time", "constant", "4", "Month"),
        V("Inventory Correction Time", "constant", "2", "Month"),
        V("Demand Adjustment Time", "constant", "2", "Month"),
        V("Demand Step", "constant", "10", "Unit/Month"),
        V("Demand", "aux", "100 + STEP(Demand Step, 5)", "Unit/Month"),
        V("Inventory", "stock", "Desired Inventory", "Unit"),
        V("Workforce", "stock", "Demand / Productivity per Worker", "Person"),
    ]
    if core:
        ops.append(V("Expected Demand", "aux", "100 + IF THEN ELSE(TIME >= 5, Demand Step * (1 - EXP(-(TIME - 5) / Demand Adjustment Time)), 0)", "Unit/Month"))
    else:
        ops.append(V("Expected Demand", "stock", "100", "Unit/Month"))
        ops.append(V("Change in Expected Demand", "flow", "(Demand - Expected Demand) / Demand Adjustment Time", "Unit/Month/Month", to="Expected Demand"))
    ops += [
        V("Production Rate", "flow", "Workforce * Productivity per Worker", "Unit/Month", to="Inventory"),
        V("Sales Rate", "flow", "Demand", "Unit/Month", **{"from": "Inventory"}),
        V("Inventory Shortfall", "aux", "Desired Inventory - Inventory", "Unit"),
        V("Production Required to Adjust Inventory", "aux", "Inventory Shortfall / Inventory Correction Time", "Unit/Month"),
        V("Desired Production", "aux", "Production Required to Adjust Inventory + Expected Demand", "Unit/Month"),
        V("Desired Workforce", "aux", "Desired Production / Productivity per Worker", "Person"),
        V("Net Hiring", "flow", "(Desired Workforce - Workforce) / Hiring Time", "Person/Month", to="Workforce"),
        {"op": "set_manifest", "manifest": {
            "title": "Workforce-Inventory " + ("core (two stocks)" if core else "(three stocks)") + ", after Hayward and Roach 2023",
            "description": "Reconstruction of the diagram in Hayward and Roach 2023, Fig. 5. Equations are the standard ones for the structure.",
            "levers": [
                {"name": "Productivity per Worker", "min": 1, "max": 4},
                {"name": "Desired Inventory", "min": 50, "max": 150},
                {"name": "Hiring Time", "min": 1, "max": 8},
                {"name": "Inventory Correction Time", "min": 0.5, "max": 8},
                {"name": "Demand Adjustment Time", "min": 0.5, "max": 8},
                {"name": "Demand Step", "min": 0, "max": 20}],
            "goals": [
                {"name": "Inventory trough", "output": "Inventory", "measure": "lowest", "direction": "max"},
                {"name": "Workforce peak", "output": "Workforce", "measure": "peak", "direction": "min"},
                {"name": "Inventory at 30 months", "output": "Inventory", "measure": "at:30", "direction": "target:100"}]}},
    ]
    out = os.path.join(HERE, "b2-1-workforce-inventory" + ("-core" if core else "") + ".sdm")
    for f in (out, out + ".oplog.jsonl"):
        if os.path.exists(f):
            os.unlink(f)
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(ops, f)
    r = subprocess.run([FC, "ops", out, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    ok = '"ok": true' in r.stdout
    print(os.path.basename(out), "ok" if ok else r.stdout[:800])
    return ok


if __name__ == "__main__":
    sys.exit(0 if all([build(False), build(True)]) else 1)
