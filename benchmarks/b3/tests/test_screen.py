import math, os, tempfile
import numpy as np
from SALib.analyze import morris as salib_morris
from fixtures import formula_model, fc_json

ISHIGAMI = "SIN(X1) + 7 * SIN(X2) ^ 2 + 0.1 * X3 ^ 4 * SIN(X1)"


def ishigami_np(x):
    return math.sin(x[0]) + 7 * math.sin(x[1]) ** 2 + 0.1 * x[2] ** 4 * math.sin(x[0])


def test_matches_salib_on_the_same_design_ishigami():
    d = tempfile.mkdtemp()
    m = formula_model(os.path.join(d, "ish.sdm"), ISHIGAMI, ["X1", "X2", "X3"], -math.pi, math.pi)
    r = fc_json("screen", m, "--trajectories", 30, "--seed", 5, "--design")
    X = np.array(r["design"]["x"])            # unit cube
    Y = np.array(r["design"]["y"])[:, 0]
    # the model was evaluated at lo + u (hi - lo); check against numpy, so the engine and the formula agree
    Yn = np.array([ishigami_np(-math.pi + u * 2 * math.pi) for u in X])
    assert np.allclose(Y, Yn, atol=1e-9), np.abs(Y - Yn).max()
    problem = {"num_vars": 3, "names": ["X1", "X2", "X3"], "bounds": [[0, 1]] * 3}
    s = salib_morris.analyze(problem, X, Y, num_levels=4, conf_level=0.95, print_to_console=False)
    ours = {e["param"]: e for e in r["goals"][0]["effects"]}
    for i, n in enumerate(problem["names"]):
        assert abs(ours[n]["mu"] - s["mu"][i]) < 1e-9, (n, ours[n]["mu"], s["mu"][i])
        assert abs(ours[n]["mu_star"] - s["mu_star"][i]) < 1e-9
        assert abs(ours[n]["sigma"] - s["sigma"][i]) < 1e-9


def test_additive_model_and_inert_parameter():
    d = tempfile.mkdtemp()
    m = formula_model(os.path.join(d, "add.sdm"), "10 * X1 + 2 * X2 + 0 * X3", ["X1", "X2", "X3"])
    r = fc_json("screen", m, "--trajectories", 10)
    e = {x["param"]: x for x in r["goals"][0]["effects"]}
    assert abs(e["X1"]["mu_star"] - 10) < 1e-9 and e["X3"]["class"] == "negligible"
    assert r["goals"][0]["converged"] and r["runs"] == 2 * 10 * 4


def test_doubling_does_not_change_the_top_group_on_ishigami():
    d = tempfile.mkdtemp()
    m = formula_model(os.path.join(d, "ish.sdm"), ISHIGAMI, ["X1", "X2", "X3"], -math.pi, math.pi)
    for seed in (1, 2, 3):
        r = fc_json("screen", m, "--trajectories", 40, "--seed", seed)
        g = r["goals"][0]
        assert g["top_overlap"] == 1.0, (seed, g)
        top = max(g["effects"], key=lambda x: x["mu_star"])["param"]
        assert top == "X2", (seed, top)


if __name__ == "__main__":
    test_matches_salib_on_the_same_design_ishigami(); test_additive_model_and_inert_parameter(); test_doubling_does_not_change_the_top_group_on_ishigami(); print("ok")
