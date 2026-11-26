"""Plots test: skipped when seaborn is not installed. Run: python -m pytest python/test_plots.py"""
import pytest

pytest.importorskip("seaborn")
from dynamist import plots

SOBOL = {"sobol": {"goals": [{"goal": "G", "indices": [
    {"param": "a", "s1": .5, "s1_conf": .02, "st": .6, "st_conf": .02},
    {"param": "b", "s1": .1, "s1_conf": .02, "st": .2, "st_conf": .02}]}]}}
SCREEN = {"goals": [{"goal": "G", "effects": [
    {"param": "a", "mu": 1, "mu_star": 1, "sigma": 0, "class": "linear"},
    {"param": "b", "mu": 0, "mu_star": 2, "sigma": 1, "class": "nonlinear_or_interacting"}]}]}
DOM = {"loops": [{"id": 1, "polarity": "R", "scored": True, "share_of_time_dominant": .6},
                 {"id": 2, "polarity": "B", "scored": True, "share_of_time_dominant": .4}]}
RUNS = [{"series": {"x": [1, 2, 3 + i]}, "times": [0, 1, 2]} for i in range(5)]


def test_all_charts_write_deterministic_svg(tmp_path):
    for name, fn, arg in [("s", plots.sobol, SOBOL), ("m", plots.morris, SCREEN), ("d", plots.dominance, DOM)]:
        a, b = tmp_path / f"{name}1.svg", tmp_path / f"{name}2.svg"
        fn(arg, a); fn(arg, b)
        assert a.read_bytes() == b.read_bytes() and a.stat().st_size > 1000
    plots.fan(RUNS, "x", tmp_path / "f.svg")


def test_unknown_goal_is_an_error(tmp_path):
    with pytest.raises(ValueError):
        plots.sobol(SOBOL, tmp_path / "x.svg", goal="nope")


def test_non_time_charts(tmp_path):
    runs = [{"series": {"a": [1, 2, 3, 2], "b": [5, 4, 6, 5]}, "times": [0, 1, 2, 3]} for _ in range(2)]
    plots.phase(runs, "a", "b", tmp_path / "p.svg", labels=["x", "y"])
    plots.response({"low": ([1, 2, 3], [3, 2, 1]), "high": ([1, 2, 3], [6, 5, 4])}, tmp_path / "r.svg", "lever", "goal")
    plots.heatmap(SOBOL, tmp_path / "h.svg")
    plots.heatmap(SCREEN, tmp_path / "hm.svg", kind="morris")
    scores = {"time": [0, 1, 2, 3], "loop1": [-.7, -.5, -.2, -.6], "loop2": [-.3, -.5, -.8, -.4]}
    plots.dominance_timeline(runs[0], ["a", "b"], scores, tmp_path / "d.svg")
    for n in "p r h hm d".split():
        assert (tmp_path / f"{n}.svg").stat().st_size > 1000


def test_landscape_tornado_root_locus(tmp_path):
    import numpy as np
    xs, ys = np.linspace(0, 1, 6), np.linspace(0, 1, 5)
    plots.landscape(xs, ys, np.add.outer(ys, xs), tmp_path / "l.svg", "x", "y", "goal", mark=(0.5, 0.5, "now"))
    plots.tornado([("a", 1, 5), ("b", 3, 2.5)], 3, tmp_path / "t.svg", "goal")
    plots.root_locus([(1, -.5, .5), (2, -.2, .4), (3, -.05, .2)], tmp_path / "r.svg", lever="delay")
    for n in "ltr":
        assert (tmp_path / f"{n}.svg").stat().st_size > 1000


def test_sweeps_on_a_real_model():
    import os, pathlib
    fc_bin = os.environ.get("FC_BIN")
    model = pathlib.Path(__file__).resolve().parent.parent / "benchmarks/b3/models/b2-1-workforce-inventory-core.sdm"
    if not fc_bin or not model.exists():
        pytest.skip("needs FC_BIN and the benchmark model")
    import dynamist as fc
    m = fc.Model(str(model))
    z = plots.grid(m, "inventory", "min", "Hiring Time", [1, 8], "Inventory Correction Time", [1, 8])
    assert z.shape == (2, 2) and z[0, 0] > z[1, 1]            # slower hiring and correcting: deeper dip
    g0, rows = plots.tornado_rows(m, "inventory", "min", {"Hiring Time": (1, 8), "Productivity per Worker": (1, 4)})
    assert abs(rows[1][2] - rows[1][1]) < 1e-6                 # productivity does not move the dip
    pts = plots.eigen_path(m, "Hiring Time", [2, 8])
    assert pts[1][1] > pts[0][1]                               # slower hiring: decays more slowly (re closer to 0)


def _model_from(tmp_path, src, name):
    import os, pathlib, subprocess
    fc_bin = os.environ.get("FC_BIN")
    if not fc_bin:
        pytest.skip("needs FC_BIN")
    root = pathlib.Path(__file__).resolve().parent.parent
    src = root / src
    if not src.exists():
        pytest.skip("model not found")
    import dynamist as fc
    dst = tmp_path / name
    if src.suffix == ".mdl":
        subprocess.run([fc_bin, "convert", str(src), str(dst)], check=True, capture_output=True)
    else:
        import shutil
        shutil.copy(src, dst)
    return fc.Model(str(dst))


def _draw_sankey(model, out, **kw):
    pytest.importorskip("plotly")
    pytest.importorskip("kaleido")
    try:
        plots.sankey(model, out, **kw)
    except Exception as e:                       # kaleido needs Chrome to draw; skip where there is none
        if "chrome" in str(e).lower():
            pytest.skip("kaleido needs Chrome: " + str(e)[:80])
        raise


def test_trace_is_exact_on_a_cycle_and_on_an_open_model(tmp_path):
    seir = _model_from(tmp_path, "book/examples/charting/seir_waning.mdl", "seir.sdm")
    tr = plots.trace(seir, n_slices=5)
    assert tr["balance_error"] < 1e-6 * 10000
    # total moved out of Susceptible plus what stayed equals what it started with, interval by interval
    for k, mv in enumerate(tr["moves"]):
        start = tr["totals"][k][tr["stocks"].index("Susceptible")]
        assert abs(sum(v for (o, d), v in mv.items() if o == "Susceptible") - start) < 1e-6 * 10000
    _draw_sankey(seir, tmp_path / "s.svg", n_slices=6)
    bath = _model_from(tmp_path, "library/lessons/01-bathtub.sdm", "bath.sdm")
    tr = plots.trace(bath, n_slices=4)
    assert any(o == "outside" for mv in tr["moves"] for (o, _) in mv) and any(d == "lost" for mv in tr["moves"] for (_, d) in mv)
    _draw_sankey(bath, tmp_path / "b.svg", n_slices=4)
    assert (tmp_path / "s.svg").stat().st_size > 1000 and (tmp_path / "b.svg").stat().st_size > 1000


def test_sequence_text_orders_reactions_and_marks_feedback(tmp_path):
    seir = _model_from(tmp_path, "book/examples/charting/seir_waning.mdl", "seir.sdm")
    text = plots.sequence_text(seir, {"contact rate": 0.30})
    assert text.startswith("sequenceDiagram") and "contact rate (changed)" in text
    assert "-->>" in text and "feedback" in text                      # the loops close with dashed arrows
    days = [float(l.rsplit("day ", 1)[1]) for l in text.splitlines() if "->>" in l and "-->>" not in l]
    assert days == sorted(days)                                       # forward messages are in time order
    import shutil
    if shutil.which("mmdc"):
        try:
            plots.sequence(seir, {"contact rate": 0.30}, tmp_path / "q.svg")
        except RuntimeError as e:
            if "chrome" in str(e).lower() or "puppeteer" in str(e).lower():
                pytest.skip("mmdc needs Chrome")
            raise
        assert (tmp_path / "q.svg").stat().st_size > 1000


def test_budget_balances_and_refuses_an_uncovered_outcome(tmp_path):
    pytest.importorskip("plotly")
    pytest.importorskip("kaleido")
    epi = _model_from(tmp_path, "book/examples/charting/epidemic_open.mdl", "epi.sdm")
    tr = plots.trace(epi, slices=[0, 730])
    d = tr["detail"][0]
    start = sum(tr["totals"][0])
    born = sum(v for (o, _), v in d.items() if o == "births")
    assert abs(sum(v for (o, _), v in d.items() if o != "births") - start) < 1e-6 * start      # what was there goes somewhere
    assert born > 0 and tr["balance_error"] < 1e-6 * start
    channels = {"died": ["disease deaths"], "natural": [f"natural death {s}" for s in "SEIR"],
                "alive": ["end:Susceptible", "end:Exposed", "end:Infectious", "end:Recovered"]}
    groups = {"Died": ("loss", ["died", "natural"]), "Alive": ("good", ["alive"])}
    try:
        plots.budget(epi, [("a", {}), ("b", {"contact rate": 0.3})], channels, groups, tmp_path / "b.png", headline="Alive")
    except Exception as e:
        if "chrome" in str(e).lower():
            pytest.skip("kaleido needs Chrome")
        raise
    assert (tmp_path / "b.png").stat().st_size > 5000
    bad = dict(channels)
    del bad["natural"]
    with pytest.raises(ValueError, match="no channel takes"):
        plots.budget(epi, [("a", {})], bad, groups, tmp_path / "c.png")
