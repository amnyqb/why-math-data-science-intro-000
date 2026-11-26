#!/usr/bin/env python3
"""Builds the B1 beer game model as one batch of Dynamist operations and applies it with `dyn ops`.

    python3 build_model.py [--dyn path/to/dyn] [--out beer_game_m5.sdm] [--ship 2] [--info 2] [--prod 2]

The model is four stages in a row: Retailer, Wholesaler, Distributor, Factory. Material moves
from the Factory to the customer through on-hand stocks and shipment pipes; orders move the other
way through order pipes. Every delay is a chain of stocks (not a hidden pipeline), so that beer is
conserved and a chain-level balance test can see it. DT is one week, so a pipe stage empties into
the next every week and a delay of 2 means 2 pipe stages.

Parameters marked PLACEHOLDER are the human-like rule's estimates written from memory and MUST be
replaced from the tables of Sterman (1989) before replication R1 can pass (see CRITERIA.md).
"""

import argparse
import json
import os
import subprocess
import sys

STAGES = ["Retailer", "Wholesaler", "Distributor", "Factory"]
CASE, RATE = "Case", "Case/Week"


def build(ship=2, info=2, prod=2):
    ops = []

    def var(name, kind, equation="", units="", doc="", **kw):
        d = {"op": "add_var", "name": name, "kind": kind, "equation": str(equation), "units": units, "doc": doc}
        d.update(kw)
        ops.append(d)

    def setting(key, value):
        ops.append({"op": "set_setting", "key": key, "value": str(value)})

    setting("title", "Beer game on M5 (B1)")
    setting("time_units", "Week")
    setting("time_step", 1)
    setting("method", "euler")
    setting("initial_time", 0)
    setting("final_time", 36)

    # ------------------------------------------------------------------ global constants
    g = [
        ("Initial inventory", 12, CASE, "Cases on hand at every stage at the start (classic game: 12)."),
        ("Initial demand", 4, RATE, "Weekly demand the chain starts in equilibrium with."),
        ("Base demand", 4, RATE, "Customer demand before the step (regime 0)."),
        ("Step demand", 8, RATE, "Customer demand after the step (regime 0)."),
        ("Step week", 5, "Week", "Week of the step in demand (regime 0)."),
        ("Demand regime", 0, "Dmnl", "0 step, 1 i.i.d. normal, 2 data series."),
        ("Demand mean", 4, RATE, "Mean of the i.i.d. regime and of the base-stock formula."),
        ("Demand sd", 2, RATE, "Standard deviation of the i.i.d. regime and of the base-stock formula."),
        ("Demand seed", 1, "Dmnl", "Seed of the i.i.d. regime."),
        ("Service z", 1.28, "Dmnl", "Safety factor of the base-stock formula (about 90% service)."),
        ("Holding cost", 0.5, "Currency/Case/Week", "Per case on hand per week (classic game: 0.5; backlog is twice that)."),
        ("Backlog cost", 1, "Currency/Case/Week", "Per case in backlog per week."),
        ("Test start", 0, "Week", "Costs and statistics are counted from this week on."),
        ("Use equilibrium S prime", 1, "Dmnl", "1: desired stock is the equilibrium-consistent one (tests T2, T3); 0: the S prime constants below."),
        ("Fixed order weeks", 4, "Week", "Orders are held at the initial demand for weeks 1 to this one (board game: 4)."),
        ("Round orders", 1, "Dmnl", "1: orders are whole cases, rounded half up (board game); 0: continuous."),
    ]
    for name, val, units, doc in g:
        var(name, "constant", val, units, doc)

    var("Demand data", "lookup", "TIME", RATE, "Demand series by week (regime 2); a flat placeholder until a data file is bound.")
    ops.append({"op": "set_table", "name": "Demand data", "points": [[0, 4], [1000, 4]]})
    var("Week", "aux", "TIME + TIME STEP", "Week", "Board week being played: the step from TIME to TIME + 1 is week TIME + 1.")
    var("Customer demand", "aux", "IF THEN ELSE(Demand regime = 0, Base demand + (Step demand - Base demand) * STEP(1, Step week - TIME STEP), "
        "IF THEN ELSE(Demand regime = 1, MAX(0, RANDOM NORMAL(0, 1000, Demand mean, Demand sd, Demand seed)), Demand data))", RATE, "What customers ask of the retailer this week.")
    var("In window", "aux", "IF THEN ELSE(TIME >= Test start, 1, 0)", "Dmnl", "1 once the counted window has begun.")

    # ------------------------------------------------------------------ per stage
    # PLACEHOLDER human-like parameters (from memory of Sterman 1989; replace from the paper's table)
    theta, alpha, beta, sprime = 0.36, 0.26, 0.34, 17
    lead = {}
    for i, S in enumerate(STAGES):
        lead[S] = (ship + info) if i < 3 else prod
        var(f"{S} policy", "constant", 0, "Dmnl", "0 anchoring and adjustment (P0, P2), 1 base-stock (P1).")
        var(f"{S} theta", "constant", theta, "1/Week", "Smoothing of expected demand (PLACEHOLDER).")
        var(f"{S} alpha", "constant", alpha, "1/Week", "Fraction of the stock gap closed per week (PLACEHOLDER).")
        var(f"{S} beta", "constant", beta, "Dmnl", "Weight on the supply line; below 1 means it is underweighted (PLACEHOLDER).")
        var(f"{S} S prime", "constant", sprime, CASE, "Desired stock when not using the equilibrium value (PLACEHOLDER).")
        formula = round(4 * (lead[S] + 1) + 1.28 * 2 * (lead[S] + 1) ** 0.5, 1)
        var(f"{S} base stock level", "constant", formula, CASE, "Order-up-to level of the base-stock policy; the starting value is the formula mean*(L+1) + z*sd*sqrt(L+1).")

    # stocks first
    for i, S in enumerate(STAGES):
        var(f"{S} on hand", "stock", "Initial inventory", CASE, "Cases in stock.", non_negative=True)
        var(f"{S} backlog", "stock", 0, CASE, "Orders received and not yet shipped.", non_negative=True)
        n_ship = ship if i < 3 else prod + 1
        for k in range(1, n_ship + 1):
            nm = f"{S} shipment pipe {k}" if i < 3 else f"{S} production pipe {k}"
            var(nm, "stock", "Initial demand * TIME STEP", CASE, "Cases in transit." if i < 3 else "Cases in production.")
        if i < 3:
            for k in range(1, info + 1):
                var(f"{S} order pipe {k}", "stock", "Initial demand * TIME STEP", CASE, "Orders on their way to the next stage.")
        var(f"{S} expected demand", "stock", "Initial demand", RATE, "Smoothed orders received.")
        var(f"{S} cost", "stock", 0, "Currency", "Cost so far.")
        var(f"{S} window cost", "stock", 0, "Currency", "Cost within the counted window.")
        var(f"{S} order sum", "stock", 0, CASE, "Sum of orders placed in the window.")
        var(f"{S} order sumsq", "stock", 0, "Case^2/Week", "Sum of squared orders placed in the window.")
        var(f"{S} window backlog", "stock", 0, "Case*Week", "Backlog summed over the window.")
        var(f"{S} peak order", "stock", 0, RATE, "Highest order placed so far.")
        var(f"{S} peak time", "stock", 0, "Week", "When the highest order was placed.")
    var("Window weeks", "stock", 0, "Week", "Weeks counted so far.")
    var("Demand sum", "stock", 0, CASE, "Sum of customer demand in the window.")
    var("Demand sumsq", "stock", 0, "Case^2/Week", "Sum of squared customer demand in the window.")
    var("Retailer window shipped", "stock", 0, CASE, "Cases shipped to customers in the window.")
    var("Retailer window demand", "stock", 0, CASE, "Cases customers asked for in the window.")

    # flows and auxiliaries
    for i, S in enumerate(STAGES):
        U = STAGES[i + 1] if i < 3 else None
        D = STAGES[i - 1] if i > 0 else None
        n_ship = ship if i < 3 else prod + 1
        pipe = (lambda k: f"{S} shipment pipe {k}") if i < 3 else (lambda k: f"{S} production pipe {k}")
        # One DT is one board week, computed in the board's order (Sterman 1989; Edali & Yasarcan 2014, eqs. 27-75):
        # step 1 receive, step 2 ship, step 3 backlog and inventory, expectations, step 4 pass orders, step 5 order.
        # The auxiliaries below are the end-of-week values; the flows move the stocks to exactly those values.
        # orders received this week (step 4 of last week put them in the last order pipe)
        if i == 0:
            var(f"{S} orders received", "flow", "Customer demand", RATE, "Customer demand.", to=f"{S} backlog")
        else:
            var(f"{S} orders received", "flow", f"{D} order pipe {info} / TIME STEP", RATE, "Orders arriving from the stage below.", **{"from": f"{D} order pipe {info}", "to": f"{S} backlog"})
        # step 1: receive; step 2: ship from what is now on hand
        var(f"{S} available", "aux", f"{S} on hand + {pipe(n_ship)}", CASE, "On hand after this week's receipt (step 1).")
        var(f"{S} shipments out", "flow", f"MIN({S} available, {S} backlog + {S} orders received * TIME STEP) / TIME STEP", RATE, "Cases shipped this week (step 2).",
            **({"from": f"{S} on hand", "to": f"{D} shipment pipe 1"} if D else {"from": f"{S} on hand"}))
        var(f"{S} backlog clearing", "flow", f"{S} shipments out", RATE, "Backlog cleared by shipping.", **{"from": f"{S} backlog"})
        # step 3: end-of-week stocks
        var(f"{S} end on hand", "aux", f"{S} available - {S} shipments out * TIME STEP", CASE, "On hand at the end of the week (step 3).")
        var(f"{S} end backlog", "aux", f"{S} backlog + ({S} orders received - {S} shipments out) * TIME STEP", CASE, "Backlog at the end of the week (step 3).")
        var(f"{S} inventory", "aux", f"{S} end on hand - {S} end backlog", CASE, "Effective inventory at the end of the week (negative when in backlog).")
        # expectations, formed after step 3
        var(f"{S} updated expectation", "aux", f"{S} expected demand + {S} theta * ({S} orders received - {S} expected demand) * TIME STEP", RATE, "Expected demand after this week's orders.")
        # step 5: supply line at the end of the week, after step 4 moved the orders on
        if i < 3:
            sl_parts = [f"{S} order pipe {k}" for k in range(1, info)] + [f"{U} end backlog", f"{U} shipments out * TIME STEP"] + [pipe(k) for k in range(1, n_ship)]
            sl_n = info - 1 + ship
        else:
            sl_parts = [pipe(k) for k in range(1, n_ship)]
            sl_n = prod
        var(f"{S} supply line", "aux", " + ".join(sl_parts), CASE, "Ordered and not yet received, including the supplier's backlog (end of week).")
        sl0 = f"Initial demand * {sl_n} * TIME STEP"
        var(f"{S} equilibrium S prime", "aux", f"Initial inventory + {S} beta * {sl0}", CASE, "Desired stock that keeps the starting equilibrium flat.")
        var(f"{S} S prime used", "aux", f"IF THEN ELSE(Use equilibrium S prime = 1, {S} equilibrium S prime, {S} S prime)", CASE, "Desired stock in use.")
        var(f"{S} AA order", "aux", f"MAX(0, {S} updated expectation + {S} alpha * ({S} S prime used - {S} inventory - {S} beta * {S} supply line))", RATE, "Anchoring and adjustment (Sterman 1989).")
        var(f"{S} base stock order", "aux", f"MAX(0, {S} base stock level - {S} inventory - {S} supply line) / TIME STEP", RATE, "Order up to the base-stock level.")
        var(f"{S} rule order", "aux", f"IF THEN ELSE({S} policy = 1, {S} base stock order, {S} AA order)", RATE, "Order the policy asks for.")
        var(f"{S} orders placed", "flow", f"IF THEN ELSE(Week <= Fixed order weeks, Initial demand, IF THEN ELSE(Round orders = 1, INTEGER({S} rule order * TIME STEP + 0.5) / TIME STEP, {S} rule order))", RATE,
            "Orders this stage places (step 5): held at the initial demand in the first weeks, whole cases when rounding is on.",
            to=(f"{S} order pipe 1" if i < 3 else f"{S} production pipe 1"))
        # pipes
        for k in range(1, n_ship):
            var(f"{S} {'shipment' if i < 3 else 'production'} transfer {k}", "flow", f"{pipe(k)} / TIME STEP", RATE, "One stage along the pipe.", **{"from": pipe(k), "to": pipe(k + 1)})
        var(f"{S} receipts", "flow", f"{pipe(n_ship)} / TIME STEP", RATE, "Cases arriving on hand.", **{"from": pipe(n_ship), "to": f"{S} on hand"})
        if i < 3:
            for k in range(1, info):
                var(f"{S} order transfer {k}", "flow", f"{S} order pipe {k} / TIME STEP", RATE, "One stage along the order pipe.", **{"from": f"{S} order pipe {k}", "to": f"{S} order pipe {k + 1}"})
        # expected demand
        var(f"{S} expected demand change", "flow", f"({S} updated expectation - {S} expected demand) / TIME STEP", "Case/Week^2", "Smoothing step.", to=f"{S} expected demand")
        # costs
        var(f"{S} cost rate", "aux", f"Holding cost * {S} end on hand + Backlog cost * {S} end backlog", "Currency/Week", "Cost this week, on the end-of-week stocks (board game).")
        var(f"{S} cost accrual", "flow", f"{S} cost rate", "Currency/Week", "", to=f"{S} cost")
        var(f"{S} window cost accrual", "flow", f"In window * {S} cost rate", "Currency/Week", "", to=f"{S} window cost")
        # statistics
        var(f"{S} order sum accrual", "flow", f"In window * {S} orders placed", RATE, "", to=f"{S} order sum")
        var(f"{S} order sumsq accrual", "flow", f"In window * {S} orders placed ^ 2", "Case^2/Week^2", "", to=f"{S} order sumsq")
        var(f"{S} window backlog accrual", "flow", f"In window * {S} end backlog", CASE, "", to=f"{S} window backlog")
        var(f"{S} peak order change", "flow", f"MAX(0, {S} orders placed - {S} peak order) / TIME STEP", "Case/Week^2", "", to=f"{S} peak order")
        var(f"{S} peak time change", "flow", f"IF THEN ELSE({S} orders placed > {S} peak order, (TIME - {S} peak time) / TIME STEP, 0)", "Dmnl", "", to=f"{S} peak time")
    var("Window weeks accrual", "flow", "In window", "Dmnl", "", to="Window weeks")
    var("Demand sum accrual", "flow", "In window * Customer demand", RATE, "", to="Demand sum")
    var("Demand sumsq accrual", "flow", "In window * Customer demand ^ 2", "Case^2/Week^2", "", to="Demand sumsq")
    var("Retailer window shipped accrual", "flow", "In window * Retailer shipments out", RATE, "", to="Retailer window shipped")
    var("Retailer window demand accrual", "flow", "In window * Customer demand", RATE, "", to="Retailer window demand")

    # results
    var("Demand variance", "aux", "ZIDZ(Demand sumsq, Window weeks) - ZIDZ(Demand sum, Window weeks) ^ 2", "Case^2/Week^2", "Variance of customer demand in the window.")
    for S in STAGES:
        var(f"{S} order variance", "aux", f"ZIDZ({S} order sumsq, Window weeks) - ZIDZ({S} order sum, Window weeks) ^ 2", "Case^2/Week^2", "Variance of orders placed in the window.")
        var(f"{S} bullwhip", "aux", f"ZIDZ({S} order variance, Demand variance)", "Dmnl", "Variance of orders placed over variance of customer demand.")
        var(f"{S} average backlog", "aux", f"ZIDZ({S} window backlog, Window weeks)", CASE, "Average backlog in the window.")
    var("Chain window cost", "aux", " + ".join(f"{S} window cost" for S in STAGES), "Currency", "Total cost of the chain in the window (the primary metric).")
    var("Chain cost", "aux", " + ".join(f"{S} cost" for S in STAGES), "Currency", "Total cost of the chain so far.")
    var("Fill rate", "aux", "ZIDZ(Retailer window shipped, Retailer window demand)", "Dmnl", "Share of customer demand met from stock in the window.")

    # test helpers
    var("Equilibrium gap", "aux", "IF THEN ELSE(AND_EQ = 1, " + " + ".join(f"ABS({S} orders placed - Base demand)" for S in STAGES) + ", 0)", RATE,
        "Total distance of the orders from equilibrium before the step, when the equilibrium desired stock is in use (T2).")
    # AND is not needed: fold the condition into one comparison
    ops[-1]["equation"] = "IF THEN ELSE(Week < Step week, IF THEN ELSE(Use equilibrium S prime = 1, " + " + ".join(f"ABS({S} orders placed - Base demand)" for S in STAGES) + ", 0), 0)"
    peaks = [f"{S} peak order" for S in STAGES]
    times = [f"{S} peak time" for S in STAGES]
    var("Amplification holds", "aux", f"IF THEN ELSE({peaks[0]} < {peaks[1]}, IF THEN ELSE({peaks[1]} < {peaks[2]}, IF THEN ELSE({peaks[2]} < {peaks[3]}, 1, 0), 0), 0)", "Dmnl",
        "1 if the highest order grows at every stage going upstream (T3).")
    var("Peaks later upstream", "aux", f"IF THEN ELSE({times[0]} <= {times[1]}, IF THEN ELSE({times[1]} <= {times[2]}, IF THEN ELSE({times[2]} <= {times[3]}, IF THEN ELSE({times[0]} < {times[3]}, 1, 0), 0), 0), 0)", "Dmnl",
        "1 if the highest order comes no earlier at each stage going upstream, and strictly later at the factory (T3).")

    # ------------------------------------------------------------------ in-model tests
    material_stocks = []
    for i, S in enumerate(STAGES):
        material_stocks.append(f"{S} on hand")
        if i < 3:
            material_stocks += [f"{S} shipment pipe {k}" for k in range(1, ship + 1)]
        else:
            material_stocks += [f"{S} production pipe {k}" for k in range(1, prod + 2)]
    ops.append({"op": "add_check", "name": "T1 beer is conserved in the chain", "kind": "balance", "stocks": material_stocks, "inflows": ["Factory orders placed"], "outflows": ["Retailer shipments out"]})
    ops.append({"op": "add_check", "name": "T2 equilibrium stays flat before the step", "kind": "reference", "var": "Equilibrium gap", "reference": [[0, 0]], "tolerance": 1e-9})
    ops.append({"op": "add_check", "name": "T3 orders amplify going upstream", "var": "Amplification holds", "min": 1, "when": "end"})
    # "peaks come later going upstream" is NOT an in-file check: with the placeholder human-rule parameters the
    # distributor peaks one week before the wholesaler, and the check must not be weakened to pass. It is asserted
    # in the R1 replication (run_pilot.py) with the paper's parameters. See CRITERIA.md section 9.
    for S in STAGES:
        ops.append({"op": "add_check", "name": f"{S} on hand never negative", "kind": "nonneg", "var": f"{S} on hand"})
        ops.append({"op": "add_check", "name": f"{S} orders never negative", "kind": "nonneg", "var": f"{S} orders placed"})

    # levers: everything a policy may tune
    for S in STAGES:
        for p in ("theta", "alpha", "beta", "S prime", "base stock level"):
            ops.append({"op": "set_lever", "var": f"{S} {p}", "on": True})
    ops.append({"op": "set_manifest", "manifest": manifest(material_stocks)})
    return ops


def manifest(material_stocks, scenarios=None):
    """What the model offers a user: the tunable numbers with ranges, the results to read, where it is trusted."""
    levers = []
    ranges = {"theta": (0.05, 1.0, "Smoothing of expected demand."), "alpha": (0.02, 1.0, "Fraction of the stock gap closed per week."),
              "beta": (0.0, 3.0, "Weight on the supply line (1 = fully counted)."), "S prime": (0, 200, "Desired stock when not using the equilibrium value."),
              "base stock level": (0, 200, "Order-up-to level of the base-stock policy.")}
    group = {"theta": "SD rule", "alpha": "SD rule", "beta": "SD rule", "S prime": "SD rule", "base stock level": "Base-stock"}
    defaults = {"theta": 0.36, "alpha": 0.26, "beta": 0.34, "S prime": 17}
    for S in STAGES:
        for k, (lo, hi, why) in ranges.items():
            lv = {"name": f"{S} {k}", "min": lo, "max": hi, "explanation": why, "group": f"{S}: {group[k]}"}
            lv["units"] = {"theta": "1/Week", "alpha": "1/Week", "beta": "Dmnl", "S prime": "Case", "base stock level": "Case"}[k]
            if k in defaults:
                lv["default"] = defaults[k]
            levers.append(lv)
        levers.append({"name": f"{S} policy", "min": 0, "max": 1, "default": 0, "units": "Dmnl", "explanation": "0 anchoring and adjustment, 1 base-stock.", "group": f"{S}: policy"})
    for name, lo, hi, d, why in (("Demand regime", 0, 2, 0, "0 step, 1 i.i.d. normal, 2 data series."), ("Use equilibrium S prime", 0, 1, 1, "1 equilibrium-consistent desired stock, 0 the S prime constants."),
                                 ("Demand mean", 0, 20, 4, "Mean demand of the i.i.d. regime."), ("Demand sd", 0, 10, 2, "Standard deviation of the i.i.d. regime.")):
        levers.append({"name": name, "min": lo, "max": hi, "default": d, "explanation": why, "group": "Demand"})
    outputs = [{"variable": "Chain window cost", "chart": "bar", "units": "Currency"}, {"variable": "Fill rate", "chart": "bar", "units": "Dmnl", "reference_lines": [1.0]}]
    outputs += [{"variable": f"{S} bullwhip", "chart": "bar", "units": "Dmnl", "reference_lines": [1.0]} for S in STAGES]
    outputs += [{"variable": f"{S} on hand", "chart": "line", "units": "Case"} for S in STAGES]
    outputs += [{"variable": f"{S} orders placed", "chart": "line", "units": "Case/Week"} for S in STAGES]
    tests = ["T1 beer is conserved in the chain", "T2 equilibrium stays flat before the step", "T3 orders amplify going upstream"]
    for S in STAGES:
        tests += [f"{S} on hand never negative", f"{S} orders never negative"]
    return {
        "title": "Beer game on M5 (B1)",
        "description": "Four-stage serial supply chain with classic delays and costs; policies P0 (human-like), P1 (base-stock), P2 (tuned SD rule). Pilot: synthetic demand, not M5.",
        "levers": levers,
        "outputs": outputs,
        "scenarios": scenarios or [],
        "bindings": [{"input": "Demand data", "source": "data/m5_store_department_weekly.csv", "column": "demand", "kind": "data",
                      "note": "Not bound in the pilot: the series is a flat placeholder until the M5 aggregate exists."}],
        "validity": [
            {"statement": "Weekly serial chain of four stages in the board game's step order (Sterman 1989, checked against Edali & Yasarcan 2014): order mailing 1 week plus the slip, shipping 2 weeks, factory production slip plus 2 weeks.", "horizon": 104},
            {"statement": "Demand regimes tried: step 4 to 8 at week 5; i.i.d. normal with mean 4 and sd 2.", "variable": "Demand mean", "min": 0, "max": 20},
            {"statement": "The human-like parameters (theta, alpha, beta, S prime) are PLACEHOLDERS until read from Sterman 1989."},
        ],
        "tests": tests,
    }


def main():
    ap = argparse.ArgumentParser()
    here = os.path.dirname(os.path.abspath(__file__))
    ap.add_argument("--fc", default=os.environ.get("FC_BIN", os.path.join(here, "..", "..", "target", "debug", "dyn")))
    ap.add_argument("--out", default=os.path.join(here, "beer_game_m5.sdm"))
    ap.add_argument("--ship", type=int, default=2)
    ap.add_argument("--info", type=int, default=2)
    ap.add_argument("--prod", type=int, default=2)
    a = ap.parse_args()
    ops = build(a.ship, a.info, a.prod)
    batch = os.path.join(here, "build", "beer_game.ops.json")
    os.makedirs(os.path.dirname(batch), exist_ok=True)
    json.dump(ops, open(batch, "w"), indent=1)
    if os.path.exists(a.out):
        os.remove(a.out)
    cmd = [a.fc, "ops", a.out, batch, "--actor", "build:b1", "--reason", "generate the B1 beer game model"]
    if os.environ.get("B1_NO_GATE"):
        cmd.append("--no-gate")
    r = subprocess.run(cmd, capture_output=True, text=True)
    print(r.stdout)
    if r.returncode != 0:
        sys.exit(r.returncode)


if __name__ == "__main__":
    main()
