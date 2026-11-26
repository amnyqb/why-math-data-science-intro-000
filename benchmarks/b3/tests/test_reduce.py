import json, os, shutil, subprocess, sys, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from fixtures import FC, fc_json, formula_model

B3 = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..")
CORE = os.path.join(B3, "models", "b2-1-workforce-inventory-core.sdm")


def ops(path, batch):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(batch, f)
    r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    assert '"ok": true' in r.stdout, r.stdout


def redundant(d):
    p = os.path.join(d, "redundant.sdm")
    shutil.copy(CORE, p)
    m = json.loads(open(p).read())
    mf = m["manifest"]
    mf["levers"] += [{"name": "Noise Factor", "min": 0, "max": 1}, {"name": "Unused Gain", "min": 0, "max": 1}]
    ops(p, [
        {"op": "add_var", "name": "Noise Factor", "kind": "constant", "equation": "0.5"},
        {"op": "add_var", "name": "Unused Gain", "kind": "constant", "equation": "0.5"},
        {"op": "add_var", "name": "Side", "kind": "stock", "equation": "5"},
        {"op": "add_var", "name": "Side Flow", "kind": "flow", "equation": "Workforce * Unused Gain", "to": "Side"},
        {"op": "add_var", "name": "Workforce Copy", "kind": "aux", "equation": "Workforce"},
        {"op": "set_equation", "name": "Production Rate", "equation": "Workforce Copy * Productivity per Worker"},
        {"op": "set_equation", "name": "Desired Production", "equation": "(Production Required to Adjust Inventory + Expected Demand) * (1 + 0.0001 * Noise Factor)"},
        {"op": "set_manifest", "manifest": mf},
    ])
    return p


def test_reduce_removes_exactly_the_planted_parts_and_separates_structure_from_parameters():
    d = tempfile.mkdtemp()
    full = redundant(d)
    before = open(full).read()
    out = os.path.join(d, "reduced.sdm")
    r = fc_json("reduce", full, "--out", out, "--samples", 1024, "--log", os.path.join(d, "log.csv"))
    assert r["ok"] and all(g["passed"] for g in r["gates"]), json.dumps(r["gates"])
    # (a) structure: Side, Side Flow, Unused Gain (unreachable once Side is gone), Workforce Copy (pass-through)
    struct = {c["target"]: c["step"] for c in r["structure_removed"]["changes"]}
    assert struct == {"Side": "prune", "Side Flow": "prune", "Unused Gain": "prune", "Workforce Copy": "merge"}, struct
    assert set(r["structure_removed"]["summary"]["elements"]) == {"Side", "Side Flow", "Unused Gain", "Workforce Copy"}
    assert r["structure_removed"]["summary"]["stocks"] == ["Side"]
    assert r["structure_removed"]["summary"]["links"] > 0
    # (b) parameters: only the inert lever, with the scenarios it was tested in named
    params = {c["target"]: c for c in r["parameters_fixed"]["changes"]}
    assert set(params) == {"Noise Factor"}, params
    assert params["Noise Factor"]["scenarios"] and r["parameters_fixed"]["scenarios_tested"]
    assert r["readmitted"] == []
    # the headline figure counts structure only
    assert r["headline"]["structure_removed"] == 4 and r["headline"]["parameters_fixed"] == 1
    assert abs(r["headline"]["structure_reduction_pct"] - 100.0 * 4 / r["before"]["variables"]) < 1e-9
    assert r["after"]["variables"] == r["before"]["variables"] - 5
    assert r["verifications"] >= 1 and r["verification_runs"] > 0 and r["verdict"]["pass"]
    assert open(full).read() == before                       # the source is untouched
    reduced = json.load(open(out))
    assert reduced["manifest"]["reduced_from"]
    v = fc_json("verify", full, out, "--ensemble", 30)
    assert v["pass"], json.dumps(v)[:600]
    cats = [l.split(",")[0] for l in open(os.path.join(d, "log.csv")).read().splitlines()[1:]]
    assert cats.count("structure") == 4 and cats.count("parameter") == 1, cats


def test_a_wrong_reduction_is_detected():
    d = tempfile.mkdtemp()
    full = os.path.join(d, "full.sdm")
    shutil.copy(CORE, full)
    wrong = os.path.join(d, "wrong.sdm")
    shutil.copy(CORE, wrong)
    mf = json.load(open(wrong))["manifest"]
    mf["levers"] = [l for l in mf["levers"] if l["name"] != "Hiring Time"]
    ops(wrong, [{"op": "set_equation", "name": "Net Hiring", "equation": "(Desired Workforce - Workforce) / 12"},
                {"op": "remove_var", "name": "Hiring Time"}, {"op": "set_manifest", "manifest": mf}])
    r = subprocess.run([FC, "verify", full, wrong, "--ensemble", "30"], capture_output=True, text=True)
    assert r.returncode == 1, r.stdout[:400]
    v = json.loads(r.stdout)
    assert not v["pass"] and any(not g["pass_error"] or not g["pass_modes"] for g in v["goals"])
    assert max(g["base"]["nrmse"] for g in v["goals"]) > 0.05


# ---- small effects that add up -------------------------------------------------------------------

SMALL = 20
XS = ["X0"] + [f"X{i}" for i in range(1, SMALL + 1)]
# X0 carries ~90% of the variance; each small lever ~0.5% (total index below 0.01), together ~10%
ADDITIVE = "10 * X0 + 0.745 * (" + " + ".join(XS[1:]) + ")"


def test_small_effects_that_add_up_are_readmitted():
    d = tempfile.mkdtemp()
    full = formula_model(os.path.join(d, "additive.sdm"), ADDITIVE, XS, 0.0, 1.0)
    out = os.path.join(d, "reduced.sdm")
    r = fc_json("reduce", full, "--out", out, "--samples", 1024, "--ensemble", 40)
    assert r["ok"], json.dumps(r["gates"])
    st = {i["param"]: i["st"] for i in r["sobol"]["goals"][0]["indices"]}
    assert st["X0"] > 0.8 and all(st[x] < 0.01 for x in XS[1:]), st
    # (1) the cut alone of all of them fails verification
    allfixed = os.path.join(d, "allfixed.sdm")
    shutil.copy(full, allfixed)
    mf = json.load(open(allfixed))["manifest"]
    mf["levers"] = [l for l in mf["levers"] if l["name"] == "X0"]
    ops(allfixed, [{"op": "set_equation", "name": "Y", "equation": "10 * X0 + 0.745 * 20 * 0.5"}] +
        [{"op": "remove_var", "name": x} for x in XS[1:]] + [{"op": "set_manifest", "manifest": mf}])
    p = subprocess.run([FC, "verify", full, allfixed, "--ensemble", "40"], capture_output=True, text=True)
    assert p.returncode == 1 and not json.loads(p.stdout)["pass"], p.stdout[:300]
    # (2) reduce notices: some are re-admitted, X0 is kept, and the model it returns passes
    assert len(r["readmitted"]) >= 1, r["cuts_tried"]
    assert {c["target"] for c in r["readmitted"]} <= set(XS[1:])
    kept = {l["name"] for l in json.load(open(out))["manifest"]["levers"]}
    assert "X0" in kept and kept & set(XS[1:]), kept
    assert len(r["parameters_fixed"]["changes"]) + len(r["readmitted"]) == SMALL
    assert r["verdict"]["pass"] and r["verifications"] > 2
    assert fc_json("verify", full, out, "--ensemble", 40)["pass"]
    assert r["headline"]["structure_removed"] == 0           # parameters do not count as structure


# ---- heavy-tailed goal ---------------------------------------------------------------------------

def growth_model(path, transform):
    """Exponential growth with the rate a sum of four levers: the final population is heavy tailed."""
    ops_ = [{"op": "set_setting", "key": "final_time", "value": "60"}, {"op": "set_setting", "key": "time_step", "value": "0.25"}]
    names = ["R1", "R2", "R3", "R4", "Noise"]
    for n in names:
        ops_.append({"op": "add_var", "name": n, "kind": "constant", "equation": "0.5" if n == "Noise" else "0.03"})
    ops_ += [
        {"op": "add_var", "name": "Rate", "kind": "aux", "equation": "R1 + R2 + R3 + R4 + 0.00001 * Noise"},
        {"op": "add_var", "name": "Pop", "kind": "stock", "equation": "1"},
        {"op": "add_var", "name": "Growth", "kind": "flow", "equation": "Pop * Rate", "to": "Pop"},
    ]
    goal = {"name": "pop", "output": "Pop", "measure": "final", "direction": "max"}
    if transform:
        goal["transform"] = transform
    ops_.append({"op": "set_manifest", "manifest": {
        "levers": [{"name": n, "min": 0, "max": 1 if n == "Noise" else 0.06} for n in names],
        "goals": [goal]}})
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(ops_, f)
    if os.path.exists(path):
        os.unlink(path)
    r = subprocess.run([FC, "ops", path, f.name], capture_output=True, text=True)
    os.unlink(f.name)
    assert '"ok": true' in r.stdout, r.stdout
    return path


def test_reduce_refuses_a_heavy_tailed_goal_without_a_transform_and_works_with_log():
    d = tempfile.mkdtemp()
    full = growth_model(os.path.join(d, "growth.sdm"), None)
    out = os.path.join(d, "reduced.sdm")
    p = subprocess.run([FC, "reduce", full, "--out", out, "--samples", "512"], capture_output=True, text=True)
    assert p.returncode == 1, p.stdout[:500]
    r = json.loads(p.stdout)
    assert not r["ok"] and r["refused"] and r["refused"][0]["goal"] == "pop"
    assert "no transform is declared" in r["refused"][0]["reason"] and "log" in r["refused"][0]["reason"]
    assert not os.path.exists(out)                           # nothing written
    assert r["structure_removed"]["changes"] == [] and r["parameters_fixed"]["changes"] == []

    full = growth_model(os.path.join(d, "growth_log.sdm"), "log")
    p = subprocess.run([FC, "reduce", full, "--out", out, "--samples", "512"], capture_output=True, text=True)
    assert p.returncode == 0, p.stdout[:800]
    r = json.loads(p.stdout)
    assert r["ok"] and r["gates"][0]["transform"] == "log" and r["gates"][0]["passed"], r["gates"]
    fixed = {c["target"] for c in r["parameters_fixed"]["changes"]}
    assert fixed == {"Noise"}, (fixed, r["readmitted"])
    kept = {l["name"] for l in json.load(open(out))["manifest"]["levers"]}
    assert kept == {"R1", "R2", "R3", "R4"}


def test_a_declared_transform_that_is_not_known_is_an_error():
    d = tempfile.mkdtemp()
    try:
        growth_model(os.path.join(d, "g.sdm"), "sqrt")        # the manifest check refuses it
        raise RuntimeError("an unknown transform was accepted")
    except AssertionError as e:
        assert "unknown transform" in str(e)


if __name__ == "__main__":
    test_reduce_removes_exactly_the_planted_parts_and_separates_structure_from_parameters(); test_a_wrong_reduction_is_detected()
    test_small_effects_that_add_up_are_readmitted(); test_reduce_refuses_a_heavy_tailed_goal_without_a_transform_and_works_with_log()
    test_a_declared_transform_that_is_not_known_is_an_error(); print("ok")
