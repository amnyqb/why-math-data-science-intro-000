"""Statistical charts for Dynamist results, drawn with seaborn (optional).

The core package needs only the standard library. These charts need seaborn and matplotlib:

    pip install seaborn matplotlib        # or: pip install -r python/requirements-plots.txt

They take the JSON that `screen`, `importance` and `dominance` return, so they work on a live
result or on a saved file:

    import dynamist as fc
    from dynamist import plots
    m = fc.Model("model.sdm")
    plots.use_style("book.style.json")      # same fonts and colours as dyn render (optional; defaults match the sample)
    plots.sobol(m.importance(goals=["Inventory trough"]), "sobol.svg")
    plots.morris(m.screen(), "morris.svg")
    plots.dominance(m.dominance(), "loops.svg")
    plots.fan([m.run(overrides={"Hiring Time": t}, series=["Workforce"], every=1) for t in range(1, 9)], "Workforce", "fan.svg")

Output is deterministic (fixed SVG hash salt, no date), so figures in the book rebuild identically.
Also: `sankey` (with `trace` and `flow_network`), `landscape`, `tornado`, `root_locus` (with the sweeps `grid`, `tornado_rows`, `eigen_path` that feed them), `phase` (one variable against another), `response` (a goal against a lever), `heatmap` (levers by goals,
from Sobol or Morris) and `dominance_timeline` (behaviour with loop dominance on a shared time axis).
For model diagrams use `Model.render` / `Model.cld`, which draw the picture the app draws.
"""

import json


DEFAULT_STYLE = {
    "font": "Source Serif 4", "mono_font": "Source Code Pro", "width_mm": 130, "text_scale": 1.0, "line_scale": 1.15,
    "colors": {"stock": "#1f3a5f", "flow": "#2b7a78", "aux": "#b5651d", "link": "#5c6370", "text": "#1a1a1a",
               "muted": "#6b7280", "grid": "#e4e7f0", "error": "#a23b3b", "stock_fill": "#eef3fa"},
}
_STYLE = {}
MM = 1 / 25.4


def use_style(style=None):
    """Use the book's style file (the one `dyn render` reads: fonts, palette, width_mm) for every chart.

    `style` is a path to a book.style.json, a dict, or None for the built-in defaults, which are the
    same as book/examples/book.style.json. Call once at the top of a script."""
    st = json.loads(json.dumps(DEFAULT_STYLE))
    if style is not None:
        user = _load(style)
        colors = user.pop("colors", {})
        st.update(user)
        st["colors"].update(colors)
    _STYLE.clear()
    _STYLE.update(st)
    return st


def _style():
    if not _STYLE:
        use_style()
    return _STYLE


def _lib():
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import seaborn as sns
    except ImportError as e:
        raise ImportError("dynamist.plots needs seaborn and matplotlib: pip install seaborn matplotlib") from e
    st = _style()
    c = st["colors"]
    sz = 9 * st.get("text_scale", 1.0)
    lw = st.get("line_scale", 1.0)
    # Chart look: no frame, one hairline grid in the value direction, ticks only where needed, text in the
    # book's font (SVG keeps it as text; a viewer without the font falls back to serif).
    sns.set_theme(style="white", rc={
        "font.family": "serif", "font.serif": [st["font"], "Source Serif Pro", "Georgia", "DejaVu Serif"],
        "font.size": sz, "axes.titlesize": sz * 1.25, "axes.titleweight": "regular", "axes.titlelocation": "left",
        "axes.titlepad": 10, "axes.labelsize": sz, "axes.labelcolor": c["muted"], "axes.edgecolor": c["grid"],
        "axes.linewidth": 0.8 * lw, "axes.spines.top": False, "axes.spines.right": False,
        "axes.grid": True, "axes.grid.axis": "y", "grid.color": c["grid"], "grid.linewidth": 0.6 * lw,
        "axes.axisbelow": True, "xtick.color": c["muted"], "ytick.color": c["muted"], "xtick.labelsize": sz * 0.9,
        "ytick.labelsize": sz * 0.9, "xtick.major.size": 3, "ytick.major.size": 0, "text.color": c["text"],
        "lines.linewidth": 1.8 * lw, "legend.frameon": False, "legend.fontsize": sz * 0.9,
        "figure.facecolor": st.get("background") or "white", "savefig.facecolor": st.get("background") or "white",
        "svg.hashsalt": "dynamist", "svg.fonttype": "none", "pdf.fonttype": 42,
        "axes.prop_cycle": matplotlib.cycler(color=[c["stock"], c["flow"], c["aux"], c["link"], c["error"]]),
    })
    return plt, sns


def _fig(height_mm, **kw):
    plt, _ = _lib()
    return plt.subplots(figsize=(_style()["width_mm"] * MM, height_mm * MM), **kw)


def _load(x):
    if isinstance(x, (str, bytes)) or hasattr(x, "__fspath__"):
        with open(x) as f:
            return json.load(f)
    return x


def plt_size(fig):
    return 9 * _style().get("text_scale", 1.0)


def _save(fig, ax, out, title, source=None):
    """Title flush left above the plot (a statement, not a label), optional source line below."""
    c = _style()["colors"]
    top = 1.0
    if title:
        fig.suptitle(title, x=0.01, y=0.99, ha="left", va="top", fontsize=plt_size(fig) * 1.25, color=c["text"])
        top = 1 - 8 / (fig.get_size_inches()[1] / MM)
    if source:
        fig.text(0.01, 0.005, "Source: " + source, ha="left", va="bottom", fontsize=7, color=c["muted"])
    fig.tight_layout(rect=(0, 0.03 if source else 0, 1, 1))
    meta = {"Date": None} if str(out).endswith(".svg") else None
    fig.savefig(out, dpi=300, metadata=meta)
    import matplotlib.pyplot as plt
    plt.close(fig)
    return out


def sobol(result, out, goal=None, top=12, title=None, source=None):
    """S1 (alone) and ST (with interactions) per parameter for one goal, with confidence ticks."""
    goals = _load(result)["sobol"]["goals"]
    g = next((x for x in goals if goal in (None, x["goal"])), None)
    if g is None:
        raise ValueError(f"goal {goal!r} not in result")
    idx = sorted(g["indices"], key=lambda i: -i["st"])[:top]
    c = _style()["colors"]
    fig, ax = _fig(30 + 9 * len(idx))
    ax.grid(axis="y", visible=False)
    ax.grid(axis="x", visible=True)
    y = list(range(len(idx)))
    h = 0.36
    ek = dict(error_kw=dict(ecolor=c["text"], elinewidth=0.7, capsize=0))
    ax.barh([i - h / 2 for i in y], [i["st"] for i in idx], h, xerr=[i["st_conf"] for i in idx], color=c["stock"], **ek)
    ax.barh([i + h / 2 for i in y], [i["s1"] for i in idx], h, xerr=[i["s1_conf"] for i in idx], color=c["flow"], **ek)
    ax.set_yticks(y, [i["param"] for i in idx])
    ax.invert_yaxis()
    ax.set_xlim(0, None)
    ax.spines["left"].set_visible(False)
    ax.set_xlabel("share of output variance")
    # direct key instead of a legend box
    ax.text(1.0, 1.02, "ST: with interactions", transform=ax.transAxes, ha="right", va="bottom", color=c["stock"], fontsize=8)
    ax.text(0.0, 1.02, "S1: alone", transform=ax.transAxes, ha="left", va="bottom", color=c["flow"], fontsize=8)
    return _save(fig, ax, out, title or f"What drives {g['goal']}", source)


def morris(result, out, goal=None, title=None, source=None, label_top=6):
    """mu* (size of effect) against sigma (nonlinearity or interaction); one point per parameter."""
    goals = _load(result)["goals"]
    g = next((x for x in goals if goal in (None, x["goal"])), None)
    if g is None:
        raise ValueError(f"goal {goal!r} not in result")
    c = _style()["colors"]
    col = {"negligible": c["grid"], "linear": c["stock"], "nonlinear_or_interacting": c["aux"]}
    name = {"negligible": "negligible", "linear": "linear", "nonlinear_or_interacting": "nonlinear or interacting"}
    fig, ax = _fig(95)
    ax.grid(axis="x", visible=True)
    for k in col:
        pts = [e for e in g["effects"] if e["class"] == k]
        if pts:
            ax.scatter([e["mu_star"] for e in pts], [e["sigma"] for e in pts], s=34, color=col[k],
                       edgecolor=c["muted"] if k == "negligible" else "white", linewidth=0.6, label=name[k], zorder=3)
    for e in sorted(g["effects"], key=lambda e: -e["mu_star"])[:label_top]:
        ax.annotate(e["param"], (e["mu_star"], e["sigma"]), fontsize=7.5, color=c["text"],
                    xytext=(5, 4), textcoords="offset points")
    ax.set_xlabel("mu*  (mean absolute effect)")
    ax.set_ylabel("sigma  (spread of effect)")
    ax.margins(x=0.12, y=0.12)
    ax.legend(loc="upper left", bbox_to_anchor=(0, -0.2), ncol=3, handletextpad=0.2, columnspacing=1.2)
    return _save(fig, ax, out, title or f"Which levers matter for {g['goal']}", source)


def dominance(result, out, title=None, source=None):
    """Share of time each loop dominates, coloured by polarity (R reinforcing, B balancing)."""
    d = _load(result)
    d = d.get("dominance_base_run", d)
    loops = sorted((l for l in d["loops"] if l.get("scored")), key=lambda l: -l["share_of_time_dominant"])[:15]
    c = _style()["colors"]
    col = {"R": c["aux"], "B": c["stock"]}
    fig, ax = _fig(26 + 11 * len(loops))
    ax.grid(axis="y", visible=False)
    ax.grid(axis="x", visible=True)
    names = [f"Loop {l['id']}  ({'reinforcing' if l['polarity'] == 'R' else 'balancing'})" for l in loops]
    ax.barh(names, [l["share_of_time_dominant"] for l in loops], 0.6, color=[col.get(l["polarity"], c["link"]) for l in loops])
    ax.invert_yaxis()
    ax.spines["left"].set_visible(False)
    ax.set_xlim(0, 1)
    ax.xaxis.set_major_formatter(lambda v, _: f"{v:.0%}")
    ax.set_xlabel("share of simulated time in which the loop dominates")
    return _save(fig, ax, out, title or "Which loop is in charge", source)


def fan(runs, series, out, title=None, source=None, quantiles=(0.05, 0.25, 0.5, 0.75, 0.95)):
    """Ribbon chart of one series across many runs (e.g. an ensemble from `Model.run`)."""
    import numpy as np
    cols, t = [], None
    for r in runs:
        r = _load(r)
        if not isinstance(r.get("series"), dict):
            raise ValueError("fan needs runs made with Model.run(series=[...], every=N)")
        cols.append(r["series"][series])
        if t is None:
            t = r["times"]
    q = np.quantile(np.array(cols), quantiles, axis=0)
    c = _style()["colors"]
    fig, ax = _fig(70)
    ax.fill_between(t, q[0], q[-1], color=c["stock"], alpha=0.14, linewidth=0)
    ax.fill_between(t, q[1], q[-2], color=c["stock"], alpha=0.28, linewidth=0)
    ax.plot(t, q[2], color=c["stock"])
    ax.set_xlim(t[0], t[-1])
    ax.set_xlabel("time")
    ax.set_ylabel(series)
    # label the band where it is widest, not in a legend box
    ax.text(0.99, 0.97, f"shaded: middle 50% and {int(quantiles[0]*100)}–{int(quantiles[-1]*100)}% of runs", transform=ax.transAxes,
            ha="right", va="top", fontsize=7.5, color=c["muted"])
    ax.annotate("median", (t[-1], q[2][-1]), xytext=(-4, 6), textcoords="offset points", ha="right", fontsize=7.5, color=c["stock"])
    return _save(fig, ax, out, title or f"{series}: spread across {len(cols)} runs", source)


def phase(runs, x, y, out, labels=None, title=None, source=None, arrow_every=12):
    """Phase portrait: variable `y` against variable `x`, one path per run (time is the direction, not an axis).

    `runs` are results of `Model.run(series=[x, y], every=N)`. Open circle = start, filled = end, arrows show
    the direction of travel. Works for two stocks, or a stock against its own net flow."""
    import numpy as np
    c = _style()["colors"]
    pal = [c["stock"], c["flow"], c["aux"], c["link"], c["error"]]
    fig, ax = _fig(95)
    ax.grid(axis="x", visible=True)
    for i, r in enumerate(runs):
        r = _load(r)
        if not isinstance(r.get("series"), dict):
            raise ValueError("phase needs runs made with Model.run(series=[x, y], every=N)")
        xs, ys = np.array(r["series"][x]), np.array(r["series"][y])
        col = pal[i % len(pal)]
        ax.plot(xs, ys, color=col)
        for k in range(0, len(xs) - 1, arrow_every):
            ax.annotate("", xy=(xs[k + 1], ys[k + 1]), xytext=(xs[k], ys[k]), arrowprops=dict(arrowstyle="-|>", color=col, lw=0))
        ax.scatter(xs[0], ys[0], s=26, color="white", edgecolor=col, zorder=4)
        ax.scatter(xs[-1], ys[-1], s=26, color=col, zorder=4)
        if labels:
            far = int(np.argmax(np.hypot(xs - xs[-1], ys - ys[-1])))
            ax.annotate(labels[i], (xs[far], ys[far]), xytext=(6, -10), textcoords="offset points", fontsize=7.5, color=col)
    ax.set_xlabel(x)
    ax.set_ylabel(y)
    return _save(fig, ax, out, title or f"{y} against {x}", source or "open circle = start, filled = end")


def response(curves, out, xlabel, ylabel, title=None, source=None):
    """A goal against a lever (x is a parameter, not time). `curves` maps a label to (xs, ys), one line each;
    the labels are written at the right-hand end of each line."""
    c = _style()["colors"]
    pal = [c["stock"], c["flow"], c["aux"], c["link"], c["error"]]
    fig, ax = _fig(85)
    ax.grid(axis="x", visible=True)
    xmax = max(max(xs) for xs, _ in curves.values())
    xmin = min(min(xs) for xs, _ in curves.values())
    for i, (lab, (xs, ys)) in enumerate(curves.items()):
        col = pal[i % len(pal)]
        ax.plot(xs, ys, color=col)
        ax.annotate(lab, (xs[-1], ys[-1]), xytext=(4, 0), textcoords="offset points", fontsize=7.5, color=col, va="center")
    ax.set_xlim(xmin, xmax + 0.45 * (xmax - xmin))
    ax.set_xlabel(xlabel)
    ax.set_ylabel(ylabel)
    return _save(fig, ax, out, title or f"{ylabel} against {xlabel}", source)


def heatmap(result, out, kind="sobol", title=None, source=None):
    """Levers (rows) by goals (columns). kind="sobol" from an `importance` result (total index ST);
    kind="morris" from a `screen` result (mu*, scaled to the largest in each goal)."""
    import numpy as np
    plt, sns = _lib()
    c = _style()["colors"]
    res = _load(result)
    if kind == "sobol":
        goals = res["sobol"]["goals"]
        cell = lambda g: {i["param"]: i["st"] for i in g["indices"]}
        label = "Total Sobol index, share of variance"
    elif kind == "morris":
        goals = res["goals"]
        def cell(g):
            top = max((e["mu_star"] for e in g["effects"]), default=0) or 1
            return {e["param"]: e["mu_star"] / top for e in g["effects"]}
        label = "Morris mu*, relative to the largest effect for the goal"
    else:
        raise ValueError("kind is 'sobol' or 'morris'")
    cells = [cell(g) for g in goals]
    params = list(cells[0])
    M = np.array([[cl.get(p, 0.0) for cl in cells] for p in params])
    order = np.argsort(-M.max(1))
    M = M[order]
    names = [params[i] for i in order]
    fig, ax = _fig(26 + 9 * len(names))
    ax.grid(False)
    ax.imshow(M, cmap=sns.light_palette(c["stock"], as_cmap=True), vmin=0, vmax=1, aspect="auto")
    ax.set_xticks(range(len(goals)), [g["goal"] for g in goals])
    ax.set_yticks(range(len(names)), names)
    ax.xaxis.tick_top()
    ax.tick_params(length=0)
    for sp in ax.spines.values():
        sp.set_visible(False)
    for i in range(M.shape[0]):
        for j in range(M.shape[1]):
            ax.text(j, i, f"{M[i, j]:.2f}", ha="center", va="center", fontsize=7.5, color="white" if M[i, j] > 0.5 else c["text"])
    return _save(fig, ax, out, title or "Which levers matter for which goal", source or label)


def dominance_timeline(run, series, scores, out, title=None, source=None, loop_names=None):
    """Behaviour on top, loop dominance underneath, one shared time axis.

    `run` is `Model.run(series=[...], every=N)`; `series` names the series to draw; `scores` is the loop
    scores over time, either a CSV path with columns `time,loop1,loop2,…` (as `dyn dominance` writes) or a dict
    `{"time": [...], "<loop>": [...]}`. The lower band shows each loop's share of the total influence."""
    import csv
    import numpy as np
    plt, sns = _lib()
    c = _style()["colors"]
    if isinstance(scores, (str, bytes)) or hasattr(scores, "__fspath__"):
        with open(scores) as f:
            rows = list(csv.DictReader(f))
        scores = {k: [float(r[k]) for r in rows] for k in rows[0]}
    t = np.array(scores["time"])
    names = [k for k in scores if k != "time"]
    A = np.abs(np.array([scores[k] for k in names]))
    pal = [c["stock"], c["aux"], c["flow"], c["link"], c["error"]]
    run = _load(run)
    fig, (a, b) = plt.subplots(2, 1, sharex=True, figsize=(_style()["width_mm"] * MM, 95 * MM),
                               gridspec_kw={"height_ratios": [4, 1.3]})
    for i, s in enumerate(series):
        col = pal[i % len(pal)]
        a.plot(run["times"], run["series"][s], color=col)
        a.annotate(s, (run["times"][-1], run["series"][s][-1]), xytext=(-4, 5), textcoords="offset points",
                   ha="right", fontsize=7.5, color=col)
    b.grid(False)
    b.stackplot(t, *A, colors=pal[:len(names)], linewidth=0)
    b.set_ylim(0, 1)
    b.set_yticks([])
    b.spines["left"].set_visible(False)
    b.set_xlim(t[0], t[-1])
    b.set_xlabel("time")
    names_shown = loop_names or names
    cum = np.concatenate([[0.0], np.cumsum(A[:, -1])])
    for i, n in enumerate(names_shown):
        b.text(1.01, (cum[i] + cum[i + 1]) / 2, n, transform=b.get_yaxis_transform(), color=pal[i % len(pal)],
               fontsize=7.5, va="center")
    return _save(fig, a, out, title or "Which loop is in charge, moment by moment", source)


# ---- sweeps: run a model over levers and summarise each run into one number -------------------------------

def _summary(values, how):
    import numpy as np
    v = np.asarray(values, dtype=float)
    return {"min": v.min, "max": v.max, "last": lambda: v[-1], "first": lambda: v[0], "mean": v.mean}[how]()


def sweep(model, series, how, vary, base=None, every=8):
    """Goal value for each setting in `vary`: a list of dicts of overrides (merged over `base`).
    The goal is `how` (min, max, last, first or mean) of the named `series`."""
    out = []
    for o in vary:
        r = model.run(overrides={**(base or {}), **o}, series=[series], every=every)
        out.append(_summary(r["series"][series], how))
    return out


def grid(model, series, how, x, xs, y, ys, base=None, every=8):
    """Goal over a grid of two levers: returns Z with one row per y and one column per x (for `landscape`)."""
    import numpy as np
    z = sweep(model, series, how, [{x: a, y: b} for b in ys for a in xs], base, every)
    return np.array(z).reshape(len(ys), len(xs))


def tornado_rows(model, series, how, ranges, base=None, every=8):
    """For each lever in `ranges` ({name: (low, high)}): the goal at its low and at its high end, the others
    at `base` (for `tornado`). Returns (goal_at_base, rows)."""
    g0 = sweep(model, series, how, [{}], base, every)[0]
    rows = []
    for name, (lo, hi) in ranges.items():
        a, b = sweep(model, series, how, [{name: lo}, {name: hi}], base, every)
        rows.append((name, a, b))
    return g0, rows


def eigen_path(model, lever, values):
    """Dominant oscillating eigenvalue of the linearised model for each value of `lever` (for `root_locus`).
    Works on a temporary copy of the model; the original is untouched. Returns [(value, re, im), …]."""
    import os, shutil, tempfile
    from . import Model
    pts = []
    with tempfile.TemporaryDirectory() as d:
        tmp = os.path.join(d, "m" + os.path.splitext(model.path)[1])
        shutil.copy(model.path, tmp)
        m = Model(tmp)
        for v in values:
            m.apply([{"op": "set_equation", "name": lever, "equation": repr(float(v))}])
            ev = m.importance(samples=8, ensemble=1, eigen_at=0)["eigenvalues"]["eigenvalues"]
            ev = [e for e in ev if e["im"] > 0] or ev
            e = max(ev, key=lambda e: e["re"])
            pts.append((v, e["re"], e["im"]))
    return pts


# ---- landscape, tornado, root locus ------------------------------------------------------------------------

def landscape(xs, ys, z, out, xlabel, ylabel, zlabel, mark=None, levels=12, title=None, source=None, reverse=False):
    """Policy landscape: a goal (`z`, one row per y, one column per x) over two levers, as filled contours with
    labelled lines. `mark` = (x, y, text) circles a point (the current policy, an optimum). `reverse=True`
    makes small values dark (use it when low is the interesting end)."""
    plt, sns = _lib()
    c = _style()["colors"]
    fig, ax = _fig(95)
    ax.grid(False)
    cs = ax.contourf(xs, ys, z, levels=levels, cmap=sns.light_palette(c["stock"], as_cmap=True, reverse=reverse))
    ln = ax.contour(xs, ys, z, levels=levels, colors="white", linewidths=0.5)
    ax.clabel(ln, fmt="%g", fontsize=6.5)
    if mark:
        ax.scatter([mark[0]], [mark[1]], s=46, facecolor="none", edgecolor=c["aux"], linewidth=1.6, zorder=5)
        ax.annotate(mark[2], (mark[0], mark[1]), xytext=(8, 8), textcoords="offset points", fontsize=7.5, color=c["aux"])
    ax.set_xlabel(xlabel)
    ax.set_ylabel(ylabel)
    cb = fig.colorbar(cs, ax=ax, pad=0.02)
    cb.set_label(zlabel, color=c["muted"])
    cb.outline.set_visible(False)
    return _save(fig, ax, out, title or f"{zlabel} over {xlabel} and {ylabel}", source)


def tornado(rows, base, out, xlabel, title=None, source=None, low_label="low end of range", high_label="high end of range"):
    """How far each lever moves a goal. `rows` = [(lever, goal_at_low, goal_at_high)], `base` = goal at the base
    settings. Sorted by swing; the colour says which end of the lever's range pushed the goal which way."""
    c = _style()["colors"]
    rows = sorted(rows, key=lambda r: -abs(r[2] - r[1]))
    fig, ax = _fig(24 + 12 * len(rows))
    ax.grid(axis="y", visible=False)
    ax.grid(axis="x", visible=True)
    for i, (_, a, b) in enumerate(rows):
        ax.barh(i, a - base, left=base, color=c["aux"], height=0.55)
        ax.barh(i, b - base, left=base, color=c["stock"], height=0.55)
    ax.axvline(base, color=c["text"], lw=0.8)
    ax.set_yticks(range(len(rows)), [r[0] for r in rows])
    ax.invert_yaxis()
    ax.spines["left"].set_visible(False)
    ax.set_xlabel(xlabel)
    ax.text(0.0, 1.02, low_label, transform=ax.transAxes, color=c["aux"], fontsize=8)
    ax.text(1.0, 1.02, high_label, transform=ax.transAxes, ha="right", color=c["stock"], fontsize=8)
    return _save(fig, ax, out, title or f"How far each lever can move {xlabel}", source or "one lever at a time, the rest at base values")


def root_locus(points, out, lever="parameter", title=None, source=None):
    """Dominant eigenvalue in the complex plane as a lever moves: `points` = [(lever_value, re, im)] (see
    `eigen_path`). Left of the black line the model is stable; a dot crossing it means instability. Height is
    the speed of oscillation."""
    import numpy as np
    plt, sns = _lib()
    c = _style()["colors"]
    P = np.array(points, dtype=float)
    fig, ax = _fig(85)
    ax.grid(True)
    ax.axvline(0, color=c["text"], lw=0.8)
    ax.plot(P[:, 1], P[:, 2], color=c["muted"], lw=0.7, zorder=2)
    sc = ax.scatter(P[:, 1], P[:, 2], c=P[:, 0], cmap=sns.light_palette(c["aux"], as_cmap=True), s=36,
                    edgecolor=c["muted"], linewidth=0.5, zorder=3)
    ax.set_xlabel("real part (decay rate; right of the line = unstable)")
    ax.set_ylabel("imaginary part (oscillation)")
    cb = fig.colorbar(sc, ax=ax, pad=0.02)
    cb.set_label(lever, color=c["muted"])
    cb.outline.set_visible(False)
    return _save(fig, ax, out, title or f"Stability as {lever} changes (root locus)", source or "dominant eigenvalue of the linearised model")


# ---- Sankey: where a quantity goes between stocks ----------------------------------------------------------

def flow_network(model, boundary=None):
    """The model's stocks and the flows between them: ([stock names], [(flow, from_stock_or_None, to_stock_or_None)]).
    None means a cloud (outside the model). `boundary` (a list of stock names) limits the system to those stocks:
    a flow to or from any other stock then counts as crossing the boundary (a cloud), and a flow that touches none of
    them is dropped (use it to leave out a bookkeeping stock such as "Catch to date")."""
    v = model.variables()
    stocks = [x["name"] for x in v if x["kind"] == "Stock"]
    flows = [(x["name"], x["from"], x["to"]) for x in v if x["kind"] == "Flow"]
    if boundary is not None:
        keep = [s for s in stocks if s in boundary]
        flows = [(f, a if a in keep else None, b if b in keep else None) for f, a, b in flows]
        flows = [(f, a, b) for f, a, b in flows if a is not None or b is not None]
        stocks = keep
    return stocks, flows


def trace(model, slices=None, n_slices=8, tol=1e-6, boundary=None, overrides=None):
    """Follow the contents of each stock through the model's own flows.

    At each slice time, every stock's contents are tagged by where they started. Each step, a flow moves the tagged
    amounts out of its source stock in proportion to what is there (people in a stage are interchangeable, which
    is what a stock-and-flow model assumes), and the tags arrive in the destination. Because the tags follow the
    same Euler steps as the engine, the tagged amounts add up to the engine's stocks exactly; if they do not
    (RK4, non-negative clipping, delays that remember who came first) this raises instead of drawing a wrong figure.

    Quantity that enters from a cloud is tagged by the flow that brought it, and quantity that leaves to a cloud is
    recorded by the flow that took it, so a budget can say "this much left through Harvest".

    Returns a dict: times, stocks, slice_times, totals (stock level at each slice), and, for each interval,
    `moves[k][(origin, dest)]` (origin a stock or "outside", dest a stock or "lost") and
    `detail[k][(origin, dest)]` (origin a stock or the inflow's name, dest a stock or the outflow's name)."""
    import numpy as np
    stocks, flows = flow_network(model, boundary)
    settings = model.info()["settings"]
    if settings["method"] != "Euler":
        raise ValueError("sankey traces Euler runs exactly; this model uses " + settings["method"])
    r = model.run(overrides=overrides, series=stocks + [f for f, _, _ in flows], every=1)
    t = np.array(r["times"])
    ser = {n: np.array(r["series"][n.lower()]) for n in stocks + [f for f, _, _ in flows]}
    if slices is None:
        slices = list(np.linspace(t[0], t[-1], n_slices))
    idx = [int(np.argmin(np.abs(t - s))) for s in slices]
    ns = len(stocks)
    inflows = [f for f, a, b in flows if a is None]
    outflows = [f for f, a, b in flows if b is None]
    origins = stocks + inflows                        # tag rows
    moves, details, worst = [], [], 0.0
    scale = max(1.0, max(np.abs(ser[s]).max() for s in stocks))
    for i0, i1 in zip(idx[:-1], idx[1:]):
        T = np.zeros((len(origins), ns))
        for j, s in enumerate(stocks):
            T[j, j] = ser[s][i0]
        out_by = {f: np.zeros(len(origins)) for f in outflows}
        for k in range(i0, i1):
            dt = t[k + 1] - t[k]
            nT = T.copy()
            for f, src, dst in flows:
                amount = ser[f][k] * dt
                if src is not None:
                    a = stocks.index(src)
                    level = ser[src][k]
                    moved = T[:, a] * (amount / level) if level != 0 else np.zeros(len(origins))
                    nT[:, a] -= moved
                    if dst is None:
                        out_by[f] += moved
                    else:
                        nT[:, stocks.index(dst)] += moved
                else:                                   # from a cloud: new quantity enters, tagged by this flow
                    nT[ns + inflows.index(f), stocks.index(dst)] += amount
            T = nT
            worst = max(worst, float(np.abs(T.sum(0) - np.array([ser[s][k + 1] for s in stocks])).max()))
        if worst > tol * scale:
            raise ValueError(f"the traced stocks differ from the model's by {worst:.3g}: this model's flows cannot be "
                             "traced exactly (non-negative clipping, a delay that remembers order, or RK4)")
        mv, dt_ = {}, {}
        for o, oname in enumerate(origins):
            agg = "outside" if o >= ns else oname
            for j in range(ns):
                if T[o, j] > 1e-12:
                    mv[(agg, stocks[j])] = mv.get((agg, stocks[j]), 0.0) + float(T[o, j])
                    dt_[(oname, stocks[j])] = float(T[o, j])
            for f in outflows:
                if out_by[f][o] > 1e-12:
                    mv[(agg, "lost")] = mv.get((agg, "lost"), 0.0) + float(out_by[f][o])
                    dt_[(oname, f)] = float(out_by[f][o])
        moves.append(mv)
        details.append(dt_)
    return {"times": t, "stocks": stocks, "slice_times": [float(t[i]) for i in idx],
            "totals": [[float(ser[s][i]) for s in stocks] for i in idx], "moves": moves, "detail": details,
            "balance_error": worst, "inflows": inflows, "outflows": outflows}


def sankey(model, out, slices=None, n_slices=8, order=None, title=None, source=None, min_share=0.004, size=(1000, 560),
           boundary=None):
    """Where a quantity is and where it went, as a Sankey (plotly draws the ribbons; needs plotly and kaleido).

    The stocks are drawn at several moments, with a ribbon for every amount that moved between them (colour =
    where it started). The amounts are exact: they come from `trace`, which follows the model's own flows.
    Quantity that entered from a cloud comes from a grey bar at the top of the column, quantity that left to a
    cloud ends in a grey bar at the bottom. Ribbons smaller than `min_share` of the total are left out (the
    figure says how much). All text sits outside the ribbons: stock names in the margins, the moments below.
    Writes SVG, PNG or PDF by the extension of `out`."""
    try:
        import plotly.graph_objects as go
    except ImportError as e:
        raise ImportError("plots.sankey needs plotly and kaleido: pip install plotly kaleido") from e
    st = _style()
    c = st["colors"]
    font = st["font"]
    tr = trace(model, slices, n_slices, boundary=boundary)
    stocks = order or tr["stocks"]
    K = len(tr["slice_times"])
    pal = [c["stock"], c["flow"], c["aux"], c["link"], c["error"], "#8aa0b8", "#7a9e7e", "#b08968"]
    col = {s: pal[i % len(pal)] for i, s in enumerate(stocks)}
    col["outside"] = col["lost"] = c["muted"]
    total = max(sum(row) for row in tr["totals"])
    links, hidden, shown = [], 0.0, 0.0
    for k, mv in enumerate(tr["moves"]):
        for (o, d), v in mv.items():
            if v < min_share * total:
                hidden += v
            else:
                shown += v
                links.append(((o, k), (d, k + 1), v))
    inn, outv = {}, {}
    for a, b, v in links:
        outv[a] = outv.get(a, 0.0) + v
        inn[b] = inn.get(b, 0.0) + v
    val = {n: max(inn.get(n, 0.0), outv.get(n, 0.0)) for n in set(inn) | set(outv)}
    GAP, THICK = 18, 14
    w, h = size
    DX0, DX1 = 0.15, 0.85
    inner = (0.13, 0.90)
    Hpx = (inner[1] - inner[0]) * h
    # the column layout: grey "outside" bar on top (a reserved slot so the stocks line up), stocks, grey "lost" bar below
    slot = max([val.get(("outside", k), 0.0) for k in range(K)] + [0.0])
    cols_nodes = []
    for k in range(K):
        stk = [(s, k) for s in stocks if (s, k) in val]
        lost = [("lost", k)] if ("lost", k) in val else []
        cols_nodes.append((stk, lost))
    units = [slot + sum(val[n] for n in stk) + sum(val[n] for n in lost) for stk, lost in cols_nodes]
    nmax = max((1 if slot > 0 else 0) + len(stk) + len(lost) for stk, lost in cols_nodes)
    ky = (Hpx - (nmax - 1) * GAP) / max(units)
    paper_y = lambda f: inner[1] - f * (inner[1] - inner[0])
    order_nodes, pos = [], {}
    for k, (stk, lost) in enumerate(cols_nodes):
        y = 0.0
        if slot > 0:
            if ("outside", k) in val:
                pos[("outside", k)] = (y, val[("outside", k)] * ky)
                order_nodes.append(("outside", k))
            y += slot * ky + GAP
        for n in stk + lost:
            pos[n] = (y, val[n] * ky)
            order_nodes.append(n)
            y += val[n] * ky + GAP
    ix = {n: i for i, n in enumerate(order_nodes)}
    xs = [0.001 + 0.998 * k / (K - 1) for _, k in order_nodes]
    ys = [(pos[n][0] + pos[n][1] / 2) / Hpx for n in order_nodes]
    ann = []
    unit = model.info()["settings"]["time_units"].lower()
    xp = lambda frac: DX0 + frac * (DX1 - DX0)
    for k, t in enumerate(tr["slice_times"]):
        ann.append(dict(x=xp(k / (K - 1)), y=inner[0] - 0.02, xref="paper", yref="paper", text=f"{unit} {t:g}", showarrow=False,
                        yanchor="top", font=dict(size=12, color=c["muted"], family=font)))
    for s in stocks:                                   # stock names in the margins, beside the first and last bars
        for k, side in ((0, "left"), (K - 1, "right")):
            if (s, k) in pos:
                f = (pos[(s, k)][0] + pos[(s, k)][1] / 2) / Hpx
                ann.append(dict(x=DX0 - 0.012 if side == "left" else DX1 + 0.012, y=paper_y(f), xref="paper", yref="paper",
                                xanchor="right" if side == "left" else "left", yanchor="middle", showarrow=False, text=s,
                                font=dict(size=13, family=font, color=col[s])))
    for kind, text, side in (("outside", "came in from outside", "left"), ("lost", "left the model", "right")):
        ks = [k_ for k_ in range(K) if (kind, k_) in pos]
        if not ks:
            continue
        k_ = ks[0] if side == "left" else ks[-1]
        f = (pos[(kind, k_)][0] + pos[(kind, k_)][1] / 2) / Hpx
        if k_ == 0:                                    # first column: the label goes in the left margin
            x, anchor = DX0 - 0.012, "right"
        elif k_ == K - 1:                              # last column: the label goes in the right margin
            x, anchor = DX1 + 0.012, "left"
        else:                                          # inside the figure: above its own bar, clear of the ribbons
            x, anchor, f = xp(k_ / (K - 1)), "center", (pos[(kind, k_)][0] - 12) / Hpx
        ann.append(dict(x=x, y=paper_y(f), xref="paper", yref="paper", xanchor=anchor, yanchor="middle", showarrow=False,
                        text=text, font=dict(size=12, family=font, color=c["muted"])))
    note = source or "ribbon = amount moving between stocks, traced through the model's flows; colour = where it started"
    if hidden > 0:
        note += f"; ribbons under {min_share:.1%} of the total are not drawn ({hidden / (hidden + shown):.1%} of the movement)"
    ann.append(dict(x=0.005, y=0.005, xref="paper", yref="paper", xanchor="left", yanchor="bottom", showarrow=False,
                    text="Source: " + note, font=dict(size=10, color=c["muted"], family=font)))
    fig = go.Figure(go.Sankey(
        arrangement="fixed", domain=dict(x=[DX0, DX1], y=list(inner)),
        node=dict(label=[""] * len(order_nodes), color=[col[kind] for kind, _ in order_nodes], x=xs, y=ys, pad=GAP, thickness=THICK, line=dict(width=0)),
        link=dict(source=[ix[a] for a, b, v in links], target=[ix[b] for a, b, v in links], value=[v for a, b, v in links],
                  color=[col[a[0]] + "66" for a, b, v in links])))
    fig.update_layout(width=w, height=h, margin=dict(l=6, r=6, t=6, b=24), annotations=ann, paper_bgcolor=st.get("background") or "white",
                      title=dict(text=f"<b>{title or 'Where it is, and where it went'}</b>", x=0.005, xanchor="left", y=0.97,
                                 font=dict(size=22, family=font, color=c["text"])))
    fig.write_image(str(out), scale=2)
    return out


# ---- sequence diagram: who reacts to a change, in what order ------------------------------------------------

def reaction_order(model, change, threshold=0.01, kinds=("Stock", "Flow", "Auxiliary")):
    """Run the model with and without `change` ({name: value}); for each variable return the first time it differs
    from the base run by more than `threshold` of its own range. Returns {name: time or None}, in model order."""
    import numpy as np
    v = [x for x in model.variables() if x["kind"] in kinds]
    names = [x["name"] for x in v]
    base = model.run(series=names, every=1)
    pert = model.run(overrides=change, series=names, every=1)
    t = np.array(base["times"])
    out = {}
    for n in names:
        a, b = np.array(base["series"][n.lower()]), np.array(pert["series"][n.lower()])
        rng = max(np.abs(a).max(), np.abs(b).max(), 1e-12)
        hit = np.abs(b - a) > threshold * rng
        out[n] = float(t[int(np.argmax(hit))]) if hit.any() else None
    return out, {x["name"]: x["kind"] for x in v}


def sequence_text(model, change, threshold=0.01):
    """The Mermaid text of the sequence diagram (see `sequence`)."""
    onset, kind = reaction_order(model, change, threshold)
    moved = {n: t for n, t in onset.items() if t is not None}
    rank = lambda n: (moved[n], 0 if kind[n] == "Flow" else 1 if kind[n] == "Auxiliary" else 2, n)
    order = sorted(moved, key=rank)
    inputs = list(change)
    ident = {n: f"P{i}" for i, n in enumerate(inputs + order)}
    day = lambda t: f"{t:.1f}".rstrip("0").rstrip(".")
    L = ["sequenceDiagram", "    autonumber"]
    for n in inputs:
        L.append(f"    participant {ident[n]} as {n} (changed)")
    for n in order:
        L.append(f"    participant {ident[n]} as {n}" + (" (stock)" if kind[n] == "Stock" else ""))
    chg = ", ".join(f"{k} = {v:g}" for k, v in change.items())
    L.append(f"    Note over {ident[inputs[0]]},{ident[order[-1]]}: change from the start: {chg}")
    msgs = []
    for e in order:
        causes = [c for c in model.dependencies(e)["causes"] if c != e]
        for c in causes:
            if c in change:
                msgs.append(((moved[e], 0, 0, c, e), f"    {ident[c]}->>{ident[e]}: {e} reacts, day {day(moved[e])}"))
            elif c in moved and rank(c) < rank(e):
                msgs.append(((moved[e], 1, moved[c], c, e), f"    {ident[c]}->>{ident[e]}: {e} reacts, day {day(moved[e])}"))
            elif c in moved:                                    # the cause reacts later: this link closes a feedback loop
                msgs.append(((moved[c], 2, moved[e], c, e), f"    {ident[c]}-->>{ident[e]}: feedback, day {day(moved[c])}"))
    L += [m for _, m in sorted(msgs, key=lambda x: x[0])]
    skipped = [n for n, t in onset.items() if t is None and kind[n] != "Constant"]
    if skipped:
        L.append(f"    Note over {ident[inputs[0]]},{ident[order[-1]]}: no reaction: {', '.join(skipped[:6])}")
    return "\n".join(L) + "\n"


def sequence(model, change, out, threshold=0.01):
    """A UML-style sequence diagram of how a change spreads through the model, drawn by Mermaid (`mmdc`, the
    Mermaid command line tool, needs Node and Chrome). Each variable that reacts is a participant (stocks are marked);
    an arrow goes from each cause to each effect along the model's own links, in the order the effects first
    differ from the base run by more than `threshold` of their range (the label gives the day); dashed arrows are
    the links that close a feedback loop (the cause reacts later than the effect). Writes SVG, PNG or PDF."""
    import json, os, shutil, subprocess, tempfile
    mmdc = shutil.which("mmdc")
    if not mmdc:
        raise RuntimeError("plots.sequence needs the Mermaid command line tool: npm install -g @mermaid-js/mermaid-cli")
    st = _style()
    c = st["colors"]
    cfg = {"theme": "base", "themeVariables": {
        "fontFamily": f'{st["font"]}, Georgia, serif', "fontSize": "14px", "primaryColor": c.get("stock_fill", "#eef3fa"),
        "primaryBorderColor": c["stock"], "primaryTextColor": c["text"], "lineColor": c["link"],
        "actorBkg": c.get("stock_fill", "#eef3fa"), "actorBorder": c["stock"], "actorTextColor": c["text"],
        "actorLineColor": c["grid"], "signalColor": c["stock"], "signalTextColor": c["text"],
        "noteBkgColor": "#fbf3e9", "noteBorderColor": c["aux"], "noteTextColor": c["text"],
        "labelBoxBkgColor": c["grid"], "sequenceNumberColor": "#ffffff", "activationBkgColor": c["flow"]}}
    text = sequence_text(model, change, threshold)
    with tempfile.TemporaryDirectory() as d:
        src, conf = os.path.join(d, "s.mmd"), os.path.join(d, "c.json")
        open(src, "w").write(text)
        json.dump(cfg, open(conf, "w"))
        pup = os.path.join(d, "p.json")
        json.dump({"args": ["--no-sandbox"]}, open(pup, "w"))
        args = [mmdc, "-i", src, "-o", str(out), "-c", conf, "-p", pup, "-b", st.get("background") or "white"]
        if str(out).endswith(".png"):
            args += ["-s", "3"]
        r = subprocess.run(args, capture_output=True, text=True)
        if r.returncode != 0:
            raise RuntimeError("mmdc failed: " + (r.stderr or r.stdout)[:400])
    return out


def budget(model, scenarios, channels, groups, out, boundary=None, window=None, sources=None, unit="units",
           headline=None, title=None, source=None, size=(1040, 820), min_share=0.003):
    """A budget Sankey, one panel per scenario: a quantity goes in (what is in the system at the start, plus what
    enters from outside), splits into channels (the flows that carry it away, or where it ends up), and the
    channels are grouped into outcomes. Shares are percentages of the total put in, so panels compare directly.
    All text is outside the ribbons: sources on the left, outcomes on the right, channel names in the gap above
    each channel bar.

    scenarios : [(panel title, {lever: value})]   (an empty dict is the model as it is)
    channels  : {channel name: [keys]}            a key is an outflow's name, or "end:<Stock>" for what remains in a stock
    groups    : {group name: (colour, [channel names])}   colour is "loss", "good", "neutral" or "#rrggbb"
    sources   : {"start": label, "<inflow name>": label}   optional labels for the left-hand nodes
    window    : (t0, t1), default the whole run;  headline: a group name, quoted in each panel's subtitle.
    Exact: the amounts come from `trace`, which refuses to continue if the traced stocks would not balance."""
    try:
        import plotly.graph_objects as go
    except ImportError as e:
        raise ImportError("plots.budget needs plotly and kaleido: pip install plotly kaleido") from e
    st = _style()
    c = st["colors"]
    named = {"loss": c["error"], "good": c["stock"], "neutral": c["muted"]}
    gcol = {g: named.get(col, col) for g, (col, _) in groups.items()}
    chan_group = {ch: g for g, (_, chs) in groups.items() for ch in chs}
    chan_rank = {ch: (list(groups).index(chan_group[ch]), i) for i, ch in enumerate(channels) if ch in chan_group}
    unit_t = model.info()["settings"]["time_units"].lower()
    w, h = size
    n = len(scenarios)
    pad = 0.012
    ph = (1 - pad * (n + 1) - 0.07 - 0.03) / n                     # panel height (paper coordinates)
    DX0, DX1 = 0.215, 0.79                                         # the drawing area; labels live outside it
    THICK, GAP = 16, 24                                            # node thickness and gap between nodes, in pixels
    traces, shapes, ann = [], [], []
    font = st["font"]

    def lighten(hexcol, a):
        return hexcol + format(int(a * 255), "02x")

    for pi, (stitle, over) in enumerate(scenarios):
        tr = trace(model, slices=list(window) if window else None, n_slices=2, boundary=boundary, overrides=over or None)
        d = tr["detail"][0]
        tot = sum(d.values())
        src_of = lambda o: "start" if o in tr["stocks"] else o
        src_label = {"start": "At the start"}
        src_label.update({f: f for f in tr["inflows"]})
        src_label.update(sources or {})
        key_of = lambda dest: (f"end:{dest}" if dest in tr["stocks"] else dest)
        amt = {}                                                   # (source, channel) -> amount
        for (o, dest), v in d.items():
            ch = next((cn for cn, keys in channels.items() if key_of(dest) in keys), None)
            if ch is None:
                raise ValueError(f"no channel takes {key_of(dest)!r}: add it to `channels`")
            amt[(src_of(o), ch)] = amt.get((src_of(o), ch), 0.0) + v
        amt = {k: v for k, v in amt.items() if v >= min_share * tot}
        pct = lambda v: "<1%" if 0 < v / tot < 0.005 else f"{v / tot:.0%}"
        src_tot, ch_tot, g_tot = {}, {}, {}
        for (so, ch), v in amt.items():
            src_tot[so] = src_tot.get(so, 0.0) + v
            ch_tot[ch] = ch_tot.get(ch, 0.0) + v
            g_tot[chan_group[ch]] = g_tot.get(chan_group[ch], 0.0) + v
        cols = [
            ("src", sorted(src_tot, key=lambda s: ["start"] + tr["inflows"] and (["start"] + tr["inflows"]).index(s)), src_tot, 0.001),
            ("chan", sorted(ch_tot, key=lambda ch: chan_rank[ch]), ch_tot, 0.5),
            ("grp", sorted(g_tot, key=lambda g: list(groups).index(g)), g_tot, 0.999),
        ]
        y1 = 1 - pad - 0.07 - pi * (ph + pad)
        y0 = y1 - ph
        inner = (y0 + 0.025, y1 - 0.15)                            # leave room for the panel title and subtitle
        Hpx = (inner[1] - inner[0]) * h
        nmax = max(len(c_[1]) for c_ in cols)
        ky = (Hpx - (nmax - 1) * GAP) / max(sum(c_[2].values()) for c_ in cols)
        index, labels, colors, xs, ys = {}, [], [], [], []
        paper_y = lambda f: inner[1] - f * (inner[1] - inner[0])  # f = 0 at the top of the drawing area
        dom_w = DX1 - DX0
        for kind, names, tots, xfrac in cols:
            colh = sum(tots[nm] * ky for nm in names) + (len(names) - 1) * GAP
            top = (Hpx - colh) / 2
            for nm in names:
                hh = tots[nm] * ky
                f_centre = (top + hh / 2) / Hpx
                index[(kind, nm)] = len(labels)
                labels.append("")
                colors.append(c["link"] if kind == "src" else gcol[nm if kind == "grp" else chan_group[nm]])
                xs.append(xfrac)
                ys.append(f_centre)
                text = {"src": src_label.get(nm, nm), "chan": nm, "grp": nm}[kind] + f" ({pct(tots[nm])})"
                col_ = colors[-1]
                if kind == "src":
                    ann.append(dict(x=DX0 - 0.006, y=paper_y(f_centre), xref="paper", yref="paper", xanchor="right", yanchor="middle",
                                    showarrow=False, text=text, align="right", font=dict(size=13, family=font, color=c["text"])))
                elif kind == "grp":
                    ann.append(dict(x=DX1 + 0.012, y=paper_y(f_centre), xref="paper", yref="paper", xanchor="left", yanchor="middle",
                                    showarrow=False, text=f"<b>{text}</b>", align="left", font=dict(size=14, family=font, color=col_)))
                else:                                              # channel: in the gap above its bar
                    node_top = paper_y((top + 0) / Hpx)
                    ann.append(dict(x=DX0 + 0.5 * dom_w + 0.012, y=node_top + 3 / h, xref="paper", yref="paper", xanchor="left", yanchor="bottom",
                                    showarrow=False, text=text, align="left", bgcolor="#eeeef1", borderpad=1, font=dict(size=12, family=font, color=col_)))
                top += hh + GAP
        S_, T_, V_, C_ = [], [], [], []
        for (so, ch), v in sorted(amt.items(), key=lambda kv: (chan_rank[kv[0][1]], kv[0][0])):
            g = chan_group[ch]
            S_.append(index[("src", so)]); T_.append(index[("chan", ch)]); V_.append(v); C_.append(lighten(gcol[g], 0.5))
        for ch in cols[1][1]:
            g = chan_group[ch]
            S_.append(index[("chan", ch)]); T_.append(index[("grp", g)]); V_.append(ch_tot[ch]); C_.append(lighten(gcol[g], 0.5))
        traces.append(go.Sankey(
            arrangement="fixed", domain=dict(x=[DX0, DX1], y=list(inner)),
            node=dict(label=labels, color=colors, x=xs, y=ys, pad=GAP, thickness=THICK, line=dict(width=0)),
            link=dict(source=S_, target=T_, value=V_, color=C_)))
        shapes.append(dict(type="rect", xref="paper", yref="paper", x0=0.005, x1=0.995, y0=y0, y1=y1,
                           fillcolor="#eeeef1", line=dict(width=0), layer="below"))
        span = tr["slice_times"][1] - tr["slice_times"][0]
        sub = f"Put 100 {unit} in; {tot:,.0f} actually went in over {span:g} {unit_t}s"
        if headline:
            sub = f"Put 100 {unit} in; {g_tot[headline] / tot * 100:.0f} end up: {headline.lower()}  ({tot:,.0f} {unit} in total over {span:g} {unit_t}s)"
        ann.append(dict(x=0.015, y=y1 - 0.012, xref="paper", yref="paper", xanchor="left", yanchor="top", showarrow=False,
                        text=f"<b>{stitle}</b>", font=dict(size=20, family=font, color=c["text"])))
        ann.append(dict(x=0.015, y=y1 - 0.06, xref="paper", yref="paper", xanchor="left", yanchor="top", showarrow=False,
                        text=sub, font=dict(size=13, family=font, color=c["text"])))
    fig = go.Figure(traces)
    ann.append(dict(x=0.005, y=0.005, xref="paper", yref="paper", xanchor="left", yanchor="bottom", showarrow=False,
                    text="Source: " + (source or "traced through the model's own flows; shares are percentages of the quantity put in"),
                    font=dict(size=10, color=c["muted"], family=font)))
    fig.update_layout(width=w, height=h, shapes=shapes, annotations=ann, margin=dict(l=6, r=6, t=6, b=22),
                      paper_bgcolor=st.get("background") or "white",
                      title=dict(text=f"<b>{title}</b>" if title else None, x=0.005, xanchor="left", y=0.985,
                                 font=dict(size=22, family=font, color=c["text"])))
    fig.write_image(str(out), scale=2)
    return out
