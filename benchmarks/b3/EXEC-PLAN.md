# B3 execution plan (for autonomous work)

*Written 20 November 2026. Read [BRIEF.md](BRIEF.md) first (the owner's brief; it calls the benchmark "B2"), then [PLAN.md](PLAN.md) (readiness audit). This file is the work order. It is written so that a fresh session can carry on alone: **start by reading [STATE.md](STATE.md)**, do the next task, update STATE.md, commit, repeat.*

## 0. Rules of engagement

1. **Do not ask the owner between tasks.** Decide with the defaults in section 1 and log each decision in STATE.md. Ask only for the items in section 2 (they cannot be done alone).
2. **No invented targets.** An answer-key entry exists only if its source has been read and cited by page, table or figure. A paper that cannot be read gives no entries; it is listed as blocked.
3. **Report failures as prominently as successes.** A function that disagrees with the literature is a result, written up with a diagnosis (method limit, version difference, or a defect in our function).
4. **Never weaken a test to make it pass.** If a check fails, the work item is not done.
5. **Freeze before scoring.** Function settings (sample sizes, seeds, thresholds, library) are committed in `config/<case>.yaml` before any score is computed. Changing them later is logged and the whole run repeated.
6. **Keep the full model intact.** A reduced model is a separate file linked to its source. A recommended structure is a proposal (a batch of operations), never an edit.
7. **Commits.** After each finished task: `cargo test --workspace` green, docs updated (help entries for any new function, wiki, CHANGELOG, backlog), then commit. Use `git -c user.name="amin" -c user.email="libraryamin@gmail.com"`, end the message with `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`. Never commit `shots/`. Never use `rm -rf`. Downloaded papers and caches go in `benchmarks/b3/papers/` and `benchmarks/b3/build/` (git-ignored; add to `.gitignore` in T0.1).
8. **Register of the final report:** impersonal, no em dashes (the brief says so for `B3_report.md`).
9. **Resume protocol.** STATE.md has one row per task with status (`todo`, `doing`, `done`, `blocked`), the commit that finished it, and the next task. Update it in the same commit as the work.

## 1. Decisions taken by default (change only if the owner says so)

| Question | Default | Why |
|---|---|---|
| F1 and F2: SALib or native? | **Native Rust** in `core`, so `dyn`, MCP and Python get it without a Python dependency. **SALib in a throwaway venv** (`uv venv benchmarks/b3/build/salib-venv`) is used only to cross-check our numbers on identical designs. Record the choice in the report. | The brief allows either if recorded; native is what ships. |
| World3 version | Start with **World3-03** (the Vensim sample, already imported). Label every World3 result "World3-03, not the 1974 DYNAMO version". Replace it if a readable 1974 source is obtained (PyWorld3 and WorldDynamics.jl are open: check which version each ports, T5.1). | The 1974 source is borrow-only on archive.org. |
| B2-4 (Saysel and Barlas) | **Skip, report "not run: models not obtained".** No outreach. | Contacting authors is an outward message (section 2). |
| Numbering | **B3** in this repository. | B2 is import fidelity. |
| Models "rebuilt from the sources" | Use the copies already here (Vensim's Market Growth, SDXorg's workforce, World3-03, Vensim's project model), record the version, and **compare with the published description** (stocks, loops, parameter values quoted in the papers). Rebuild only when the copy differs. | Faster; the brief says to record which version. |
| Policy structures for F5 | Start from the brief's list (added information links, supply-line correction, buffers, forecast-based steering, delay changes, table-function reshaping), each as an operation template. | The brief's starter set. |

## 2. What cannot be done alone (report, then continue with the rest)

- Paywalled or archive-only papers (SDR, IEEE, Eberlein 1985 full text, Thissen, Saysel and Barlas, Schoenberg et al.): need the PDFs from the owner. Their answer-key entries stay `blocked`.
- Any message to authors, any publication, any licence decision.
- A decision to change the defaults above.

## 3. Phases and tasks

Each task: **what**, **files**, **done when**. IDs are used in STATE.md.

### Phase 0. Groundwork

- **T0.1 Repo and tooling.** Add `benchmarks/b3/papers/`, `benchmarks/b3/build/` to `.gitignore`. Create `benchmarks/b3/{config,answer_keys,results,models}/`. Create the SALib venv with `uv` (Python 3.12; `salib`, `numpy`, `scipy`, `pyyaml`). *Done when:* `build/salib-venv/bin/python -c "import SALib"` works and the folders exist.
- **T0.2 Goals in the manifest.** In `core/src/manifest.rs` add `goals: Vec<Goal>`: `{ name, output (variable), measure (final | peak | mean:from-to | at:time | trajectory), direction (min | max | target:value), policies: [scenario names] , protected_ranking: bool }`. Validate in `manifest::check` (output exists, scenarios exist). Serialise with the model; draft/read/write through the operations language (`set_manifest`) and `dyn manifest`. *Done when:* a model file round-trips goals; a bad goal is rejected with a message; tests in `core`; documented in `wiki/Model-API.md` and `API-Reference.md` (regenerate with `--write-docs`).
- **T0.3 Measures.** A function `goal_value(run, goal) -> f64` reusing `optimize::measure` (extend it with `mean:from-to`, `at:` if missing). *Done when:* unit tests for each measure on a tiny model.
- **T0.4 Answer-key checker.** `benchmarks/b3/answer_key.py`: loads `answer_keys/*.yaml`, checks the fields in the brief (case, finding, type, phase, value, qualitative, source, version), that `source` names a page, table or figure, that `value` is absent when `qualitative: true`, and that `digitised` is set when a value comes from a plot. Exit non-zero on any violation. *Done when:* it passes on a sample file and fails on three deliberately broken ones (tests in `benchmarks/b3/tests/`).
- **T0.5 Paper fetch.** For each open-access paper in BRIEF section 9 (ISDC proceedings and archives.albany.edu links, PyWorld3, WorldDynamics.jl), fetch the PDF or page into `papers/` (use a web-fetch agent, never execute anything fetched; open PDFs only with `pdftotext`). Write `papers/INDEX.md`: for each, **read / blocked / partial**, with the reason. *Done when:* every paper in section 9 has a row.
- **T0.6 Runner skeleton.** `benchmarks/b3/run_b3.py`: for a case, load `config/<case>.yaml`, refuse to score if the config hash differs from the one committed in `results/<case>/CONFIG.sha256`, call the functions through `dyn`, write CSVs to `results/<case>/`. *Done when:* a dry run on a toy case writes the hash and refuses a changed config.

### Phase 1. F1 `screen` (Morris)

- **T1.1 Design.** `core/src/screen.rs`. Morris elementary effects: `r` trajectories, `p` levels (default 4), step `delta = p / (2(p-1))`, parameters scaled to [0, 1] over each lever's range (levers and ranges from the manifest; a parameter without a range is refused with a message). Per parameter: `mu`, `mu_star` (mean absolute effect), `sigma`. Classification: **negligible** if `mu_star < tau * max(mu_star)` (default tau 0.1); **linear** if `sigma/mu_star < 0.1`; otherwise **non-linear or interacting**. Thresholds are configuration, frozen per case.
- **T1.2 Convergence.** Run with `r` and `2r` trajectories (the first `r` shared); report Kendall tau between the two `mu_star` rankings and the overlap of the top-k set. A ranking is "converged" if tau at least 0.9 (configurable). Return the flag and both rankings.
- **T1.3 Goal outputs.** Run once per design point and evaluate every declared goal on the same run (cost counts runs, not goals).
- **T1.4 Interfaces.** `dyn screen FILE [--goal NAME]... [--trajectories N] [--levels P] [--seed S] [--csv OUT]`; an MCP tool; a Python wrapper method; a help entry in the wiki (`Model-API.md`). Output JSON and CSV.
- **T1.5 Validation.** (a) On the Morris (1991) test function and on the Ishigami function as `.sdm` formula models, compare our `mu_star`, `sigma` with SALib's on **the same design** (generate the design in Rust, export it, evaluate with SALib); they must agree to 1e-9. (b) A model with a known inert parameter must classify it negligible. (c) Doubling `r` must not change the top group on the test functions. *Done when:* these tests run in `cargo test` or in `benchmarks/b3/tests/` and pass.

### Phase 2. Loop dominance, F2 `importance`

- **T2.1 Loops that matter.** In `core`: link scores and loop scores following Schoenberg, Davidsen and Eberlein (2020) (read the paper first: T0.5; if blocked, use Eberlein and Schoenberg's open descriptions and the SDR abstract only, and say so). For each link `x -> z` in an equation `z = f(x, y, ...)`: `link score = | change in z caused by the change in x | / | total change in z |` with the sign of the partial effect, computed per time step by recomputing `f` with only `x` changed (`f(x_t, others_{t-dt}) - f(x_{t-dt}, others_{t-dt})`); for a flow into a stock, the link score is the flow's share of the stock's change. A loop's score is the product of its link scores; the relative loop score divides by the sum of absolute loop scores at that time. Loops come from `cld::find_loops` or `ops::loops`; large models need the "strongest path" shortcut (record if used).
- **T2.2 Validation of dominance.** Hand-checkable models: (a) exponential growth `x' = g x` (one reinforcing loop, score 1); (b) goal seeking `x' = (G - x)/T` (one balancing loop); (c) S-shaped growth (logistic): the reinforcing loop dominates first, the balancing loop later, the switch near the inflection. *Done when:* tests assert these.
- **T2.3 Eigenvalue elasticity (optional, near-linear models).** Numerical Jacobian of the state equations at chosen times; eigenvalues; elasticity of the eigenvalues to each parameter by finite differences. Validate on a two-stock linear oscillator with known eigenvalues. Document that it is not meaningful in strongly non-linear phases (Kampmann and Oliva 2006).
- **T2.4 Sobol.** Native Saltelli sampling (`N (2k + 2)` runs) and the Jansen estimators for first-order `S1` and total `ST`; bootstrap confidence intervals; convergence by doubling `N`. Validate on Ishigami (`a = 7, b = 0.1`): analytic `S1 = (0.3139, 0.4424, 0)`, `ST = (0.5576, 0.4424, 0.2437)` within the estimator's confidence interval, and against SALib on the same sample to 1e-9.
- **T2.5 Dependency pruning.** From `ops`/`deps`: the set of variables with no path to any goal output. Return the removable elements and the reason.
- **T2.6 `importance` interface.** `dyn importance FILE --params ...` (the parameters surviving F1 by default), returns Sobol indices, loop dominance over the ensemble (the share of runs and time in which each loop is the strongest), optional eigenvalue elasticity, pruning list. CLI, MCP, Python, docs.

### Phase 3. Cases B2-1 and B2-2 (analysis)

For each case, in this order: (1) base-run replication; (2) goals declared; (3) answer key written from the papers; (4) configuration frozen; (5) F1 and F2; (6) scoring; (7) disagreements examined.

- **T3.1 B2-1 Workforce-Inventory.** Model: `benchmarks/b2/external/test-models/samples/Workforce/workforce.mdl` (import it; copy the `.sdm` into `models/`). Compare with the description in Hayward and Roach 2023 (three stocks, three loops, parameter values they quote). *Answer key:* the dominant-loop findings per phase from Hayward and Roach 2023 (open), and Hekimoğlu and Barlas 2010 (open). *Score:* section 6.1 of the brief.
- **T3.2 B2-2 Market Growth.** Model: Vensim `MARKET.mdl` (already imported; version recorded). Compare with Forrester 1968 and the reconstructions in Mojtahedzadeh 2015, Phaff et al. 2008 and Saleh et al. (all open). *Answer key:* dominant structure per phase (Mojtahedzadeh; Phaff; Saleh), the leverage parameters and the simplification result (Eberlein 1985 is blocked unless the PDF is supplied; record that). Where papers disagree, keep both and score against each.
- **T3.3 Kendall tau and scoring code.** `benchmarks/b3/score.py`: Kendall tau (with ties), top-group overlap, share of key findings reproduced, extra dominant loops reported.

### Phase 4. F3 `reduce`, F4 `verify_reduction`

- **T4.1 Reduction algorithm.** Inputs: model, goals, F1/F2 results, thresholds (frozen). Steps, in order, each recorded with its reason in a change log: (1) prune elements with no path to a goal output; (2) fix negligible parameters at nominal values (replace the variable by a constant); (3) remove or merge loops that never dominate a goal output (only when the loop's variables are not needed by other loops: otherwise record "kept"); (4) aggregate chains where justified (a series of identical first-order delays into one higher-order delay; a pass-through variable removed). Output: a **new** `.sdm` linked to the source (manifest: `reduced_from`, `goals`, `validity`) and the change log as `reduction_log.csv`. Implemented as a batch of operations through the operations language so that it is atomic and logged.
- **T4.2 Verification.** `verify_reduction(full, reduced, goals, ensemble)`: normalised RMSE and Theil's inequality decomposition (bias `U_m`, variance `U_s`, covariance `U_c`) of each goal output for the base run and across an ensemble of parameter draws (the ensemble from the frozen config); behaviour modes (classify each run: growth, decay, S-shape, overshoot and collapse, oscillation damped or sustained, by features of the series; compare labels run by run and report the share equal); Kendall tau of the policy ranking on the declared policies (must be 1 unless explained); extreme-condition tests (the model's own checks: nonneg, bounds, balance, conservation) pass on the reduced model. Pass or fail with the diagnostics.
- **T4.3 Interfaces and docs.** `dyn reduce`, `dyn verify`; MCP; Python; help; wiki.
- **T4.4 Validation.** On a model with a known redundant part (build one: a chain of three equal first-order stocks into one stock that does not affect the goal; an inert parameter) the reduction must remove exactly the redundant parts and pass verification. Then run on Market Growth (B2-2): report size removed, errors, modes, ranking.

### Phase 5. World3

- **T5.1 Versions.** Read the READMEs of PyWorld3 and WorldDynamics.jl (clone shallow into `build/`): which World3 version each ports; record. Run both if their dependencies are installable without side effects (venv), export the standard run.
- **T5.2 Base run.** Our World3-03 standard run against PyWorld3 and the Vensim-run difference already known (B2: up to 0.17% from PySD). State a tolerance (for example 1% on population and industrial output at 1950, 2000, 2050, 2100) before comparing; explain any difference.
- **T5.3 Goals and answer key.** Goals: population, industrial output per capita, food per capita, persistent pollution (peak or final values over 1900 to 2100) and the ranking of the scenarios in `World3_03_Scenarios.mdl` (if used). Answer key from Thissen 1976 and 1978 (blocked unless supplied) and Taylor, Ford and Ford 2007 (open). Without Thissen, the structure findings are blocked and the report says so.
- **T5.4 F1, F2, F3, F4** with a frozen config; the model has about 260 variables and many table functions: set sample sizes that fit (record runs and wall time; each run is milliseconds).

### Phase 6. F5 `recommend_structures`

- **T6.1 Structure library.** `core/src/structures/` (or JSON templates in `config/`): each entry is `{name, description, preconditions (what variables or loops must exist), operations (a parameterised batch of operations), real_world_decision, adds_loop / weakens_loop}`. Starter set from the brief: added information link, supply-line correction, buffer (stock cover), forecast-based steering (uses `EXPECT`), delay change (uses `DELAY`), table-function reshaping. Versioned (`library_version`).
- **T6.2 Search.** For each applicable structure and parameter set: apply the template to a copy of the model, tune its parameters with `optimize` (existing) against the declared goals, evaluate over the ensemble. Return the Pareto set of options with trade-offs, robustness (share of ensemble runs beating the base policy), and the loops added or weakened (checked with F2). Flag options with no plausible real-world counterpart.
- **T6.3 Project model (B2-5).** Rebuild or check Vensim's `PROJ.MDL` against Richardson and Pugh 1981 (book: description only) and Keloharju and Wolstenholme 1986 (open). Declare the same goals; run F5; compare goal improvement with their reported improvement (qualitative: "much better"; score agreement).
- **T6.4 World3 structure (B2-3, tier 3).** Compare with Thissen's simpler stabilising policy when readable; otherwise report as blocked.

### Phase 7. Report and Specs

- **T7.1 `B3_report.md`** with the sections of the brief (section 8): replication of base runs, scores per section 6, disagreements and their causes, cost (runs and seconds per function and case), gaps found in Dynamist, references with links, known risks (brief section 10). Impersonal register, no em dashes.
- **T7.2 Specs changes** (brief section 11) as `SPECS-CHANGES.md`, plus an update of `wiki/Specs-Conformance.md`.
- **T7.3 Wiki and help.** Function reference, a page `Find-What-Matters.md`, backlog items B49 to B55 closed or updated, CHANGELOG.
- **T7.4 Book hook.** One chapter in `book/Book-Outline.md` ("What matters in this model"), using the workforce model as the example, with screenshot notes.

## 4. Order and dependencies

T0.x (any order, T0.5 early) → Phase 1 → Phase 2 → T3.1, T3.2 → Phase 4 → Phase 5 → Phase 6 → Phase 7. Phase 3 cannot score until Phases 1 and 2 pass their validation tasks. If a task is blocked (papers, a missing PDF), mark it `blocked`, write why in STATE.md, and move to the next task that does not depend on it.

## 5. Acceptance (per function)

| Function | Accepted when |
|---|---|
| F1 `screen` | T1.5 passes; ranking convergence reported on every case; a case with a non-converged ranking is reported, not used silently |
| F2 `importance` | Sobol agrees with Ishigami and SALib; loop dominance passes T2.2; pruning returns the right set on a model with known dead ends |
| F3 `reduce` | T4.4 passes on the built redundant model; every change in the log has a reason; the source model is untouched |
| F4 `verify_reduction` | detects a deliberately wrong reduction (a removed important parameter) as a failure with diagnostics |
| F5 `recommend_structures` | recovers a known improvement on a model built to have one (a goal-seeking stock with an obvious missing supply-line correction); robustness and explanation reported |

## 6. Risks to watch (also for the report)

- Published findings are often qualitative: score agreement, not numbers.
- Model versions differ: record the version in every answer-key entry and every result.
- Linear methods mislead in non-linear phases: say so where it applies.
- Non-converged rankings select the wrong parameters: the convergence check is mandatory.
- Structure search can reach goals for the wrong reasons: robustness, explanation and implementability checks are part of the output.
- Licences: model and paper licences before anything is published.
