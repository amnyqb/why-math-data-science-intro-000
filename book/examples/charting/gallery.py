"""Builds the figures of wiki/Charting-Ideas.md from the Workforce-Inventory benchmark model. Run from the repo root:
FC_BIN=target/release/dyn PYTHONPATH=python python book/examples/charting/gallery.py  (needs seaborn; see python/requirements-plots.txt)"""
import json, csv, shutil, subprocess, numpy as np
import dynamist as fc
from dynamist import plots
plots.use_style()
plt, sns = plots._lib()
c = plots._style()["colors"]; MM = plots.MM
O = "wiki/img/charting/"
R = "benchmarks/b3/results/b2-1/"
SRC = "benchmarks/b3/models/b2-1-workforce-inventory-core.sdm"
m = fc.Model(SRC)
PAR = {"Hiring Time": (1, 8), "Inventory Correction Time": (0.5, 8), "Desired Inventory": (50, 150),
       "Demand Step": (0, 20), "Demand Adjustment Time": (0.5, 8), "Productivity per Worker": (1, 4)}
base_over = {"Hiring Time": 4, "Inventory Correction Time": 4, "Desired Inventory": 100, "Demand Step": 10, "Demand Adjustment Time": 3, "Productivity per Worker": 2}
def run(**o): return m.run(overrides=o, series=["inventory", "workforce"], every=8)
inv = lambda **o: np.array(run(**o)["series"]["inventory"])
rng = np.random.default_rng(1)
pal = [c["stock"], c["flow"], c["aux"], c["link"], c["error"]]

# 1 policy landscape: lowest inventory over two levers, contours + chosen point
ht = np.linspace(1, 8, 22); ct = np.linspace(0.5, 8, 22)
Z = np.array([[inv(**{"Hiring Time": h, "Inventory Correction Time": k}).min() for h in ht] for k in ct])
fig, ax = plots._fig(95); ax.grid(False)
cs = ax.contourf(ht, ct, Z, levels=12, cmap=sns.light_palette(c["stock"], as_cmap=True, reverse=True))
ln = ax.contour(ht, ct, Z, levels=12, colors="white", linewidths=0.5)
ax.clabel(ln, fmt="%d", fontsize=6.5)
ax.set_xlabel("hiring time (months)"); ax.set_ylabel("inventory correction time (months)")
cb = fig.colorbar(cs, ax=ax, pad=0.02); cb.set_label("lowest inventory", color=c["muted"]); cb.outline.set_visible(False)
plots._save(fig, ax, O + "01_landscape.png", "Where the dip is shallow: a policy landscape", "Workforce-Inventory, 484 runs")

# 2 tornado: goal swing from each lever's low to high, others at base
goal = lambda **o: inv(**o).min()
b0 = goal()
rows = []
for p, (lo, hi) in PAR.items():
    rows.append((p, goal(**{p: lo}), goal(**{p: hi})))
rows.sort(key=lambda r: -abs(r[2] - r[1]))
fig, ax = plots._fig(24 + 12 * len(rows)); ax.grid(axis="y", visible=False); ax.grid(axis="x", visible=True)
for i, (p, a, b) in enumerate(rows):
    ax.barh(i, a - b0, left=b0, color=c["aux"], height=0.55); ax.barh(i, b - b0, left=b0, color=c["stock"], height=0.55)
ax.axvline(b0, color=c["text"], lw=0.8)
ax.set_yticks(range(len(rows)), [r[0] for r in rows]); ax.invert_yaxis(); ax.spines["left"].set_visible(False)
ax.set_xlabel("lowest inventory")
ax.text(0.0, 1.02, "low end of range", transform=ax.transAxes, color=c["aux"], fontsize=8); ax.text(1, 1.02, "high end of range", transform=ax.transAxes, ha="right", color=c["stock"], fontsize=8)
plots._save(fig, ax, O + "02_tornado.png", "How far each lever can move the dip", "one lever at a time, the rest at base values")

# 3 spaghetti + outcome distribution at a time slice (joint view)
N = 150
S_ = []; X = []
for _ in range(N):
    o = {p: rng.uniform(*PAR[p]) for p in PAR}; X.append(o); S_.append(inv(**o))
S_ = np.array(S_); t = run()["times"]
fig = plt.figure(figsize=(plots._style()["width_mm"] * MM, 85 * MM))
gs = fig.add_gridspec(1, 2, width_ratios=[4, 1], wspace=0.03)
a = fig.add_subplot(gs[0]); b = fig.add_subplot(gs[1], sharey=a)
for s in S_: a.plot(t, s, color=c["stock"], alpha=0.12, lw=0.8)
a.plot(t, np.median(S_, 0), color=c["stock"], lw=2)
k = int(np.argmin(np.abs(np.array(t) - 30))); a.axvline(30, color=c["muted"], lw=0.7, ls=(0, (3, 3)))
a.set_xlabel("time (months)"); a.set_ylabel("inventory")
b.hist(S_[:, k], bins=24, orientation="horizontal", color=c["aux"], alpha=0.9); b.grid(False); b.axis("off")
b.text(0.5, 1.02, "at month 30", transform=b.transAxes, ha="center", fontsize=8, color=c["muted"])
fig.subplots_adjust(left=0.1, right=0.97, top=0.82, bottom=0.2)
fig.suptitle("Every plausible future, and how they land", x=0.01, ha="left", fontsize=11.25, color=c["text"])
fig.text(0.01, 0.01, "Source: 150 draws over all six levers", fontsize=7, color=c["muted"])
fig.savefig(O + "03_spaghetti_marginal.png", dpi=300)

# 4 time-coloured line: behaviour coloured by the loop in charge
rows_ = list(csv.DictReader(open(R + "dominance.csv"))); tt = np.array([float(r["time"]) for r in rows_])
sc = np.abs(np.array([[float(r["loop1"]), float(r["loop2"])] for r in rows_])); dom = sc.argmax(1)
bs = m.run(series=["inventory"], every=1); bt = np.array(bs["times"]); bv = np.array(bs["series"]["inventory"])
dv = np.interp(bt, tt, dom)
fig, ax = plots._fig(70)
from matplotlib.collections import LineCollection
pts_ = np.column_stack([bt, bv]); segs = np.stack([pts_[:-1], pts_[1:]], 1)
lc = LineCollection(segs, colors=[[c["stock"], c["aux"]][int(round(d))] for d in dv[:-1]], linewidths=2.4, capstyle="butt"); ax.add_collection(lc)
ax.set_xlim(bt[0], bt[-1]); ax.set_ylim(bv.min() - 3, bv.max() + 3)
ax.set_xlabel("time (months)"); ax.set_ylabel("inventory")
ax.text(0.99, 0.95, "Loop 1 in charge", transform=ax.transAxes, ha="right", color=c["stock"], fontsize=8); ax.text(0.99, 0.87, "Loop 2 in charge", transform=ax.transAxes, ha="right", color=c["aux"], fontsize=8)
plots._save(fig, ax, O + "04_line_by_dominant_loop.png", "The line changes colour when the loop in charge changes", "Loops That Matter")

# 5 parallel coordinates of draws, coloured by the outcome
M_ = np.array([[o[p] for p in PAR] for o in X]); lowest = S_.min(1)
Mn = (M_ - np.array([PAR[p][0] for p in PAR])) / np.array([PAR[p][1] - PAR[p][0] for p in PAR])
fig, ax = plots._fig(85); ax.grid(axis="y", visible=False); ax.grid(axis="x", visible=True)
cmap = sns.light_palette(c["stock"], as_cmap=True); nrm = (lowest - lowest.min()) / (lowest.max() - lowest.min())
Mn2 = np.column_stack([Mn, nrm])
deep = nrm <= np.quantile(nrm, 0.15)
for r, d in zip(Mn2, deep):
    if not d: ax.plot(range(Mn2.shape[1]), r, color=c["grid"], lw=0.8, zorder=1)
for r, d in zip(Mn2, deep):
    if d: ax.plot(range(Mn2.shape[1]), r, color=c["aux"], lw=1.5, alpha=0.9, zorder=3)
ax.set_xticks(range(7), [p.replace(" ", "\n", 1) for p in PAR] + ["lowest\ninventory"], fontsize=7); ax.set_yticks([])
ax.spines["left"].set_visible(False)
plots._save(fig, ax, O + "05_parallel.png", "The deepest dips share a recipe", "150 draws; the deepest 15% of dips in orange; each lever scaled to its range (bottom = low)")

# 6 time-resolved sensitivity: share of variance explained by each lever through time
Xs = (M_ - M_.mean(0)) / M_.std(0); share = []
for j in range(S_.shape[1]):
    y = S_[:, j] - S_[:, j].mean(); beta, *_ = np.linalg.lstsq(Xs, y, rcond=None)
    if y.var() < 1e-12: share.append(np.zeros(len(PAR))); continue
    contrib = (beta ** 2) * 1.0; tot = y.var() * 1.0
    share.append(contrib / max(contrib.sum(), 1e-12) * min(1, contrib.sum() / (tot + 1e-12)))
share = np.array(share).T
fig, ax = plots._fig(80)
ax.stackplot(t, *share, labels=list(PAR), colors=sns.color_palette([c["stock"], c["flow"], c["aux"], c["link"], c["error"], "#8aa0b8"]), linewidth=0, alpha=0.95)
ax.set_xlim(t[0], t[-1]); ax.set_ylim(0, 1); ax.set_xlabel("time (months)"); ax.set_ylabel("share of spread explained")
ax.legend(loc="center left", bbox_to_anchor=(1.0, 0.5), fontsize=7)
plots._save(fig, ax, O + "06_time_resolved_sensitivity.png", "Which lever matters changes with the clock", "standardised regression on 150 draws, per time step")

# 7 step-response fingerprint: annotated behaviour features
y = inv(); tt2 = np.array(run()["times"]); i0 = int(np.argmax(np.abs(y - y[0]) > 1e-6)); kmin = int(np.argmin(y)); kmax = int(np.argmax(y[kmin:])) + kmin
fig, ax = plots._fig(80)
ax.plot(tt2, y, color=c["stock"]); ax.axhline(y[-1], color=c["muted"], lw=0.7, ls=(0, (3, 3)))
ax.scatter([tt2[kmin], tt2[kmax]], [y[kmin], y[kmax]], color=c["aux"], zorder=4, s=28)
ax.annotate(f"trough {y[kmin]:.0f} at month {tt2[kmin]:.0f}", (tt2[kmin], y[kmin]), xytext=(10, -4), textcoords="offset points", fontsize=7.5, color=c["aux"])
ax.annotate(f"overshoot {y[kmax]-y[-1]:.1f} at month {tt2[kmax]:.0f}", (tt2[kmax], y[kmax]), xytext=(10, 2), textcoords="offset points", fontsize=7.5, color=c["aux"])
ax.annotate("shock", (tt2[i0], y[0]), xytext=(-4, 10), textcoords="offset points", ha="right", fontsize=7.5, color=c["muted"])
ax.set_xlabel("time (months)"); ax.set_ylabel("inventory")
plots._save(fig, ax, O + "07_fingerprint.png", "The behaviour in numbers: trough, overshoot, settling", "base run")

# 8 small multiples: one chart per hiring time, shared axes, base in grey
hs = [1, 2, 4, 6, 8, 12]
fig, axs = plt.subplots(2, 3, sharex=True, sharey=True, figsize=(plots._style()["width_mm"] * MM, 80 * MM))
yb = inv()
for ax, h in zip(axs.flat, hs):
    ax.plot(t, yb, color=c["grid"], lw=1.4); ax.plot(t, inv(**{"Hiring Time": h}), color=c["stock"], lw=1.6)
    ax.set_title(f"hiring time {h}", fontsize=8, color=c["muted"], loc="left")
for ax in axs[1]: ax.set_xlabel("month", fontsize=8)
plots._save(fig, axs[0][0], O + "08_small_multiples.png", "Same shock, six hiring times", "grey = base run")

# 9 root locus: dominant eigenvalue in the complex plane as hiring time sweeps
import os, tempfile
pts = []
for h in np.linspace(1, 12, 23):
    tmp = O + "tmp.sdm"; shutil.copy(SRC, tmp); mm = fc.Model(tmp)
    mm.apply([{"op": "set_equation", "name": "Hiring Time", "equation": str(h)}])
    try:
        r = subprocess.run([plots.__dict__.get("FC", "./target/release/dyn"), "importance", tmp, "--samples", "8", "--ensemble", "1", "--eigen-at", "0"], capture_output=True, text=True)
        d = json.loads(r.stdout); EV = d["eigenvalues"]["eigenvalues"]; ev = [e for e in EV if e["im"] > 0] or EV
        e = max(ev, key=lambda e: e["re"]); pts.append((h, e["re"], e["im"]))
    except Exception as ex: pass
if pts:
    P = np.array(pts); fig, ax = plots._fig(85); ax.grid(True)
    ax.axvline(0, color=c["text"], lw=0.8)
    sc_ = ax.scatter(P[:, 1], P[:, 2], c=P[:, 0], cmap=sns.light_palette(c["aux"], as_cmap=True), s=36, edgecolor=c["muted"], linewidth=0.5, zorder=3)
    ax.plot(P[:, 1], P[:, 2], color=c["muted"], lw=0.7, zorder=2)
    ax.set_xlabel("real part (decay rate; right of the line = unstable)"); ax.set_ylabel("imaginary part (oscillation)")
    cb = fig.colorbar(sc_, ax=ax, pad=0.02); cb.set_label("hiring time", color=c["muted"]); cb.outline.set_visible(False)
    plots._save(fig, ax, O + "09_root_locus.png", "How the oscillation changes as hiring slows (root locus)", "dominant eigenvalue of the linearised model")
print("points", len(pts))
