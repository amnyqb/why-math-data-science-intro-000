# Backlog

Ideas collected while building and testing with the fuel-supply model and while reading the Powersim rebuild. Order = the order of implementation. Status: ⬜ to do · ✅ done (see [FEATURES.md](FEATURES.md) for how each was checked) · ⏭ deliberately left.

| # | Item | Why | Status |
|---|---|---|---|
| B1 | **RK2 and RK3 integration** next to Euler and RK4 | Powersim offers them; cheap, and a way to check a model is not sensitive to the method | ✅ |
| B2 | **Powersim-style functions**: `SINWAVE`, `COSWAVE`, `GRAPH`, `GRAPHSTEP`, `DELAYINF`, `DELAYMTR` (with an order) | Widely used in models people bring over; rules are written down in the Powersim rebuild's MECHANICS.md | ✅ |
| B3 | **Autocomplete for your own functions** | They should feel like built-ins | ✅ |
| B4 | **Drag a lever circle** to change it | Controls should be where you look | ✅ |
| B5 | **Remember the layout** (panels shown or hidden, look, module style) between sessions | Hiding the results panel every launch is tedious | ✅ |
| B6 | **Legends name the selected tank/region** | With the Showing bar, a chart should say whose it is | ✅ |
| B7 | **Units warnings**: adding or subtracting values with different units | Most common modelling mistake; units were stored but never checked | ✅ |
| B8 | **Outline panel** (View → Outline): the model as a tree by module, click to jump, tick to plot | Massive models need a map; idea from the Powersim project tree | ✅ |
| B9 | **Sensitivity runs** (Monte Carlo): vary the key levers at random, show a band of outcomes | "How sure are we?" — missing in Dynamist and in the Powersim rebuild | ✅ |
| B10 | Causal loop diagram that follows the module levels | The CLD is still flat (49 words) | ⏭ needs a design: which loops belong to which level |
| B11 | Sets that belong to several sets (tank → region *and* fuel) | Totals by fuel as well as by region | ⏭ changes how `TOTAL(x)` picks its items; needs a design |
| B12 | Kept runs re-focus when the Showing bar changes | Pinned runs are snapshots of totals | ⏭ needs the per-item series stored with each kept run |

## Round 2 (after the review of system dynamics, 19 November 2026)

| # | Item | Status |
|---|---|---|
| B13 | Model checks and a conservation check inside the model | ✅ R20 |
| B14 | Compare with data: held-out scoring against last-value, trend, seasonal and smoothed-trend guesses | ✅ R21 |
| B15 | Core crate, operations API with a gate and audit log, `validate` | ✅ R22 |
| B16 | Units as algebra | ✅ R22 |
| B17 | `dyn` command line, MCP server, Python wrapper | ✅ R22 (the wrapper is a subprocess one, not PyO3) |
| B18 | Module definitions and instances with typed ports | ✅ R22 (engine and API; no screen yet) |
| B19 | Semantic model diff and run diff; meaning/layout files | ✅ R22 |
| B20 | XMILE reader (writer best-effort) with loss reports | ✅ R22d: a bridge for Stella files, not a target format; not opened in Stella |
| B21 | Vensim `.mdl` subscripts, macros, data variables and a writer with the sketch | ✅ R22b (not opened in Vensim) |
| B22 | Fit levers to data with intervals; a forecast as an input driving the model across its prediction interval | ⬜ (fitting is possible today through Optimize) |
| B23 | The desktop app edits its model only through the operations API | partly ✅ R23: *Run operations* and the History window use it; the diagram, inspector and other screens still edit the model directly |
| B24 | A screen for module instances ✅ R23 (no diagram box yet); spreadsheet adapter in its own crate ⬜; Optimize in the core crate ⬜ | partly |
| B25 | A compiled Python binding (PyO3); HTTP and WebAssembly | ⬜ |
| B26 | The Specs-check fixes: logged rejections, test kinds, manifest, strict units, Optimize in core, loss-report shape | ✅ R24 |
| B27 | Fill `location` in the adapters; a manifest screen; read `bindings`; the remaining Specs items in [Specs-Conformance](Specs-Conformance.md) | ⬜ |
| B28 | Benchmark B1 (beer game on M5) | ✅ R25 to R27: M5 runs, forecasts, R1 passes on cost (benchmark 204 exact), R2 agrees in sign, R3 in a narrower form; see `benchmarks/b1/RESULTS.md` |

## Round 3: matching Vensim and Stella exactly (benchmark B2, 20 November 2026)

Found by [benchmark B2](Import-Fidelity.md): each item is a reason a published model does not import, does not run, or gives different numbers from Vensim, Stella or PySD. The counts are model files in the test suites (and in the Vensim sample library where noted). Order = suggested order of work.

| # | Item | Models affected | Status |
|---|---|---|---|
| B29 | **XMILE/Stella arrays give wrong numbers.** Mostly the test files: 16 of the suite's XMILE files contain `???` where Vensim's exporter failed, and 8 use a macro that is not in the file. The reader used to turn those into silent zeros; it now refuses them. Real gaps fixed: arrays over two or more dimensions (were dropped), comma lists, a flow shared by two stocks, flows declared as `<aux>`, `{comment}` module inputs. | XMILE/Stella exact 84 (was 78). Left: `INT`/`MOD` follow the XMILE meaning (floor), which differs from the Vensim-made reference in 1 test; the hares-and-lynxes module example; `SIZE` and `LOOKUP` in XMILE; subranges in XMILE | partly ✅ R31 |
| B30 | **Legacy Stella files (`.stmx` from version 9/10) import as an empty model without any error.** At least report an error; better, read them. | 6 files (teacup, SIR, hares and lynxes, comparisons, eval order, lookups) | ⬜ |
| B31 | **Time settings that change during a run**: `TIME STEP` or `FINAL TIME` written as a formula (a variable step, a run that stops when a condition is met) | 5 (`control_vars`, `dynamic_final_time`; Vensim samples BALL, BALLMODEL, PROJ) | ⬜ |
| B32 | **Vensim functions not yet in the engine.** Done in R31, each checked against Vensim's output where a test exists: `NPV`, `DEPRECIATE STRAIGHTLINE`, `GAMMA LN`, `RANDOM EXPONENTIAL`, `LOOKUP FORWARD`/`BACKWARD`/`INVERT`/`EXTRAPOLATE`, `ALLOCATE AVAILABLE`, `ALLOCATE BY PRIORITY`, `VECTOR SORT ORDER`/`RANK`/`REORDER`/`ELM MAP`/`SELECT`, `INVERT MATRIX`, `FORECAST`, `TIME BASE`, `REINITIAL`, `DELAY INFORMATION`, `:NA:`. R32 added `LOOKUP SLOPE`/`AREA`, `NPVE`, `DERIVN`, `VECTOR LOOKUP`, the `GET DATA` family and the Excel date functions, built from the Vensim help and checked against the values it prints (unverified against Vensim itself). **Still missing**, by family: Vensim DSS discrete delays and queues (`DELAY CONVEYOR`, `DELAY BATCH`, `DELAY PROFILE`, `DELAY MATERIAL`, `QUEUE …`); market functions (`FIND MARKET PRICE`, `DEMAND AT PRICE`, `SUPPLY AT PRICE`, `ALLOC P`, `MARKETP`); solvers (`FIND ZERO`, `SIMULTANEOUS`); `GET TIME VALUE`; finance on vectors (`INTERNAL RATE OF RETURN`, `NET PRESENT VALUE`); `SHIFT IF TRUE`; `ALLOCATE AVAILABLE` profile types 0 and 5 (PySD does not have them either) | about 45 remain, almost all in single Vensim samples | partly ✅ R31 |
| B33 | **A delay order that changes over time**: `DELAY N` / `SMOOTH N` with an order such as `2 + STEP(1, 10)`. Vensim fixes the order at the start, so evaluate it at the initial time. | 3 (`delays`, `smooth`, `subscripted_smooth`) | ⬜ |
| B34 | **Subscript corner cases**: a numbered range inside a mixed list (`(bc1-bc5), subsubdim, bc17`); a numbered range as a subscript with a table (`Matrix_A[(da1-da4), db1, dc1] = …`); 3-D `:EXCEPT:` with subranges; `SUM` over one dimension of a 2-D variable (SDEverywhere `sum`); `ELMCOUNT` of a subrange; subranges in XMILE | about 8 | ⬜ |
| B35 | **Same name, slightly different result.** Fixed in R31: `DELAY FIXED`'s initial value (now evaluated once), `ZIDZ`/`XIDZ` (a divisor below 1e-6 counts as zero), `TREND` starting before its input. Left: `MODULO` for negative numbers (2), `GET TIME VALUE` (1) | 3 | partly ✅ R31 |
| B36 | **World3-03 differs from PySD by up to 0.17%** from the first step. To settle it: in Vensim PLE, run `wrld3-03` and read *Perceived Food Ratio* at 1900.5. 1.0427 means Dynamist is right; 1.0 means PySD is. Then fix whichever is wrong. | World3-03 (3 copies) | ⬜ needs one look in Vensim |
| B37 | **Syntax the reader does not take**: `+` inside an inline lookup, names with quotes and arithmetic characters, an odd number of quotes, `:NA:`, a `:` form inside `XIDZ` (Vensim sample) | about 5 | ⬜ |
| B38 | `A FUNCTION OF` (unfinished equations) | 17 Vensim samples | ⏭ Vensim itself does not simulate these |
| B39 | Random numbers equal to Vensim's | 5 Vensim samples | ⏭ needs Vensim's generator; the distribution is the same, the draws differ |
| B40 | `GET XLS …`, `GET VDF …`, data files | several | ⏭ data is kept as a placeholder and reported; reading the files is its own feature |
| B41 | Benchmark B1 leftovers: the DQN's gap to base-stock is 45% against the paper's 8% (training was 10,000 games, not 60,100), and R3 against the organisers' M5 code needs `sell_prices.csv` and the right repository address | B1 | ⬜ |

## Round 4: fewer, simpler, adaptive functions (decided 20 November 2026)

Direction: modelers should do expert things at once, without learning an atlas of formulas. One function per family, options by name with sensible defaults, Python style. Vensim's and Stella's names stay accepted on import. Every verb is proven to give exactly the numbers of the function it replaces, and the book explains each through a real case.

| # | Item | Status |
|---|---|---|
| B42 | **Verbs with named options.** `DELAY(x, time, order=, init=, kind=, exact=)` ✅ R33 (rewritten to the engine's functions on reading and on export; equal to `DELAY1/3`, `DELAYMTR`, `SMOOTH*`, `DELAYINF`, `DELAY FIXED` to the last digit). `ALLOCATE(requests, supply, by=, overlap=, curve=, spread=)` ✅ R34 (strict priority, proportional share and the four priority curves, each equal to Vensim's `ALLOCATE BY PRIORITY` / `ALLOCATE AVAILABLE` to the last digit; exports `ALLOCATE BY PRIORITY`). `VALUE(stream, rate, init=, factor=, timing=)` ✅ R35 (equal to `NPV` and `NPVE`). `QUEUE(arrivals, served, init=)` and `WAIT(queue, measure=, …)` ✅ R36 (first in, first out; ages from cohorts; checked against a hand-worked case, not against Vensim). `AGE(stock, every=, entrants=, open_ended=)` ✅ R37 (whole classes move each period; checked against a hand-worked case; exports as a formula). `EXPECT(x, time, ahead=, init=, measure=)` ✅ R38, `LOOK(curve, x, mode=)` ✅ R38, `VALUE(…, measure=irr \| writeoff=)` ✅ R38, `DELAY(…, exact=True, leak=)` ✅ R38. Next: `DELAY` with `batch=` and `profile=`, `SOLVE` (forecast, trend, smooth), `LOOK` (lookup modes), `QUEUE`, `AGE`, `SOLVE` | partly ✅ |
| B43 | `DELAY(…, leak=, batch=, profile=)`: conveyor with leakage, batches, and a delivery profile, written from the Vensim help and checked against its documented behaviour | ⬜ |
| B44 | **Say what a call means, in words**, when the modeler hovers over it ("pipeline delay: material arrives exactly 6 weeks after it leaves, nothing is lost") | ⬜ |
| B45 | **Autocomplete and help for options** (type `DELAY(x, 6, ` and see order, init, kind, exact) | ⬜ |
| B46 | **The book**: outline, cases and screenshot notes in [`book/Book-Outline.md`](../book/Book-Outline.md); three runnable examples written (`book/examples/`): shipment pipeline, water shortage, investment. A test runs the three examples and checks the numbers the text quotes. To do: take the screenshots; write the chapters | partly ✅ |
| B47 | **`SOLVE`** (`FIND ZERO`, `SIMULTANEOUS`): solving equations that depend on each other inside a time step. Needs the engine to re-evaluate part of the model several times per step, which it does not do (it evaluates each variable once, in order). A design is needed before building: where the solver lives, how the loop is cut, how convergence is reported | ⬜ |
| B48 | `DELAY(…, batch=, profile=)` (`DELAY BATCH`, `DELAY PROFILE`) from the Vensim help; `QUEUE` attributes; `SHIFT IF TRUE` import | ⬜ |

## Round 5: finding what matters (benchmark B3, brief of 20 November 2026)

Brief: [`benchmarks/b3/BRIEF.md`](../benchmarks/b3/BRIEF.md) (its own name for it is "B2"; B2 is taken here). Readiness audit, blockers and order of work: [`benchmarks/b3/PLAN.md`](../benchmarks/b3/PLAN.md).

| # | Item | Status |
|---|---|---|
| B49 | Goal declaration in the manifest; answer-key format and checker; runner skeleton with a frozen configuration | ✅ (goals not yet shown in the app) |
| B50 | **F1 `screen`** (Morris elementary effects, classification, convergence) | ✅ |
| B51 | Loop dominance (Loops That Matter) and eigenvalue elasticity | ✅ for SMOOTH and DELAY1/3, Euler; open: DELAY FIXED, TREND, queues, strongest-path search for large models (**B56**) |
| B52 | **F2 `importance`** (Sobol indices, dominance over an ensemble, dependency pruning) | ✅ |
| B53 | **F3 `reduce`** and **F4 `verify_reduction`** | 🔶 prune, fix, merge, verification done; delay aggregation and loop removal open (**B57**) |
| B54 | **F5 `recommend_structures`** and the structure library | 🔶 five structures; automatic detection of the decision point and table reshaping open (**B58**) |
| B55 | The six cases: models, answer keys from the papers, base-run replication | 🔶 B2-1 and B2-2 done and scored; B2-3 run without a key; B2-4, B2-5, B2-6 need papers or models (see `benchmarks/b3/STATE.md`, Blocked) |
| B56 | Strongest-path loop search and loop scores through every stateful function | ⬜ World3-03 exhausts the 20 000-loop cap |
| B57 | `reduce`: aggregate chains of identical delays, remove loops that never matter, record the validity domain of a reduction | ⬜ |
| B58 | `recommend_structures`: detect the decision flow automatically, add table reshaping, scale-free improvement measure | ⬜ |
| B59 | Goal transforms (logarithm) for outcomes with an exponential tail; show goals in the app | ⬜ Market Growth scenario F |

## Round 6: book figures (request of the book team, 20 November 2026)

Request: [`book/FIGURE-LIBRARY-REQUEST.md`](../book/FIGURE-LIBRARY-REQUEST.md) (K18 to K23 of the book plan). Manual: [Figure library](Figure-Library.md).

| # | Item | Status |
|---|---|---|
| B60 | Headless figures: `render`, `chart`, `layout`, `cld`, `set`, `style`; style file; examples chapters 10 to 15; tests | ✅ R41 |
| B61 | **Lint** for figures: arrow without a sign, loop without a name, variable named with a verb, overlapping labels | ⬜ |
| B62 | **Hierarchical causal loop diagram** by module level (one word per module, then open a module) | ⬜ |
| B63 | **Direct PDF output** (today: SVG, then `rsvg-convert`) | ⬜ |
| B64 | Options for the pictures: hide the values on the cards (or pick the time they show), draw module and entities sheets, set the pixel density of the layout, text width of a different font measured instead of egui's | ⬜ |

## Round 7: a ribbon and Office-like editing (decided 20 November 2026)

Direction: the app should have as many buttons within reach as Vensim, Stella and Powersim, laid out like the Office ribbon.

| # | Item | Status |
|---|---|---|
| B65 | **Ribbon, step 1:** tabs Home, Insert, Arrange, Analyze, View; groups of buttons wired to what exists | ✅ R42 |
| B66 | **Ribbon, step 2: text formatting** on notes: size, bold, italic, underline, fixed-width, colours, alignment, in a contextual Text tab; saved as `fmt`; drawn in the window and in every export | ✅ R43 (bold is a double draw; no choice of font family beyond fixed-width; variable labels and chart text not formattable; no `dyn set` options; no copy format) |
| B67 | **Ribbon, step 3: shapes and pictures** ✅ R44 (rectangle, rounded box, ellipse, line, arrow, PNG/JPEG pictures; contextual Shape tab). Diagram controls ✅ R45 (slider, knob, switch, number input; contextual Control tab). Still to do: shadow variable, Delay and Sector frame tools; text inside shapes; bring forward and send back; drag a picture file onto the window | 🔶 |
| B68 | **Pause, Step, Reset and a speed control** for runs; needs the engine to advance in increments | ⬜ |
| B69 | Ribbon in a narrow window (wrap or collapse groups); keyboard access to tabs; remove the left tool strips once the ribbon is proven | ⬜ |
| B70 | A "What matters" group on Analyze for the B3 analysis (screening, loops over time, reduce, verify) | ⬜ |
| B71 | **File → Open example ▸** and an Open example dialog for the model library (`library/index.json`: title, level, concepts, prerequisites, question); examples open as unsaved copies; files embedded with `include_str!`. Design in [Model library](Model-Library.md) | ⬜ |
| B72 | **Lesson side panel**: the tutorial page's steps (question, what to click, the check) beside the model, with Next lesson | ⬜ |

## Round 8: the sd-ai benchmark, why results changed, the audit pack, the blind hindcast (decided 23 November 2026)

Prompt: [inbox/round-8-prompt.md](inbox/round-8-prompt.md). Branch `round-8`. Wording as in [FEATURES.md](FEATURES.md), "Proposed on 23 November 2026".

| # | Item | Status |
|---|---|---|
| B73 | **Dynamist tools inside sd-ai's agent**: `dynamist_check` and `dynamist_extreme_tests` behind the experiment flag `dynamistTools`, found through `DYN_BIN` | 🔶 R47: tools, flag and tests built in a sd-ai fork branch; see `benchmarks/sdai/REPORT.md` |
| B74 | **With-and-without protocol**, development groups only, `scripts/sdai-compare.py`, `benchmarks/sdai/REPORT.md`, tag `b74-freeze` | 🔶 R47: dev runs made, formulationCritique re-graded with DeepSeek as the judge; held-out and Opus not run |
| B75 | **`dyn attribute OLD NEW`**: Shapley split of why every goal changed, waterfall chart | ✅ R47 (T) |
| B76 | **`dyn pack FILE --out DIR`**: audit pack with model card | 🔶 R47 (T): everything except `reporting-checklist.md`, whose item list is not in the repository yet (`papers/TO_FETCH.md`) |
| B77 | **`dyn hindcast FILE --data CSV --cut DATE [--fit]`**: blind hindcast with coverage | ✅ R47 (T) |
