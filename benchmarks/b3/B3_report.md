# B3 report: Find what matters

*Written 20 November 2026 for the benchmark described in [BRIEF.md](BRIEF.md) (the brief calls it B2; it is B3 here because B2 is the import-fidelity benchmark). Work order: [EXEC-PLAN.md](EXEC-PLAN.md). State and decisions: [STATE.md](STATE.md).*

## 1. Summary

Five functions were built and tested: `screen` (F1, Morris), `importance` (F2, Sobol indices, Loops That Matter, eigenvalue elasticity, removable variables), `reduce` (F3), `verify_reduction` (F4) and `recommend_structures` (F5). Each is available from the command line (`dyn`), the agent tools (F1, F2 and the loop analysis), and Python, and each has a unit test against a known answer. Two of the five cases of the brief were scored against published findings, one was run without a key, and two could not be run.

| Case | Model | Key | Result |
|---|---|---|---|
| B2-1 Workforce-Inventory | rebuilt from the diagram of Hayward and Roach 2023 | 7 findings from that paper | **7 of 7 reproduced** |
| B2-2 Market Growth, damped oscillation (scenario C) | Stella rebuild of Forrester 1968 (third party) | 3 findings from Mojtahedzadeh 2015, 1 from Phaff 2008 | 3 of 4 reproduced; the half-cycle durations of his Table 1 are matched to 2.4% |
| B2-2 Market Growth, growth with cycles (scenario F) | same | 3 findings from Mojtahedzadeh 2015, 1 from Phaff 2008 | 3 of 4 reproduced; his Table 2 durations are matched to 12.5%; variance-based indices not converged |
| B2-3 World3-03 | Vensim sample (not the 1974 version) | none obtainable | run without a score; **the reduction fails verification** |
| B2-4 irrigation (Saysel and Barlas) | not obtained | n/a | not run |
| B2-5 project model | not run | Keloharju and Wolstenholme 1986 not readable | not run |
| B2-6 optional | not run | n/a | not run |

What the benchmark found about the functions:

- The loop-dominance function reproduced the qualitative loop findings of two papers once it was corrected twice: the score of a flow into a stock had to be the share of the change in the stock's **net flow** (the paper states this on p. 7), and SMOOTH and DELAY chains had to be followed as hidden stocks (13 of the 22 loops of Market Growth run through a DELAY3).
- `reduce` produced an invalid model on Market Growth (it removed constants that only a stock's initial value and a delay time read). This was fixed, and `reduce` now refuses any result that does not compile.
- `verify_reduction` first drew only the levers both models share, which hides a wrongly fixed parameter. It now draws every lever of the full model. With that change it **refuses the World3 reduction** (food per capita: ensemble error 0.066 against a limit of 0.05, behaviour mode kept in 80% of runs against 90%).
- Variance-based indices are unusable for an outcome with an exponential tail (Market Growth scenario F): the total indices exceed 1 and do not converge. Morris screening was also not converged there (Kendall tau 0.82 between 50 and 100 trajectories). The report flags this; a transformed goal (the logarithm of the backlog) is the remedy and was not tried.
- Two answer-key entries were not reproduced (section 5), and the reasons are stated.

## 2. Replication of the base runs

| Case | Check | Result |
|---|---|---|
| B2-1 | eigenvalues of the two-stock core at month 20, against the analytic values for the model as reconstructed (Jacobian [[0, 2], [-1/16, -1/4]]) | -0.125 ± 0.3307 i, period 19.0 months: equal to the analytic values to 1e-10. The paper shows damped oscillations and ranks the same loops; it gives no numbers for the run |
| B2-2 C | spacing of the extrema of Backlog against Table 1 of Mojtahedzadeh 2015 | ours 16.5, 17.5, 17.5, 17.0, 17.5; paper 16.9, 17.7, 17.6, 17.1, 17.4 (largest relative difference 2.4%) |
| B2-2 F | the same against Table 2 | ours 37.5, 14.0, 32.0, 13.5; paper 35.3, 14.0, 31.2, 12.0 (largest 12.5%) |
| B2-3 | World3-03 against PySD (benchmark B2): identical at 1900, then 0.004% to 0.17% apart; cause open ([backlog B36](../../wiki/BACKLOG.md)) | not compared with the 1974 version, which cannot be imported here |

Versions. **B2-1**: the SDXorg `workforce.mdl` named in the first plan is a different model (experts and rookies); the Workforce-Inventory model of the paper was rebuilt from its diagram (p. 4) and parameters (Fig. 6), with the standard equations of that structure (the paper gives no formulas). **B2-2**: the `MARKET.mdl` of the Vensim sample library, which earlier benchmark reports called Forrester's Market Growth, is another model (products, receivables, sales representatives); that mislabel was corrected in the B2 report and wiki. Forrester's model was taken from the Stella rebuild of Warren Farr in the repository CBSDLab/market-growth (no licence stated: the file is kept out of git, only results are committed). The equations were **not** checked against the MIT memo D-4079 because its PDF is a scan without text. The agreement of the half-cycle durations with two tables of a paper that used Forrester's original is the evidence that the rebuild is the same model for these two runs.

## 3. Scores per function

### 3.1 Analysis (F1, F2)

B2-1 (frozen configuration `config/b2-1.yaml`, scoring `score_b2_1.py`):

| Key finding (Hayward and Roach 2023) | Reproduced | Evidence |
|---|---|---|
| damped oscillation, only balancing loops (p. 4) | yes | 5 extrema, amplitudes 2.73, 0.84, 0.28, 0.08; all loops balancing |
| Expected Demand outside the loops of the two-stock core (p. 4) | yes | in the three-stock model its loop shares no variable with the other two |
| B2 dominates the turning points of Workforce (p. 6) | yes | B2 dominant in 100% of the time within 0.5 month of the 5 extrema |
| B1 dominates the slowing-down phases (p. 6) | yes | 67% of 417 steps |
| dominance alternates between B1 and B2 (p. 7) | yes | 12 switches after the step |
| damping explained by B1 only (p. 10) | yes | derivative of the real part of the eigenvalue with respect to ln(parameter): Hiring Time 0.125, Inventory Correction Time 0.000 |
| frequency affected by both, most by B2 (p. 10) | yes | imaginary part: Hiring Time 0.142, Inventory Correction Time 0.189 |

The mapping of loops to parameters (B1 to Hiring Time, B2 to Inventory Correction Time) is part of the frozen configuration. The answer key contains no parameter ranking, so no Kendall tau against a key could be computed. As an internal check, Morris mu* and Sobol total index order the six parameters identically (15 of 15 pairs agree) for all three goals, and both converge. Productivity per worker matters only for the workforce goal and trivially so (workers = output divided by productivity).

B2-2 (`config/b2-2c.yaml`, `config/b2-2f.yaml`, `score_b2_2.py`):

| Key finding | Scenario | Reproduced | Evidence |
|---|---|---|---|
| half-cycle durations of Backlog (Mojtahedzadeh Tables 1, 2) | C, F | yes, yes | section 2 |
| the third-order delivery delay loop is responsible for the periodicity (p. 10) | C | yes | the largest share (24.0%) among the 16 loops with three or more stocks |
| first-order loops around stocks drive the stability (pp. 10 to 11) | C | yes | 9 first-order loops, combined share 66.5% (limit 50%) |
| the sales growth loop dominates the growth trend (p. 14) | F | yes | reinforcing loop through Salesmen and Orders Booked, 68.5% of the time, the strongest of 25 |
| delivery delay loop and capacity expansion loop both matter (p. 14) | F | yes | mean absolute relative scores 0.032 and 0.088 (limit 0.01) |
| fixing the capacity utilisation variable leaves the run virtually identical (Phaff 2008, p. 15) | C, F | **no** | section 5 |

Screening and variance results of B2-2 are in `results/b2-2c` and `results/b2-2f`. Scenario C: Morris and Sobol converge, the salesman constant and the maximum sales effectiveness dominate the backlog peak (total indices 0.58 and 0.41). Scenario F: not converged (section 1). The papers give no parameter ranking, so these results are unscored.

### 3.2 Reduction (F3, F4)

| Case | Variables | Levers | Removed | Verification |
|---|---|---|---|---|
| redundant test model (built: dead branch, inert parameter, pass-through) | 22 to 17 | 8 to 6 | exactly the five planted parts | passes |
| wrong test reduction (a parameter that matters fixed at a wrong value) | | | | refused (NRMSE above 0.05) |
| B2-2 C | 55 to 47 | 13 to 6 | 1 pruned, 7 fixed | passes; the reduced runs equal the full runs to numerical precision because the fixed levers act on no goal in that scenario |
| B2-2 F | 55 to 51 | 13 to 10 | 1 pruned, 3 fixed | passes, but the Sobol indices it relied on were not converged |
| B2-3 World3-03 | 311 to 279 | 18 to 12 | 26 pruned (for example the human welfare index and the footprint, which no goal reads), 6 fixed | **fails**: food per capita, ensemble NRMSE 0.066 (limit 0.05) and 80% equal modes (limit 90%); the other three goals pass with mean NRMSE 0.03 to 0.04 |

Not done in F3: aggregation of chains of identical delays and removal of loops (loops that never matter are listed as kept). On Market Growth the paper's finding of superfluous structure (Phaff) is therefore not reproduced by a loop removal. On World3 the answer key (Thissen, Eberlein) could not be read, so the size of the reduction is not compared with a published one.

### 3.3 Structure (F5)

Tested on a built stock controlled by an order rule with a delivery delay (the textbook case that needs a supply-line correction). The library returns `supply_line` first, with improvement in every draw of the ensemble (robustness 1.0) and a balancing loop added; `buffer` trades a better trough for a higher peak; `forecast` helps little; `delay_change` does nothing for this goal; `information_link` is skipped because no perceived variable is named. The project model (B2-5) and the structural comparison on World3 (B2-3, tier 3) could not be run (sources unreadable), so the **agreement with a published structural improvement was not measured**.

### 3.4 Cost

Wall-clock seconds and model runs, release build, 4 threads (`cost.py`, written to `results/COST.csv`). Per model run, the small cases take about 0.07 ms to 0.1 ms of wall-clock time and World3-03 about 0.7 ms.

| Case | screen | importance (Sobol, dominance, ensemble) | reduce | verify |
|---|---|---|---|---|
| B2-1 (6 levers) | 0.05 s, 700 runs | 1.9 s, 32 768 runs | not run | not run |
| B2-2 C (13 levers) | 0.12 s, 1 400 runs | 2.8 s, 30 720 runs | 2.4 s | 0.03 s, 51 runs |
| B2-2 F (13 levers) | 0.12 s, 1 400 runs | 2.4 s, 30 720 runs | 2.4 s | 0.03 s, 51 runs |
| B2-3 World3-03 (18 levers) | 2.7 s, 3 800 runs | 118 s, 163 840 runs (almost all of it enumerating 20 000 loops for each of 11 runs) | 110 s (loop enumeration for the counts) | 0.46 s, 101 runs |

The cost grows with the number of levers k: screening `2r (k + 1)` runs, Sobol `2N (k + 2)`. The loop enumeration, not the simulation, is the expensive part on a model of the size of World3.

## 4. Examination of disagreements and failures

| Item | Diagnosis |
|---|---|
| Phaff (2008) utilisation finding, not reproduced in C and F | In our rebuild the variable that stands for utilisation (production capacity fraction, delivery rate over capacity) moves from 0.333 to 0.96 (C) and 0.82 (F), so fixing it changes the run (NRMSE 2.3 and 0.85). Phaff analysed the model of Morecroft (1983) with another parameterisation, and states the variable "hardly deviates". This is a version difference, not a defect found in the function, but it is not proven: Phaff's equations were not available. |
| World3-03 reduction fails food per capita | The threshold of 0.01 on the total Sobol index fixes parameters whose small effects add up in the tail. A lower threshold would keep more levers. The Sobol indices for the pollution goal were not converged and the loop enumeration reached its cap of 20 000 loops, so `reduce` could not use loop dominance on this model. |
| Scenario F indices above 1 | the goal is exponential in the parameters (unbounded growth), the estimator's variance is dominated by a few draws |
| Loop dominance on World3 | the enumeration of all simple loops is exhausted at the cap; the strongest-path shortcut of the Loops That Matter paper is not implemented, so no dominance result exists for this model |
| `reduce` first produced an invalid model; `verify` first could not see a wrongly fixed parameter | defects of the functions, found by the benchmark and fixed (section 1). The results of B2-2 and B2-3 were produced after the fixes. |

## 5. Gaps found in Dynamist

1. Loop scores follow SMOOTH, SMOOTH3, DELAY1 and DELAY3 only; DELAY FIXED, TREND, queues and the other stateful functions leave their loops unscored. Euler integration only.
2. No strongest-path search: large models exhaust the loop cap.
3. Eigenvalue analysis is at one time per call, with a numerical Jacobian; behaviour modes and the weights of eigenvectors (Saleh et al.) are not implemented.
4. `reduce` does not aggregate delays or remove loops.
5. `recommend_structures` needs its roles named by the caller; the improvement measure is relative per goal and weighs goals of different scales unequally; the library has five structures.
6. The goals declared in the manifest are not shown anywhere in the app's windows.

## 6. Guardrails kept

The reduced model is always a new file linked to its source (`manifest.reduced_from`); `reduce` never writes into the source. Recommendations are proposals: `recommend` applies nothing and returns the operations. No learned component is used. Answer-key entries come only from papers that were read in full (Hayward and Roach 2023, Mojtahedzadeh 2015, Phaff et al. 2008, all open access); no entry was written from memory. The configuration of each case is frozen by hash before any score is computed (`run_b3.py` refuses a changed configuration); B2-1's configuration was refrozen once, before any score, to add the loop-to-parameter mapping (logged in `results/b2-1/FREEZE_LOG.md`).

## 7. Known risks and what was not controlled

- **Models are reconstructions.** B2-1 is rebuilt from a diagram; B2-2 is a third-party rebuild checked only through two tables of one paper.
- **Choices made after a first look.** The scoring definitions of B2-1 (the 0.5 month window and the slowing-down interval) were fixed after viewing the sequence of dominant loops of the full model, before scoring. The loop identification and the criteria M2, M3, M5 and M6 of B2-2 were written after seeing the loop list and shares of an exploratory run (needed to map loops of our model to the paper's); their thresholds are generous and fixed in the frozen configuration. The reader should weigh the 7 of 7 and the 3 of 4 with this in mind.
- **Qualitative keys.** All entries of the answer keys except the half-cycle durations are qualitative; agreement is a yes or no on a stated direction.
- **Lever ranges and goals are this benchmark's choice**, not the literature's.
- **Papers not read.** Eberlein 1985 and 1989, Saleh et al., Taylor, Ford and Ford 2007 and 2010, Keloharju and Wolstenholme 1986, Thissen 1976 and 1978, Schoenberg et al. 2020, Kampmann and Oliva 2006, Saysel and Barlas 2006: either behind a paywall or behind a verification page that was not bypassed. Their findings are absent from the keys, so B2-3, B2-4 and B2-5 have no score. They are listed in [STATE.md](STATE.md) as needing a manual download.
- **Licences.** The Market Growth rebuild has no stated licence; the Vensim sample models are used as shipped with the programme; no model or paper text is redistributed in the repository.
- **Two stages of loop analysis are linear or local**: eigenvalue elasticity is meaningful near linear phases only (Kampmann and Oliva 2006), and the loop scores depend on how a loop is cut at a stateful function.

## 8. References

Hayward and Roach 2023, ISDC P1092. Mojtahedzadeh 2015, ISDC P1307. Phaff et al. 2008, ISDC PHAFF116. Hekimoğlu and Barlas 2010, ISDC P1260 (read, not used for a score). Forrester 1968, Industrial Management Review 9(2). Schoenberg, Davidsen and Eberlein 2020, SDR 36(2), 158 to 190 (method as described by Hayward and Roach 2023). SALib 1.6.0 (cross-check of Morris and Sobol to 1e-9 on identical designs). Links in [BRIEF.md](BRIEF.md) section 9; fetch status in `papers/INDEX.md` (local, not committed).
