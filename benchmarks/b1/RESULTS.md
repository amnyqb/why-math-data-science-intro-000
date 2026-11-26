# B1 results: the beer game on M5 (19 November 2026)

Everything below is reproducible from this folder (`README.md`). Three reports hold the detail: [pilot](B1_pilot_report.md), [M5](B1_M5_report.md), [forecasts](B1_forecasts.md). The success criteria were frozen first ([CRITERIA.md](CRITERIA.md)).

## What was run

A four-stage supply chain (retailer, wholesaler, distributor, factory; the board game's delays and weekly step order; holding 0.5 and backlog 1 per case per week) as a Dynamist model with in-file tests, driven by **real M5 demand**: 70 store-by-department weekly series of 277 weeks, each scaled to 100 cases a week by its training mean. Policies were tuned on weeks 0 to 224 and frozen; the **last 52 weeks are the test window**.

| Policy | What it is |
|---|---|
| P0 | the human-like anchoring-and-adjustment rule with the **mean** of Sterman's estimated parameters (theta 0.36, alpha 0.26, beta 0.34, S' 17; Table 4 of the working paper D-3919, which the model's "placeholder" values turned out to equal; the reports below still say placeholder) |
| P1 | installation base-stock, formula levels and levels tuned on the training window |
| P2 | the same SD rule as P0 with its four parameters per stage tuned (16 in all) at the same search effort |

## Headline (M5, all four stages use the policy)

*Re-run on 19 November 2026 after the model was rebuilt to follow the board game's weekly steps (R1 below). Every number here changed; the earlier ones are in git history (commit 9b282ee).*

| | median chain cost | cheapest on (of 70) | fill rate | bullwhip at the factory |
|---|---|---|---|---|
| P0 human-like (Table 4 means) | 29,618 | 4 | 0.999 | 31 |
| **P1 base-stock, formula levels** | **10,358** | **43** | 0.985 | 0.96 |
| P1 base-stock, tuned | 16,442 | 11 | 0.975 | 0.96 |
| P2 SD rule, tuned | 19,676 | 12 | 0.956 | 1.65 |

- **P2 against tuned P1: median +1.5%** (90% of series between -36.1% and +97.8%; P2 cheaper on 46%; Wilcoxon p = 0.15). By the pre-fixed reading this is **parity**, with a very wide spread.
- **Tuning hurt on the test year.** Tuned base-stock costs 58.8% more than formula base-stock (median; cheaper on only 23% of series), and the formula levels are the best policy here. Before the rebuild, tuning helped by 55.8%. The cause found so far: in the test year, M5 demand runs 20 to 60% above the training mean on most series. The rebuilt model receives before it ships, so tuning on the training years picks lower levels (about 430 against the formula's 520), and those run into backlog when demand rises. The damage grows with the demand rise (Spearman 0.40 across the 70 series). The formula's levels are higher and hedge the trend by accident. This is a finding about tuning on non-stationary demand, not about the SD rule.
- **Cost of misperception** (P2 against the human rule with Table 4 means): -55.2% median, cheaper on 83% of series.
- One stage tuning with the others fixed (retailer): against base-stock others +0.8% (p = 0.07); against human-like others -3.5% (P2 cheaper on 77%, p = 7.8e-08).

## Pilot on synthetic i.i.d. demand (100 seeds)

On demand with no trend or seasonality the tuned SD rule beat tuned base-stock by **24.0%** (median; winning on 83% of seeds, p = 1.5e-13); tuned base-stock beat formula base-stock by 45.0%. The SD rule smooths orders (factory bullwhip 0.30), so upstream stages absorb the retailer's variation, while installation base-stock passes every order on (bullwhip 1.05). **On real M5 demand the advantage disappears.** The criteria say a win above 2% is suspected first; the search, window and leakage checks in the pilot report found nothing, but this is a comparison against installation base-stock, not the optimum. A first run of this pilot was void (seeds did nothing, an engine bug, fixed; see CRITERIA.md section 9).

## Forecasts (F1 against F2, rolling origin, test window)

`augurs` AutoETS (non-seasonal) beats the seasonal-naive floor on point accuracy on 86% of series (median RMSSE 1.13 against 1.52, -28%) and on scaled pinball loss on 77% (-35%). Seasonal naive copies one noisy year-ago week; F2 has no seasonality at all (a limit of the crate version). The forecasts are scored only; **no policy used them**.

## Checks

In-file tests T1 (chain balance), T2 (equilibrium), T3 (amplification) and never-negative tests pass on the rebuilt model; strict units are clean; T4 (base-stock does not amplify) gives bullwhip 1.05 at every stage (whole-case rounding adds a little variance); two runs give byte-identical result files; no tuned beta sits at its bound; the last 52 weeks cost about twice as much per week as the training average, for every policy.

## Not done, and why

- **R1 (Sterman replication): cost passes, shape is mixed.** The paper was read, and the simulator now follows the board game's weekly steps in order: receive, ship, backlog, expectations, pass orders, then order. Orders are held at 4 for weeks 1 to 4 and are whole cases. The rebuilt Dynamist model reproduces Sterman's **benchmark exactly (46/50/54/54, team 204)**. With the 11 teams' Table 4 parameters the mean team cost is **1789 against Sterman's 2028 (12% below, within the 15% tolerance)**; by stage 404/559/504/322 against 383/635/630/380 (`r1_sterman.py`, `results/r1_sterman_output.txt`).
  - **Cross-check:** `r1_board.py`, a line-by-line Python port of Edali & Yasarcan's (2014) exact board-game replica (CoMSES 4163, GPL-3.0), gives the same numbers to the decimal (`results/r1_board_output.txt`).
  - **What was wrong before:** the old model shipped from start-of-week stock and charged cost on it, let the rule act from week 1, and gave the factory one production stage too few. It cost 5733 for the teams and 3012 for the benchmark. In the replica, letting the rule act from week 1 alone raises the teams' cost to 2160, and dropping the supplier's backlog from the supply line raises the benchmark from 204 to 1052.
  - **Shape is weaker than the people's:** mean order variance rises upstream (8/13/23/34 against the paper's 13/23/45/72), but team by team it rises in 6 of 11, and peak orders (11/13/15/18) are below the paper's (15/19/27/32). The estimated rule is smoother than the players it was fitted to, which is expected of a fitted rule, so no claim is made that it reproduces the players' extremes.
- **P3 (DQN) and R2: run; both settings agree in sign, S1 not in size.** The authors' code was ported to PyTorch (`dqn/port/`, `dqn/PORT_NOTES.md`) and trained on a Colab T4 for 10,000 games per setting (the paper used 60,100). Logs: `dqn/results/run_gc3.log`, `run_gc7.log`.
  - **S1 (DQN retailer, base-stock co-players):** the DQN retailer's final cost is 10.08. The code's own "base-stock" column (retailer level 8, from the `--f1` default; 29.95) was a poor comparator: an untrained network that just passes demand through already scores 15.26. A sweep of the retailer level on the same simulator and 100 test games (`dqn/results/bs_sweep_*.txt`; others held at the code's levels 8, 0, 0) puts the best level at about 56, with cost 6.94 (7.07 at 54, 6.98 at 58, 8.12 at 64, 29.95 at 8). Against that, the DQN is **45% worse**. R2 S1 passes on sign (not better than base-stock), but the paper's gap is +8.15%, so the size is not matched; training here is 10,000 games against 60,100. The sweep used the test games themselves, which favours base-stock slightly.
  - **S2 (DQN retailer, Sterman co-players):** final chain cost 32.29 against 41.46 for base-stock, 22% lower (paper: 18.23% lower). Same sign, similar size, but the checkpoints swing between 29.5 and 44.8 from one 1,000-game checkpoint to the next, so one number is not a result. The paper's exact gap definition has not been checked against the code's columns.
  - The port has not been checked against the authors' published numbers, and training is shortened. Treat these as an unreplicated pilot.
- **R3: passed in a narrower form; the original form is blocked.** `r3_check.py` recomputes the seasonal-naive RMSSE of all 70 series independently (from the raw daily files, with its own weekly aggregation and the Guide's first-non-zero rule) and matches `F1_RMSSE` in `forecast_scores.csv` to a largest relative difference of 7.95e-07, which is CSV rounding (details: `results/r3_check.csv`). It does **not** check F2 (AutoETS), the pinball loss, or the official daily WRMSSE. The original R3, against the organisers' `validation` code, could not be run: the repository address I tried returns 404, and `data/raw/` has no `sell_prices.csv`, which the official weights need. Our scores are weekly RMSSE on 70 aggregated series, not the official daily 28-day score, so no claim is made about the official M5 ranking.
- **Forecasts as policy inputs**, echelon base-stock, and a seasonal F2: not run.
- **M5 licence for redistributing aggregates:** not found; no data is committed.

## Pictures

[charts/](charts/): `m5_cost_s3.png`, `m5_paired.png`, `m5_bullwhip.png`, `m5_one_series.png`, `step_orders.png`, `cost_s3.png`, `bullwhip_s3.png`, `paired_s3.png`, `positions_s1_s2.png`. App screenshots (not committed): `shots/b1/`.
