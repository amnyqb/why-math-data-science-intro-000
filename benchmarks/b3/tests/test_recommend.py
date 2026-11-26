import json, os, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fixtures import FC, fc_json


def stock_control_model(path, weights=None, offset=False, perceived=False):
    ops = [
        {"op": "set_setting", "key": "final_time", "value": "80"}, {"op": "set_setting", "key": "time_step", "value": "0.125"}, {"op": "set_setting", "key": "method", "value": "euler"},
        {"op": "add_var", "name": "Desired Stock", "kind": "constant", "equation": "100"},
        {"op": "add_var", "name": "Adjustment Time", "kind": "constant", "equation": "4"},
        {"op": "add_var", "name": "Delivery Delay", "kind": "constant", "equation": "6"},
        {"op": "add_var", "name": "Usage Step", "kind": "constant", "equation": "2"},
        {"op": "add_var", "name": "Usage", "kind": "aux", "equation": "10 + STEP(Usage Step, 5)"},
        {"op": "add_var", "name": "Stock", "kind": "stock", "equation": "Desired Stock"},
        {"op": "add_var", "name": "Perception Time", "kind": "constant", "equation": "3"},
        {"op": "add_var", "name": "Perceived Usage", "kind": "aux", "equation": "SMOOTH(Usage, Perception Time)"},
        {"op": "add_var", "name": "Stock Shifted", "kind": "aux", "equation": "Stock * 1000 + 50000"},
        {"op": "add_var", "name": "Orders", "kind": "aux", "equation": "MAX(0, %s + (Desired Stock - Stock) / Adjustment Time)" % ("Perceived Usage" if perceived else "Usage")},
        {"op": "add_var", "name": "Arrivals", "kind": "flow", "equation": "DELAY3(Orders, Delivery Delay)", "to": "Stock"},
        {"op": "add_var", "name": "Shipments", "kind": "flow", "equation": "Usage", "from": "Stock"},
        {"op": "set_manifest", "manifest": {
            "levers": [{"name": "Adjustment Time", "min": 2, "max": 8}, {"name": "Delivery Delay", "min": 3, "max": 9}, {"name": "Usage Step", "min": 1, "max": 4}],
            "goals": [dict({"name": "Stock trough", "output": "Stock Shifted" if offset else "Stock", "measure": "lowest", "direction": "max"}, **({"weight": weights[0]} if weights else {})),
                      dict({"name": "Stock peak", "output": "Stock", "measure": "peak", "direction": "min"}, **({"weight": weights[1]} if weights else {}))]}},
    ]
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(ops, f)
    if os.path.exists(path):
        os.unlink(path)
    r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    assert '"ok": true' in r.stdout, r.stdout


def test_supply_line_correction_is_recovered_as_the_known_improvement():
    d = tempfile.mkdtemp()
    m = os.path.join(d, "stock.sdm")
    stock_control_model(m)
    r = fc_json("recommend", m, "--role", "flow=Orders", "--role", "arrival=Arrivals", "--role", "delay=Delivery Delay", "--role", "time=Adjustment Time",
                "--role", "input=Usage", "--role", "target=Desired Stock", "--ensemble", 20)
    top = next(p for p in r["proposals"] if p["structure"] == "supply_line")   # no longer first: with a scale-free measure the buffer's large trough gain leads the mean
    assert 0.0 < top["weight"] <= 1.5 and top["plausible"], top["weight"]
    assert top["mean_improvement"] > 0.02 and top["robustness"] >= 0.9, top
    assert top["loops_after"][1] > top["loops_before"][1]            # it adds a balancing loop
    names = {p["structure"] for p in r["proposals"]} | {s["structure"] for s in r["skipped"]}
    assert {"forecast", "buffer", "delay_change", "information_link"} <= names
    assert any(s["structure"] == "information_link" for s in r["skipped"])   # no perceived variable named: skipped with a reason
    assert "supply line" not in open(m).read()                      # nothing applied to the model


def _by(r):
    return {p["structure"]: p for p in r["proposals"]}


def _model(**kw):
    m = os.path.join(tempfile.mkdtemp(), "stock.sdm")
    stock_control_model(m, **kw)
    return m


def test_pareto_set_on_a_two_goal_tradeoff_buffer_vs_supply_line():
    r = fc_json("recommend", _model(), "--role", "flow=Orders", "--ensemble", 20)
    p = _by(r)
    imp = lambda s, g: dict(p[s]["improvement"])[g]
    # the buffer wins the trough and loses the peak; the supply line wins the peak: neither dominates
    assert imp("buffer", "Stock trough") > imp("supply_line", "Stock trough") and imp("buffer", "Stock peak") < 0 < imp("supply_line", "Stock peak")
    assert p["buffer"]["pareto"] and p["supply_line"]["pareto"]
    assert not p["delay_change"]["pareto"]                     # changes nothing here: dominated
    flags = [q["pareto"] for q in r["proposals"]]
    assert flags == sorted(flags, reverse=True)                # Pareto set listed first
    assert "ranking" not in r                                  # no goal weights declared: no single ranking
    assert "weighted_improvement" not in p["buffer"]


def test_a_single_ranking_only_when_every_goal_declares_a_weight():
    r = fc_json("recommend", _model(weights=[1, 20]), "--role", "flow=Orders", "--ensemble", 20)
    assert r["ranking"][0] == "supply_line" and set(r["ranking"]) == set(_by(r)), r["ranking"]
    r = fc_json("recommend", _model(weights=[20, 1]), "--role", "flow=Orders", "--ensemble", 20)
    assert r["ranking"][0] == "buffer", r["ranking"]
    assert all("weighted_improvement" in q for q in r["proposals"])


def test_roles_are_detected_from_the_decision_flow_alone():
    r = fc_json("recommend", _model(), "--role", "flow=Orders", "--ensemble", 10)
    got = {x["role"]: x for x in r["roles"]}
    assert {k: got[k]["name"] for k in ("stock", "arrival", "delay", "time", "target")} == {"stock": "Stock", "arrival": "Arrivals", "delay": "Delivery Delay", "time": "Adjustment Time", "target": "Desired Stock"}
    assert got["flow"]["how"] == "given" and got["arrival"]["how"].startswith("detected: DELAY3")
    assert "supply_line" in _by(r)
    why = {s["structure"]: s["reason"] for s in r["skipped"]}
    assert "not detected" in why["forecast"] and "--role input" in why["forecast"]       # asked for only what cannot be detected
    assert "SMOOTH or DELAY" in why["information_link"]
    # a given role overrides detection
    r = fc_json("recommend", _model(), "--role", "flow=Orders", "--role", "time=Delivery Delay", "--ensemble", 5)
    assert {x["role"]: x for x in r["roles"]}["time"]["how"] == "given"


def test_perceived_variable_is_detected_when_the_flow_reads_a_smooth():
    r = fc_json("recommend", _model(perceived=True), "--role", "flow=Orders", "--ensemble", 10)
    got = {x["role"]: x["name"] for x in r["roles"]}
    assert got["perceived"] == "Perceived Usage" and got["actual"] == "Usage"
    assert "information_link" in _by(r)


def test_improvement_is_scale_free():
    a = _by(fc_json("recommend", _model(), "--role", "flow=Orders", "--ensemble", 20))
    b = _by(fc_json("recommend", _model(offset=True), "--role", "flow=Orders", "--ensemble", 20))      # trough read as Stock*1000 + 50000
    for s in ("buffer", "supply_line"):
        assert abs(dict(a[s]["improvement"])["Stock trough"] - dict(b[s]["improvement"])["Stock trough"]) < 1e-6, s


def test_every_structure_names_a_source_or_is_flagged():
    r = fc_json("recommend", _model(perceived=True), "--role", "flow=Orders", "--role", "input=Usage", "--ensemble", 5)
    p = _by(r)
    assert "Sterman" in p["supply_line"]["source"] and not p["supply_line"]["unsourced"]
    for q in r["proposals"]:
        assert q["source"] and (q["source"] != "to be recorded" or q["unsourced"]), q["structure"]
    assert p["buffer"]["unsourced"] and p["buffer"]["source"] == "to be recorded"


if __name__ == "__main__":
    for k, f in list(globals().items()):
        if k.startswith("test_"):
            f(); print("ok", k)
