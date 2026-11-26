# Features

Legend: ✅ done and checked · 🟡 done in part (see note) · ⬜ planned / not started.
*Checked by* says how it was verified: **T** automated test, **S** screenshot looked at, **M** manual run.

To request a feature, add it to [INBOX.md](INBOX.md); the builder adds it here.

## Modelling

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | Stock-and-flow diagram: stocks, flows with valves and clouds, variables, constants, lookups, notes | R1 | S | |
| ✅ | Curved arrows (circular arcs) with bend handle, +/− polarity, delay mark, weight, dash, colour | R3 | S | |
| ✅ | Equations with names containing spaces; 36 built-in functions | R1 | T | |
| ✅ | Lookup tables edited by dragging points | R1 | S | |
| ✅ | Simulation: Euler and RK4, delays (DELAY1/3/FIXED), smoothing, TREND, INITIAL | R1 | T | |
| ✅ | Live re-simulation on every edit | R1 | M | |
| ✅ | Problems list: errors, unused arrows, missing arrows, stocks without flows | R1 | S | |
| ✅ | Causes & uses trees | R1 | S | |
| ✅ | Multi-select, box select, copy/cut/paste/duplicate across sheets | R5 | T | |
| ✅ | Align and distribute, snap to grid, nudge | R5 | S | |
| ✅ | Groups | R5 | T | |
| ✅ | Modules with inputs/outputs, own sheet, dissolve | R5 | T | |
| ✅ | Module files (`.flowmodule`) export/import with input mapping | R7 | T | |
| ✅ | Autocomplete and argument hints in the equation editor | R8 | T | |
| ✅ | Function reference window and wiki page | R8 | T | Generated page: `--write-docs` |
| ✅ | **Sets** (arrays in plain words): one variable per set of similar things, per-item values and curves, `TOTAL`/`AVERAGE`/`[item]`, sets that belong to sets | R16 | T S | One parent set per set; not in `.mdl` import |
| ✅ | Stack-of-cards drawing and *Tank ×57* badge for set variables; *One for each* selector and *Differs by…* list | R16 | S | |
| ✅ | Exact units: dimensions plus scale, brackets, ambiguity choices, value scale for % and ppm; `dyn check` with ranked fixes | P1 | T | See Units |
| ✅ | Model doctor stages C1 to C5 with 30 rules, gate Draft, Review or Publish (`dyn check --experiments --gate publish`) | P2 | T | Rules and what is not built: Model doctor |
| ✅ | Model health panel: findings by family with severity, rule and source, fix buttons with a preview of each edit, gate badge in the status bar, C1 to C3 on every edit, C4 and C5 on demand in the background | P2 | T, S | Model health |
| ✅ | Finance functions (Excel's PMT family, LEVELISED, LOAN, TAX, PAYBACK), `dyn finance` | P3 | T | Golden values computed independently in Python |
| ✅ | Random distributions with a model seed and one stream per call; normal and log-normal cdf and inverse | P3 | T | |
| ✅ | Calendar and seasonality functions (DAY OF WEEK, PROFILE, SEASON, DAYLIGHT…) | P3 | T | |
| ✅ | Model doctor (`dyn check`): units, magic numbers, division by zero, time step, DT sensitivity, lookups, ratios, equilibrium, benchmarks | P1 | T | Rules of phase 2 are added in P2 |
| ✅ | Calendar time: start date, Calendar convention, days and hours in a step, YEAR FRACTION, Excel serials | P1 | T | Dated data: resampling is a library function, not yet wired to GET DATA |
| ✅ | Never-negative stocks conserve material (outflows constrained, constraint log) | P0 | T | |
| ✅ | Loop polarity from partial derivatives along the base run; loops that switch | P0 | T | |
| ✅ | More maths functions (hyperbolic, inverse trig, ARCTAN2, CLAMP, MEAN, MEDIAN, PI, ERF, GAMMA, FACTORIAL…) | R17 | T | Checked against Python |
| ✅ | Your own functions in the equation language (Edit → My functions…, Functions sheet) | R17 | T S | No autocomplete for them yet |
| ✅ | Function from Python → lookup curve (button-only, never on load) | R17 | T S | Needs python3 |
| ✅ | Powersim-style functions: SINWAVE, COSWAVE, GRAPH, GRAPHSTEP, DELAYINF, DELAYMTR (order 1–20) | R17 | T | DELAYMTR(x,d,3) = DELAY3, DELAYINF(x,d,3) = SMOOTH3 (tested). Powersim's PULSE/STEP differ from ours and were not copied |
| ✅ | Integration: RK2 (Heun) and RK3 next to Euler and RK4 | R17 | T | Errors shrink with the order (test) |
| ✅ | Outline tree by module with plot checkboxes (View → Outline) | R17 | S | |
| ✅ | Units warnings: different kinds of units added or compared; flows that don't match their stock | R17 | T | Rates may meet amounts (MB and MB/Day); the fuel model has none |
| ✅ | Sensitivity runs (Monte Carlo) on the key levers, with a band chart | R17 | T S | Fixed seed, repeatable; 40 fuel-model runs take about 0.2 s |
| ✅ | Rejected batches logged with reasons; `nonneg`, `reference`, `balance` tests; manifest; strict units; `dyn optimize`; `explain_problems`; richer loss reports | R24 | T S | The manifest has no screen and nothing generates an app from it |
| ✅ | File menu: import and export Vensim and XMILE with a conversion report | R23 | T S | Nothing opened in Vensim or Stella |
| ✅ | Reusable modules window (definitions, ports, instances, plot outputs) | R23 | T S | Instances have no box on the diagram |
| ✅ | History, Compare with a saved model, Run operations windows | R23 | S | History is derived from undo steps, not from the operation log |
| ✅ | Vensim and Stella equivalents in the function reference | R23 | T S | |
| ✅ | Core crate with no desktop code; the app, `dyn` and MCP use it | R22 | T | The desktop app still edits its model directly in many screens; only the engine side moved |
| ✅ | Operations API: atomic validated batches, audit log, undo/redo, generation gate | R22 | T | |
| ✅ | Units as algebra (replaces label matching) | R22 | T | Scale (MB vs barrel) is not tracked |
| ✅ | Module definitions and instances with typed ports | R22 | T | Engine and API only: no screen for them yet |
| ✅ | `dyn` command line, MCP server, Python wrapper (subprocess) | R22 | T S | The Python wrapper is not a compiled binding |
| ✅ | Vensim `.mdl` read and write; XMILE read (write best-effort), with loss reports | R22b, R22d | T | Not opened in Vensim or Stella |
| ✅ | Semantic model diff and run diff; meaning/layout split files | R22 | T | |
| ✅ | Compare with data: held-out scoring of the model against last-value, trend and seasonal guesses | R21 | T S | Not run on the fuel model: its workbook holds no actuals to score against |
| ✅ | Model checks: limits on variables, plus an automatic stock-conservation check, shown in Problems | R20 | T S | The Problems tab with a broken check was not looked at on screen; the window was |
| ✅ | Layout remembered between sessions (panels, dark mode) | R17 | | `~/.dynamist-prefs.json` |
| ✅ | Chart legends name the selected tank/region; user functions in autocomplete; drag a lever circle | R17 | S (legend) | Autocomplete and dragging checked by test/code only, not by hand |

## Proposed on 23 November 2026 (round 8 onward)

Requested by Amin on 23 November 2026. Numbers **B73 to B91** are reserved for [BACKLOG.md](BACKLOG.md), round 8. Statuses were checked against `master` at `d44cb42` (23 November 2026), which added `dyn serve`, `dyn ablate`, `dyn export --xlsx` and `dyn check --gate deliver`. Sources for the notes: `review/Charts_and_Model_Replication_23Nov.md` in the book folder.

**Order of work.**
- **Priority 1 (round 8):**
  - B73 and B74, the sd-ai benchmark;
  - B75, why results changed;
  - B76, the audit pack;
  - B77, the blind hindcast.

  The programmer's prompt is [inbox/round-8-prompt.md](inbox/round-8-prompt.md).
- **Priority 2 (round 9):** B78 to B84.
- **Priority 3 (later):** B85 to B91.

The model doctor (C1 to C5) and the automatic extreme-condition tests (X1 to X6) are listed under Modelling.

### Benchmark against sd-ai

Join the community's own benchmark instead of asking people to install another tool. sd-ai (github.com/UB-IAD/sd-ai, MIT licence) runs the BEAMS evaluations: four leaderboards, with development, held-out and hard groups, graders frozen before held-out runs, and null-agent floors.

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| 🔶 | **B73 · P1 · Dynamist tools inside sd-ai's agent**: behind one experiment flag (`dynamistTools: true`), sd-ai's own agent (Merlin or Socrates) gains tools that call `dyn` on the current model (doctor, extreme tests, verify); with the flag off it is the published baseline | R47 (round 8) | T M | Built in a fork branch of sd-ai (local clone, not published): `dynamist_check` and `dynamist_extreme_tests` behind the eval-only flag `dynamistTools` (default off; with it off the tool list and the response are identical to upstream, tested); binary through `DYN_BIN`; sd-ai's own SD-JSON to XMILE converter reused; the import loss report is listed for all 8 dev models (nothing computed is dropped). In the dev runs the agents called a tool in 23 of 36 extremeConditions runs and 30 of 42 formulationCritique runs, with no tool errors; on the hard extremeConditions group they ran fewer what-if runs with the tools (0.58 per run against 1.25) Template: the eval-only `analysisTools` flag |
| 🔶 | **B74 · P1 · With-and-without protocol**: the same agent and LLM, flag off and flag on, run at the same sd-ai commit; three passes each; Wilson intervals; every category reported, losses first. First targets: extreme conditions and formulation critique, developed on dev groups only, Dynamist frozen before one held-out run | R47 (round 8) | T M | Dev runs made (Merlin and Socrates on Sonnet 5.5, 3 passes per arm, $30.08 in all): Sd-ai's judge ran out of quota, so formulationCritique was re-graded with DeepSeek as the judge (not comparable with published numbers). extremeConditionsHard 72.2% without, 66.7% with (-5.6 points, interval -25.8 to +15.3); formulationCritiqueHard 77.1% and 75.0%; plain formulationCritique 58.3% in both arms; no group shows a gain, every interval includes zero. Opus, the held-out run and the leaderboard are not done. Report: `benchmarks/sdai/REPORT.md`; script `scripts/sdai-compare.py` |

### Expert functions in one step

Do in one step, with a plain-language result, what other tools leave to experts, add-ons or hand-written code.

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | **B75 · P1 · Why results changed**: split the change in every goal between the edits that caused it ("the peak fell 12%; 9 points came from the new lookup"), parts adding up exactly to the whole | R47 (round 8) | T | Built: `dyn attribute OLD NEW [--json] [--out PREFIX] [--permutations N] [--seed S]`. One group per changed variable; exact Shapley split for up to 10 groups, sampled permutations with a fixed seed and standard errors above; parts add up to the change; Markdown, JSON, SVG waterfall. An added or removed variable and the equations that read it form one group. Comments, units text, checks and manifest entries cannot move a run and are listed, not split. Manifest lever defaults and ranges do not enter the split because the base run does not read them |
| 🔶 | **B76 · P1 · Audit pack**: one folder with the doctor report and gate, the sanity sheet, the checks, the conversion loss report, the edit log and the workbook, plus a model card (intended use, out of scope, evaluation, limits; Mitchell et al. 2019) and a checklist against the reporting guidelines of Rahmandad and Sterman (2012) | R47 (round 8) | T | Built: `dyn pack FILE --out DIR` (doctor report and gate, sanity sheet, checks, conversion loss when imported, operation log when there is one, workbook, `model-card.md` with "not stated" for every missing field, `index.md`; exit 1 if a part fails). Missing: `reporting-checklist.md`; the paper's items could not be read (the host answered 429 on 23 November 2026), see `papers/TO_FETCH.md` |
| 🟡 | **B78 · P2 · Modes in plain words**: "it oscillates about every 9 years, and loop B2 drives it" | | | Exists: `importance --eigen-at T` (eigenvalues, periods, time constants, elasticities as numbers). Missing: the sentence and the link to named loops |
| 🟡 | **B79 · P2 · Equilibrium start**: solve for the initial values that set every net flow to zero, offered as a fix | | | Exists: rule I2 finds a model that claims equilibrium but moves. Missing: the solver |
| 🟡 | **B80 · P2 · Behaviour tests against reference modes**: does the run show the pattern the brief asks for (growth, overshoot, damped oscillation…), and how close is it to the reference data | | | Exists: objectives store the pattern kind; `verify` classifies modes and computes Theil's decomposition. Prior art: BTS and BTS II (Barlas), BATS (Sücüllü and Yücel 2014) |
| ⬜ | **B87 · P3 · Scenario discovery and robust decisions**: under which combinations of assumptions a policy fails, and which policy holds up across them | | | Reads the manifest's levers, ranges and goals. Prior art: EMA workbench (Kwakkel 2017) |
| 🟡 | **Model as a service**: the manifest's levers and outputs called by a dashboard or an agent | | | Exists: `dyn serve` (manifest, run, batch, OpenAPI, token, call log) and `dyn mcp`. Missing: WebAssembly (B25), a manifest screen (B27) |

### Borrowed from other fields

Methods that fields with high-stakes models already rely on. Each note names the field and the source.

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | **B77 · P1 · Blind hindcast**: cut the data at a past date, fit only on what came before, run forward, score against what followed and against simple guesses, and report the share of outcomes inside each uncertainty band | R47 (round 8) | T | Built: `dyn hindcast FILE --data CSV --cut DATE [--var] [--fit [--lever Name:min:max]…]`. Fit through `optimize` on the points up to the cut only (test: changing every point after the cut leaves the levers unchanged); R21 scores; coverage of the 50% and 90% bands of `sens`, next to the nominal shares; fan chart SVG. The bands hold lever uncertainty only, not noise in the data |
| 🟡 | **B81 · P2 · Testing the tests**: plant faults (a flipped sign, a doubled delay, a dropped flow) and report which ones no check of the model notices | | | Software engineering: mutation testing, used on Simulink models. Exists: `dyn ablate` already takes structure out one piece at a time |
| 🟡 | **B82 · P2 · Credibility matched to the stakes**: the modeller states how much the decision rests on the model and what a wrong answer would cost; the gate and the required tests follow | | | Medical devices: ASME V&V 40 (2018), relied on by the FDA's 2023 guidance. Exists: gates Draft, Review, Publish and Deliver, not tied to stakes |
| ⬜ | **B83 · P2 · A pedigree for every number**: score each input on proxy, empirical basis, method and validation; one chart against its Sobol index shows the assumptions that are both weak and decisive | | | Environmental assessment: NUSAP (Funtowicz and Ravetz 1990; van der Sluijs et al. 2005), applied to the TIMER energy model |
| ⬜ | **B84 · P2 · Calibrated confidence language**: each finding carries evidence and agreement terms and a confidence level, filled from the doctor, hindcast and pedigree results | | | Climate assessment: IPCC guidance note (Mastrandrea et al. 2010). Needs B77 and B83 |
| ⬜ | **B85 · P3 · Value of information**: which uncertainty is worth resolving before deciding ("a survey of X could change the decision; more data on Y could not") | | | Health economics: EVPI and EVPPI (ISPOR task force, 2020) |
| 🟡 | **B86 · P3 · Rival structures**: keep two or three versions of a contested sector, run them all, report which conclusions hold under every version | | | Climate: multi-model ensembles (Tebaldi and Knutti 2007); epidemiology: ensembles were the most consistently accurate COVID-19 forecasts (Cramer et al. 2022). Exists: named ablations in the manifest remove structure; rival versions are not stored |

### Rebuilding large energy and economic models

Fit a transparent model to a large model's published scenarios, check it on scenarios it has not seen, then add the feedbacks the large model leaves out (book folder: `review/Charts_and_Model_Replication_23Nov.md`, section 3).

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ⬜ | **B88 · P3 · Scenario-database import**: read scenario tables in the IAMC template (model, scenario, region, variable, unit, years) as data series | | | Format of the AR6 Scenarios Database (3,131 scenarios), NGFS and many national models |
| 🟡 | **B89 · P3 · Emulate across scenarios**: fit the levers to many scenarios at once, hold some out, report the error per variable in the client's units | | | Climate practice: RCMIP (Nicholls et al. 2020); En-ROADS comparison pages (self-reported). Exists: `optimize` fits one run to data. Needs B88 |
| 🟡 | **B90 · P3 · Retrospective scorecard**: a large model's past projections, the outcomes and this model's blind hindcast from the same start year, scored together and against a naive guess | | | EIA AEO retrospective; backtesting in Way et al. (2022). Needs B77 and B88 |
| 🟡 | **B91 · P3 · Web calculator**: a browser page on top of `dyn serve`: levers as sliders, outputs as dotplots and fans, a pathways map | | | UK 2050 Calculator; En-ROADS. Exists: `dyn serve` (the API). Missing: the page |

## Sheets

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | Architecture sheet: purpose, numbered layers, subsystems, contracts, variable roles | R46 | S | [Layers](Layers.md) |
| ✅ | Multiple diagram sheets (views) | R3 | S | |
| ✅ | Equations table (edit in cells) and results table (copy for Excel) | R3 | S | |
| ✅ | Dashboards: number, graph, slider and text cards | R3 | S | |
| ✅ | Causal loop sheets with automatic loop finding (R/B) | R4 | S | |
| ✅ | Optimization sheet (background search, limits) | R4 | T | |
| ✅ | Scenarios sheet: table, comparison charts, key numbers | R10 | T | |

## Results and charts

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | Runs play in over time (Run / Replay) | R4 | S | |
| ✅ | Kept runs for comparison | R1 | S | |
| ✅ | Chart types: line, area, stacked, step, bars, phase, donut | R5 | S | |
| ✅ | Scenarios shown in every chart (toggle) | R10 | S | |
| ✅ | Customize chart: type, variables, titles, axes, ranges, legend, grid, width, markers, colours | R12 | S | Trends and dashboard graphs; not the side-panel graph or scenario/optimize charts |
| ✅ | Legends name variables, scenarios and kept runs; caption explains line styles | R12 | S | |

## Sharing

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | Model library catalogue with generic and special tags; `dyn library` search, filter, facets, check, ingest | R46 | T | [Model library](Model-Library.md) |
| ✅ | Transparent high-resolution PNG of any sheet, selection, chart or card | R6 | T S | Clipboard: macOS checked; Windows/Linux path untested |
| ✅ | Copy chart data (tab-separated, pastes into Excel) | R9 | T | |
| ✅ | Save/open `.sdm` (JSON) | R1 | M | |
| ✅ | Import classic `.mdl` | R1 | T | Basic: no arrays, macros, data |
| ✅ | Spreadsheet template: import / export / blank template | R14 | T S | Round trip gives identical results |
| ✅ | Module boxes with inputs left / outputs right, purpose line and role colour; optional Compact look with labelled arrows | R17 | S | Entities-as-cards sheet withdrawn (kept readable in old files) |
| ✅ | Fold a module card to name + purpose (chevron, View → Fold all) | R17 | S | |
| ✅ | Key levers: flagged constants get the controls (card on the diagram + side panel), others fold away; per-set levers have an all-items × slider or a per-item slider | R17 | T S | Levers are constants only |
| ✅ | Modules of modules to any depth: path labels on import, breadcrumb, ports traced through levels | R17 | T S | Making deep levels by hand uses ⇧⌘M on modules |
| ✅ | Levels: modules built from a spreadsheet's Module column, overview sheet, *Modules ▾* menu, up-one-level button | R17 | T S | Module boxes list ports in a narrow space; long names can overlap |
| ✅ | Showing bar: choose a region or one tank from dropdowns; the whole model shows that selection; sliders edit only it | R17 | T S | Kept runs are not re-focused |
| ✅ | Arrows through several draggable turning points (smooth path, no Bézier handles) | R17 | T S | Not yet: automatic routing around boxes |
| ✅ | Diagram look: Classic (plain labels, thin dark arrows) or Cards | R17 | S | |
| ✅ | Arrow styles: curved / straight / right-angle, filled / open / dot tips; per diagram and per arrow | R17 | S | Not copied from any other tool |
| ✅ | Tidy up: layered automatic layout of a diagram sheet | R17 | T S | |
| ✅ | Quiet arrows in crowded diagrams; one-item-at-a-time chart picker for set variables | R17 | S | |
| ✅ | Template sheets Sets, Values, ItemSeries and the Per/Combine columns; import a folder of CSVs; `--convert` | R16 | T | |
| ✅ | Fuel-supply workbook restructured and checked against the original's numbers | R16 | T S | [Example](Fuel-Supply-Example.md) |
| ✅ | Wiki with user guide, template spec, function reference, records | R14 | — | This folder |
| ✅ | Inbox window: read and write messages between you, the builder and other conversations (INBOX.md / OUTBOX.md) | R19 | T S | [Other projects](Other-Projects.md) |

## App

| | Feature | Since | Checked by | Notes |
|---|---|---|---|---|
| ✅ | Light and dark themes | R1 | S | |
| ✅ | Icons throughout (hand-drawn vector set) | R7 | S | |
| ✅ | Command palette, shortcut sheet, About | R5 | S | |
| ✅ | Undo/redo for every change | R1 | M | |
| ✅ | `--screenshot` option for automated checks | R1 | S | |
| ⬜ | Packaged installer (.app / .exe) | — | | Runs from `target/debug/dynamist` today |
