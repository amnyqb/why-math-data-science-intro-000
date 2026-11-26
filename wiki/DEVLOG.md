# Development log

One entry per working round, newest first. Each entry: **Request** (in the person's words, shortened), **Decisions**, **Changes**, **Checks**, **Open**. The builder adds an entry at the end of every round; others may append notes under an entry.

## R47 · 2026-11-23 (round 8)

**Request.** B73 to B77 as written in `inbox/round-8-prompt.md`: Dynamist tools inside sd-ai's agent, the with-and-without protocol on development groups, `dyn attribute`, `dyn pack`, `dyn hindcast`.
**Decisions.**
- *Attribute.* A group is one changed variable; an added or removed variable, and every variable whose equation reads one, is one group, found by trying each group alone and falling back when any fails to run (the note is printed). A change that cannot move a run (comment, unit text, positions, checks, manifest entries) is listed, not split: the manifest's lever defaults and ranges are not read by the base run. The total is new minus old as the two models run; the parts add up to the old model with every group applied, and the gap to the new model's own value is printed as "not in any group" (zero in every test). Sampled permutations use the crate's SplitMix generator.
- *Charts.* The model-chart code draws egui plots inside the window and has no waterfall or fan; a small SVG writer (`core/src/plotsvg.rs`) draws both. No PNG for them yet.
- *Pack.* Built in the CLI crate from the same functions the other commands use. The model card reads only the manifest and the brief (`Purpose`: problem, kind, out of scope, objectives). Parts made but not delivered (the sanity sheet of a model with no recorded human checks) are made, with the status said in the index. The reporting checklist is not made: the open copy of the paper answered 429 three times, and nothing was written from memory (`papers/TO_FETCH.md`).
- *Hindcast.* The fit is a model augmented with a squared-gap stock, minimised through `optimize` on the data up to the cut; the data handed to the fit is filtered at the cut first. The data are interpolated between points, so a step finer than the data weighs the gap at every step, not only at the data points. The bands come from `dyn sens`, so they hold lever uncertainty only; the report says so.
- *sd-ai.* A clone outside this repository (`vensimple/sd-ai-work`, branch `dynamist-tools`, based on commit 58cfc86). No fork was made on GitHub (an outward step the owner has not asked for). The XMILE conversion is sd-ai's own (`utilities/SDJsonToXMILE.js`), reused. Nothing from sd-ai is copied here. The comparison script reads only fields every result row has.
**Checks.** Server, `scripts/server-test.sh round-8`: see the round 8 reply in INBOX.md. New tests: 6 in `attribute`, 2 in `plotsvg`, 1 in `modelcard`, 4 in `hindcast`, 4 pack tests with 6 snapshots; sd-ai side: 5 tool tests, 3 flag tests (flag absent, false and unset give the same tool list and the same response), 4 for the comparison script (Python).
**Open.** B73 and B74 see `benchmarks/sdai/REPORT.md`; the reporting checklist of B76.

## Doctor fixes · 2026-11-22

**Request.** Fourteen fixes to the model doctor from the book-side review of the handover, and corrections to the solar model and the allow-list count.
**Decisions.** A link with zero gain is dead only when its target has no hidden state (SMOOTH, DELAY, TREND…): those show zero instantaneous gain by construction, and the chapter 2 market loop must stay balancing. The default of `validate` stays lenient about the time unit by dropping the flow-against-stock findings whose message starts with "Flow" (the old tests ask for it); strict mode is the doctor's own reading. Names of conversion constants pluralise only the words for units of time ("Days per Week"; "MB per barrel" stays, an existing test asks for it). `needs_decision` is set from the unit texts, never from the factor. The IRR of the ch11 plant is 1.2924% a month compounded; the book's 1.284% is the same rate as a continuous rate (ln(1.012924)), so the CLI prints both. G2 stays an error only for a stock reading itself. N2 and N5 are not reported when the reruns show under 1% of the range, and are unconfirmed (and kept) when no rerun could be made (a model with no stock, a run over 400 000 steps). ch09 depot still moves 1.1% of its range at DT/4, so its N5 stays (allow-listed).
**Changes.** `core/src/polarity.rs`, `validate.rs`, `doctor/{fixes,units_rules,base,rules_f,rules_g,rules_n,rules_x,experiments,interval,mod}.rs`, `finance.rs`, `units.rs`, `cli/src/main.rs`, `src/health_ui.rs`, `models/solar/`, `core/tests/doctor_fixes.rs`, `scripts/doctor-sweep.py`.
**Checks.** Whole workspace on the server: 339 passed, 0 failed, 4 ignored. Sweep of the 215 book models with `dyn check --gate review --json` (`handover-out/`); 35 confirmed errors in 7 models, all in `allow.toml` (35 entries at that point; 21 after the second round).
**Second round.** Items 15 to 19 of the coordinator's list. *Decisions:* a fixed lever stays a constant (only its lever flag and slider go), so units survive; the seed of the three old random functions is appended at compile time as the call's own key, which is how the other random functions get it; `order` is the compiler's state count (a fixed delay keeps a history, not states, so it is not counted); `figure_model` lives in the settings, written only when true, and only the units rules read it; the render keeps pipes raw through a per-thread flag set by `Headless`, so the window and its tests are unaffected. The saved SVG `ch13/diagram.svg` changed (its cloud was 700 units from the stock before, clamped to 200 by the window's rule). `ch03-programme.sdm` is not in this repository, so the comparison was made on a model with a far cloud (`cli/tests/figures.rs`). The publish-gate sweep ran experiments for the first time on the book set: X6 gives 44 errors in 49 models (mostly the venture balance check, a NaN when a divisor constant such as Acquisition Cost is set to 0), which are not allow-listed (the list may only shrink).

**Third round.** Items 21, 20, 24, 22 and 23. *Decisions:* the sanity sheet has twelve rows, not the "ten checks" of the book (the book section was not available): the seven automated ones and five human ones listed in the changelog; a human row is `pass` only when it is signed (by and date) and complete (expected value: the number; the others: a note). `ablate` cuts a link by making the target's equation read `INITIAL(source)`, and freezes a stock by detaching its flows (so the stock keeps its initial value and the other end of a flow is unharmed). The xlsx has values only: live formulas (one column per time step) are not built yet. `serve` needs no HTTP library: requests are read with `std::net`, one thread per connection, a body of up to 1 MB, `Connection: close`; the test starts it on 127.0.0.1 port 0 inside the test process. X6: the 44 errors of the publish sweep were a value that is not a number when a divisor constant is set to 0; that is X1's finding (an unguarded division), so X6 now skips it (a value that is not a number or infinite); the other X6 findings stay (a declared check broken in an extreme case): 37 errors in 37 models (28 venture models whose "Balance sheet balances" check, tolerance 0.01, shows a gap of 0.0156 to 0.0625 when Hiring Delay is set to one time step, and nine chapter models: ch07 service-draft, ch23 crews-packaged, ch24 fuel, ch25 service-firm, ch29 pipeline v1 and v2 and others). Recommendation for the book: give the balance check a relative tolerance (or a tolerance of 0.1), keep Hiring Delay at least two steps, and guard the ch07 backlog and the ch24 idle trucks at their start.
**Open.** The book side has 219 models; ch26 to ch29 are not all here. F3 gives many "possible" findings (151) that no run confirms.

## R46 — 2026-11-22

**Request.** "Change Flowcanvas to Dynamist" (`fc` to `dyn`, `.flowmodel` to `.sdm`); a model library with tags to search and filter; the classic models; "no heavy tests on the laptop, GitHub as staging".
**Decisions.** Old files must keep opening, so the old extension, tags and preferences file are read as well. Internal name prefixes written into imported data (`fc graph`, `fc lookup`, `fc init`) stay. History was rewritten in a clean clone to drop result files over 100 MB; the old repository is untouched. The classics are our own implementations (own numbers, independent reference checks) so no third-party terms apply; importing existing files is the fast path where the licence allows it, so `--ingest` requires a stated source and licence.
**Changes.** Renames across 160 files; `core/src/catalog.rs`, `cli/src/library.rs`; `library/catalog.json`, `make_catalog.py`, `classics/`; `scripts/server-test.sh`; `core/src/mdl.rs` (tag length). Architecture sheet: `src/architecture.rs`, `core/src/architecture.rs`.
**Checks.** Full workspace tests on the server (all `ok`); catalogue check (19 entries); the six classics' own checks; strict units on each.
**Open.** Library browser in the app; World2, Urban Dynamics, World3; ingest of the MIT test-model collections is running; free-form Excel converter.

## P2 and P3 · 2026-11-22 (enhancement programme, phases 2 and 3)

**Request.** Phase 2 (the 72-rule doctor in seven stages) and Phase 3 (the function library, release 1).
**Decisions** (where the prompt left a choice).
- *Missing inputs.* The Doctor specification (rule catalogue), `golden.json`, `golden.py`, `units_trap.mdl` and the two other specifications were not available. The trap model is the text of the prompt. The golden values were recomputed in Python from textbook formulas (`core/tests/data/golden.py`, `golden.json`). They agree with the prompt except the NREL levelised cost: the prompt's 41.4332 is not reproduced by its own inputs (real WACC 0.036007 gives 41.4407); the test uses the independent value.
- *Rules.* Only the rules the prompt describes well enough to build were built (see Model-Doctor.md); a rule's finding code is the rule id for new rules, and the phase 1 codes map to rule ids (`doctor::rule_of`). The test corpus is `core/tests/doctor/<ID>.json`, a batch of operations that builds the broken model (a model of a rule that needs a setting, a never-negative stock or a drawn arrow cannot be written in `.mdl`), with the rule named in its title.
- *Intervals.* Aux and flow intervals start at "anything" and are recomputed 5 times, which is sound; a bound still moving is simply the loosest the five passes gave. "Can" findings (F1 to F3, F8, F14) have `confirmed: false`; F3, F8 and F14 are Info so that they do not fail the gates.
- *F1 and F2 fix.* The control the prompt asks for (`MIN(desired, Stock / Stock Minimum Time)` with a time of 2 DT) has a time constant of twice the step, so N2 and N3 flag it afterwards. The corpus test accepts N2 and N3 after an F1 or F2 fix and says why.
- *Experiments.* X1 runs at most 200 cases (no Morris screening). A value that is missing by design (LEVELISED, PAYBACK, RATE) is not a break. Benchmarks are tested in the base case only.
- *Finance.* A rate's period is read from the declared unit of a rate given as a variable (1/Year in a monthly model: 12 steps). Stateful finance functions update once per step (exact for Euler; the Runge-Kutta stages evaluate them at stage times). LEVELISED, PAYBACK and the others report the amount of the step that starts now, as NPV does. LOAN pays in the step it opens: a loan drawn at t = 0 over 360 months is paid off at month 360.
- *Random.* The argument order of the `RANDOM …` family is minimum, maximum, then the distribution's parameters, as recalled from Vensim; it could not be checked against the manuals (no network access to the help pages). `seed=` is read as the next positional argument. RANDOM LOOKUP is not built.
- *Alias table.* An alias applies only when the target takes that number of arguments, so a lookup called `Effect` still works.
**Changes.** `core/src/doctor/*`, `calendar.rs`, `finance.rs`, `stats.rs`, `sim.rs` (finance, random, calendar, probe states), `expr.rs` (functions, aliases, verbs), `sets.rs`, `checks.rs`, `ops.rs`, `cli/src/main.rs` (`check`, `finance`, `compare`, `--seed`), `models/solar/`, tests in `core/tests/`.
**Open.** Rules not built (S2 to S4, G2 to G10, G12, F5 to F7, F9 to F11, F13, N6, X4, X5, C-rules, R2, P-rules); the app panel for findings; CONVERT(); dated data through GET DATA; RANDOM LOOKUP.

## P1 · 2026-11-22 (enhancement programme, phases 0 and 1)

**Request.** The Dynamist enhancement programme: three quick fixes, then exact units and calendar time with the doctor's base. The specification files and `golden.json` were not available; everything below follows the prompt text.
**Decisions** (where the prompt left a choice).
- *Outflow constraint:* proportional scaling only; the model has no priority setting, so "declared priority" is not implemented. A constraint is logged once at the start of each spell (consecutive constrained steps are one entry), which is what makes the `nonneg_conserves` test see one entry. The old exemption of never-negative stocks from the conservation check is removed, because they now conserve.
- *Loop polarity:* a link's sign at each saved time is the sign of the change of the output when the input is raised by 1e-4 relative, other inputs held (`sim::link_signs`). A link with no effect at some time keeps its last sign; a link that never has an effect (a smoothing function whose input does not enter its output at once) keeps the sign drawn on it, `+` by default. Links into stocks are structural. A loop "switches" when the product of its signs changes; the saved time of the change is reported.
- *Unit engine:* the older name-based `Dim` checks stay for `dyn validate` (their tests state the lenient rule); the doctor uses the exact engine. `Dim` is kept as the alias for the dimension map. The exact `Unit` and its parser are in `units.rs`, the atom table in `unit_table.rs`.
- *Value scale:* only percent and ppm rescale values when `units_scale_values` is on. Rescaling every literal to the SI unit of its dimension would turn a stock in MB into cubic metres and change what every chart shows. Imported Vensim and Stella models, and files without the setting, are off.
- *Strict time:* the doctor treats a rate meeting an amount as an error; `--lenient-time` restores the old rule. `dyn validate` keeps the lenient default because existing tests assert it.
- *Ambiguous symbols:* the first meaning in the table is used until the modeller chooses. Choices are written with `set_setting` key `unit_choice` (`SYMBOL=meaning`).
- *Typos:* an unknown word within edit distance 1 (2 for atoms over 4 letters) of a known unit symbol is an error with a suggestion; any other unknown word is a counted thing. Only a short list of symbols is offered as suggestions, so ordinary words are not flagged.
- *Doctor layout:* `core/src/doctor/` from the start (`mod.rs`, `units_rules.rs`, `fixes.rs`, `base.rs`), as the phase 2 prompt asks, instead of one `doctor.rs`.
- *DT sensitivity:* the relative difference of a stock is taken against the larger of the halved-step value and a millionth of the stock's largest value, so a crossing of zero does not give a false alarm.
- *Excel serials:* Excel's own value is returned for 1900-01-01 to 1900-02-28 (one below days since 1899-12-30); the sentence in the prompt about "1 lower" was read that way.
- *Calendar flows:* in a Calendar model a flow declared per Day or Hour is scaled by the real days or hours of the step when it enters its stock; the reported flow value is unchanged. The step must be one time unit (a month or a year).
**Changes.** `core/src/{sim,checks,ops,units,unit_table,calendar,polarity,model,mdl,xmile}.rs`, `core/src/doctor/`, `cli/src/main.rs` (`dyn check`), `src/spreadsheet.rs`, tests in `core/tests/`, the 215 book models copied into `core/tests/book_models/`.
**Checks.** Core unit and integration tests, the units trap, the doctor on every book model (no crash; report in `build/model_health.json`).
**Open.** `CONVERT(x, "from", "to")`; the app panel for findings; dated data is not yet read through GET DATA.

## R45 — 2026-11-20

**Request.** "Diagram controls" (the rest of ribbon step 3).
**Decisions.** The controls reuse the key levers' machinery: a control writes a number into a constant's `equation` and takes its range from the constant's `slider`, so the lever card, the side panel and every control stay in step with no new state. They are painted (not egui widgets in areas) so exports see them and zoom scales them; operating one is hit testing in `handle_mouse` with four parts (body to move, the operated part, minus, plus). A switch is on above the middle of the range. Knob and number drags are relative and round to a tidy step when the drag ends, because rounding each small step would swallow slow drags. A control holds a variable id, and lives in the layout file with the shapes. Only plain numeric constants can be tied to one; anything else leaves the control showing "Pick a variable".
**Changes.** `core/src/model.rs` (`Control`, `ControlKind`, `Model.controls`), `core/src/io.rs` (layout), `mdlw.rs` and `xmile.rs` (loss reports); `src/controls_ui.rs` (geometry, drawing, operating, creating); `src/canvas.rs`, `src/arrange.rs`, `src/app.rs` (hit, drag, click, select, copy, delete, bounds); `src/ribbon.rs` (Controls group, Control tab, icons); `src/inspector.rs`; `src/headless.rs`.
**Checks.** 167 core tests (1 new), 52 library tests (6 new: creation and range, slider clamping, switch, number and knob, hit testing and a deleted variable, copy/move/delete/undo, SVG with values), 8 figure tests unchanged, macOS window build. The drawn controls and the Control tab were looked at as PNGs.
**Open.** Not operated with a real mouse; a control for a constant per a set; B68 Pause, Step and Reset.

## R44 — 2026-11-20

**Request.** "Shapes and pictures" (step 3 of the ribbon).
**Decisions.** One new object, `Shape`, for the five drawn kinds and for pictures (a picture is a shape whose `image` points at an `ImageAsset`), so selection, moving, copying and deleting needed one new case each and not two. Pictures are stored in the model as base64 so the file is self-contained; the split format moves shapes and images to the layout file because they carry no meaning. Shapes are drawn first and found last. The exporters cannot read a texture back, so while exporting a picture is drawn as an egui `PaintCallback` carrying its pixels: the PNG exporter blits them, the SVG writer embeds the re-encoded PNG. Ellipses are 72-point polygons, which both exporters already handle.
**Changes.** `core/src/model.rs` (`Shape`, `ShapeKind`, `ImageAsset`, `Model.shapes/images`), `core/src/b64.rs`, `core/src/io.rs` (layout), `core/src/mdlw.rs` and `xmile.rs` (loss reports); `src/shapes_ui.rs` (drawing, hit tests, handles, creation, picture loading), `src/pictures.rs` (decode, texture cache, export stand-in); `src/canvas.rs`, `src/arrange.rs`, `src/app.rs` (selection, drag, resize, clipboard, delete, bounds); `src/export.rs`, `src/svg.rs` (picture output); `src/ribbon.rs` (Shapes and Picture groups, Shape tab); `src/inspector.rs`; `Cargo.toml` (`image` with JPEG).
**Checks.** 166 core tests (3 new for shapes, 1 for base64), 46 library tests (4 new: what is under the pointer, resizing, draw/move/copy/delete/undo, a picture reaching the SVG and PNG), the 8 figure tests (byte-identical book figures), the CLI build still has no GUI crates, and the window build on macOS. Looked at the drawn sheet and the Insert and Shape tabs as PNGs.
**Open.** Nobody has dragged a handle or loaded a picture through the file dialog by hand; Transparent pictures draw correctly in the PNG export but not checked in the window; B67's remaining parts.

## R43 — 2026-11-20

**Request.** "Text formatting" (step 2 of the ribbon: font, size, bold, italic, colour, alignment, copy and paste).
**Decisions.** A contextual Text tab rather than a Font group on Home, because Home was already about as wide as a 1300-point window. The format lives on the note (`Note.fmt`, skipped when plain) so old files load and plain notes write exactly what they wrote before. Bold is a second draw 0.6 points to the right: egui's fonts have one weight, and shipping a bold face would have changed every export. Alignment needed the galley's own origin (rows start left of it when centred), so the note box is sized from `galley.rect`.
**Changes.** `core/src/model.rs` (`TextFmt`, `TextAlign`, `Note.fmt`, tests); `src/canvas.rs` (notes drawn from a layout job); `src/export.rs` (italic slant and underline in the PNG rasteriser); `src/svg.rs` (underline run attribute); `src/ribbon.rs` (Text tab, `selected_notes`, `edit_fmt`); `src/arrange.rs` (paste keeps the format).
**Checks.** Library tests (42, one new: a formatted note reaches the SVG with italic, underline and a larger size), core tests (two new: plain note writes no `fmt`, round trip, partial and unknown keys), the 8 figure tests unchanged (book figures byte-identical). The note was looked at as a PNG from `window_shot`.
**Open.** Copy format, a real bold face, more fonts, formatting other text; the dropdowns and colour pickers were not clicked by hand.

## R42 — 2026-11-20

**Request.** "We don't have many buttons compared to Vensim, Stella and Powersim", then "more like the Office ribbon"; start with step 1 and take over the UI work here.
**Decisions.** Keep every existing action and move it, rather than rewrite: the bodies of the File, Edit, View and Help menus became functions (`file_menu`, `edit_menu`, `view_menu`, `help_menu`) and the ribbon calls the same App methods. The Arrange menu was replaced outright by the Arrange tab. The time settings moved from the title row into Home → Simulation so the tabs fit. The left tool strips stay for now. Pause and Step were left out because the engine runs a whole simulation at once.
**Changes.** `src/ribbon.rs` (tab strip, ribbon body, button and group helpers, five tabs); `src/app.rs` (menus as functions, a ribbon panel under the title row, `ribbon_tab` and `ribbon_open`); `src/headless.rs` (`window_png`); `examples/window_shot.rs`.
**Checks.** The window build compiles on macOS and Linux (`gui` feature). 40 library tests, 8 figure tests (the book figures are byte-identical) and a new test that draws every tab. Each tab was looked at as a PNG from `window_shot`.
**Open.** Nobody has clicked the dropdowns; narrow windows overflow; B66 to B70 in the backlog.

## R41 — 2026-11-20

**Request.** The book team's figure library: draw sheets and charts as SVG with the same drawing rules as the app, on Linux without a display; commands render, layout, cld, set, style, chart; examples for chapters 10 to 15; docs.
**Decisions.** Run the app's own egui drawing code headlessly (an `egui::Context` with a fake `RawInput`) and convert the painted shapes to SVG, rather than copying the geometry into `core/`: one set of drawing rules, same picture as the PNG export by construction. A `gui` feature gates eframe, dialogs and clipboard; the root crate became a library plus a binary. Style lives in the model (`figure_style`) so a call needs no flags. Text gets one x per letter so any substituted font keeps egui's spacing. The app is drawn at 2 pixels per point.
**Changes.** `src/lib.rs`, `src/headless.rs`, `src/svg.rs`, `App::render_shapes` and `frame_ui`; `core/src/figure.rs`, `core/src/figstyle.rs`; `cli/src/render.rs`; Python wrapper; `book/examples`; `wiki/Figure-Library.md`.
**Checks.** Built and tested on the Contabo server (Linux x86_64): 160 core, 40 app (now headless), 8 figure tests; `cargo tree` of the CLI has no eframe, winit, rfd or arboard; SVGs rendered with `rsvg-convert` and looked at. A first build showed letters spaced evenly: egui needs its `default_fonts` feature, which eframe had been switching on (now explicit).
**Open.** macOS window build not re-run; B61 to B64.

---

## R25 — 2026-11-19

**Request.** Benchmark B1: criteria first, then everything "in the order you find best".
**Decisions.** Freeze the criteria under the owner's delegation, with my proposed tolerances. Build the model from a generator script. Treat anything that depends on the unread paper as a placeholder and say so. Call the synthetic-demand run a pilot.
**Changes.** `benchmarks/b1/` (criteria, generator, harness, results, report); the engine's random functions now honour their seed; `docs` text for them.
**Checks.** Model tests T1 to T3 and never-negative tests pass in the file; strict units clean; T4 gives bullwhip 1.00 per stage for base-stock; a second harness run gives byte-identical CSVs; P1's tuned levels cannot be improved by an independent coordinate search; P2 versus P1 holds with wider bounds. **The first harness run was void**: identical seeds (see the changelog).
**Open.** R1 cannot pass with placeholder parameters; the Sterman paper was rate-limited. M5 needs files or an account; the DQN needs an old Python environment; `augurs` is an unchecked dependency.

---

## R24 — 2026-11-19

**Request.** After the check of the Specs and Vision against the app: fix what the check found, as the first step of benchmark B1.
**Decisions.** Keep the gate on for every actor (stricter than the Specs). Keep lenient units as the default so the fuel model stays clean, and add a strict option. The manifest is a model field, not a sidecar file. The `nonneg` test and the `non_negative` clamp stay separate things, and the docs say so.
**Changes.** Log entries and rejection logging in `ops`; `CheckKind` and three new evaluators in `checks`; `manifest` module and `set_manifest`; `validate_with` options, `explain`; `optimize` moved to core with `dyn optimize`; loss report fields and statuses; the Checks sheet reads and writes all kinds; the conversion window shows the new statuses.
**Checks.** `cargo test --workspace` (core 107, app 40, CLI 1); `dyn optimize`, `dyn manifest --draft` and `dyn explain` run on the fuel model; the generated operations reference regenerated and guarded.
**Open.** `location` on loss items is not filled by the adapters yet; the manifest has no screen; nothing reads `bindings`; the Specs and Vision text still describe the crate layout and operation names as planned.

---

## R23 — 2026-11-19

**Request.** "Have we updated our app to reflect all the features we discussed?" — then "all": bring the engine work into the desktop app.
**Decisions.** Reuse the existing patterns (a report window like the spreadsheet import's, windows like My functions). The History window is derived from undo snapshots by semantic diff, instead of rewriting every screen onto the operations API first. Modules instances get a window, not a diagram box, because they are expanded before a run.
**Changes.** New `convert_ui`, `modules_ui`, `history_ui`; File and Edit menu items; the function browser and the generated reference show Vensim/Stella equivalents; warnings cached per revision; the unit check no longer clones its table; the diff detects renames.
**Checks.** `cargo test --workspace`; screenshots of the conversion report, Reusable modules, History, Run operations (a refused batch), Compare with a saved model, and the function browser; an XMILE import of the flattened fuel model (hung before the performance fix, 4 s after).
**Open.** The app still edits directly in most screens. Instances have no diagram box. Nothing was opened in Vensim or Stella. The flattened XMILE import of the fuel model is large and slow to read on screen (1,500 variables).

---

## R20–R22 — 2026-11-19

**Request.** Align the app with the review of system dynamics: checks and conservation inside the model, a comparison with simple guesses, and one model API across `.sdm`, XMILE and Vensim `.mdl`; "do everything now".
**Decisions.** The native model stays the internal form. The engine moved into its own crate (`core/`) so the app, a command line and an MCP server are all clients. No Simlin dependency (licence and quality unchecked); own adapters, written by two parallel forks working in private copies so the shared build never broke. The Python binding is a subprocess wrapper over `dyn` (PyO3 would need Python development headers and a build tool). Module instances are written out as ordinary variables before a run, as sets are.
**Changes.** New `checks`, `forecast`, `validate`, `ops`, `diff`, `io`, `runner`, `modules`, `loss`, `sensitivity` modules; `units` rewritten as dimensional algebra; the app split into engine and `*_ui` files; `dyn` and the MCP server in `cli/`; `python/dynamist`; wiki pages Model-API, File-Formats, User-Guide, Fuel-Supply-Example, Equation-Language, BACKLOG updated. Units algebra found a wrong unit in the fuel workbook (Barrels per truck).
**Checks.** `cargo test --workspace` (engine and app); an MCP session driven by a test (build, validate, run, refused batch, save, reopen); `dyn` run on the real fuel model (validate clean, 100 sensitivity runs); a screenshot of the app after the split.
**Open.** Both adapters (`.mdl`, XMILE) are finished but self-checked only: nothing was opened in Vensim or Stella. The app still edits its model directly in many screens. No screen for module instances.

---

## R19 — 2026-11-18

**Request.** Make the inbox, so other conversations can talk with the builder.
**Decisions.** Keep the messages in the wiki's Markdown files (INBOX.md and OUTBOX.md), so any assistant that can read files can take part. The app is a window onto those files and doesn't keep its own copy.
**Changes.** New `src/inbox.rs` (reads and writes messages, shows the window), a Mail icon, an envelope button with a badge in the top bar, a Help menu item, a palette command, and `--inbox`.
**Checks.** `cargo test`: 28 passed, 1 skipped (new: adding to and reading the inbox, outbox newest first, plain-text display, date). Screenshot of the Inbox window shows the open Fuel Supply message. `rebuild_check`.

---

## R18 — 2026-11-18

**Request.** If another project is added, can the builder discuss with that conversation?
**Decisions.** Conversations can't talk live. They talk through files instead: the other conversation writes to INBOX.md, and the builder writes to OUTBOX.md, because it may only change files inside `app/`.
**Changes.** Added `Other-Projects.md` and `OUTBOX.md`, plus links to them from README and INBOX. No code changes.
**Checks.** Build and `rebuild_check`.

---

## R17 — 2026-11-18

**Request.** Less tangled diagram and a per-item chart picker; then: beautify and organise the model, and offer other arrow types (like Powersim's).
**Decisions.** Arrow *route* and *tip* are separate choices (model default + per-arrow override). Right-angle routing is the circuit-diagram look; I did not copy Powersim's exact style, which I can't inspect. Tidy up reuses the importer's layered layout; blocks stack downwards so the picture is compact. Imported models with more than 25 arrows default to straight arrows.
**Changes.** `model.rs` (ArrowShape, ArrowHead, `items_of`, `by_name` accepts `Name[item]`), `theme.rs` (elbow path, tip styles), `canvas.rs`, `inspector.rs`, `app.rs` (View → Arrows, Arrange → Tidy up), `spreadsheet.rs` (tidy, compact blocks), `results.rs`/`chartstyle.rs` (item pickers).
**Checks.** `cargo test` (24 pass); screenshots of straight and right-angle arrows looked at. The item pickers and the arrow controls in the menus were not clicked through on screen.

---

## R16 — 2026-11-18

**Request.** Test the app with *Fuel Supply Planning Simulator.xlsx*: show it as stock-and-flow and causal loop diagram with all the details, allow grouping/abstraction, allow things with the same structure to be written once (arrays) in plain words, and make Excel import understand this workbook.
**Decisions.** (1) Sets are *expanded* into plain copies just before compiling (`sets.rs`), so the simulator, optimizer and charts are unchanged; a set variable's series is its total (or average). (2) The workbook is arbitrary Excel with 5,000-row formula sheets, so the importer does not guess: it reads the template, extended with Sets / Values / ItemSeries / Per, and a script (`inbox/restructure_fuel_workbook.py`) restructures this workbook into it, as the wiki's checklist describes. (3) Start the model at the as-of date from actual stock; day-of-month is the time axis. (4) Plain words: *set*, *one for each*, *Total/Average*.
**Changes.** `sets.rs` (new), `model.rs` (ItemSet, per-item values), `sim.rs` (expansion hook, roll-ups, CEILING), `expr.rs` (TOTAL/AVERAGE/CEILING, `X[item]`), `spreadsheet.rs` (sheets, Per/Combine, folder input, block layout and groups, export), `canvas.rs`/`inspector.rs` (badge, stack, selector, per-item list), `main.rs` (`--convert`), scenarios on one item.
**Checks.** Model reproduces the workbook's optimal trucks, target-day fill (optimal and current plan) and shortage for all 49 ready tanks (test). New tests: expansion/totals/mistakes, Excel round trip with sets. Screenshots of the diagram looked at; the causal loop diagram has 49 words, 66 arrows and a balancing loop (test).
**Open.** Diagram arrows are many for a model this size (use groups/hide); no per-item charts picker yet (type `Name[item]` in Customize chart); a set can have one parent only; the 8 not-ready tanks need data from the owner.

---

## R15 — 2026-11-18

**Request.** The full path to the project directory.
**Changes.** Added a *Full paths on this Mac* table to the wiki home page. No code changes.
**Checks.** Build and `rebuild_check`.

---

## R14 — 2026-11-18

**Request.** Build a wiki with full documentation and a feature document with checklist, logging and history, so another assistant (Claude Desktop / Claude Code in another app) can restructure a complicated Excel workbook and talk to the builder.

**Decisions.**
- The wiki lives in `app/wiki/` (Markdown, readable anywhere, editable by any tool).
- The contract between the two assistants is the **spreadsheet template** — one sheet per kind of model element — plus `INBOX.md` for messages.
- The app imports and exports that template, so a structured workbook becomes a model in one step and any model can be reviewed in Excel.

**Changes.** `spreadsheet.rs` (read xlsx/xls/ods/csv, model ⇄ workbook, xlsx writer, generated docs), File and Help menu items, import report window, wiki pages, `templates/`, `inbox/`.

**Checks.** `cargo test`: 18 passed, 1 picture test skipped by default (new: Excel round trip of the example gives identical results; loose CSV import). Imported the generated template in the app and looked at the result: 12 variables, 12 arrows, 3 scenarios, nothing skipped. `rebuild_check` passed.

**Open.** Waiting for the restructured workbook (see INBOX).

---

## R13 — 2026-11-18

**Request.** How to send an `.xlsx` with all the parameters of a system to be analysed and built as a model and causal loop diagram.
**Changes.** Added the spreadsheet-reading library. **Open.** The file hasn't been shared yet; the builder did not open personal files on its own.

## R12

**Request.** Customize charts (titles, axes, type…); legends should say whether lines are variables or scenarios.
**Changes.** `ChartStyle`, `chart_styled`, *Customize chart* window, legend labels and caption.
**Checks.** Screenshots of the Trends chart with title, subtitle and axis title; found and fixed a panel grow/shrink feedback loop.

## R10

**Request.** Run multiple scenarios and compare them in charts. **Changes.** Scenarios model, sheet, comparisons, colours. **Checks.** Test: Distancing and Both lower the peak.

## R9 · R8 · R7 · R6 · R5 · R4 · R3 · R2 · R1

See [CHANGELOG.md](CHANGELOG.md); detailed notes for these rounds are in the builder's handoff records.
