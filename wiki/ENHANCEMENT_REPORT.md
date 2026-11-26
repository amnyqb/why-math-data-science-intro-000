# Enhancement programme: final report

Branch `enhancement` (worktree `enhance-wt`), started from the tip of `layers-ui` (commit ef8fb60, the Dynamist rename). Nothing was merged or pushed. Commits, one per phase section group:

| Commit | Content |
|---|---|
| 7e02e6c | Phase 0, and Phase 1 sections 0 to 4 (units, doctor base, calendar) |
| 580f00b | Phase 3 section 2: scalar finance, set aggregations |
| 01704d4 | Phase 3 sections 5 and 6: calendar functions, logic, aliases |
| 2603211 | Phase 3 section 3: running finance |
| 25cce70 | Phase 3 section 4: random distributions, seeds, cdf and inverse, PREVIOUS, HISTORY |
| 53e6588 | Phase 3 sections 7 and 8: benchmarks, the solar model, `dyn finance` |
| 3454eb8 | Phase 2: the doctor in stages |

The order of the commits is not the order of the phases: phase 3 pieces that did not depend on the doctor were built before phase 2, because the phase 2 rule catalogue was missing (see "Not done").

## Inputs, and what came late

The Doctor specification (the catalogue of 72 rules) and the reference table `golden.json` arrived after the first pass and were used for the second one: `wiki/specs/Dynamist_Model_Doctor_Research_and_Rules.md` is the authority for Phase 2, `core/tests/data/golden.json` holds the supplied reference values (`golden_own.json` is my earlier independent set). The Units/Time and Function Library specifications were never supplied; the phase prompts were self-contained and were followed. `units_trap.mdl` is the text printed in the prompt.

NREL levelised cost: with the combined tax rate T = 0.21 + 0.06 x (1 - 0.21) = 0.2574 the inputs give 41.4332 (test `levelised_nrel`, tolerance 1e-4); with the rounded T = 0.257 they give 41.4407, which the test also pins. There is no mismatch.

The book source came from `~/Downloads/System_Dynamics_Dynamist_Source.zip` (215 models), copied to `core/tests/book_models/`.

## Phase 0: three fixes

1. **Never-negative stocks conserve material.** Outflows are scaled down together (proportional sharing; the model has no priority setting) so a stock cannot be drained below zero within a step; the clamp is gone. The run carries a `constraints` log (one entry at the start of each spell). New check kind `conservation`. Tests `nonneg_conserves`, `conservation_check`.
2. **Loop polarity from the numbers.** `dyn loops` takes each link's sign from the partial derivative along the base run (`sim::link_signs`), and labels a loop reinforcing, balancing or `switches` with the times it changes. Test: the ch02 growth model's market-limit loop is balancing (`core/tests/loop_polarity.rs`).
3. **Units with brackets.** `kWh/(kW*Year)` parses; the new exact parser reads brackets anywhere.

## Phase 1: units, calendar, doctor base

* **Units engine:** `Unit { dims, scale, offset }` in `units.rs`, the atom table with sources in `unit_table.rs`, `parse_unit_in` with a position and a suggestion on error (`kwhh` suggests `kWh`), ambiguous symbols with a stored choice (`settings.unit_choices`), money with a price year (`USD_2024`, error `money-real-nominal`), and `units_scale_values` (new models on; imports and old files off). Value scale applies to `%` and `ppm` only (see the DEVLOG for why not to every unit).
* **Doctor base (`dyn check`):** ranked exact unit fixes (a: conversion constant in a Conversions group, b: factor from a variable of the model, c: declaration change), `units-inferred`, `magic-number`, `zero-division`, `dt-too-large`, `time-equality`, `dt-sensitivity`, `lookup-outside`, `ratio-outside`, `equilibrium`, `benchmark` (as a check with a source), `--fix`, `--fix-all-safe`, `--lenient-time`. The trap model gives exactly the three findings with the exact fixes (factor 0.001 for barrels to MB, 24,000 for MW to kWh per day, 0.05 per Day read for `%/Day`), and after the first fixes it passes the strict check and the tank stays positive.
* **Calendar:** `calendar.rs` (Hinnant's day algorithms, YEAR FRACTION with the five bases and the US 30/360 February rules, Excel serials with the 1900 quirk, ISO weeks, resampling and gap/duplicate detection), the Calendar convention in the integrator (10 per Day over January and February 2027 gives 310 and 280), `DAYS IN STEP()`, `HOURS IN STEP()`. `dyn run` reports the convention.
* `dyn check` ran on all 215 book models without crashing; the report is `build/model_health.json`.

## Phase 2: the doctor in stages

All 72 rules of the catalogue (S1-4, G1-12, U1-12, F1-15, N1-8, I1-3, X1-6, C1-5, R1-4, P1-3) are in the rule table (`core/src/doctor/`, one `Rule` per id, `doctor::rules()`), with stages C1 to C7, `--stage`, `--gate`, `--experiments`, `--jobs`, intervals (`interval.rs`), parallel experiments (`experiments.rs`), Newton equilibrium solve, and the gate of Part C7 (Draft: a confirmed error; Review: none; Publish: Review, the experiments run, and X1-X5, N3, R1-R4, P3 clear). A "can" finding of the interval analysis that no run has confirmed counts as a warning for the gate. The calibration rules read a calibration record (`Model::calibration`, operation `set_calibration`), since the repository has no calibration module. U1-U9 follow the list in the Doctor specification; the Units specification itself was not available, so their detectors are the ones of phase 1 (the mapping is `doctor::rule_of`). U9 is checked on lookup tables named as data in a model with a start date.

Tests: `core/tests/doctor_corpus.rs` (72 broken models in `core/tests/doctor/<ID>.json`, one per rule, the table and the corpus must list the same ids; the doctor reports the rule, the first fix removes it and adds no finding of equal or higher severity), `book_models_pass_review` with `allow.toml`, the speed test (C1 to C3 under 200 ms on 500 variables, release), `doctor_catalogue` (the generated Model-Doctor page).

Severities and fixes follow the table in the specification. Two points where the specification and itself pull apart: the F1/F2 fix (a minimum time of 2 DT) is flagged by N2 and N3 afterwards, which the corpus test allows for those two rules; and N5 (time constant below DT, "error") fires on two book models that run fine under Euler, which are in the allow-list.

### The app: Model health panel

Built by a delegated session on branch `enhancement-ui` and merged here: `src/health_ui.rs`, wiki page `Model-Health.md`, screenshots `wiki/img/health-panel-units-trap.png` and `-dark.png`. Findings grouped by family with severity badges ("can" findings marked unconfirmed), fix buttons with a preview of each edit and a "keeps the numbers" tag, source citation, click-to-select; stages C1 to C3 rerun 300 ms after the last edit; C4 and C5 run on demand in a background thread; the gate badge sits in the status bar. On the units trap it shows Draft with the two scale errors (Delivery, Power), the exact fix "add constant MB per barrel = 0.001 ... set equation of Tank net flow ..." and the percent note.

### Timings of each stage on the largest book model

`ch11-plant` (3 stocks, DT 0.0625, a fixed delay whose history makes each run quadratic), release build: C1 and C2 under 1 ms; C3 about 1.7 s; C4 about 16 s (structure 1.6 s, zero-division probe 1.6 s, eigenvalues 3.1 s, DT sensitivity 6.5 s). Over the whole book sweep on the server the DT sensitivity (94 s of 239 s) and the eigenvalue pass (46 s) dominate. The generated 500-variable model runs C1 to C3 in under 200 ms.

## Phase 3: function library, release 1

Everything of sections 2 to 8 except what is listed under "Not done".

* **Scalar finance** (Excel's conventions, golden values to 1e-9): REAL RATE, NOMINAL RATE, EFFECTIVE RATE, WACC, ANNUITY FACTOR, PMT, FV, PV, NPER, IPMT, PPMT, RATE (Newton then bisection), DSCR.
* **Running finance:** VALUE with `timing=period_start` and `mid_period`, LEVELISED (alias LCOE), ESCALATE, PAYBACK, LOAN (annuity, linear, bullet; grace; balance, payment, interest, principal), TAX (loss carry-forward, expiry).
* **Statistics:** 13 `RANDOM ...` functions, the short forms and the Stella names; model seed and one stream per call (adding a variable never changes another's draws: test `seed_stability`), `dyn run --seed`, `dyn compare --same-draws`; NORMAL and LOGNORMAL CDF and INV (erf by series and continued fraction, Acklam with Halley steps); PREVIOUS, HISTORY, ROUND WITH CARRY. 200,000-draw mean tests for 19 samplers.
* **Calendar and seasonality:** DATE, YEAR, MONTH, DAY OF MONTH, DAY OF YEAR, HOUR OF DAY, WEEK NUMBER, DAY OF WEEK, IS WEEKEND, IS WORKDAY, WORKDAYS BETWEEN, PROFILE, PROFILE STEP, SEASON, DAYLIGHT, AT TIME, EVERY, settings `start_date`, `weekend`, `holidays`.
* **Logic and aliases:** AND, OR, NOT, BETWEEN, the alias table (guarded by argument count so a lookup called `Effect` still works), MAX OF, MIN OF, COUNT OF.
* **Benchmarks and tools:** checks with `at` and `source`; `dyn finance`; the solar model `models/solar/solar_lcoe.sdm` (8 inputs, 3 equations, passes `dyn check --gate publish`; default inputs give 31.19, its Fraunhofer inputs 41.10 inside the benchmark 40.5 to 41.5; reports in `build/doctor/`).

## Tests added

Core unit tests: finance 4, stats 3, calendar 8, polarity 2, exact units 5, bracket parsing 1, checks 2. Integration tests (`core/tests/`): `loop_polarity`, `units_trap` (4), `doctor_base` (4), `calendar_run` (3), `calendar_fn` (4), `finance_fn`, `finance_run` (8), `stats_fn` (4), `solar_model` (2), `doctor_corpus`, `doctor_review` (2), `doctor_catalogue`, `book_models_check`. All pass on the Contabo server in the whole-workspace run, except one pre-existing problem: `architecture::tests::the_sheet_draws_and_lanes_fold` aborts inside egui's texture code on the server (it also aborts when run alone and does not touch anything changed here); the run skips it.

## Book models whose numbers changed

None. No book model has a never-negative stock, so the outflow constraint changes nothing there; imported models keep `units_scale_values = false`; the byte-identical figure test (`book_examples_rebuild_to_the_saved_figures`) passes. The `.sdm` files do not gain new fields (the new settings are written only when they differ from the default).

## Allow-list of accepted doctor findings

`core/tests/book_models/allow.toml`: at the handover it had 57 entries (not 58): 36 unit entries (10 `unit-declared`, 25 `unit-dimension-mismatch`, 1 `unit-scale-mismatch`), 14 `F12`, 4 `G2`, 2 `N5` and one failed service-level check. The branch `doctor-fixes` removed the 14 `F12` entries (a loop with a zero-gain link is now "inactive"), the 4 `G2` entries (a start value that reads a flow is Info) and 4 of the unit entries (TREND is per time), so it had 35 entries; the second round marked the ch25 overview as a figure model (`figure_model`), which took out its 14 unit entries, so it now has 21: 4 `unit-declared`, 13 `unit-dimension-mismatch`, 1 `unit-scale-mismatch`, 2 `N5` and one failed check. Their reasons are generic ("kept as the book prints it"); **I did not review them one by one**, so each is a finding about the book model that a person should look at, not a finding that has been judged harmless. Warnings (magic numbers, DT sensitivity) do not fail the review gate and are not listed.

## Not done, and why

* The Units/Time and Function Library specifications were not available; the detectors of U1 to U9 and the function library follow the phase prompts only. Functions in `golden.json` that belong to later releases (SLN, SYD, DDB, DB, XNPV, XIRR, MIRR, WEIBULL CDF, POISSON PMF, BETA INV, GAMMA INV) are not built; the table's entries for the functions of release 1 are tested (`golden_json_reference_values`).
* `CONVERT(x, "from", "to")` (fix rank d of the units prompt) and the Morris screening above 200 cases.
* Detectors of several rules are heuristics from the one-line description in the catalogue (F5, F9, F10, F13, G4, G7, G10, U10, U11, X4, X5, I3, R4, C1): they are tested on one broken model each and give warnings on some book models that a person should judge.
* The allow-list reasons are generic and were not reviewed one by one.
* GET DATA with timestamps is not wired to the resampling functions (`calendar::resample`, `calendar::data_issues` exist and are tested).
* RANDOM LOOKUP; the argument order of the `RANDOM ...` family is as recalled from Vensim and could not be checked.
* The one test `architecture::tests::the_sheet_draws_and_lanes_fold` aborts inside egui on the server (also alone); it is skipped in the runs.

## Efficiency of the doctor sweep (measured on the server, 12 cores, release)

| | Before | After |
|---|---|---|
| Whole 215-model doctor sweep (`book_models_check`) | 239 s, one core | 16 to 19 s wall, 110 s of CPU; peak 293 MB |
| `doctor_review` (sweep plus allow-list) | about 240 s | 20 s wall, 314 MB |
| C1 to C3 on the slowest book model | 2.1 s (one of 12 parallel runs) | 0.44 s under the same load; 0.05 s alone on the largest model (168 variables) |
| `ch11-plant` review gate | 16 s | 10 s |

What was slow and what was done: (1) every stage compiled and ran the model again (validate, the base run, the zero-division probe, the lookup probe): now one base run is shared and one probe run answers both questions; (2) the internal-rate-of-return function bisected 200 times over the whole history at every step: now it stops when the interval is as narrow as the numbers allow and uses one logarithm per rate; (3) every rule parsed every equation, and every unit string was parsed again for every variable: parsed equations and units are cached per thread; (4) C1 to C3 no longer start the base run; (5) X1 skips constants that cannot reach a stock, and above 200 constants only the 20 that move the stocks most in a one-at-a-time screening get the extreme cases; (6) the sweep tests run models on all cores with `std::thread::scope` (no new dependencies); the experiments use `--jobs`, default `min(4, cores)`. What stays slow: the DT-halving rerun and the eigenvalue passes of a model whose own run is slow (`ch11-plant`: its internal-rate-of-return and NPV functions rescan the history).
