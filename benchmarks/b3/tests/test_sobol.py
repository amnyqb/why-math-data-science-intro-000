import math, os, tempfile
import numpy as np
from SALib.analyze import sobol as salib_sobol
from fixtures import formula_model, fc_json

ISHIGAMI = "SIN(X1) + 7 * SIN(X2) ^ 2 + 0.1 * X3 ^ 4 * SIN(X1)"


def test_matches_salib_on_the_same_sample_and_the_analytic_values():
    d = tempfile.mkdtemp()
    m = formula_model(os.path.join(d, "ish.sdm"), ISHIGAMI, ["X1", "X2", "X3"], -math.pi, math.pi)
    r = fc_json("importance", m, "--samples", 1024, "--seed", 3, "--ensemble", 0, "--design")
    s = r["sobol"]
    Y = np.array(s["design"]["y"])[:, 0]
    problem = {"num_vars": 3, "names": ["X1", "X2", "X3"], "bounds": [[0, 1]] * 3}
    ref = salib_sobol.analyze(problem, Y, calc_second_order=False, num_resamples=100, print_to_console=False)
    ours = s["goals"][0]["indices"]
    for i in range(3):
        print(i, ours[i]["s1"], ref["S1"][i], ours[i]["st"], ref["ST"][i])
        assert abs(ours[i]["s1"] - ref["S1"][i]) < 1e-9, ("S1", i)
        assert abs(ours[i]["st"] - ref["ST"][i]) < 1e-9, ("ST", i)
    analytic_s1, analytic_st = [0.3139, 0.4424, 0.0], [0.5576, 0.4424, 0.2437]
    for i in range(3):
        assert abs(ours[i]["s1"] - analytic_s1[i]) < max(ours[i]["s1_conf"], 0.03)
        assert abs(ours[i]["st"] - analytic_st[i]) < max(ours[i]["st_conf"], 0.03)
    assert r["removable"] == []


if __name__ == "__main__":
    test_matches_salib_on_the_same_sample_and_the_analytic_values(); print("ok")
