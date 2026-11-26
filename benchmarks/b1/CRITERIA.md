# Benchmark B1, "Supply line on M5": success criteria

*Frozen 19 November 2026 (see section 9). Before the freeze this was a draft; the owner then said "do all in the order you find best", so the **[proposal]** values below stand as written and nothing has been computed yet. After the freeze, every change goes in section 9 with its date and reason, and the report says so. Items marked **[proposal]** are mine and need your decision; nothing is invented about what the cited papers report: those numbers are read from the papers' tables when the replication is done.*

## 1. The question

Can a system dynamics ordering rule, tuned in Dynamist on a training window, compete on total cost with an installation base-stock policy and a published deep reinforcement learning agent, in a four-stage serial chain driven by M5 retail demand, and does it explain the behaviour the others hide?

"Compete" is defined in section 5. The benchmark is a success whether or not the SD rule wins, provided sections 3 and 4 are met.

## 2. What is fixed before results

- **Chain:** four stages (retailer, wholesaler, distributor, factory), classic delays and costs as in Sterman (1989), read from the paper and recorded in the manifest. Euler, DT = 1 week.
- **Policies:** P0 human-like rule with the paper's estimated parameters; P1 base-stock; P2 tuned SD rule (θ, α_S, β, S′); P3 the published DQN agent; P4 optional.
- **Fairness rule for P1 and P2:** P1's base-stock level is formula-based **and** re-tuned on the same training window and the same search budget as P2. The report shows both. A P2-versus-P1 result counts only against the tuned P1.
- **Windows:** tuning and fitting on the earlier part of each series, test on the last 52 weeks (or what remains after the lengths are confirmed). No parameter is changed after seeing a test result.
- **Demand:** M5 aggregated to store by department, weekly, scaled to a mean of 100 cases (the unscaled results are reported too). Forecast inputs F1 (seasonal naive) and F2 (AutoETS); F0 (oracle) is an upper bound, never a policy input.
- **Scenarios:** S1 others on base-stock, S2 others on the human-like rule, S3 all four on the same policy. Demand regimes: step 4 to 8, i.i.d., M5.
- **Metrics:** total cost (holding plus backlog) over the test window (primary); bullwhip ratio per stage; fill rate; average backlog; forecast RMSSE and scaled pinball loss.

## 3. Gate A: the infrastructure must be sound (pass or fail, independent of performance)

| # | Criterion |
|---|---|
| A1 | Model tests T1 to T5 pass on every run, with zero validation errors; `dyn validate --strict-units` is clean. |
| A2 | T1 is a `balance` check over the whole chain; T2 and T3 are `reference` checks; no check is weakened to pass. |
| A3 | The whole benchmark reruns from the repository files, with fixed seeds, and gives byte-identical result CSVs on a second run. |
| A4 | The manifest validates and names every lever, output, scenario, data binding and test. |
| A5 | The data licence for M5 is stated; if redistribution is not allowed, no data is committed and the pipeline script reproduces the aggregates. |
| A6 | Every Dynamist gap met during the work is listed in the report (that list is a deliverable). |
| A7 | The report states plainly that M5 is sales, not orders, and that costs are simulated. |

## 4. Gate B: replication before any M5 result (stop if one fails)

| # | Criterion | Pass rule |
|---|---|---|
| R1 | Sterman 1989, classic step | The qualitative pattern: order variance rises at each stage going upstream; the peak of orders comes later at each stage upstream; inventory swings below zero (backlog) before recovering. If the paper tabulates costs for its estimated rule, ours are within **[proposal] 15%** of the tabulated value, read from the cited table. |
| R2 | Oroojlooyjadid et al. 2022, on their code and settings | S1: the DQN's cost is within the paper's reported gap of base-stock (read from its table); S2: the DQN's cost is below base-stock. |
| R3 | M5 benchmarks | Our seasonal-naive and ETS scores on the official 28-day window equal those of the M5-methods `validation` code, at the same level and metric, to **[proposal] a relative 1e-6** (or the difference is explained). |

If a gate fails, the report stops there, states the gap and its cause, and the M5 main results are not run.

## 5. Main results: what is reported, and what "competitive" means

For each scenario (S1, S2, S3) and each demand regime, over all series:

- median and interquartile range of total cost per policy; the share of series each policy wins;
- the paired comparison **P2 versus tuned P1** and **P2 versus P3**: median relative cost difference, a 90% interval, the Hodges-Lehmann estimate, a Wilcoxon signed-rank p-value and the effect size. These two comparisons per scenario are the only confirmatory tests; p-values are Holm-corrected across them. Everything else is descriptive.

**"The SD rule is competitive"** means: the median relative cost of P2 against tuned P1 is within **[proposal] +5%**, and against P3 within **[proposal] +5%**, in S1 to S3 on the M5 regime.

Interpretation fixed in advance:

| Outcome | What it means, and what the report must do |
|---|---|
| P2 within ±2% of tuned P1 | Expected: with β near 1 the SD rule is an order-up-to rule. Report it as parity, not as superiority. |
| P2 worse than tuned P1 by more than 5% | The tuned SD rule loses. Report where and why (which series, which regime). |
| P2 better than tuned P1 by more than 2% | Suspect first. Check the base-stock level, the tuning budget and window, and leakage before reporting; rerun with a larger P1 search. |
| P2 worse than P3 | Report; state whether the gap is in cost, bullwhip or fill rate. |

## 6. The questions that matter beyond win or loss (always reported)

1. **Cost of misperception:** (P0 − P2) / P0 on the same series: how much of the cost is the underweighted supply line.
2. **β found by tuning:** does the optimiser move β towards 1, and by how much does the bullwhip fall when it does?
3. **Whole-chain behaviour (S3):** bullwhip ratio by stage for P0, P1, P2, P3.
4. **Forecast quality:** does F2 beat F1 (RMSSE, pinball loss)? If not, say so. Does a better forecast lower cost for P1 and P2?
5. **Sensitivity:** the cost range of P2 over ±10% on the key levers (fixed seed), so the reader sees how fragile the tuned values are.

## 7. What would make B1 uninformative

- a rival tuned less than P2, or on different data or a different budget;
- any use of test-window values in tuning, forecasting origin or scaling;
- an untested simulator: R1 and the tests must pass first;
- a result reported without the number of series, the window and the seed;
- an average reported without the distribution.

## 8. Reporting commitments

Negative and null results are published. Every deviation from this file is listed in section 9. The report names the commit of this file that was in force. Claims stay inside the evidence: B1 supports statements about this chain, these policies and this data, not about system dynamics in general.

## 9. Freeze record and deviations

**Freeze, 19 November 2026.** Tolerances accepted by delegation (R1 cost 15%, R3 1e-6, competitive +5%, parity ±2%); test window 52 weeks or what remains; aggregation store by department pending the confirmed series count; P3 added after the first pilot; a run on synthetic demand is named **B1-pilot** and makes no claim about M5.

**Targets read for R2** (Oroojlooyjadid et al. 2022, v4 HTML of arXiv 1708.05924, read through an agent; not yet checked against the PDF): Table 3, average cost gap of the single DQN agent against the base-stock agent in the same position, in the order U[0,8] / N(10,2²) / C(4,8): base-stock co-players +8.15% / +5.80% / +41.50% (the DQN is worse, and on the classic C(4,8) the absolute costs are 0.49 against 0.34); human-rule (Sterman) co-players −18.23% / −14.47% / −34.96% (the DQN is better); random co-players +9.42% / +17.22% / +8.17%. Section 4.1.1 reports a 2.31% gap on its first setting. The R2 pass rule is therefore: S1 the DQN is not better than base-stock and the gap has the same sign; S2 the DQN is better than base-stock. Their simulator: four stages, information and shipment lead times 2 (the manufacturer's shipment lead 1 in Section 4.2), holding cost 0.5 and backlog cost 1 for U[0,8] and C(4,8).

**Open at the freeze:** Sterman's (1989) parameters and costs could not be read; resolved by deviation 2 below (the paper was read; R1 cost clause fails).

**Deviations after the freeze:**

0. *19 November 2026.* **The first pilot run was void and was discarded.** The engine's `RANDOM NORMAL` and `RANDOM UNIFORM` ignored their seed argument, so all 100 test seeds produced the same demand and every statistic (interquartile ranges of zero width, p-values of 1.55e-23) was meaningless. It was caught by reading the report, not by a test. The engine now starts an independent, repeatable stream from the seed at each call site (a test proves it), and the harness refuses to report statistics when the test seeds give identical costs. No criterion changed; all results below come from the corrected engine. This also changes the numbers of any older model that used these functions.

1. *19 November 2026.* The in-file check "peaks come later going upstream" (part of T3) was **removed from the model file** and moved to the R1 replication. Reason: with the placeholder human-rule parameters the distributor's highest order comes one week before the wholesaler's, so the check fails, and it must not be weakened to pass. The helper variable `Peaks later upstream` stays in the model and R1 asserts it with the paper's parameters. The in-file T3 check on amplification (the highest order grows at every stage going upstream) stays.

2. *19 November 2026.* **Sterman's paper was read (MIT Sloan working paper D-3919, "Misperceptions of Feedback in Dynamic Decision Making", uploaded by the user); R1 was run (`r1_sterman.py`, output in `results/r1_sterman_output.txt` and `r1_sterman_teams.csv`) and its cost clause FAILS.** Facts read: board in Figure 2 (12 cases on hand, 4 in every pipe, shipping delay 2 boxes, order slip 2 boxes, factory production delay 2 boxes), customer demand 4 to 8 in week 5, about 36 weeks; costs 0.50 per case on hand and 1.00 per case in backlog per week (p. 11); rule IO = Le + alpha(S' - S - beta SL) with Le(t) = theta L(t-1) + (1-theta) Le(t-1) (eqs. 10-15); Table 4 estimates for the 11 teams (mean theta 0.36, alpha 0.26, beta 0.34, S' 17: the pilot's "placeholders from memory" turn out to equal these means); Table 2 team cost 2028 against a benchmark of 204; Table 3 peaks 15/19/27/32 and variance 13/23/45/72; "optimal" parameters theta 0, alpha 1, beta 1, S' 28 (20 for the factory), note 9, p. 28. Result with the 44 estimated parameter sets in the model: **shape passes** (order variance rises upstream in 9 of 11 teams, the highest order rises upstream in 7 of 11, every stage goes into backlog in 9 of 11; peak orders 15.0/20.4/32.3/39.7 against 15/19/27/32); **magnitude does not**: team cost 5733 against 2028 (2.8 times), order variance 2 to 6 times the paper's, deepest backlog about 2 times, and the paper's own optimal parameters cost 3012 in this model against 204 (a coarse search over one common parameter set finds no team cost below 874). The 15% cost clause is therefore not met and is **not** relaxed. Cause unknown; candidates, none tested: the paper's cost accounting (what is summed, over which weeks), the exact order of the weekly steps, and the length of the supply line (the text says the lag is never less than 4 weeks, S' = 28 implies 16 in the pipe, the factory's S' = 20 implies 8, while p. 21 calls the factory's supply line three weeks). **Follow-up test (same day):** the supply-line length was varied (shipping 1 to 3, order slip 1 to 2, production 1 to 3, nine combinations) with the paper's optimal parameters: team cost ranged from 696 (all delays 1) to 3576, never near 204. Holding the starting 12 cases at four stages for 36 weeks costs 4 x 216 = 864 by itself, so a benchmark of 204 is not reachable under the stated 0.50 holding cost, a 12-case start and 36 counted weeks; the paper's cost accounting must differ from that reading (not recoverable from the text). Delay length is therefore ruled out as the cause. Consequence: the pilot and M5 numbers keep their meaning as comparisons between policies inside this simulator, but **no claim is made that the simulator reproduces Sterman's costs**, and the "cost of misperception" is a statement about the human-like rule in this simulator.

   **Resolved (19 November 2026, later):** the cause was the simulator, not the paper's cost accounting. Edali & Yasarcan (2014, JASSS 17(4) 2; R code on CoMSES, codebase 4163, GPL-3.0) give an exact replica of the board game that reproduces Sterman's benchmark. Ported line by line (`r1_board.py`), it gives the benchmark exactly (46/50/54/54, team 204). The B1 model was then rebuilt to follow the board's steps in order (`build_model.py`). Within a week it receives, ships from what is then on hand, updates backlog and inventory, forms expectations, passes orders, and orders on the end-of-week stocks, charging cost on them too. Orders are held at 4 for weeks 1 to 4 and are rounded to whole cases. The factory has a production slip plus 2 weeks. The rebuilt model matches the replica to the decimal: benchmark 204, and the teams' mean cost 1789 against Sterman's 2028 (12% below), so **R1's cost clause passes** at the frozen 15%. The earlier statement that 204 was unreachable was wrong: it assumed costs on start-of-week stocks with the rule active from week 1. Every pilot and M5 number was re-run on the rebuilt model. The customer-demand step now comes in board week 5, one DT later than before.

3. *19 November 2026.* **R2 was run on a PyTorch port, with shortened training.** The authors' TensorFlow 1.15 code cannot run here, so it was ported (`dqn/port/`, `dqn/PORT_NOTES.md`) and trained on a Colab T4 for 10,000 games per setting, against the paper's 60,100. The port has not been checked against the authors' published numbers. Outcome (`dqn/results/run_gc3.log`, `run_gc7.log`; `RESULTS.md`): **S1 passes on sign, not on size**: the code's base-stock column used an untuned retailer level of 8 (cost 29.95), against which the DQN (10.08) looked 66% better; against the best level found by a sweep, about 56 (cost 6.94, `dqn/results/bs_sweep_*.txt`, which used the test games), the DQN is 45% worse, where the paper has it 8.15% worse and R2 requires "not better". The size gap is probably from the shortened training, which is untested. **S2 passes on sign**: chain cost 22% below base-stock (paper 18.23%), but checkpoints swing between 29.5 and 44.8, and the paper's gap definition has not been matched to the code's columns. No tolerance was changed.

4. *19 November 2026.* **R3 was run in a narrower form.** The organisers' `validation` code could not be read (repository address returns 404) and `sell_prices.csv` is not in `data/raw/`, so the official weighted score cannot be computed. R3 was instead checked by `r3_check.py`, an independent recomputation of the seasonal-naive RMSSE of all 70 weekly series from the raw daily files; the largest relative difference to `forecast_scores.csv` is 7.95e-07, within the 1e-6 tolerance. It covers F1 only, not F2, the pinball loss or the official daily metric, and no claim is made about the official M5 ranking.

## 10. Decisions needed from you before the freeze

1. The three **[proposal]** tolerances: R1 cost (15%), R3 score (1e-6), "competitive" (+5%), and the ±2% parity band.
2. The test window: 52 weeks, or what remains once the series lengths are confirmed.
3. The aggregation level (store by department) once the number of series is confirmed.
4. Whether P3 (the DQN) is in the first run or added later; R2 needs it.
5. Whether the report may use the word "benchmark" for a first version with synthetic demand only (my suggestion: call that "B1-pilot").
