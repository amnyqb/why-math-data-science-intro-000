# Changelog

Newest first. "R" numbers are working rounds; each has a matching entry in [DEVLOG.md](DEVLOG.md).

## R47 · 2026-11-23 · Round 8 (branch `round-8`): why results changed, the audit pack, the blind hindcast, Dynamist tools for sd-ai's agent
- **`dyn attribute OLD NEW`** splits the change in every goal between the edits by the Shapley value, one group per changed variable: exact up to 10 groups (2^k runs, cached), sampled permutations with a fixed seed and standard errors above; the parts add up to the change within 1e-9 relative; a Markdown table, JSON and an SVG waterfall per goal (`--out PREFIX`). Details and a measured example in [Attribute, pack, hindcast](Attribute-Pack-Hindcast.md).
- **`dyn pack FILE --out DIR`** writes the audit pack: doctor report and gate, sanity sheet, checks, conversion loss (imports), operation log (when there is one), workbook, `model-card.md` ("not stated" for every field the model does not state) and `index.md`; exit 1 when a part fails. **Missing: `reporting-checklist.md`** (the paper's items could not be fetched; `papers/TO_FETCH.md`).
- **`dyn hindcast FILE --data CSV --cut DATE [--fit]`** fits levers through `optimize` on the data before the cut only, runs forward, scores against last value, straight-line trend, smoothed trend and seasonal guesses (the R21 function), and reports the share of actual points inside the 50% and 90% bands of a `sens` ensemble next to the nominal shares; a fan chart.
- **sd-ai (B73):** a fork branch holds the `dynamistTools` flag with the tools `dynamist_check` and `dynamist_extreme_tests` (see `benchmarks/sdai/REPORT.md`). Nothing from sd-ai is copied into this repository. `scripts/sdai-compare.py` compares two arms from their result files.
- **No new Rust dependency.** `doctor::num` is public. `core/src/plotsvg.rs` draws the waterfall and the fan chart.

## Doctor fixes · 2026-11-22 · Fixes after the book-side review (branch `doctor-fixes`)
- **Loop polarity:** a loop with a link of zero gain at every time of the run is labelled `inactive` (`reinforcing` null in `dyn loops`) instead of taking the drawn sign; F12 and X5 ignore inactive loops (the venture models with Anchor Weight = 1). A link into a function with a hidden state (SMOOTH, DELAY, TREND…) is a delay, not zero gain.
- **One units engine:** `validate` (default and `--strict-units`), `dyn check` and the gate of `dyn ops` all read units through `doctor::units_rules`; the default of `validate` stays lenient about the time unit. Notes that only inform stay with the doctor.
- **Fix names and safety:** conversion constants are named from the declared unit texts ("Days per Week" = 7 in "Day/Week"), never from base units; displayed units are simplified ("(kWh/Year)*Dmnl" is "kWh/Year"). A conversion between a week, month, quarter or year and a smaller unit is a calendar choice: `Fix.needs_decision` is true and `--fix-all-safe` skips it.
- **Exponents:** an exponent with units is a U1 error ("an exponent must be a number without units"), for `^`, `POWER` and `EXP`; the fix divides it by one unit in the Conversions group. The solar model uses `(1 - Degradation)^(Time / Year Length)`, carries the three cases as scenarios (Fraunhofer ISE 2024, programme defaults, NREL) and calls its conversion "kWh per MWh".
- **`dyn finance`:** the rate is per model time unit whatever the step (a flow at step k is discounted by (1 + r)^((k + offset) x step)); IRR is found by a scan from -99% to 10 000 000% and is per time unit (`irr_continuous` is ln(1 + r)); the conventions say that period_end discounts the time-0 flow by one period and period_start matches Excel's NPV(r, flows after the first) + the first flow.
- **Doctor rules:** TREND is 1/time (U rules); G2 is Info for a start value that reads a flow (an equilibrium start), an error only for a stock reading itself; N2, N3 and N5 are reported only when reruns at DT/2 and DT/4 move a stock by more than 1% of its range, and N2 and N5 are confirmed only then; X1 flags a stock below zero only if it is declared never negative, a fraction only if it has no units, and tries a time constant at one step; X6 holds the extreme cases to invariants only; G7 is Info when no outputs or KPIs are declared and never offers to delete the end of a chain; F3 evaluates constant expressions exactly and knows that a positive base to any power is positive; unconfirmed findings read "possible" in the text, in `dyn check` and in the Model health panel; F9 lets period indices and tests of Time against a constant through; G4 is Info for a stock over a flow (Little's law); F4 ignores pi and makes the calendar numbers a note; X5 skips pure accumulators; the doctor prints "not a number" where `fmt_num` writes a dash.
- **Book sweep:** 215 models (the book side has 219; ch26 to ch29 are not all here). `allow.toml` went from 57 to 21 entries (the ch25 overview is now a figure model).
- **Fourth round (branch `doctor-fixes-3`, not merged: these change rules, wording and the sanity sheet that the b74-freeze tag pins).** X6 names a run that diverges and reports the worst broken check, not the first; a check can carry a `role` ("validity" or "promise", op `add_check` field `role`), and X6 reports its breach under an extreme input as information; X1 and X6 try a constant with a declared range (a manifest lever or a slider) at the two ends of that range only; unit findings between days and weeks say "assuming a 7-day week"; `dyn convert` lists a dropped `figure_model` (and `teaching_draft`) setting as a loss; a setting `teaching_draft` makes the extreme-condition findings (X rules) information and puts an open row on the sanity sheet; the sanity sheet has 14 rows: S13 (extreme runs and the expected direction of each scenario, manifest scenario field `expect`), S14 (the best scenario of every goal across stratified draws over the lever ranges, 16 draws, 64 for stakes "loan"), S3 also keeps the ranking of the scenarios at DT/2, and the manifest field `stakes` ("workshop" requires S1, S2, S3 and S8 only; "board", "loan" or empty requires all).
- **Third round (branch `doctor-fixes`).** `dyn diff` reports changes to the manifest (levers and their ranges, scenarios, goals, validity, tests, outputs, bindings), listed first; before, a changed scenario gave "same": true. `dyn check --gate deliver` prints the sanity sheet: seven rows made by the program (strict units; the model's checks and conservation; a rerun at DT/2 with the largest relative change per goal; every scenario differs from the first in one lever only; every goal read inside the validity horizon; every scenario inside the validity range up to the horizon; two clean runs identical) and five recorded by a person in the manifest (`sanity`: expected value written before the run, actuals reconciled, cross-check, benchmark, reviewer and date), each pass, fail or open, as Markdown or with `--json`; the exit code is 1 unless every row passes. `dyn ablate FILE` takes out one thing at a time (the manifest's `ablations`, or each loop cut at one link and each stock frozen at its initial value) and prints the change in every goal, flagging a removal that improves a goal as "check the goal". `dyn export FILE --xlsx OUT` writes Cover, Inputs, Scenarios, Series and Checks as an Excel workbook (an own zip and OOXML writer in the `dynamist-cli` crate; no new dependency anywhere; live formulas are not built). `dyn serve FILE` makes the model a web service on plain `std::net`: `GET /manifest`, `POST /run` (a scenario or lever values), `POST /batch`, `GET /openapi.json`; a name that is not a lever is refused (400) and so is a value outside its tested range (422); outputs are read at the validity horizon and later values come under `beyond_horizon`; a bearer token is needed (made up and printed when none is given); every call is logged with the version of the model; it listens on 127.0.0.1 unless `--bind` says otherwise. None of these five was built from a reference script (none is in this repository): they follow the task descriptions. X6 leaves a value that is not a number to X1 (one finding, not two).
- **Second round (branch `doctor-fixes`):** `dyn reduce` keeps a fixed constant as a named constant with its units and value (it used to write the number into the equations and lose the unit); the seed of `RANDOM UNIFORM`, `RANDOM NORMAL` and `RANDOM EXPONENTIAL` is optional like in every other random function (left out, the call's own stream from the model seed, the variable and the call; a model that gives its seed keeps its numbers); `dyn info` reports `order`, the number of state variables including those inside SMOOTH, DELAY and the like, next to `stocks`; a setting `figure_model` marks a model drawn for a picture (constants folded into the equations on purpose) and the units rules report it as information (`dyn ops` key `figure_model`); `dyn render` draws a pipe where the model has it again, as `fc render` did (valve halfway between the ends, clouds and stocks not moved or shortened; the window still straightens). The saved figure `book/examples/ch13/diagram.svg` was rebuilt for it. X1 treats constants named length or horizon like time constants (tried at one step, not zero), and X6 says "not a number" for a NaN inside a check message.

## R46 — 2026-11-22 · Dynamist: new name, layered architecture sheet, a searchable model library
- **Renamed to Dynamist.** Command line `dyn` (was `fc`), native file `.sdm` (was `.flowmodel`), crates `dynamist`, `dynamist-core`, `dynamist-cli`, Python package `dynamist`. Old `.flowmodel` files open unchanged; the old XMILE and Vensim settings tags and the old preferences file are still read. A fixed-length tag read in the Vensim reader was corrected for the shorter tag (it had cut two characters from the first setting). See [Working on Dynamist](Working-on-Dynamist.md).
- **Architecture sheet** (File → New sheet): the purpose (problem, objectives, KPIs), the layers as numbered lanes ("Layer 1: World") that start at a normal height and grow only when needed, subsystems as frames, contracts between layers, and the role of each variable (exogenous, lever, constant, lookup, switch, loop member, KPI). New operations `set_purpose`, `add_objective`, `remove_objective`, `add_group`, `set_slider`. See [Layers](Layers.md).
- **Model library catalogue:** `library/catalog.json` with generic and special tags, simplicity and difficulty, authors, year, paper, features; `dyn library` searches and filters it, `--facets`, `--stats`, `--check`. See [Model library](Model-Library.md).
- **Six classics** built as our own implementations with independent reference checks: Bass diffusion, SIR, predator-prey, fishery commons, industrial dynamics, market growth.
- **`dyn library --ingest`:** import every Vensim, Stella or XMILE file in a folder, run it, save as `.sdm`, print catalogue entries; the source and licence are required.
- **Repository on GitHub** (private, `amnyqb/dynamist`); tests run on the server through `scripts/server-test.sh`.
- **Not yet:** a library browser in the app (B71, B72); World2, Urban Dynamics and World3 as library entries; a converter for free-form Excel sheets; the enhancement programme (exact units, calendar time, model doctor, function library) is on its own branch.

## P2 app · 2026-11-22 · Model health panel
- **Model health** (Check group of the ribbon, or click the gate badge in the status bar): the model doctor's findings grouped by family (definition, graph, units, formulation, numerical, initialisation, experiments, calibration, reproducibility, purpose), each with its severity, the variable (click to select it), the rule and its source, and one button per fix with a plain-words preview of the edit. A fix is one undoable step. Stages C1 to C3 run again 300 ms after the last edit; "Run C4 checks" and "Run experiments (C5)" run in the background. See [Model health](Model-Health.md).

## P2 and P3 · 2026-11-22 · The model doctor in stages, and the function library, release 1 (enhancement programme)
- **Model doctor stages.** `dyn check` runs C1 to C4 (names, graphs, units with intervals, base run and numerical health), `--experiments` adds C5 (extreme values, sources at zero, arrow signs), `--gate publish` runs everything and counts warnings as errors, `--stage C3` stops early. The report ends with the model's summary (variables by kind, loops, units and description coverage) and the gate: Draft, Review or Publish. The rule table is in [Model doctor](Model-Doctor.md), generated from the code.
- **Rules built:** S1, G1, G11, F1, F2, F3, F4, F8, F12, F14, N1 to N5, N7, N8, I1, I2, X1, X2, X3, X6, R1, R3 and the unit rules U1 to U9. Not built: see the page.
- **Finance:** REAL RATE, NOMINAL RATE, EFFECTIVE RATE, WACC, ANNUITY FACTOR, PMT, FV, PV, NPER, IPMT, PPMT, RATE, DSCR (Excel's conventions); LEVELISED, LOAN, TAX, PAYBACK, ESCALATE; VALUE with `timing=period_start` and `mid_period`. A rate declared per Year in a monthly model is read per Year.
- **Statistics:** RANDOM TRIANGULAR, LOGNORMAL, GAMMA, BETA, WEIBULL, POISSON, BINOMIAL, NEGATIVE BINOMIAL, GEOMETRIC, PARETO, PERT, RANDOM 0 1 and the short and Stella forms; model seed (`dyn run --seed`, `dyn compare --same-draws`), one stream per call so adding a variable never changes another's draws; NORMAL CDF and INV, LOGNORMAL CDF and INV; PREVIOUS, HISTORY, ROUND WITH CARRY.
- **Calendar:** DATE, YEAR, MONTH, DAY OF MONTH, DAY OF YEAR, HOUR OF DAY, WEEK NUMBER, DAY OF WEEK, IS WEEKEND, IS WORKDAY, WORKDAYS BETWEEN, PROFILE, PROFILE STEP, SEASON, DAYLIGHT, AT TIME, EVERY.
- **Logic and aliases:** AND, OR, NOT, BETWEEN; the alias table (SUM, VMAX, VMIN, ELMCOUNT, SIZE, SAFEDIV, MOD, INT, SMTH1, SMTH3, SMTHN, EFFECT, CRF, LCOE, IRR); MAX OF, MIN OF, COUNT OF over sets.
- **Benchmarks:** a check with a `source` (and optionally `at`) is a benchmark; a failure names its source. `dyn finance` prints NPV, IRR, MIRR, paybacks and the levelised cost with the convention of each line. The solar LCOE model is in `models/solar/`.

## P1 · 2026-11-22 · Exact units, calendar time and the model doctor (enhancement programme, phases 0 and 1)
- **Material is conserved by never-negative stocks.** A stock marked never negative no longer has its level clamped (which made material). Its outflows are scaled down together so that no more leaves than the stock holds plus what flows in; the run records each spell of constraint in `constraints`. Models whose never-negative stocks did hit zero change their numbers; see the report.
- **Loop polarity from the numbers.** `dyn loops` takes each link's sign from the partial derivative along the base run, not from the syntax, and labels a loop reinforcing, balancing or switching (with the times). The market-limit loop of the chapter 2 growth model is balancing.
- **Units.** New exact units engine (dimensions plus scale, brackets, money with a price year), `unit-ambiguous` choices, value scale for % and ppm (`units_scale_values`). See [Units](Units.md).
- **`dyn check`.** The model doctor: unit findings with ranked exact fixes, magic numbers, division by zero, time step against time constants, `Time =` tests, DT sensitivity, lookups and ratios outside their range, equilibrium, benchmarks, inferred units. `--fix 1,3`, `--fix-all-safe`, `--publish`, `--json`, `--lenient-time`.
- **Calendar time.** Start date, Abstract or Calendar convention, `DAYS IN STEP()`, `HOURS IN STEP()`, `YEAR FRACTION`, Excel serials, dated data resampling.
- **New check kind** `conservation` and benchmark fields `at` and `source` on checks. New operations `add_to_group`; `set_setting` takes the new settings.

## R45 — 2026-11-20 · Controls on the diagram: slider, knob, switch, number input
- **Insert → Controls:** a slider, a knob, a switch or a number input. Click on the diagram to put one down; it is tied to the constant you had selected, or else to the first constant that has no control yet. Operate it right on the diagram and the model runs again, as with the key levers: drag along a slider's track (or click on it), drag a knob up or down, flip a switch (the top or the bottom of its range), drag a number sideways or press − and +. Values settle on a tidy step (a hundredth of the range, as a power of ten) when the drag ends.
- **Control tab** (while a control is selected): which constant it sets, its label, the range (the constant's slider range, shared with the lever card and the side panel), the value, and a switch between the four looks. Drag the card's edge to move it; copy, paste, duplicate, delete and undo work as for other items.
- **Same picture everywhere:** controls are drawn, so a figure shows each control at the model's current value in the window, in PNG and in `dyn render` SVG (labels and values are real SVG text).
- **Saved:** `controls` in the model (absent when none); in the split format they go in the layout file. Vensim and XMILE exports list them as dropped. A control whose constant was deleted stays, shows "Pick a variable" and can be re-tied on the Control tab.
- **Not yet:** controls for a constant that is per a set, or for a variable that is not a plain number; a control that runs the model only when released; a different size; a list or radio control; `dyn set` options for controls. Pause, Step and Reset (B68) are still open.

## R44 — 2026-11-20 · Shapes and pictures on diagrams
- **Insert → Shapes:** rectangle, rounded box, ellipse, line and arrow. Drag on the diagram to draw (a plain click makes a standard size). Select one, then drag the square handles to resize; lines have a handle at each end. Move, copy, paste, duplicate, delete and Select all work on them like on notes.
- **Insert → Picture:** a PNG or JPEG file, kept inside the model file (base64, up to 12 MB), so the model travels alone. Pictures keep their proportions when resized.
- **Shape tab** (appears while a shape is selected): fill, line colour and width, arrow head on or off, width and height, Duplicate, Delete.
- **Behind everything:** shapes and pictures are drawn under the diagram's own parts and are found last by the pointer, so a picture or a big box never hides a variable. Unfilled shapes are picked on their outline only.
- **Saved:** `shapes` and `images` in the model; absent when there are none. In the split format they go in the layout file (they are not part of the model's meaning). Vensim and XMILE exports list them as dropped.
- **Same picture everywhere:** the window, Copy as picture, PNG and `dyn render` SVG all show them. In SVG a picture is an `<image>` with the re-encoded PNG as a data URI; the same file gives the same bytes.
- **Not yet:** text inside a shape, shape colour with transparency, rotate, bring forward and send back (shapes are drawn in the order they were made), a shape in a causal loop diagram, dragging a picture file onto the window, `dyn set` options for shapes.
- **Dependency:** the `image` crate now also reads JPEG (`zune-jpeg`, two lines in `Cargo.lock`).

## R43 — 2026-11-20 · Text formatting for notes
- **A Text tab** appears in the ribbon while a note is selected (one or several): size (with A+ and A−), bold, italic, underline, fixed-width letters, text colour, background colour, left, centre or right alignment, *Plain* (back to the usual note), Duplicate and Delete. One undo step per change.
- **Saved with the note:** a `fmt` object with `size`, `bold`, `italic`, `underline`, `mono`, `color`, `fill`, `align`. A plain note writes nothing, so old files and old readers are unaffected; unknown keys in `fmt` are an error.
- **Same picture everywhere:** the window, *Copy as picture*, PNG and `dyn render` SVG all show the format. SVG has `font-style="italic"`, `text-decoration="underline"` and the real size and colour; the exports now draw italics (a slant) and underlines, which they ignored before.
- **Bold is drawn twice**, a hair apart, because the bundled fonts have one weight: in SVG a bold note therefore has two `<text>` elements per line. A real bold face is a later change.
- **Not yet:** a font choice beyond proportional and fixed-width; formatting variable labels, module names or chart text; `dyn set` options for notes; copying a format from one note to another.

## R42 — 2026-11-20 · A ribbon in place of the menu bar
- **Tabs and groups of buttons, as in Office:** Home, Insert, Arrange, Analyze, View, with File and Help as menus on the title row. Every button calls what the app already did (the old Edit, View and Arrange menus are now functions the ribbon opens: *More* on Home, *Look & arrows* on View). The time settings moved from the title row to Home → Simulation. Double-click a tab to hide the ribbon.
- **New in reach:** align buttons, a *Sheets → New sheet* list, results-panel buttons (Trends, Table, Causes & uses, Problems, Runs), Sensitivity, Compare data, Optimise, Model checks, History, zoom in and out, and the causal loop tools on the Insert tab of a causal loop sheet.
- **Not yet:** Pause, Step and Reset (the engine runs a whole simulation at once); text formatting, shapes, pictures and diagram controls (the model has no such objects yet); a contextual Format tab; wrapping of the ribbon in a narrow window. The left tool strips stay.
- **Look at the window without a display:** `cargo run --example window_shot -- model.sdm Home out.png [w h]` (or `-` for the example model) draws the whole window to a PNG, one ribbon tab at a time; a test draws every tab.
- **Verified:** the window build on macOS (`cargo build -p dynamist`) after the library split. Not clicked through by hand: the dropdowns (More, New sheet, Look & arrows) were checked to compile, not opened.

## R41 — 2026-11-20 · Figure library: `dyn render`, `chart`, `layout`, `cld`, `set`, `style`
- **Pictures with no window.** `dyn render FILE --sheet NAME --out fig.svg` draws a stock and flow or causal loop sheet with the app's own drawing code (egui run without a window), so it is the picture the app shows; `--png --scale N` writes the app's PNG; `--with-caption` adds the sheet's caption and source. `dyn chart` draws variables over time (time window, scenarios, `--set`). Text is real SVG text in the style's font; the same file and style give the same bytes. See [Figure library](Figure-Library.md).
- **Editing a figure:** `dyn layout` (`tidy` = the app's Tidy up; `loops` = each loop a circle, words evenly spaced), `dyn cld --from-model --loops R1,B2` (a causal loop sheet that keeps the named loops; `--list` names them as the side panel does), `dyn set` (`--pos`, `--polarity`, `--delay`, `--loop-name`, `--caption`, `--source`), `dyn style --apply book.style.json` (font, sizes, colours, arrow style, line widths). The same calls are in the Python wrapper.
- **The crate `dynamist` is now a library and a binary.** Feature `gui` (default) holds eframe, file dialogs and the clipboard; without it the library builds on Linux with no display. Sheets have `caption` and `source`; the model has an optional `figure_style`. The app's own unit tests now run without a window (40).
- **Fixed in the app:** the picture a causal loop diagram exports (and a diagram with bent arrows) was cropped at curved arrows that leave the box of the words; the export area now includes the arrows and their signs.
- **Examples:** `book/examples/ch10` to `ch15` (saved layouts, diagram, chart, loops where there are any), rebuilt by `book/examples/build.sh` and checked by `cli/tests/figures.rs`.
- **Not verified:** the window build on macOS after the split (it compiles on Linux with the `gui` feature); the SVG against Inkscape or a browser (checked with `rsvg-convert` only).

## R40 — 2026-11-20 · `reduce` is a gated cut, verify, add-back loop
- **`reduce`** checks the Morris and Sobol gates for every goal before any cut and refuses (exit 1, no file) when the rankings are not stable; a goal may declare `transform: "log"`, applied only when the gates fail. Cuts are ranked (unreachable, pass-through, levers by ascending total index), made in batches, verified after each, bisected on failure, and culprits re-admitted. Output separates structure removed from parameters fixed (scenarios named); the headline counts structure only; loops are proposals.
- `Goal.transform`; Sobol reports `rank_tau_n_to_2n`; Morris and Sobol ranking agreement treats statistically tied effects as ties.

## R38 — 2026-11-20 · Verbs EXPECT and LOOK; IRR, write-off and conveyor leakage
- **`EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)`** is what people believe: the level they perceive, the trend they see, or where they expect it to be some time ahead. Equal to `SMOOTH`, `SMOOTHI`, `TREND` and the `FORECAST` formula.
- **`LOOK(curve, x, mode=, start=, outside=)`** reads a curve normally, as steps, the other way round (`inverse`), beyond its ends, for its slope or for the area under it.
- **`VALUE(stream, measure=irr)`** is the rate of return that makes the stream worth nothing (engine function `INTERNAL RATE OF RETURN`); **`VALUE(stream, writeoff=time)`** is straight-line write-off. **`DELAY(x, time, exact=True, leak=)`** is a conveyor that loses a fraction of what is on it each step. These follow the Vensim help; no Vensim output is available to compare.
- **Fixed:** `LOOKUP INVERT` (and `VECTOR LOOKUP` mode 5) read a *falling* curve wrongly (it needed rising x after swapping); the swapped table is now sorted.
- **Not built:** `SOLVE` (backlog B47, needs a design), `DELAY` with `batch=` or `profile=`.

## R37 — 2026-11-20 · Verb AGE
- **`AGE(stock, every=, entrants=, open_ended=)`** is the net change from ageing of one class of a set: the class before it moves in, its own people move on, every `every` time units, the whole class at once. `entrants=` refills the youngest class with a period's worth of newcomers (births) at each move; `open_ended=False` lets the oldest leave the model. Use it inside the stock's change with births and deaths: `Change[age] = AGE(Population, every=5, entrants=Births) - Deaths[age]`.
- **Checked** by a case worked out on paper (classes of 30, 20, 10 moving every 2 years, with and without the open end and with entrants) and by the book's ageing-population example. The stock changes in steps once per period, so read it at the years the classes move. Use Euler integration.
- **Vensim:** the writer gives each class's ageing flow as a formula (Vensim's `SHIFT IF TRUE` changes the stock itself); the file reads back to the same numbers. Importing `SHIFT IF TRUE` is not built.

## R36 — 2026-11-20 · Verbs QUEUE and WAIT
- **`QUEUE(arrivals, served, init=)`** is a waiting line that remembers how long each has waited; the oldest leaves first. **`WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)`** reads it: the average wait, the longest wait, or how many have waited longer than a limit.
- **Vensim:** `QUEUE FIFO`, `QUEUE AGE AVERAGE`, `QUEUE AGE OLDEST` and `QUEUE AGE IN RANGE` are read (the five-argument `QUEUE FIFO` loses its starting-age profile, reported). Vensim's `queue1` to `queue3` samples and `QUEUE functions` now run (294 of 367 samples). Attribute queues (`QUEUE FIFO ATTRIB`, `QUEUE ATTRIB …`) are not built.
- **How it works:** the queue is a level kept from the running totals that came in and went out; ages come from the groups that arrived in each step. Never more leave than are waiting.
- **Checked** by a case worked out on paper (54 waiting, the oldest 6 steps old, average 174/54, 24 older than 4) and by the book's emergency-room example. **Not checked against Vensim's own numbers**, which are not available for this DSS function.

## R35 — 2026-11-20 · Verb VALUE, and the book outline
- **`VALUE(stream, rate, init=, factor=, timing=continuous|period_end)`** is the present value of a stream of money; equal to `NPV` and (with `timing=period_end`) `NPVE`, and exported as those.
- **The book**: [`book/Book-Outline.md`](../book/Book-Outline.md) holds the plan, 31 chapters in four parts with appendices, the status of each technique, the conventions (screenshot notes, status marks, chapter structure) and the production plan. Three runnable examples with verified numbers are in `book/examples/`.

## R34 — 2026-11-20 · Verb ALLOCATE
- **`ALLOCATE(requests, supply, by=, overlap=, curve=, spread=)`** shares a short supply among the items of a set: in proportion to what each asks, by strict priority, or along a smooth priority curve (`rectangular`, `triangular`, `normal`, `exponential`). Written in a formula per the same set as the requests, it gives each item its share. It replaces Vensim's `ALLOCATE BY PRIORITY` and `ALLOCATE AVAILABLE`.
- **Same numbers as Vensim's functions**, tested for four combinations to the last digit. A file exported to Vensim contains `ALLOCATE BY PRIORITY(…)` or, for the proportional share, a formula; `curve=` has no one-line Vensim form and is kept as written and reported. Errors name the valid options (`bye=` → "did you mean 'by'?").

## R33 — 2026-11-20 · Verbs: one function with named options (first: DELAY)
- **Named options in the equation language**, Python style: `DELAY(Shipments, 6, order=3, kind=information, init=1)`. Options come after the plain arguments; a misspelt one says what the valid ones are; a bare word (`kind=information`, `exact=True`) is read as a choice, not a variable.
- **`DELAY(x, time, [order], init=, kind=material|information, exact=True|False)`** replaces `DELAY1`, `DELAY3`, `DELAYMTR`, `DELAYINF`, `SMOOTH`, `SMOOTH3` and `DELAY FIXED`. It is rewritten into the engine's own functions when an equation is read and when a model is exported, so Vensim files exported from Dynamist contain the Vensim functions. Seven combinations are tested to equal the functions they replace to the last digit; B2 is unchanged.
- **Direction** (backlog B42 to B46): fewer, simpler, adaptive functions; Vensim and Stella names stay for import; a book of expert cases.

## R32 — 2026-11-20 · Vensim functions that the help documents fully
- **New:** `LOOKUP SLOPE`, `LOOKUP AREA`, `NPVE`, `DERIVN`, `VECTOR LOOKUP` (all ten modes), the `GET DATA` family (`AT TIME`, `BETWEEN TIMES`, `FIRST TIME`, `LAST TIME`, `MAX`, `MIN`, `MEAN`, `MEDIAN`, `STDV`, `TOTAL`) and the Excel date functions `EXCELDATESERIAL`, `… YEAR`, `… MONTH OF YEAR`, `… DAY OF MONTH`, `… DAY OF YEAR`.
- **Checked against the values the Vensim help prints** (these functions have no PLE-runnable reference output): `LOOKUP AREA` 1, 4 and 3; `LOOKUP SLOPE` at a point 0.75, outside 0, and :NA: for m = −1; `EXCELDATESERIAL(2008, 1, 1)` = 39448; day of year; `DERIVN` of t² gives 1, 3, 5, 7. The help's printed `LOOKUP SLOPE` table does not match its own examples; ours follows the written definition, which the examples fit. These are **unverified against Vensim itself**.
- **Result:** 292 of 367 Vensim samples run (was 285); no test file got worse; PySD agreement unchanged at 175 of 185.
- **Not yet:** `DELAY MATERIAL`, `DELAY CONVEYOR`, `DELAY BATCH`, `DELAY PROFILE`, the `QUEUE …` family, `SHIFT IF TRUE`, `GET TIME VALUE`, the solvers and the market functions (backlog B32).

## R31 — 2026-11-20 · XMILE arrays, and twenty more Vensim functions
- **XMILE/Stella:**
  - Variables over two or more dimensions are read as one variable per combination, as the `.mdl` reader does, instead of being dropped. That covers per-element `<element>` equations and comma or `;` lists of values; references to them are resolved per combination.
  - A flow shared by two stocks now gives each stock its own flow.
  - An `<aux>` that a stock names as its flow is read as a flow.
  - A `{comment}`-only module input is 0 until it is connected.
  - Comma lists in 1-D arrays are read.
  - An equation the reader cannot read is now **kept as written, so the model refuses to run**, instead of silently becoming 0. 16 XMILE files in the SDXorg suite, which Vensim's exporter left with `???`, now fail clearly instead of giving zeros.
- **New engine functions:**
  - `NPV`, `DEPRECIATE STRAIGHTLINE`, `GAMMA LN` and `RANDOM EXPONENTIAL`;
  - `LOOKUP FORWARD`, `LOOKUP BACKWARD`, `LOOKUP INVERT` and `LOOKUP EXTRAPOLATE`;
  - the array functions `ALLOCATE AVAILABLE`, `ALLOCATE BY PRIORITY`, `VECTOR SORT ORDER`, `VECTOR RANK`, `VECTOR REORDER`, `VECTOR ELM MAP`, `VECTOR SELECT` and `INVERT MATRIX`. Each is written out element by element when a Vensim model is read, with `!` optional.
- **Read as other functions:** `FORECAST`, `TIME BASE`, `REINITIAL` and `DELAY INFORMATION`. Vensim's `:NA:` and the `VECTOR SELECT` constants (`VSSUM` … `VSERRATMOSTONE`) are also read.
- **Fixes that change results:**
  - `DELAY FIXED`'s initial value is evaluated once, at the start, as in Vensim. It used to be re-evaluated while the delay filled.
  - `TREND` (and so `FORECAST`) waits for its input before it starts; it used to start on an unset value.
  - `ZIDZ` and `XIDZ` treat a divisor below 1e-6 in magnitude as zero, as Vensim does. The XMILE writer says the same.
- **Checked against Vensim's output:** `NPV`, `GAMMA LN`, `DEPRECIATE STRAIGHTLINE`, the lookup modes, `FORECAST`, `DELAY FIXED`, `INVERT MATRIX` (2 tests) and `ALLOCATE BY PRIORITY` (2 tests) are exact. `ALLOCATE AVAILABLE` is within 1e-3, since Vensim searches for the allocation level approximately.
- **Result:** 219 of 335 test-suite files exact (was 203); XMILE/Stella exact 84 (was 78); 285 of 367 Vensim samples run (was 268); Dynamist matches PySD exactly on 175 of the 185 PySD can run.

## R30 — 2026-11-20 · Subscripts: subranges, mappings, dimensions as values
- **MDL reader:**
  - A dimension read through a mapping (`prev: infant, child -> older`) now gives the mapped item, item by item.
  - Mapped dimensions are no longer fixed by an item they happen to contain.
  - A whole-set formula that reads another set, a subrange or a 2-D variable by dimension is written item by item, instead of being left unresolved.
  - A dimension or item name used as a value is its position, numbered in the dimension that first declares it, as Vensim does.
  - `:NOT:` covers a whole comparison.
  - A dimension may list other dimensions among its items.
- **Fixed a silent error:** when a formula had to be written item by item, every item after the first was skipped, so all items took item 1's formula. Vensim's AGE models now match PySD exactly; before, their stocks were silently wrong.
- **Result:** 203 of 335 test-suite files now give the original tool's numbers exactly, up from 184, and none got worse. 268 of 367 Vensim samples run, and Dynamist matches PySD exactly on 174 of the 183 PySD can run. The 9 that differ are random numbers, a binary `.vdf` input, PULSE TRAIN (where Dynamist agrees with Vensim) and World3.
- **Gaps to PySD:** down from 66 to 37 test models. Still open:
  - time settings that change during a run;
  - `ALLOCATE BY PRIORITY`, `INVERT MATRIX`, `FORECAST` and `VECTOR SELECT`;
  - delay orders that change over time;
  - a few subscript cases (ranges inside mixed lists, 3-D exceptions, `SUM` over a 2-D variable);
  - XMILE/Stella arrays.

## R29 — 2026-11-20 · Closing the gap to PySD
- **Measured:** PySD was run on every model Dynamist failed. It matched Vensim exactly on 73 test models that Dynamist did not, and ran 25 Vensim samples that Dynamist could not.
- **Fixed:**
  - array constants written as tables (`1, 2, 3; 4, 5, 6;`, last subscript fastest);
  - `DELAY N` and `SMOOTH N` with their own initial value, and with the order given as a named constant (every stage of the chain now starts at that value);
  - `SAMPLE IF TRUE`, a new stateful function;
  - `QUANTUM`;
  - model files that are not UTF-8 (older Vensim encodings), in the app and in `dyn`.
- **Result:** 184 of 335 test-suite files now give the original tool's numbers exactly, up from 151, and none got worse. 259 of 367 Vensim samples run, and Dynamist matches PySD exactly on 169 of the 179 PySD can run.
- **Still open** (`B2_report.md`):
  - subscript subranges and mappings (the largest group, about 25 tests);
  - time settings that change during a run;
  - `ALLOCATE BY PRIORITY`, `INVERT MATRIX` and `FORECAST`;
  - delay orders that change over time;
  - World3-03's small difference from PySD.

## R28 — 2026-11-20 · B2: published Vensim and Stella models, and the classics now run
- **Benchmark B2** (`benchmarks/b2/`): every model of the SDXorg test suite and SDEverywhere's test models is imported, run and compared with the original tool's exported output, variable by variable and step by step. 151 of 335 model files give Vensim's or Stella's numbers exactly. Every `.mdl` in a local Vensim install is also checked for whether it imports and runs: 246 of 367 do. The fix list is in `B2_report.md`.
- **Classic models now import and run:** World3-03, World Dynamics, Urban Dynamics and Market Growth. Their results are not yet checked against Vensim.
  - Old DYNAMO-style lookup tables (all x values, then all y values) are now read.
  - `ACTIVE INITIAL` is a real engine function: its first argument is used during the run and its second while initial values are computed, which breaks World3's loop among initial values. Before, the importer dropped the initial value.
  - `GAME(x)` is read as `x`.
  - An alias dimension that comes before the dimension it copies is now resolved, and a dimension with no items no longer crashes the reader.
  - The SDXorg tests for `ACTIVE INITIAL` and `GAME` now match Vensim exactly.
- **Every Vensim sample model that runs, against PySD** (`pysd_all.py`): PySD runs 170 of the 246, and Dynamist matches it exactly on 163.
  - Of the 7 that differ: 3 use random numbers, 1 reads a binary `.vdf` file, and 2 are World3.
  - The last one is PULSE TRAIN at its end time, where Vensim's own output shows that Dynamist is right and PySD is not.

## R27 — 2026-11-19 · B1 replications: Sterman matched, DQN ported, R3 narrowed
- **R1 passes on cost.** The B1 beer model now follows the board game's weekly steps in order: receive, ship from what is on hand, backlog, expectations, pass orders, order on end-of-week stocks. Orders are held at 4 for weeks 1 to 4 and are whole cases, and the factory gets a production slip. It reproduces Sterman's benchmark exactly (team 204) and gives 1789 for the 11 teams against his 2028. It was checked against a Python port of Edali & Yasarcan's (2014) exact replica (`r1_board.py`, GPL-3.0).
- **M5 and pilot re-run** on the rebuilt model. SD rule against tuned base-stock: +1.5% on M5 (parity), -24% on the i.i.d. pilot. On M5, tuning base-stock on the training years now loses to the formula levels (+59%), because test-year demand runs above the training mean.
- **R2:** the authors' DQN was ported to PyTorch and trained on Colab (10,000 games). It agrees with the paper in sign in both settings. In size it is 45% worse than a swept base-stock level against the paper's 8%; the code's default base-stock levels were a poor comparator.
- **R3** in a narrower form: an independent RMSSE recomputation matches to 8e-07. The organisers' code and the price data were not available.

## R26 — 2026-11-19 · B1 on real M5 data
- **M5 run** (`benchmarks/b1/run_m5.py`): 70 store-by-department weekly series, tuned on weeks 0 to 224, tested on the last 52. Tuned SD rule against tuned base-stock: median -0.7%, parity. Forecasts F1 and F2 scored; the DQN could not be run on this machine (no AVX under Rosetta for TensorFlow 1.15). See [RESULTS.md](../benchmarks/b1/RESULTS.md).
- **App**: `--plot`, `--tab` and `--set` start options for screenshots.

## R25 — 2026-11-19 · Benchmark B1 pilot, and a random-number bug it found
- **`benchmarks/b1/`**: the first benchmark. Success criteria written and frozen first ([CRITERIA.md](../benchmarks/b1/CRITERIA.md)), a generator for the four-stage beer game model with in-file tests and a manifest, and a harness that tunes a base-stock policy and an SD rule at the same effort and scores them over 100 seeds with paired statistics. It is a **pilot**: synthetic demand, placeholder human-rule parameters, no M5, no DQN.
- **Fixed: `RANDOM NORMAL` and `RANDOM UNIFORM` ignored their seed argument.** Every seed gave the same numbers. The seed now starts an independent, repeatable stream at each call site (two call sites with the same seed still differ). **Results of older models that use these functions change.** Found because the first benchmark report showed interquartile ranges of zero width; the harness now refuses to report statistics from identical runs.

## R24 — 2026-11-19 · The fixes the Specs check asked for
- **The log records rejections.** Every submitted batch is logged: `result` (committed or rejected), `revision`, an optional `reason`, and the problems for a rejected one. `dyn ops --reason`, MCP `reason`, and the app's Run operations window.
- **More kinds of model test**: `nonneg` (a test, unlike the stock clamp), `reference` (a tolerance band round a reference series; one point is a flat line) and `balance` (chosen stocks change only by the boundary flows: a chain-level conservation test). In the Checks sheet, the operations language and native and XMILE files.
- **Manifest** in the model (levers with ranges, outputs, scenarios, bindings, validity, tests): `set_manifest`, checked by `validate`, `dyn manifest --draft`, a warning from `dyn run --set` outside a lever's range.
- **Units**: `--strict-units` (a rate may not meet its amount); unreadable units give a warning; `dyn explain` and the MCP `explain_problems` give a hint for every issue.
- **Optimize moved into the core crate**; `dyn optimize` and an MCP `optimize` tool. On the fuel model, two levers within limits: end shortage 611.7 → 472.7 MB in about two seconds.
- **Loss reports** gained `source`, `target`, an optional `location`, and the statuses *flattened* and *preserved, cannot run*.

## R23 — 2026-11-19 · The new engine in the app
- **File menu:** import Vensim or Stella models (`.mdl`, XMILE) and export both, each with a **conversion report** window. Opening a model by double-click or the Open dialog also accepts them.
- **Edit → Reusable modules…**: define modules with typed ports, add instances with their own parameters and inputs, see outputs and plot them.
- **Edit → History…**, **File → Compare with a saved model…** and **Edit → Run operations…** (validated batches, with the same refusal messages as the command line).
- **Function reference** shows the Vensim and Stella equivalents of each function; the generated wiki page has them too.
- **Fixed:** opening a big flattened model (an XMILE file with 1,500 variables) hung the app for over a minute: the unit check copied its name table for every variable and the Problems tab ran it three times a frame. It no longer copies, and suggestions are cached per edit (opens in about 4 s).
- The semantic diff now recognises a **rename** as one change.

## R22d — 2026-11-19 · XMILE as a bridge
- **XMILE reader and best-effort writer** (`core/src/xmile.rs`, no new dependencies), with loss reports. The reader handles stocks, flows, auxiliaries, units, graphical functions, one-dimensional arrays, expression macros, sim specs and view positions. Checked against the SDXorg test-models repository (MIT): of 67 XMILE files, 40 read and run to the expected outputs; the rest are malformed upstream, use features Dynamist lacks (two-dimensional arrays, subranges, Stella's non-negative semantics) or were not diagnosed. See [Format mapping: XMILE](Format-Mapping-XMILE.md).
- Written XMILE round-trips the fuel model's results (1,524 series) and the starter model within 1e-9, but only against our own reader: it was **not opened in Stella or any other tool**. Parent sets are written flattened, so a trip through XMILE loses the set structure (reported).
- The fuel model survives sdm → mdl → xmile → sdm with identical results.
- Fixed: names containing dots (`Shoaiba C.C. - Diesel`) were split when read.

## R22c — 2026-11-19 · The operations language is documented from the code
- **[Operations language reference](API-Reference.md)**: all 26 operations with fields, types, an example and when each is refused, plus validation codes and queries. Generated by `dyn docs` (or `dyn docs --json`; an agent gets it through the MCP tool `describe_api`). Tests parse every example as a real operation, check that every operation in the code has an entry, and check that the wiki page is the generated one.
- Kinds, port directions and port kinds are accepted in lowercase everywhere (module variables used to need capitals).

## R22b — 2026-11-19 · Vensim `.mdl` and the vendor reference
- **Vensim `.mdl` reader and writer** (subscripts, macros, data variables kept as placeholders, a full sketch, loss reports). The fuel model written to `.mdl` and read back gives the same numbers for all 1,558 series. Not opened in Vensim itself. See [Format mapping: Vensim](Format-Mapping-Vensim.md).
- **[Vensim and Stella side by side](Vensim-Stella-Reference.md)**: terms and every function compared with the vendors' documentation, with the traps that change numbers (`PULSE`, `GAMMA`, `INTEGER` of negatives, `RANDOM NORMAL` argument order). It led to a fix: `GAMMA` and `FACTORIAL` are written to `.mdl` as `EXP(GAMMA LN(…))`, because Vensim has only the logarithm.

## R22 — 2026-11-19 · One model API
- **Core crate** (`core/`, `dynamist-core`): the model, equations, simulator, sets, units, checks, causal-loop finding and the `.mdl` importer moved out of the desktop app into a crate with no window code. The app is now a client of it. See [Model API](Model-API.md).
- **Operations API**: atomic batches of operations, validated before they are accepted, with an audit log, undo/redo and a generation gate.
- **`validate`**: one call with structured issues (equation, check, conservation, arrows, units).
- **Units as algebra** replacing label matching. It found a real mistake in the fuel workbook: *Barrels per truck* was declared in barrels, not barrels per truck (fixed in the sheet and the restructure script).
- **Module definitions and instances** with declared, typed ports (`material` / `information`) and units; instances expand to ordinary variables before a run.
- **`dyn`** command line (info, validate, test, run, sens, convert, diff, list, get, deps, loops, ops) and an **MCP server** (`dyn mcp`), both JSON; a **Python wrapper** around `dyn`.
- **Semantic diff** of two models and of two runs; **`.sdm` can be written as meaning + layout** files so version-control diffs show model changes.
- **Loss reports** for every format read and write (see the two Format-Mapping pages).

## R21 — 2026-11-19 · Compare with data
- **Edit → Compare with data…**: pick a result and a data series (a TimeSeries column), hold out the data after a split time, and see whether the model beats guesses that only see the data before it: last value, straight-line trend, and the same time one season earlier. RMSE and average miss for each, a plain verdict, and a picture.
- The model is scored as it stands (levers as set, or fitted with Optimize) and is not restarted from the data at the split.
- A fourth guess, **smoothed trend** (exponential smoothing with a damped trend, fitted on the data before the split), joined the three above. Written in-house, no new dependency.
- `--compare-window` opens it on a small demo.
- Fitting levers to data needs no new feature: add an aux such as `(Customers - Customers actual)^2` and minimise its *average* on an Optimize sheet.

## R20 — 2026-11-19 · Checks inside the model
- **Edit → Model checks…**: state limits the results must respect ("Network shortage stays between 0 and 1000 MB", at all times or at the end). A broken limit shows in the Problems tab and counts in its badge; click it to select the variable.
- **Conservation check**, automatic: every stock must change by exactly what flowed in minus what flowed out. Stocks held at zero ("never negative"), averaged, or filled by flows of another set are left out. Euler is checked to 1e-6, the Runge-Kutta methods to 2%.
- A **Checks** sheet (Name, Variable, Min, Max, When) in spreadsheets and CSV folders; the fuel model carries four.
- `--checks-window` start option.

## R19 — 2026-11-18 · Inbox in the app
- Inbox window (envelope in the top bar with a count of open messages, Help → Inbox, command palette). It reads open, handled and outbox messages, lets you write to the builder or to other projects, and copies a ready-made note for another conversation.
- `--inbox` start option.

## R18 — 2026-11-18 · Other projects
- New wiki pages: Other projects (how separate conversations work together through shared files) and Outbox (the builder's messages to them).

## R17 — 2026-11-18 · Tidier diagrams
- **Backlog batch** ([BACKLOG.md](BACKLOG.md)): RK2 and RK3; SINWAVE, COSWAVE, GRAPH, GRAPHSTEP, DELAYINF, DELAYMTR; your own functions in autocomplete; drag a lever circle up or down; panel layout and dark mode remembered; legends name the selected tank/region; units warnings; View → Outline; Edit → Sensitivity runs… (Monte Carlo band on the key levers).
- **Functions**: 20 more maths functions (SINH…GAMMA, CLAMP, MEAN, MEDIAN, PI()…); **My functions** (Edit menu, Functions sheet in spreadsheets) to write your own in the equation language; **Function from Python** to sample any standard-library function into a lookup curve. See [Functions](Functions.md).
- **Fold**: a chevron on each module card folds it to name + purpose or unfolds its inputs and outputs; View → Fold all / Unfold all.
- **Levers as circles** outside the module boxes with arrows to the modules that use them (the card is used on sheets without modules).
- **Key levers card on the diagram** and an **all-items slider** (× factor) for per-set levers.
- **Key levers**: mark constants as levers (Details → Lever, or a Lever column in Parameters); the side panel then shows just those as sliders, the rest folded, and per-tank levers once a tank is picked in the Showing bar.
- **Modules of modules**: a Module path (`Plans / Optimal plan / Trucks & skids`) builds nested modules on import; a breadcrumb in the tab bar (Model › Plans ›) goes up any number of levels; a module's part count includes everything below it. The fuel model is now three levels deep.
- Module boxes keep their inputs (left) and outputs (right) and gain a one-line purpose and a role colour (inputs / mechanism / results); *View → Modules → Compact* is an option that hides the ports and labels the arrows instead. Purposes come from an optional *Modules* sheet (Module, Description). The Entities cards sheet was tried and withdrawn: tables of every variable are not what this view is for.
- **Levels**: a *Module* column in the spreadsheet builds modules on import; the Model sheet becomes an overview of a few boxes; module sheets are reached by double-click or *Modules ▾*, with ◂ up one level, and no longer crowd the tab bar.
- **Showing bar**: dropdowns per set (Region, Tank). Pick one and the diagram numbers, graphs, tables and sliders are for that item; sliders for the selection edit just that item.
- Panel controls are icons: layout toggles in the top bar, chevrons on the panels and their strips.
- Side panel (right): Hide ⟩ button, ⌘B, command palette entry, slim ⟨ strip to bring it back.
- Results panel: ⌄ Hide button, ⌘J, command palette entry; when hidden a slim bar shows the tabs (and the problem count) and reopens the panel.
- Arrows can have several turning points: a smooth path through points you drag (double-click an arrow to add one).
- *View → Diagram look*: Classic (plain labels, thin dark arrows) or Cards; Classic is the default for big imported models.
- Arrow styles (curved, straight, right-angle; filled, open, dot tip) in View → Arrows and per arrow.
- Arrange → Tidy up this diagram (layered layout, blocks stacked downwards).
- Crowded diagrams draw arrows quietly until something is selected.
- Charts: *one … at a time* in the + Add menus plots a single item of a set variable (`Name[item]`).

## R16 — 2026-11-18 · Sets
- **Sets**: variables can be one for each item of a set (tanks, regions…); `TOTAL`, `AVERAGE`, `X[item]`, `CEILING`; sets can belong to sets.
- Diagram: stacked cards and *Tank ×57* badge; *One for each* and *Differs by…* in the variable panel.
- Spreadsheet template: Sets, Values, ItemSeries sheets, Per and Combine columns; folder of CSVs; `dynamist --convert`.
- Scenarios can change one item: `Max trucks per day[Jazan - Diesel]`.
- Example: the fuel-supply workbook as a model.

## R15 — 2026-11-18 · Paths
- Wiki home page lists the full paths of the project, app, program, wiki and inbox.

## R14 — 2026-11-18 · Wiki and spreadsheets
- Added this wiki: user guide, getting started, spreadsheet template, equation language, function reference (generated), file formats, shortcuts, architecture, features checklist, changelog, development log, inbox.
- File → Import model from spreadsheet… (Excel, OpenDocument, CSV) using the template; report window of anything skipped.
- File → Export model to spreadsheet…, Save blank spreadsheet template….
- Help → Documentation (wiki).
- `--write-docs`, `--import-sheet` command-line options.

## R13 — 2026-11-18 · Groundwork for Excel
- Added the spreadsheet reading library (no visible change).

## R12 · Chart customization
- *Customize chart* window: type, variables, titles, axis titles and ranges, legend position, grid, line width, markers, colours, scenarios on/off.
- Legends say what each line is (current model, scenario, kept run) with a caption under the chart.
- Fixed charts growing or shrinking their panel.

## R11 · Questions answered
- No app changes (which AI builds the app; where the app files are).

## R10 · Scenarios
- Scenarios sheet, scenarios in every chart, distinct scenario colours, Runs & scenarios tab.

## R9 · Copy any chart
- Right-click any chart: copy/save picture, copy data; header copy icons.

## R8 · Function help
- Function reference window, autocomplete, argument hints, F1.

## R7 · Icons and module files
- Vector icon set across the app; module export/import with input mapping.

## R6 · Pictures for Office
- Transparent high-resolution PNG copy/save of sheets, selections, charts and cards.

## R5 · Editing power
- Multi-select, copy/paste across sheets, groups, modules, alignment tools, command palette, mini-map, more chart types.

## R4 · Loops, playback, optimization
- Causal loop sheets with loop finding; runs play in over time; optimization sheet.

## R3 · Arrows and sheets
- Curved arrows; diagram, table and dashboard sheets.

## R2 · Build fix
- The build tools were made visible to Craft Anything's checker.

## R1 — 2026-11-13 · First version
- Stock-and-flow editor, equations, lookups, simulation, trend graphs, live sliders, `.mdl` import.

## R39: find what matters (B3)

- **Goals** in the manifest (what a study is after, with measure and direction), checked by `validate`.
- **`dyn screen`** (Morris with convergence check), **`dyn importance`** (Sobol indices, Loops That Matter, eigenvalue elasticity, removable variables), **`dyn dominance`**, **`dyn reduce`**, **`dyn verify`**, **`dyn recommend`**; Python methods and agent tools for the first three. See [Find what matters](Find-What-Matters.md).
- Benchmark B3 (`benchmarks/b3/`): two cases scored against published findings (7 of 7; 3 of 4 in each Market Growth scenario), one run without a key, report and proposed specs changes.
- Found and fixed by the benchmark: link scores for stocks use the change in the net flow; SMOOTH and DELAY chains are followed; `reduce` removed things that initial values read; `verify` did not vary levers the reduced model lacks.
- The Vensim `MARKET` sample is not Forrester's Market Growth; labels in the B2 report and wiki corrected.

