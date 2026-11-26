#!/usr/bin/env python3
"""Rebuild every figure of the tutorial, and the numbers the pages quote, from the library models.

    FC_BIN=target/release/dyn python3 library/build_tutorial.py [--only 04]

Writes wiki/tutorial/img/*.svg and wiki/tutorial/img/facts.json (every number quoted in a lesson page).
Diagrams come from `dyn render`, `dyn cld` and `dyn chart` (the app's own drawing code); statistical charts from
dynamist.plots. Needs seaborn and matplotlib (python/requirements-plots.txt). Nothing here is heavy: the
largest job is a few thousand runs of models that take milliseconds each. Output is deterministic.
"""
import json, os, shutil, subprocess, sys, tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(HERE)
FC = os.environ.get("FC_BIN") or os.path.join(REPO, "target", "release", "dyn")
os.environ["FC_BIN"] = FC
sys.path.insert(0, os.path.join(REPO, "python"))
import dynamist as fcl                      # noqa: E402
from dynamist import plots                   # noqa: E402

LESSONS = os.path.join(HERE, "lessons")
IMG = os.environ.get("FIG_OUT") or os.path.join(REPO, "wiki", "tutorial", "img")   # FIG_OUT: write somewhere else (the test does)
STYLE = os.path.join(REPO, "book", "examples", "book.style.json")
FACTS = {}
ONLY = sys.argv[sys.argv.index("--only") + 1] if "--only" in sys.argv else None
plots.use_style(STYLE)


def sh(*a):
    r = subprocess.run([FC, *map(str, a)], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"dyn {' '.join(map(str, a))}: {r.stdout[:500]} {r.stderr[:500]}")
    return r.stdout


def fact(key, value):
    FACTS[key] = round(float(value), 2)
    return float(value)


class Lesson:
    """A styled working copy of a lesson model; the library file itself stays plain."""

    def __init__(self, slug, tmp):
        self.slug = slug
        self.path = os.path.join(tmp, slug + ".sdm")
        shutil.copy(os.path.join(LESSONS, slug + ".sdm"), self.path)
        sh("style", self.path, "--apply", STYLE)
        self.m = fcl.Model(self.path)

    def out(self, name):
        return os.path.join(IMG, f"{self.slug[:2]}-{name}.svg")

    def diagram(self, caption, name="diagram"):
        sh("set", self.path, "--caption", caption, "--source", f"Library model {self.slug}")
        self.m.render(self.out(name), with_caption=True)

    def chart(self, vars, name="chart", **kw):
        self.m.chart(vars, self.out(name), **kw)

    def ser(self, names, ov=None):
        """{'times': [...], name: [...]} with the model's own step (names are case-insensitive)."""
        r = self.m.run(overrides=ov, series=names, every=1)
        return {"times": r["times"], **{k.lower(): v for k, v in r["series"].items()}}

    def last(self, name, ov=None):
        return self.m.run(overrides=ov, series=[name])["series"][0]["last"]

    def loops(self, caption, names=None, name="loops"):
        sh("cld", self.path, "--from-model", "--out", self.path, "--layout", "loops")
        args = ["--sheet", "Causal loops", "--caption", caption]
        for k, v in (names or {}).items():
            args += ["--loop-name", k, v]
        sh("set", self.path, *args)
        self.m.render(self.out(name), sheet="Causal loops", with_caption=True)


def first_time(s, name, pred):
    return next(t for t, v in zip(s["times"], s[name]) if pred(v))


def l01(L):
    L.diagram("Figure 1.1  One stock, a tap in, a drain out.")
    L.chart(["Water in tub", "Tap", "Drain"], title="The tub fills while the tap beats the drain")
    s = L.ser(["Water in tub"])
    fact("l01_peak", max(s["water in tub"]))
    fact("l01_end", s["water in tub"][-1])
    fact("l01_ex_drain6_end", L.last("Water in tub", {"Drain rate": 6}))
    fact("l01_ex_tap10_peak", max(L.ser(["Water in tub"], {"Tap setting": 10})["water in tub"]))


def l02(L):
    L.diagram("Figure 2.1  Population, with births in and deaths out.")
    L.loops("Figure 2.2  Births feed the population back to itself; deaths hold it back.", {"R1": "Births", "B1": "Deaths"})
    L.chart(["Population"], scenarios="all", title="Births beat deaths, or the other way round")
    fact("l02_end", L.last("Population"))
    fact("l02_end_deaths5", L.last("Population", {"Death fraction": 0.05}))
    fact("l02_end_deaths4", L.last("Population", {"Death fraction": 0.04}))
    fact("l02_end_births2", L.last("Population", {"Birth fraction": 0.02}))


def l03(L):
    L.diagram("Figure 3.1  Orders go in a pipeline; arrivals come out, two ways.")
    L.chart(["Orders", "Arrivals mixed", "Arrivals fixed"], title="A step in orders arrives late: gradually or all at once")
    s = L.ser(["Orders", "Arrivals mixed", "Arrivals fixed"])
    fact("l03_fixed_jump_time", first_time(s, "arrivals fixed", lambda v: v > 110))
    fact("l03_mixed_at_8", s["arrivals mixed"][s["times"].index(8.0)])
    fact("l03_mixed_t110", first_time(s, "arrivals mixed", lambda v: v >= 110))
    fact("l03_mixed_t119", first_time(s, "arrivals mixed", lambda v: v >= 119))
    fact("l03_transit_end", L.last("In transit"))
    fact("l03_transit_end_d8", L.last("In transit", {"Delivery time": 8}))
    fact("l03_mixed_t110_d8", first_time(L.ser(["Arrivals mixed"], {"Delivery time": 8}), "arrivals mixed", lambda v: v >= 110))


def l04(L):
    L.diagram("Figure 4.1  Hiring follows desired production, with a delay.")
    L.chart(["Inventory"], name="chart", title="Inventory dips, overshoots, and settles")
    L.chart(["Workforce"], name="chart-workforce", title="Workforce overshoots its new level")
    s = L.ser(["Inventory", "Workforce"])
    inv, wf, t = s["inventory"], s["workforce"], s["times"]
    fact("l04_trough", min(inv))
    fact("l04_trough_time", t[inv.index(min(inv))])
    i0 = inv.index(min(inv))
    fact("l04_peak_after", max(inv[i0:]))
    fact("l04_peak_after_time", t[i0 + inv[i0:].index(max(inv[i0:]))])
    fact("l04_wf_peak", max(wf))
    fact("l04_wf_start", wf[0])
    fact("l04_wf_end", wf[-1])
    fact("l04_trough_hire8", min(L.ser(["Inventory"], {"Hiring Time": 8})["inventory"]))
    fact("l04_trough_hire1", min(L.ser(["Inventory"], {"Hiring Time": 1})["inventory"]))
    plots.phase([L.m.run(series=["Inventory", "Workforce"], every=2)], "inventory", "workforce", L.out("phase"),
                title="Workforce against inventory: a loop that spirals in",
                source="Library model 04-workforce-oscillation")
    pts = plots.eigen_path(L.m, "Hiring Time", [1, 2, 3, 4, 5, 6, 7, 8])
    plots.root_locus(pts, L.out("locus"), lever="Hiring Time", title="Longer hiring delays give slower, wider swings",
                     source="Library model 04-workforce-oscillation")
    fact("l04_eig_hire1_im", pts[0][2]); fact("l04_eig_hire1_re", pts[0][1])
    fact("l04_eig_hire8_im", pts[-1][2]); fact("l04_eig_hire8_re", pts[-1][1])


def l05(L, tmp):
    L.diagram("Figure 5.1  Growth, slowed by crowding.")
    L.chart(["Population", "Growth"], title="Growth speeds up, peaks, then slows to a halt")
    s = L.ser(["Population", "Growth"])
    p, g, t = s["population"], s["growth"], s["times"]
    fact("l05_end", p[-1])
    fact("l05_peak_growth", max(g))
    fact("l05_peak_growth_time", t[g.index(max(g))])
    fact("l05_pop_at_peak_growth", p[g.index(max(g))])
    fact("l05_end_rate025", L.last("Population", {"Growth rate": 0.25}))
    fact("l05_end_cap2000", L.last("Population", {"Capacity": 2000}))
    fact("l05_peak_growth_cap2000", max(L.ser(["Growth"], {"Capacity": 2000})["growth"]))
    fact("l05_peak_growth_time_cap2000", (lambda q: q["times"][q["growth"].index(max(q["growth"]))])(L.ser(["Growth"], {"Capacity": 2000})))
    csv = os.path.join(tmp, "dom.csv")
    sh("dominance", L.path, "--csv", csv)
    run = L.m.run(series=["Population"], every=2)
    plots.dominance_timeline(run, ["population"], csv, L.out("dominance"), title="Reinforcing loop first, balancing loop after",
                             source="Library model 05-limits-to-growth", loop_names=["balancing (crowding)", "reinforcing (growth)"])
    sc = {k: [float(r[k]) for r in __import__("csv").DictReader(open(csv))] for k in ("time", "loop1", "loop2")}
    fact("l05_switch_time", next(t for t, a, b in zip(sc["time"], sc["loop1"], sc["loop2"]) if abs(a) > abs(b)))
    dom = L.m.dominance()["loops"]
    fact("l05_share_B", next(l["share_of_time_dominant"] for l in dom if l["polarity"] == "B"))
    fact("l05_share_R", next(l["share_of_time_dominant"] for l in dom if l["polarity"] == "R"))
    L.loops("Figure 5.2  The same two loops as in lesson 2: births reinforce, crowding balances.")


def l06(L):
    import numpy as np
    sh("set", L.path, "--pos", "Regrowth rate", 80, 40, "--pos", "Sea capacity", 200, 40, "--pos", "Regrowth", 140, 150,
       "--pos", "Fish", 320, 150, "--pos", "Harvest", 480, 150, "--pos", "Boats", 480, 40, "--pos", "Catchability", 620, 40,
       "--pos", "Landings", 480, 280, "--pos", "Catch to date", 320, 280)
    L.diagram("Figure 6.1  A fishery: boats take a share of the stock; the sea regrows it.")
    L.chart(["Fish"], scenarios="all", title="Fish in the sea under three fishing efforts")
    boats = [0, 5, 10, 15, 20, 25, 30, 40, 50, 60]
    catch = plots.sweep(L.m, "catch to date", "last", [{"Boats": b} for b in boats])
    fish_left = plots.sweep(L.m, "fish", "last", [{"Boats": b} for b in boats])
    plots.response({"Total catch in 50 years": (boats, catch), "Fish left in the sea": (boats, fish_left)}, L.out("response"),
                   "Boats", "Tonne", title="Fishing harder stops paying: catch peaks, then falls",
                   source="Library model 06-fishery-policy")
    k = int(np.argmax(catch))
    fact("l06_best_boats", boats[k]); fact("l06_best_catch", catch[k])
    for b, c, fl in zip(boats, catch, fish_left):
        fact(f"l06_catch_b{b}", c); fact(f"l06_fish_b{b}", fl)
    fact("l06_fish_b16", L.last("Fish", {"Boats": 16}))
    fact("l06_catch_b16", L.last("Catch to date", {"Boats": 16}))
    fact("l06_fish_b17", L.last("Fish", {"Boats": 17}))
    xs = np.linspace(0, 60, 13); ys = np.linspace(0.2, 0.8, 7)
    z = plots.grid(L.m, "catch to date", "last", "Boats", xs, "Regrowth rate", ys)
    j = np.unravel_index(np.argmax(z), z.shape)
    plots.landscape(xs, ys, z, L.out("landscape"), "Boats", "Regrowth rate (per year)", "Total catch (tonne)",
                    mark=(10, 0.4, "current setting"), title="The best number of boats depends on how fast the sea regrows",
                    source="Library model 06-fishery-policy")
    fact("l06_land_best_boats_slow", xs[int(np.argmax(z[0]))]); fact("l06_land_best_boats_fast", xs[int(np.argmax(z[-1]))])
    fact("l06_land_best_catch", z[j])


def l07(L, tmp):
    L.diagram("Figure 7.1  The firm of lesson 4, with its idea of demand as a third stock.")
    scr = L.m.screen(trajectories=20)
    sob = L.m.importance(goals=["Inventory trough", "Workforce peak"], samples=512, ensemble=10)
    plots.morris(scr, L.out("morris"), goal="Inventory trough", title="Morris screening: which levers move the inventory trough",
                 source="Library model 07-workforce-what-matters")
    plots.sobol(sob, L.out("sobol"), goal="Inventory trough", title="Sobol: how much of the trough's variance each lever explains",
                source="Library model 07-workforce-what-matters")
    plots.heatmap(sob, L.out("heatmap"), kind="sobol", title="Which levers matter for which goal",
                  source="Library model 07-workforce-what-matters")
    rng = {"Productivity per Worker": (1, 4), "Desired Inventory": (50, 150), "Hiring Time": (1, 8),
           "Inventory Correction Time": (0.5, 8), "Demand Adjustment Time": (0.5, 8), "Demand Step": (0, 20)}
    g0, rows = plots.tornado_rows(L.m, "inventory", "min", rng)
    plots.tornado(rows, g0, L.out("tornado"), "Lowest inventory", title="One lever at a time: the swing in the inventory trough",
                  source="Library model 07-workforce-what-matters")
    fact("l07_base_trough", g0)
    for n, a, b in rows:
        FACTS[f"l07_swing_{n}"] = round(abs(b - a), 2)
    for g in scr["goals"]:
        if g["goal"] == "Inventory trough":
            for e in g["effects"]:
                FACTS[f"l07_mustar_{e['param']}"] = round(e["mu_star"], 2)
                FACTS[f"l07_class_{e['param']}"] = e["class"]
    for g in sob["sobol"]["goals"]:
        tag = "st" if g["goal"] == "Inventory trough" else "stwf"
        for e in g["indices"]:
            FACTS[f"l07_{tag}_{e['param']}"] = round(e["st"], 2)
    loops = L.m.dominance()["loops"]
    FACTS["l07_loops"] = [(l["polarity"], round(l["share_of_time_dominant"], 2)) for l in loops if l.get("scored")]


def main():
    os.makedirs(IMG, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        steps = [("01", "01-bathtub", l01), ("02", "02-population-loops", l02), ("03", "03-delays", l03),
                 ("04", "04-workforce-oscillation", l04), ("05", "05-limits-to-growth", lambda L: l05(L, tmp)),
                 ("06", "06-fishery-policy", l06), ("07", "07-workforce-what-matters", lambda L: l07(L, tmp))]
        for num, slug, fn in steps:
            if ONLY and ONLY != num:
                continue
            fn(Lesson(slug, tmp))
            print(slug, "ok")
    path = os.path.join(IMG, "facts.json")
    old = json.load(open(path)) if ONLY and os.path.exists(path) else {}
    old.update(FACTS)
    with open(path, "w") as fh:
        json.dump(old, fh, indent=1, sort_keys=True)
        fh.write("\n")


if __name__ == "__main__":
    main()
