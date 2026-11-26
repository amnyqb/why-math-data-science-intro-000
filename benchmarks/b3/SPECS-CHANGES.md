# Specs changes proposed by B3 (for agreement)

Section 11 of the brief asked for five changes. Each is listed with what exists now and what the benchmark showed is needed.

| Proposed change | State after B3 | What the benchmark adds |
|---|---|---|
| Section "Model reduction and structural policy design" with F1 to F5, their checks and acceptance criteria | Implemented as `screen`, `importance`, `reduce`, `verify_reduction`, `recommend_structures` (`dyn`, Python; `screen`, `importance`, `dominance` also as agent tools). Documented in [Find-What-Matters](../../wiki/Find-What-Matters.md) | Acceptance criteria per function are in [EXEC-PLAN.md](EXEC-PLAN.md) section 5 and were met by the unit tests. A reduction should be accepted only when `verify_reduction` passes on an ensemble that draws every lever of the full model |
| Goal declaration in the manifest (goal outputs, measures, protected policy rankings) | Implemented: `goals` in the manifest, checked by `validate`; measures final, peak, lowest, total, average, mean:FROM-TO, at:TIME; directions min, max, target:VALUE; `policies` and `protected_ranking` | No screen in the app shows or edits goals yet. A goal on an output with an exponential tail should be able to name a transform (logarithm): Market Growth scenario F |
| Reduced-model versions linked to their source model, with goals and validity domain recorded | Partly: `manifest.reduced_from` names the source, the goals are carried into the new model | The validity domain of a reduction (the lever ranges it was verified over) is not yet recorded in the reduced model; it should be, as a `validity` entry written by `reduce` |
| Structure library for F5 (starter set from section 2), versioned | Implemented: library version 1, five structures (`supply_line`, `forecast`, `information_link`, `buffer`, `delay_change`) | Roles must be named by the caller; automatic detection of the decision point (the flow that closes a gap) is open. The brief's "table-function reshaping" is not in the library |
| Checklist items for F1 to F5 and for B3 replications | This report and [STATE.md](STATE.md) are the checklist for B3 | Add to the conformance checklist: frozen configuration by hash, convergence reported for every screening and every variance estimate, versions recorded in every answer-key entry |

Also proposed, from what failed:

1. Loop scores must follow every stateful function (DELAY FIXED, TREND, queues) and large models need the strongest-path search, or the dominance function is unusable on models of the size of World3.
2. `reduce` and `verify_reduction` should be one atomic command when used by an agent: a reduction that does not verify is not delivered.
3. Variance-based importance needs a convergence rule that is stated before the run (here: largest change of any index between N and 2N base samples, at most 0.05) and that makes the function refuse to rank when it is not met.
