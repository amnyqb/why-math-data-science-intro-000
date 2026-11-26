"""Flow views for wiki/Charting-Ideas.md (Sankey, stage board, cycle, sequence) on a small SEIR model with waning immunity.
Build the model first:  dyn convert book/examples/charting/seir_waning.mdl /tmp/seir.sdm   then run from the repo root with FC_BIN set. ROUGH PROTOTYPES."""
import numpy as np
import dynamist as fc
from dynamist import plots
from matplotlib.patches import PathPatch, Polygon, FancyArrowPatch, Rectangle
from matplotlib.path import Path
plots.use_style()
plt, sns = plots._lib()
c = plots._style()["colors"]; MM = plots.MM
O = "wiki/img/charting/"
m = fc.Model("/tmp/seir.sdm")
names = ["Susceptible", "Exposed", "Infectious", "Recovered"]; cols = [c["link"], c["aux"], c["error"], c["flow"]]
rates = {("Susceptible", "Exposed"): "infection", ("Exposed", "Infectious"): "onset", ("Infectious", "Recovered"): "recovery", ("Recovered", "Susceptible"): "waning"}
r = m.run(series=[n.lower() for n in names] + list(rates.values()), every=8)
t = np.array(r["times"]); dt = t[1] - t[0]
stock = {n: np.array(r["series"][n.lower()]) for n in names}
flow = {k: np.array(r["series"][v]) for k, v in rates.items()}

# ---- 1. Sankey: exact, traced through the model's own flows (see plots.trace)
plots.sankey(m, O + "13_sankey.png", slices=[0, 20, 40, 60, 80, 100, 140, 200, 400], order=["Recovered", "Infectious", "Exposed", "Susceptible"],
             title="Where the population is, and who moved where",
             source="SEIR with waning immunity; ribbon = people moving between stages (traced through the model's own flows); colour = where they started")

# ---- 2. kanban board with arrows: columns = stages, cards = people, arrow thickness = current flow
def board(ax, i, label, ymax):
    xs = np.arange(4)
    for x, n, col in zip(xs, names, cols):
        h = stock[n][i]; ax.add_patch(Rectangle((x - 0.32, 0), 0.64, h, color=col, lw=0, alpha=0.9))
        ax.text(x, -ymax * 0.05, {"Susceptible": "S", "Exposed": "E", "Infectious": "I", "Recovered": "R"}[n], ha="center", va="top", fontsize=6.5, color=c["muted"])
    for (s, d), f in flow.items():
        a, b = names.index(s), names.index(d); w = flow[(s, d)][i]
        wmax = max(fl.max() for fl in flow.values())
        lw = 0.4 + 9 * w / wmax
        rad = -0.5 if (s, d) == ("Recovered", "Susceptible") else 0
        y0 = ymax * (0.92 if rad else 0.8)
        ax.add_patch(FancyArrowPatch((a + (0 if rad else 0.34), y0), (b + (0 if rad else -0.34), y0), connectionstyle=f"arc3,rad={rad}", arrowstyle="-|>", mutation_scale=6, lw=lw, color=cols[a], alpha=0.9))
    ax.set_xlim(-0.6, 3.6); ax.set_ylim(0, ymax); ax.axis("off"); ax.set_title(label, fontsize=8, color=c["muted"], loc="left")
fig, axs = plt.subplots(2, 3, figsize=(plots._style()["width_mm"] * MM, 90 * MM))
ymax = max(stock[n].max() for n in names) * 1.15
for ax, s in zip(axs.flat, [0, 40, 70, 100, 160, 300]): board(ax, int(np.argmin(abs(t - s))), f"day {s}", ymax)
plots._save(fig, axs[0][0], O + "14_board.png", "A board of stages: who is where, and how fast cards move", "S, E, I, R = Susceptible, Exposed, Infectious, Recovered. Arrow thickness = current flow; the long arrow over the top is waning, back to the start")

# ---- 3. cycle diagram: stages on a ring, thickness = total people passed through over the run
fig, ax = plots._fig(90); ax.grid(False); ax.axis("off"); ax.set_aspect("equal")
ang = np.array([135, 45, -45, -135]) * np.pi / 180; pos = {n: np.array([np.cos(a), np.sin(a)]) for n, a in zip(names, ang)}
tot = {k: v.sum() * dt for k, v in flow.items()}; tmax = max(tot.values())
for (s_, d), v in tot.items():
    p, q = pos[s_], pos[d]
    ax.add_patch(FancyArrowPatch(p, q, connectionstyle="arc3,rad=-0.18", arrowstyle="-|>", mutation_scale=14, lw=0.8 + 8 * v / tmax, color=cols[names.index(s_)], alpha=0.85, shrinkA=16, shrinkB=16))
    mid = (p + q) / 2; out = mid / np.linalg.norm(mid)
    ax.text(*(mid + out * 0.62), f"{v:,.0f} people", ha="center", va="center", fontsize=7.5, color=cols[names.index(s_)])
for n, col in zip(names, cols):
    ax.scatter(*pos[n], s=180 + 900 * stock[n].max() / N, color=col, zorder=3)
    ax.text(*(pos[n] * 1.42), n, ha="center", va="center", fontsize=8.5, color=col, zorder=4)
ax.set_xlim(-2.2, 2.2); ax.set_ylim(-1.9, 1.9)
plots._save(fig, ax, O + "15_cycle.png", "The cycle the population goes round", "arrow = total people moving over 400 days; circle area = peak number in the stage")

# ---- 4. sequence diagram (UML style, drawn by Mermaid): how a change spreads, and where the loops close
plots.sequence(m, {"contact rate": 0.30}, O + "16_sequence.png")
