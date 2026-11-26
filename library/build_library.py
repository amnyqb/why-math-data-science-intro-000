#!/usr/bin/env python3
"""Build the lesson models of the Dynamist library:  FC=path/to/dyn python3 library/build_library.py

Writes library/lessons/NN-name.sdm for each lesson, one model per lesson, and library/index.json.
Every model is our own work, written from textbook structures (no third-party file is copied); the source and
licence of each is recorded in INDEX below, which is also what gets written to index.json.
"""
import json, os, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
FC = os.environ.get("FC", os.path.join(REPO, "target", "release", "dyn"))
OWN = "Written for Dynamist from a textbook structure; no third-party file or text."
LICENCE = "Same licence as the Dynamist project (own work); free to use, copy and teach with."


def V(n, k, eq, units="", doc="", **kw):
    d = {"op": "add_var", "name": n, "kind": k, "equation": eq, "units": units}
    if doc:
        d["doc"] = doc
    d.update(kw)
    return d


def settings(t0, t1, dt, unit):
    return [{"op": "set_setting", "key": k, "value": str(v)} for k, v in
            [("initial_time", t0), ("final_time", t1), ("time_step", dt), ("time_units", unit), ("method", "euler")]]


def lever(name, lo, hi, units=""):
    return {"name": name, "units": units, "min": lo, "max": hi}


def goal(name, out, measure, direction):
    return {"name": name, "output": out, "measure": measure, "direction": direction}


def manifest(title, desc, levers, goals, outputs, scenarios=()):
    return {"op": "set_manifest", "manifest": {
        "title": title, "description": desc, "levers": levers, "goals": goals,
        "outputs": [{"variable": o, "chart": "line"} for o in outputs], "scenarios": list(scenarios)}}


def workforce(core):
    ops = settings(0, 60, 0.0625, "Month") + [
        V("Productivity per Worker", "constant", "2", "Unit/Person/Month", "Units one person makes in a month."),
        V("Desired Inventory", "constant", "100", "Unit", "The stock of goods the firm wants on hand."),
        V("Hiring Time", "constant", "4", "Month", "How long the firm takes to close the gap in its workforce."),
        V("Inventory Correction Time", "constant", "2", "Month", "How long the firm takes to correct an inventory gap."),
        V("Demand Adjustment Time", "constant", "2", "Month", "How slowly the firm's idea of demand follows real demand."),
        V("Demand Step", "constant", "10", "Unit/Month", "Size of the jump in demand at month 5."),
        V("Demand", "aux", "100 + STEP(Demand Step, 5)", "Unit/Month", "Real demand: 100 a month, then a step up at month 5."),
        V("Inventory", "stock", "Desired Inventory", "Unit"),
        V("Workforce", "stock", "Demand / Productivity per Worker", "Person")]
    if core:
        ops.append(V("Expected Demand", "aux", "100 + IF THEN ELSE(TIME >= 5, Demand Step * (1 - EXP(-(TIME - 5) / Demand Adjustment Time)), 0)", "Unit/Month", "What the firm expects: demand, smoothed."))
    else:
        ops += [V("Expected Demand", "stock", "100", "Unit/Month", "What the firm expects: it follows demand with a lag."),
                V("Change in Expected Demand", "flow", "(Demand - Expected Demand) / Demand Adjustment Time", "Unit/Month/Month", to="Expected Demand")]
    ops += [
        V("Production Rate", "flow", "Workforce * Productivity per Worker", "Unit/Month", to="Inventory"),
        V("Sales Rate", "flow", "Demand", "Unit/Month", **{"from": "Inventory"}),
        V("Inventory Shortfall", "aux", "Desired Inventory - Inventory", "Unit"),
        V("Production Required to Adjust Inventory", "aux", "Inventory Shortfall / Inventory Correction Time", "Unit/Month"),
        V("Desired Production", "aux", "Production Required to Adjust Inventory + Expected Demand", "Unit/Month"),
        V("Desired Workforce", "aux", "Desired Production / Productivity per Worker", "Person"),
        V("Net Hiring", "flow", "(Desired Workforce - Workforce) / Hiring Time", "Person/Month", to="Workforce")]
    return ops


MODELS = []


def model(slug, title, level, concepts, prereq, question, source, ops, pos=None):
    MODELS.append(dict(slug=slug, title=title, level=level, concepts=concepts, prerequisites=prereq,
                       question=question, source=source, licence=LICENCE, ops=ops, pos=pos or {}))


model("01-bathtub", "The bathtub", "beginner", ["stock", "flow", "net flow", "accumulation"], [],
      "If the tap runs faster than the drain, what happens to the water in the tub?", OWN,
      settings(0, 20, 0.25, "Minute") + [
          V("Water in tub", "stock", "50", "Litre", "The stock: what has built up."),
          V("Tap setting", "constant", "6", "Litre/Minute", "Tap flow for the first ten minutes."),
          V("Drain rate", "constant", "4", "Litre/Minute", "How fast the drain empties the tub."),
          V("Tap", "flow", "IF THEN ELSE(TIME < 10, Tap setting, 2)", "Litre/Minute", "Inflow: the tap runs at its setting for ten minutes, then is turned down to 2.", to="Water in tub"),
          V("Drain", "flow", "Drain rate", "Litre/Minute", "Outflow.", **{"from": "Water in tub"}),
          manifest("The bathtub", "A stock fed by one flow and drained by another.",
                   [lever("Tap setting", 0, 10, "Litre/Minute"), lever("Drain rate", 0, 10, "Litre/Minute")],
                   [goal("Water at the end", "Water in tub", "final", "min"), goal("Most water", "Water in tub", "peak", "min")],
                   ["Water in tub"])])

model("02-population-loops", "Births and deaths", "beginner", ["feedback", "reinforcing loop", "balancing loop", "loop polarity"], ["01-bathtub"],
      "Why does a population grow when births exceed deaths, and what does each loop do on its own?", OWN,
      settings(0, 100, 0.25, "Year") + [
          V("Population", "stock", "100", "Person"),
          V("Birth fraction", "constant", "0.04", "1/Year", "Births a year per person."),
          V("Death fraction", "constant", "0.02", "1/Year", "Deaths a year per person."),
          V("Births", "flow", "Population * Birth fraction", "Person/Year", "More people, more births: a reinforcing loop.", to="Population"),
          V("Deaths", "flow", "Population * Death fraction", "Person/Year", "More people, more deaths: a balancing loop.", **{"from": "Population"}),
          manifest("Births and deaths", "One stock, one reinforcing and one balancing loop.",
                   [lever("Birth fraction", 0.01, 0.08, "1/Year"), lever("Death fraction", 0.01, 0.08, "1/Year")],
                   [goal("Population at the end", "Population", "final", "max")], ["Population"],
                   [{"name": "Deaths win", "values": {"Death fraction": 0.05}}])])

model("03-delays", "A shipment delay", "beginner", ["delay", "first-order delay", "fixed (pipeline) delay", "material in transit"], ["01-bathtub"],
      "Orders step up. When do the goods arrive, and how does the kind of delay change the answer?", OWN,
      settings(0, 40, 0.125, "Week") + [
          V("Delivery time", "constant", "4", "Week", "Average time a shipment spends on the way."),
          V("Orders", "aux", "100 + STEP(20, 5)", "Unit/Week", "Orders placed: 100 a week, 120 from week 5."),
          V("In transit", "stock", "Orders * Delivery time", "Unit", "Goods on the road (starts balanced with orders)."),
          V("Shipping", "flow", "Orders", "Unit/Week", "Goods leave the factory as soon as they are ordered.", to="In transit"),
          V("Arrivals mixed", "flow", "In transit / Delivery time", "Unit/Week", "First-order delay: the more on the road, the more arrive.", **{"from": "In transit"}),
          V("Arrivals fixed", "aux", "DELAY(Orders, Delivery time, exact=True)", "Unit/Week", "Fixed delay: every shipment takes exactly the delivery time."),
          manifest("A shipment delay", "Two ways to model the same delay.",
                   [lever("Delivery time", 1, 12, "Week")],
                   [goal("Goods in transit at the end", "In transit", "final", "min")], ["Orders", "Arrivals mixed", "Arrivals fixed"])])

model("04-workforce-oscillation", "Hiring and stock: oscillation", "intermediate", ["oscillation", "overshoot", "delays in a balancing loop", "damping"], ["02-population-loops", "03-delays"],
      "Demand steps up. Why do the workforce and inventory swing before they settle?",
      "Structure after Hayward and Roach (2023, Fig. 5): three balancing loops. The diagram is theirs; the formulas, names and numbers are the standard ones for this structure, written by us. No text, table or file of the paper is included.",
      workforce(True) + [manifest("Workforce and inventory (two stocks)", "A firm hires to keep inventory at its target; demand steps up at month 5.",
          [lever("Hiring Time", 1, 8, "Month"), lever("Inventory Correction Time", 0.5, 8, "Month"), lever("Demand Step", 0, 20, "Unit/Month")],
          [goal("Inventory trough", "Inventory", "lowest", "max"), goal("Workforce peak", "Workforce", "peak", "min")],
          ["Inventory", "Workforce"])])

model("05-limits-to-growth", "Growth runs into a limit", "intermediate", ["S-shaped growth", "carrying capacity", "shifting loop dominance", "nonlinearity"], ["02-population-loops"],
      "Why does growth that starts exponential slow down and level off?", OWN,
      settings(0, 30, 0.0625, "Year") + [
          V("Population", "stock", "10", "Person"),
          V("Growth rate", "constant", "0.5", "1/Year", "Growth per person a year when there is plenty of room."),
          V("Capacity", "constant", "1000", "Person", "How many the place can carry."),
          V("Crowding", "aux", "Population / Capacity", "", "Share of the capacity in use: 0 empty, 1 full."),
          V("Growth", "flow", "Population * Growth rate * (1 - Crowding)", "Person/Year", "Births net of crowding.", to="Population"),
          manifest("Growth runs into a limit", "Logistic growth: a reinforcing loop that crowding turns into a balancing one.",
                   [lever("Growth rate", 0.1, 1, "1/Year"), lever("Capacity", 500, 2000, "Person")],
                   [goal("Population at 30 years", "Population", "final", "max")], ["Population"])])

model("06-fishery-policy", "A fishery: testing a policy", "intermediate", ["policy", "lever", "scenario", "trade-off", "sweep", "policy landscape"], ["05-limits-to-growth"],
      "How hard should boats fish to bring home the most fish over fifty years?", OWN,
      settings(0, 50, 0.125, "Year") + [
          V("Fish", "stock", "800", "Tonne", "The fish stock in the sea."),
          V("Catch to date", "stock", "0", "Tonne", "Everything brought ashore so far."),
          V("Regrowth rate", "constant", "0.4", "1/Year", "Growth per tonne a year when there is room."),
          V("Sea capacity", "constant", "1000", "Tonne", "Most fish the sea can carry."),
          V("Boats", "constant", "10", "Boat", "Fishing effort (the policy lever)."),
          V("Catchability", "constant", "0.01", "1/Boat/Year", "Share of the stock one boat takes in a year."),
          V("Regrowth", "flow", "Fish * Regrowth rate * (1 - Fish / Sea capacity)", "Tonne/Year", "Fish are born, slowed by crowding.", to="Fish"),
          V("Harvest", "flow", "Boats * Catchability * Fish", "Tonne/Year", "Fish taken: boats times catchability times the stock.", **{"from": "Fish"}),
          V("Landings", "flow", "Harvest", "Tonne/Year", "What is brought ashore.", to="Catch to date"),
          manifest("A fishery", "Effort is the lever; the goal is the total catch over fifty years, while keeping a stock in the sea.",
                   [lever("Boats", 0, 60, "Boat"), lever("Catchability", 0.005, 0.02, "1/Boat/Year"), lever("Regrowth rate", 0.2, 0.8, "1/Year"), lever("Sea capacity", 500, 2000, "Tonne")],
                   [goal("Total catch", "Catch to date", "final", "max"), goal("Fish left", "Fish", "final", "max")], ["Fish", "Catch to date"],
                   [{"name": "Few boats", "values": {"Boats": 5}}, {"name": "Many boats", "values": {"Boats": 40}}])])

model("07-workforce-what-matters", "Which lever matters?", "advanced", ["screening", "sensitivity", "Sobol indices", "loop dominance", "which levers to study"], ["04-workforce-oscillation", "06-fishery-policy"],
      "Six numbers could be changed. Which ones decide the outcome, and which can be left alone?",
      "Structure after Hayward and Roach (2023, Fig. 5); see lesson 4. Three stocks: expected demand follows demand. Lever ranges are ours.",
      workforce(False) + [manifest("Workforce and inventory (three stocks)", "Same firm as lesson 4, with the firm's idea of demand as a third stock.",
          [lever("Productivity per Worker", 1, 4, "Unit/Person/Month"), lever("Desired Inventory", 50, 150, "Unit"), lever("Hiring Time", 1, 8, "Month"),
           lever("Inventory Correction Time", 0.5, 8, "Month"), lever("Demand Adjustment Time", 0.5, 8, "Month"), lever("Demand Step", 0, 20, "Unit/Month")],
          [goal("Inventory trough", "Inventory", "lowest", "max"), goal("Workforce peak", "Workforce", "peak", "min"),
           goal("Inventory at 30 months", "Inventory", "at:30", "target:100")],
          ["Inventory", "Workforce", "Expected Demand"])])


def fc(*a):
    r = subprocess.run([FC, *a], capture_output=True, text=True)
    if r.returncode not in (0,):
        sys.exit(f"dyn {' '.join(a)} failed: {r.stdout[:600]} {r.stderr[:600]}")
    return r.stdout


def main():
    out_dir = os.path.join(HERE, "lessons")
    os.makedirs(out_dir, exist_ok=True)
    index = []
    for m in MODELS:
        path = os.path.join(out_dir, m["slug"] + ".sdm")
        for f in (path, path + ".oplog.jsonl"):
            if os.path.exists(f):
                os.unlink(f)
        ops = []
        for o in m["ops"]:
            ops.append(o)
        # a stock's arrows: let the engine draw them from equations; add_var with to/from makes flows
        with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
            json.dump(ops, f)
        r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
        os.unlink(f.name)
        if '"ok": true' not in r.stdout:
            sys.exit(f"{m['slug']}: {r.stdout[:1200]}")
        fc("layout", path, "--mode", "tidy")
        log = path + ".oplog.jsonl"
        if os.path.exists(log):
            os.unlink(log)
        index.append({k: m[k] for k in ("slug", "title", "level", "concepts", "prerequisites", "question", "source", "licence")}
                     | {"file": "lessons/" + m["slug"] + ".sdm", "tutorial": "wiki/tutorial/" + m["slug"] + ".md"})
        print(m["slug"], "ok")
    with open(os.path.join(HERE, "index.json"), "w") as f:
        json.dump({"about": "Dynamist model library: one model per lesson. Every file is our own work; see each entry's source and licence.",
                   "models": index}, f, indent=2)
        f.write("\n")


if __name__ == "__main__":
    main()
