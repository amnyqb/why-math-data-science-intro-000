# Why results changed, the audit pack and the blind hindcast

Three commands of round 8 (branch `round-8`). Each prints Markdown for a person and JSON for a program. Every number below was measured on the Contabo server on 23 November 2026.

## `dyn attribute OLD NEW`: why the goals changed

```
dyn attribute old.sdm new.sdm [--json] [--out PREFIX] [--permutations 200] [--seed 1]
```

The edits between two versions are grouped, one group per changed variable. For every goal in the manifest of NEW (or, when it has none, the final value of every stock) the change is split between the groups by the Shapley value: a group's share is its average effect over every order in which the groups could be applied to OLD. The shares add up to the whole change.

- **Up to 10 groups:** every one of the 2^k combinations is run once (cached). Exact.
- **More than 10:** permutations are drawn with a fixed seed (`--seed`, `--permutations`). Each permutation adds up to the whole change on its own, so the sampled shares still add up exactly; the standard error of every share is printed.
- **A variable added or removed, and the equations that read it,** cannot be applied one at a time. When that happens they become one group, and the output says so.
- **What is not a player:** a changed comment, unit text, check, scenario, lever range or goal cannot move the base run. They are listed under "No effect on the goals". The manifest's lever defaults and ranges are not read by the base run, so they do not take part.
- **Columns:** the share, its percent of the change, what the group does alone, and the difference ("through others": what combining adds or takes away).
- `--out PREFIX` writes `PREFIX.md`, `PREFIX.json` and one SVG waterfall per goal (`PREFIX-waterfall-N.svg`).

Measured: on an additive model the shares equal the one-at-a-time changes; on a model where the flow is the product of two edited constants (A from 1 to 2, B from 1 to 3, flow = A × B, horizon T) the shares are 2T and 3T against changes alone of 1T and 2T, as the hand calculation says; on every test the parts add up to the change within 1e-9 relative, and repeated runs give identical text. On the Bass diffusion model with advertising effect 0.02 to 0.03 and word-of-mouth effect 0.45 to 0.35, the final potential adopters went from 969.5 to 3149.7; the word-of-mouth cut alone raised it by 4623.1 and the advertising rise alone lowered it by 428.7 (the sign is real: more advertising used the market up earlier); with both, the shares were 3616.0 and -1435.9, together 2180.1, the whole change. Six runs.

The waterfall is drawn by a small SVG writer in `core/src/plotsvg.rs`, not by the model-chart code, because that code is tied to the window's plots and has no waterfall.

## `dyn pack FILE --out DIR`: the audit pack

One folder:

| File | What it holds |
|---|---|
| `index.md` | links to every part, and what was not made and why |
| `doctor-report.md`, `.json` | the model doctor (C1 to C5) and the gate reached |
| `sanity-sheet.md`, `.json` | the sanity sheet of `--gate deliver` |
| `checks.md` | the model's own checks and conservation |
| `conversion-loss.md` | what the import kept and dropped (only when the model was imported) |
| `operation-log.jsonl` | the log of `dyn ops` (only when there is one) |
| `workbook.xlsx` | the Excel workbook of `dyn export --xlsx` |
| `model-card.md` | title, description, problem, kind, out of scope, validity, goals, objectives, tests, levers; filled only from the manifest and the brief. A field the model does not state reads "not stated" |

The exit code is 1 when any part failed to generate, and the index marks it FAILED.

**Not yet built: `reporting-checklist.md`.** It must list each item of Rahmandad and Sterman (2012), *System Dynamics Review* 28(4): 396-411, marked met, not met or not applicable, with the page. On 23 November 2026 the open copy of the paper (MIT DSpace) answered "429, retry after 900 seconds" three times, and the landing page gives only the abstract, so no item could be copied. Nothing was written from memory. The paper is in [papers/TO_FETCH.md](../papers/TO_FETCH.md); the index says the checklist was not made.

The pack is tested for the fuel supply model (the Vensim import `ch24-fuel`) and two library models (Bass diffusion, predator-prey) against stored snapshots (`cli/tests/snapshots/`). Measured: the three classic and imported models are all "not delivered" by the sanity sheet, because none carries the five human checks. The fuel supply model states none of the model card's fields.

## `dyn hindcast FILE --data CSV --cut DATE [--fit]`: the blind hindcast

```
dyn hindcast model.sdm --data actuals.csv --cut 2024-12-31 --fit --lever "Rate:0:1" [--var NAME] [--runs 200] [--spread 0.2] [--seed 1] [--season S] [--out PREFIX] [--json]
```

- **Data:** two columns, time (a number of model time, or an ISO date when the model has a start date) and value; a header names the variable (or `--var`).
- **Fit (`--fit`):** the levers (`--lever Name:min:max`, or the manifest's levers with ranges) are fitted through `optimize` on the data up to the cut only: the data is filtered at the cut before anything else happens. The squared gap between the model and the data (interpolated between points) is accumulated over the data window and minimised.
- **Score:** the model, run past the cut, against last value, straight-line trend, smoothed trend and (with `--season`) the same time last season, by RMSE and MAE (the same function as Compare with data, R21).
- **Coverage:** the share of the actual points after the cut inside the 50% (25th to 75th percentile) and 90% (5th to 95th) bands of a `sens` ensemble, next to the nominal shares.
- **Output:** Markdown and JSON, and `PREFIX-fan.svg`: the bands, the median, the actual points (dark before the cut, orange after) and the cut.

**What the bands are not.** The bands are the spread of the levers only (`dyn sens`), not noise in the data. A model that fits well can still sit outside them more often than nominal. Measured on the Bass model with synthetic data (the model with other parameters, plus 3% noise, cut at year 8 of 20, 8 points before and 12 after; the data and the levers are invented, so this shows the tool, not a forecast): the fit moved advertising effect from 0.02 to 0.0283 and word-of-mouth effect from 0.45 to 0.369 (the data were made with 0.03 and 0.35); the model's RMSE after the cut was 14 759 against 148 012 for the best guess (last value); the actual points were inside the 50% band 5 times in 12 (42%) and inside the 90% band 8 times in 12 (67%). So here the model beats every guess and the bands are too narrow.

Tests: changing every data point after the cut (and adding a wild one) leaves the fitted levers identical; changing a point before the cut moves them. Coverage on a model with known noise (Tank = Rate × time, the ensemble draws Rate within 20% of 1, and 300 independent "worlds" draw their own Rate from the same distribution; 3000 points after the cut): 48.0% in the 50% band and 88.3% in the 90% band. The test's tolerance is 8 points at 50% and 5 points at 90% (about 2.8 standard errors of a share from 300 independent draws).
