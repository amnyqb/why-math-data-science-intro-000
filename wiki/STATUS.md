# Status of the enhancement programme

Written for the book handover. Measured on the Contabo server (12 cores), not on the laptop. Status words: `done`, `partial`, `not started`, `blocked`.

## 1.1 Build identity

| Item | Value |
|---|---|
| Git commit and date | tested at `ba489fd` of branch `enhancement` (2026-11-22); the handover branch `handover` adds this file, the functions generator, the equivalents on the help pages and the screenshot test |
| Repository | github.com/amnyqb/dynamist (private; clean clone at `vensimple/dynamist-clean`) |
| Branch | `enhancement` (merged with `layers-ui`, the rename, and `enhancement-ui`, the Model health panel); not merged into `master`; handover work on `handover` |
| `dyn` first line of its help | "`dyn`: the Dynamist model tool for scripts, CI and agents. Every command prints JSON." (there is no version number in the output) |
| Rust toolchain | rustc 1.100.0 (0ef1bc386 2026-11-09) |
| Platforms built | Linux x86_64 (server, release). macOS arm64: not built, needs the user's go-ahead (nothing heavy runs on the laptop). Windows: not built |

## 1.2 Per section

Commits are those of the clean repository. "Tests added" counts test functions; one of them (`every_rule_has_its_broken_model`) runs 72 models.

| Section | Title | Status | Tests added | Passing | Commit | Notes |
|---|---|---|---|---|---|---|
| P0.1 | Non-negative stocks conserve material | done | 2 | 2 | f949d42 | Proportional sharing only (the model has no priority setting). A constraint is logged once per spell. `nonneg_conserves`, `conservation_check` |
| P0.2 | Loop polarity from partial derivatives | done | 3 | 3 | f949d42 | `dyn loops` labels loops reinforcing, balancing or `switches` (with times). ch02 market-limit loop: balancing |
| P0.3 | Units with brackets | done | 1 | 1 | f949d42 | `kWh/(kW*Year)` |
| P1 §0 | The units trap | done | 4 | 4 | f949d42 | `core/tests/units_trap.rs`; the model text is the one printed in the prompt |
| P1 §1 | Exact units core | done | 6 | 6 | f949d42 | `Unit {dims, scale, offset}`; table `unit_table.rs`; value scale applies to % and ppm only (see 1.4) |
| P1 §2 | The doctor, base | partial | 4 | 4 | f949d42 | Unit fixes ranks a to c; rank d `CONVERT()` not built. Other base checks built. `dyn check` flags as asked |
| P1 §3 | Calendar time | partial | 11 | 11 | f949d42 | Everything but connecting dated data to GET DATA: `calendar::resample` and `data_issues` exist and are tested, not wired |
| P2 §0 | Structure | done | 1 | 1 | f949d42, 8e3594b | `core/src/doctor/`; rule table; files `rules_s,g,u,f,n,i,x,c,r,p`, `rules_base`, `interval`, `experiments`, `fixes` |
| P2 §1 | Stages C1 to C7 | partial | 3 | 3 | 8e3594b, b4f0567 | All stages. C5 does not use Morris screening: above 200 constants a one-at-a-time screening picks the top 20. C6 reads a calibration record (`Model::calibration`) because the repository has no calibration module |
| P2 §2 | Fixes | partial | (in the corpus) | | b4f0567 | A fix builder per rule that has a fix. The "rerun the affected stages and roll back if a new finding appears" step is enforced by the corpus test, not by `apply_fix` at run time |
| P2 §3 | Test corpus | done | 5 | 5 | b4f0567 | 72 broken models as operation batches (`core/tests/doctor/<ID>.json`, not `.mdl`: some rules need settings, never-negative stocks or sheets that `.mdl` cannot carry); book review with `allow.toml`; speed test |
| P2 §4 | App and docs | done | 5 | 5 | 3b075d0, 194d462 | Model health panel, gate badge in the status bar, C1 to C3 debounced 300 ms, C4 and C5 in the background; `Model-Doctor.md` generated from the rule table |
| P3 §1 | Two fixes first | done | | | f949d42 | Done in phase 0 and phase 1 as the programme says |
| P3 §2 | Scalar finance | done | 6 | 6 | a71c151 | |
| P3 §3 | Running finance | done | 8 | 8 | 66cdde4 | `timing=period_start`, `mid_period`; LEVELISED, LOAN, TAX, PAYBACK, ESCALATE |
| P3 §4 | Statistics and seeds | partial | 7 | 7 | 5a64fe3 | `RANDOM LOOKUP` not built; argument order of the `RANDOM ...` family not checked against Vensim |
| P3 §5 | Calendar and seasonality | done | 4 | 4 | 2534b30 | |
| P3 §6 | Logic, whole units, aliases | done | 2 | 2 | 2534b30 | |
| P3 §7 | Benchmarks, help, `dyn finance` | done | 2 | 2 | ec6347f | Help pages of the new functions with an "In Excel, Vensim and Stella" block (handover branch) |
| P3 §8 | Acceptance | done | 2 | 2 | ec6347f, a4e480a | Solar model: 8 inputs, 3 equations, 31.19 / 41.10 / 41.4332, gate Publish |

## 1.3 Test totals

`cargo test --release --workspace --no-fail-fast` on the server, summed over all test binaries:

* before the programme (`f05ccd8`): 247 passed, 0 failed, 4 ignored;
* now (`ba489fd`): 313 passed, 0 failed, 4 ignored.

| Suite | Total | Passing | Failing | Ignored |
|---|---|---|---|---|
| Existing tests before the programme | 247 | 247 | none | 4 (`render_venture_png`, `picture_of_autocomplete`, `fuel_sensitivity_timing`, `corpus_xmile`: they draw pictures or time runs and are run by hand) |
| Golden values (`golden.json`) | 31 | 16 | none | the other 15 are values of functions that were not built (see 1.4): IRR, NPV excel-style, MIRR, SLN, SYD, DDB, DB, XNPV, XIRR, TRIANGULAR mean, WEIBULL CDF, POISSON PMF, BETA INV, GAMMA INV |
| Doctor rule tests (one per rule) | 72 | 72 | none | |
| Book models false-alarm test | 2 (`book_models_pass_review`, `the_doctor_runs_on_every_book_model`) | 2 | none | |
| Speed test (C1 to C3, 500 variables) | 1 | 1 | none | |

(The 16 golden values that pass: PMT, IPMT, PPMT, FV, PV, NPER, RATE, CRF, REAL RATE, EFFECTIVE RATE, WACC, NORMAL INV, NORMAL CDF, LOGNORMAL INV, and the two levelised costs.)

## 1.4 Deviations

| Prompt said | What was done | Why |
|---|---|---|
| `Dim` replaced by a unit with scale | `Dim` kept (older name-based checks used by `dyn validate`); exact `Unit` and `parse_unit_in` added beside it | existing tests assert the older lenient rule |
| Remove the rule that ignores time when a rate meets its amount | `dyn check` is strict, `--lenient-time` restores the old rule; `dyn validate` keeps its lenient default | existing tests |
| Declared units carry value scale | applies to `%` and `ppm` only | scaling every unit to SI would turn a stock in MB into cubic metres |
| Imported models default `units_scale_values = false` | as asked; files without the key read as false; new models true | |
| `doctor.rs` | `core/src/doctor/` directory | Phase 2 asks for it |
| Test models `S1.mdl`... | `core/tests/doctor/<ID>.json` operation batches (with `raw` entries for what `add_var` refuses) | some rules need settings or sheets |
| Rule ids F1-14, I1-2, R1-3 (master prompt) | the Doctor specification's ids: F1-15, I1-3, R1-4, X1-6, P1-3 | the specification is the authority |
| Phase 1 codes (`unit-scale-mismatch`...) | kept as the `code`; the catalogue id is in `rule` (`doctor::rule_of`: U1 dimension, U2 scale, U3 percent, U4 flow, U5 money, U6 ambiguous, U7 unreadable, U8 inferred) | the Units specification was not available; the order follows the Doctor specification's list |
| Gate grades | Draft: a confirmed error; Review: none; Publish: Review, the experiments run, X1-X5, N3, R1-R4, P3 clear (Part C7). A "can" finding of the interval analysis that no run confirmed counts as a warning. `--publish` (a flag) turns warnings into errors; `--gate publish` does not | Part C7 |
| `dyn check` exit code | non-zero on a confirmed error, and with `--gate publish` when the gate is below Publish | so a book build can stop |
| X1 above 200 cases: Morris screening | one-at-a-time screening (each constant +10%), top 20 get the extreme cases; constants that cannot reach a stock are skipped | simplicity; speed |
| Calibration module (C-rules) | a calibration record on the model and the operation `set_calibration` | no calibration module exists |
| N5 "error" | N5 is an error as the specification says; two book models are in the allow-list | |
| F1/F2 fix constant DT x 2 | as written; N2 and N3 flag the result afterwards (corpus test allows it) | the specification's own numbers |
| `SMOOTH`/`DELAY` rates | a rate declared per Year in a monthly model has a period of 12 steps; undeclared rates are per model time unit | |
| `RANDOM ...` argument order | min, max, then the distribution's parameters (as `RANDOM NORMAL`); short forms and Stella names take parameters only; `seed=` reads as the last positional argument | Vensim's pages could not be opened |
| Alias table | an alias applies only when the target takes that many arguments, so a lookup called `Effect` still works | |
| `golden.json`: IRR, NPV, MIRR, SLN, SYD, DDB, DB, XNPV, XIRR, TRIANGULAR mean, WEIBULL CDF, POISSON PMF, BETA INV, GAMMA INV | not built (not in the phase 3 release 1 prompt) | |
| Test of Excel serial dates "1 lower" | `EXCELDATESERIAL` returns Excel's own value (one below days since 1899-12-30) for 1900-01-01 to 1900-02-28 | |
| `dyn compare A B --same-draws` | `dyn compare` is `dyn diff --runs`; `--same-draws` gives both models the first one's seed | no separate command existed |
| `--help` per command | `dyn` has one usage text; `dyn COMMAND --help` prints it | `cli_help.txt` contains it and the lines of the commands asked for |
| 219 book models | the source holds 215 model files | the attachment was not available |
| `ch18/ch18-venture-layers.mdl`, `solar_lcoe.mdl` | not in the source; the chapter 18 publish gate was run on `ch18-stage2.flowmodel`, the solar model is `models/solar/solar_lcoe.sdm` (exported as `solar_lcoe_v2.mdl`) | |

## 1.5 Known issues

* **Slow model.** `ch11-plant` takes about 10 s for `dyn check` (the doctor reruns the model for DT halving, eigenvalues and probes). The whole 215-model sweep: 239 s on one core before, 16 to 19 s on 12 cores now.
* **Rules that are heuristics.** F5, F9, F10, F13, G4, G7, G10, U10, U11, X4, X5, I3, R4, C1 follow the one-line description of the catalogue and were tested on one broken model each; they give warnings on book models that a person must judge (`doctor_book_summary.md`).
* **Doctor fixes rewrite equations from the parsed tree.** A fix that rewrites a whole equation (F3, G8, F11) prints verbs in their engine form (`LEVELISED STATE(...)`); G8 refuses such equations, F3 and F11 may not. Reproduction: `dyn check` on a model with `x = LEVELISED(...) / y` and take F3's fix.
* **Export classification in `functions.json`.** The Vensim exporter treats every function without a special case as a raw formula, so `export_vensim` says "raw formula" for functions Vensim supports natively.
* **Stateful finance functions update once per step** (exact for Euler; Runge-Kutta evaluates them at stage times).
* **Calendar convention needs a step of one time unit.**
* **Interval analysis is loose** (5 passes): F3, F8 and F14 "can" findings are unconfirmed unless a run reproduces them.
* **The headless font has no check mark**, so the gate badge screenshot is cropped.

## 1.6 Round 8 (branch `round-8`, 23 November 2026)

| Item | Status | Notes |
|---|---|---|
| B75 `dyn attribute` | done | Shapley split, exact to 10 groups, sampled above; tests T |
| B76 `dyn pack` | partial | everything but `reporting-checklist.md` (paper items could not be fetched: `papers/TO_FETCH.md`) |
| B77 `dyn hindcast` | done | the bands hold lever uncertainty only |
| B73 sd-ai tools | done in a fork branch | flag, tools and tests; the tools were called in 53 of 78 agent runs, without errors |
| B74 protocol | partial | dev runs made; formulationCritique re-graded with DeepSeek as the judge (not comparable with published numbers); see `benchmarks/sdai/REPORT.md` |

## Part 4: five questions

1. **Differences from `Dynamist_Function_Library_Spec.md`.** That file was never available; the phase 3 prompt was followed. Known differences from the prompt: the `RANDOM ...` argument order and `seed=` (see 1.4); `RANDOM LOOKUP` missing; `dyn compare` is the old `diff --runs`; internal names `LEVELISED STATE`, `LOAN STATE`, `TAX STATE`, `NPV AT` appear in doctor fixes.
2. **Settings in the model file.** Under `"settings"`: `units_scale_values` (written only when true; absent reads as false, so old files and every import keep today's arithmetic), `time_convention` (`"Calendar"`; absent is `"Abstract"`), `year_days_abstract` (absent is 365.25), `start` (ISO date), `weekend`, `holidays`, `seed` (absent is 1), `seed_recorded`, `unit_choices`, `starts_in_equilibrium`. Set them with `set_setting` using the keys `units_scale_values`, `unit_choice` (`MB=thousand barrels`), `start_date`, `time_convention`, `year_days_abstract`, `weekend`, `holidays`, `seed`, `starts_in_equilibrium`.
3. **Exit code.** Yes: `dyn check` exits 1 on a confirmed error-level finding, and with `--gate publish` also when the gate is below Publish. Unconfirmed interval findings do not stop it. In the sweep, after the doctor fixes 7 of 215 book models exit 1 (all of them in the allow-list; the sweep covers the 215 models that are in the repository).
4. **False alarms on book models.** After the first sweep the rules G2 (stock start values that read a rate: narrowed to flows), U10 (unit-bearing 1: now not for time or per-unit rates) and U12 (time constants: only names ending in time, delay, period) were narrowed, and F3/F8/F14 interval findings made informational for the gate. What remains is allow-listed in `core/tests/book_models/allow.toml` (21 entries after the doctor fixes, 57 before: unit mismatches 18, N5 2, one failed check; the F12 and G2 entries are gone): these are findings about the models, with generic reasons that were not reviewed one by one. Warning-level findings that remain frequent: G7 (terminal variables), G4, F4 (magic numbers), R1, R3, F9.
5. **Run times.** Largest book model by variables: `venture__models__venture-v14-fit-rival.flowmodel` (168 variables): C1 to C3 0.05 s, `--gate publish` 0.58 s, 54 MB. Slowest model: `ch11-plant` about 10 s for the review gate. 500-variable generated model: C1 to C3 under 200 ms (test).
