# B3 state

*Update this file in the same commit as the work. A fresh session reads this first, then [EXEC-PLAN.md](EXEC-PLAN.md).*

**Next task:** T7.1

| Task | What | Status | Commit | Notes |
|---|---|---|---|---|
| T0.1 | Repo and tooling (folders, .gitignore, SALib venv) | done | see git log | SALib 1.6.0 in build/salib-venv |
| T0.2 | Goals in the manifest | done | see git log | `Goal` in core/src/manifest.rs; checked by `validate`; in API-Reference |
| T0.3 | Measures for goals | done | see git log | `goal_value`, `goal_score`; measures final, peak, lowest, total, average, mean:A-B, at:T |
| T0.4 | Answer-key checker | done | see git log | answer_key.py, tests/test_answer_key.py |
| T0.5 | Fetch open papers, write papers/INDEX.md | done | see git log | read: Mojtahedzadeh 2015, Phaff 2008, Hayward&Roach 2023, Hekimoglu&Barlas 2010. All archives.albany.edu items blocked by Cloudflare check (needs manual download). papers/ is git-ignored; INDEX.md is local only |
| T0.6 | Runner skeleton with frozen configuration | done | see git log | run_b3.py, tests/test_runner.py |
| T1.1 | F1 design and Morris sampling | done | see git log | core/src/screen.rs: morris_design, effects, classify |
| T1.2 | F1 convergence | done | see git log | r vs 2r Kendall tau and top-group overlap |
| T1.3 | F1 goal outputs | done | see git log | all goals read from the same runs |
| T1.4 | F1 interfaces (dyn, MCP, Python, docs) | done | see git log | dyn screen, MCP tool, Python Model.screen, wiki/Find-What-Matters.md |
| T1.5 | F1 validation (Morris and Ishigami, SALib cross-check) | done | see git log | tests/test_screen.py: SALib agrees to 1e-9 on Ishigami; additive, inert, doubling. Morris (1991) function itself not used (Ishigami and additive models instead) |
| T2.1 | Loops that matter (link and loop scores) | done | see git log | core/src/loopscore.rs, sim.rs simulate_ltm. Euler only; SMOOTH followed, DELAY/TREND/queue loops listed unscored; interface comes with F2 (T2.6) |
| T2.2 | Dominance validation (growth, goal seeking, S-shape) | done | see git log | tests: exponential, goal seeking, logistic (switch near S=K/2), smooth/delay |
| T2.3 | Eigenvalue elasticity | done | see git log | core/src/eigen.rs (own Hessenberg+QR solver), sim.rs jacobian_at; linear oscillator test |
| T2.4 | Sobol indices (native) and validation | done | see git log | core/src/sobol.rs; matches SALib 1e-9 and Ishigami analytic; tests/test_sobol.py |
| T2.5 | Dependency pruning | done | see git log | loopscore::unreachable_from_goals |
| T2.6 | F2 interface | done | see git log | core/src/importance.rs; dyn importance, dyn dominance, MCP, Python, wiki/Find-What-Matters.md |
| T3.1 | B2-1 Workforce-Inventory | done | see git log | 7 of 7 key findings reproduced; model is a reconstruction of the paper diagram (the SDXorg Workforce sample is a different model: experts and rookies); core variant used because the paper analyses the two-stock subsystem; results/b2-1/ |
| T3.2 | B2-2 Market Growth | done | see git log | Stella rebuild (CBSDLab/market-growth, no licence: kept in papers/, models in build/, both git-ignored). Scenario C reproduces Table 1 half-cycles (max 2.4% off), F Table 2 (max 12.5%). Scored: C 3 of 4, F 3 of 4 (M7 Phaff utilisation finding NOT reproduced). Memo D-4079 is a scan without text: equations not checked against it |
| T3.3 | Scoring code (Kendall tau, top-group overlap, findings reproduced) | done | see git log | scoring scripts score_b2_1.py, score_b2_2.py (Kendall tau lives in core screen.rs; answer keys hold no rankings, so no tau against a key) |
| T4.1 | F3 reduction algorithm | done | see git log | core/src/reduce.rs: prune, fix, merge, loops listed as kept. Not done: delay-chain aggregation, loop removal |
| T4.2 | F4 verification | done | see git log | core/src/verify.rs: Theil, modes, ensemble tau, policy ranking, checks |
| T4.3 | F3 and F4 interfaces and docs | done | see git log | dyn reduce, dyn verify, Python; MCP not added; wiki/Find-What-Matters.md |
| T4.4 | F3 and F4 validation, then Market Growth | done | see git log | tests/test_reduce.py (redundant model, wrong reduction). Market Growth run waits on T3.2 |
| T5.1 | World3 versions (PyWorld3, WorldDynamics.jl) | done | see git log | pyworld3 README: 1974 book version; WorldDynamics.jl: World3 (fig 7.7 of 1974), World3_91, World3_03. Not run (would need Python/Julia envs; not needed: the 1974 model cannot be imported here) |
| T5.2 | World3 base run | done | see git log | World3-03 base run: B2 result vs PySD (0.004 to 0.17%, backlog B36). No comparison with the 1974 version is possible here |
| T5.3 | World3 goals and answer key | todo | | Thissen papers blocked unless the owner supplies PDFs |
| T5.4 | World3 F1 to F4 | done | see git log | config/b2-3.yaml, results/b2-3: Morris converged for all goals; Sobol not converged for pollution; dominance infeasible (20000-loop cap reached); reduction (311 to 279 variables) FAILS verification on Food per capita (ensemble NRMSE 0.066, modes 80%) |
| T6.1 | Structure library | done | see git log | core/src/recommend.rs: library v1 of 5 structures |
| T6.2 | Structure search | done | see git log | grid search over an ensemble, robustness, loops added, plausibility flag |
| T6.3 | Project model (B2-5) | blocked | see git log | Keloharju and Wolstenholme 1986 blocked (albany); no answer key, not run |
| T6.4 | World3 structure (tier 3) | blocked | see git log | blocked: Thissen papers not readable |
| T7.1 | B3_report.md | todo | | |
| T7.2 | Specs changes | todo | | |
| T7.3 | Wiki and help | todo | | |
| T7.4 | Book chapter hook | todo | | |

## Decision log

| Date | Decision | Why |
|---|---|---|
| 20 Nov 2026 | Defaults in EXEC-PLAN section 1 adopted: native F1/F2 with SALib as a cross-check only; World3-03 as the World3 version until a 1974 source is readable; B2-4 skipped (no outreach); numbering B3 | owner asked for a plan that can be executed without asking |

| 20 Nov 2026 | LTM stock links changed to share of the change in the net flow (Hayward & Roach p. 7 describe it so); first version used the share of the net flow, which gave a wrong S-shape result | found while reading the paper for B2-1 |
| 20 Nov 2026 | B2-1 analysed on a two-stock core with Expected Demand exogenous | paper p. 4 does so |
| 20 Nov 2026 | Scoring definitions (turning window 0.5 month, slowing-down interval from fastest change to window) were fixed after a first look at the full model's dominance sequence, before any scoring | disclosed in the report |

| 20 Nov 2026 | LTM follows SMOOTH, SMOOTH3, DELAY1, DELAY3 as hidden stocks (needed: 13 of 22 Market Growth loops ran through DELAY3I) | benchmark finding |
| 20 Nov 2026 | Bug found by B2-2: prune removed constants read only by stock initial values and delay times; `unreachable_from_goals` now counts every name in an equation; `reduce` refuses a result that does not compile | benchmark finding |
| 20 Nov 2026 | verify draws every lever of the full model, the reduced model gets those it still has (before: only shared levers, so a wrongly fixed parameter could not show) | benchmark finding |
| 20 Nov 2026 | Loop-identification and M2/M3/M5/M6 criteria for B2-2 were written after seeing the loop list and shares of an exploratory run (needed to map our loops to the paper's); thresholds are generous and fixed in the frozen config | disclosed in the report |
| 20 Nov 2026 | B2 labels corrected: Vensim MARKET sample is not Forrester Market Growth | found while preparing B2-2 |

## Blocked

| Item | Needs | Since |
|---|---|---|
| Paywalled papers (SDR, IEEE, Eberlein 1985 full text) | PDFs from the owner | 20 Nov 2026 |
| archives.albany.edu PDFs (Eberlein 1985, Saleh et al., Taylor/Ford/Ford 2007, Keloharju&Wolstenholme 1986, Thissen 1976) | manual browser download into benchmarks/b3/papers/ | 20 Nov 2026 |
| Saysel&Barlas working paper | manual download: https://nva.sikt.no/registration/0198f231e064-7b31beab-12f9-4f87-afd2-9476fa32b0e2 | 20 Nov 2026 |
| B2-2 Market Growth (Forrester 1968) | the model equations (Forrester 1968 or Morecroft 1983 paper, or an open copy) | 20 Nov 2026 |
| B2-4 | the authors' models (outward message: owner's decision) | 20 Nov 2026 |


# Round 2 (started 20 Nov 2026)

Case ID mapping (R1): B2-1 = B3-1 Workforce-Inventory, B2-2 = B3-2 Market Growth, B2-3 = B3-3 World3-03 (2003 version, unscored), B2-4 = B3-4, B2-5 = B3-5 project model, B2-6 = B3-6. Round-1 files keep their names; new files use B3-n. World3 1974 is B3-3b.

Instructions: [ROUND2-PROMPT.md](ROUND2-PROMPT.md). Agents: `.claude/agents/`. Information barriers: no hooks configured; each restricted agent logs the files it reads to `logs/<agent>_reads.txt` and red-team audits the logs (method to be stated in the report).

| Node | What | Status | Notes |
|---|---|---|---|
| setup | agent files, round-2 state | done | |
| L0 | papers present? | doing | fetch agent; albany items need manual download |
| E1 | W1 strongest path | doing | method source (Schoenberg et al. 2020) not readable: approximate best-first labelled `approximate_strongest_path` |
| E2 | W2 reduce loop + gates | doing | |
| E3 | W3 stateful functions | doing | |
| E4 | W4 two-part report | doing | with E2 |
| E5 | W5 recommend | done (merged) | Pareto set, weights, role detection, scale-free measure, library v2 with sources flagged; Sterman chapter numbers unconfirmed |
| M1 | World3 1974 | done (merged) | built from PyWorld3 equations; worst error 0.70% at 1950/2000/2050/2100 against PyWorld3 (tolerance 1%); 1900 start values differ 1.7% to 5% because PyWorld3 k=0 row is not a consistent state; PyWorld3 vs the 1974 book NOT checked |
| M2 | project model | blocked | Richardson and Pugh listing not available: ASK |
| H1 | held-out case | blocked | ASK |

## Round 2 progress (20 Nov 2026, later)

| Item | Status | Notes |
|---|---|---|
| E1 W1 | done, partly | published strongest-path algorithm (Eberlein and Schoenberg 2020, arXiv 2006.08425, read) implemented; recall 100% on small models and workforce-inventory; World3-03 and 1974 complete in 0.3 s without the cap; two heuristics (ours best-first vs published) agree on only 26 of 42 and 25 of 43 loops ever dominant, so recall on World3 cannot be established |
| E2 to E4 | merged | gated reduce; round-1 reruns: B3-2 C accepted (1 element, 7 parameters), B3-2 F refused (heavy tail, no transform), World3-03 refused (pollution goal) |
| E5 | merged | |
| M1 | merged | World3 1974 within 0.70% of PyWorld3 |
| L0/L1 | done | papers from the owner's server library: Thissen 1976, Thissen 1978 (paper and thesis), Kampmann and Oliva 2006, Vermeulen and de Jongh (file was misnamed Meadows-1974). Richardson and Pugh file is a 3-page review, NOT the book: M2 stays blocked |
| keys | frozen | keys/FREEZE.md: b3-3b (41 entries), addenda Thissen 1976 (31), Vermeulen and de Jongh (14), Thissen 1978 thesis (28) |
| B3-3b | scored | 6 of 6 testable entries of the Thissen 1978 key; reduction passes verification but removes 1.9% of structure |
| B3-3c | scored | config frozen before running; C2 reproduced, C1 not (ALIC1 rank 7); all four goals converged for Morris and Sobol |
| R1/R2 red-team | running on the server | |
| H1 held-out | blocked | ASK: owner to choose (Kampmann and Oliva 2006 now available) |
| M2 project model | blocked | book listing not available |
