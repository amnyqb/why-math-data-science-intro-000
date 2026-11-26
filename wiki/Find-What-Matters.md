# Find what matters

Benchmark **B3** (`benchmarks/b3/`) tests five functions that answer "what matters in this model?". This page
documents them as they are built. The work order is [`benchmarks/b3/EXEC-PLAN.md`](../benchmarks/b3/EXEC-PLAN.md).

Every function reads two things from the model's **manifest**: the **levers** (constants with a `min` and `max`: the
range each is varied over) and the **goals** (what a study is after, declared before any analysis is run).

```json
"goals": [{"name": "Backlog at the end", "output": "Backlog", "measure": "final", "direction": "min"}]
```

Measures: `final`, `peak`, `lowest`, `total`, `average`, `mean:FROM-TO`, `at:TIME`. Direction: `min`, `max`, `target:VALUE`.
`policies` names manifest scenarios whose ranking a simplified model must keep (`protected_ranking`).

## F1 `screen` (Morris)

`dyn screen FILE` (also the `screen` tool of `dyn mcp` and `Model.screen()` in Python). Each lever is scaled to 0..1 over its
declared range; `r` trajectories move one lever at a time by `delta = p / (2 (p - 1))` (`p` = `--levels`, default 4).
Per goal and lever:

| Field | Meaning |
|---|---|
| `mu` | mean effect of moving the lever across its range (sign shows direction) |
| `mu_star` | mean absolute effect: the importance |
| `sigma` | sample standard deviation of the effects: large when the effect depends on the lever's value or on other levers |
| `class` | `negligible` (`mu_star` below `--tau` × the largest, default 0.1), `linear` (`sigma / mu_star` below 0.1), otherwise `nonlinear_or_interacting` |
| `converged` | the ranking with `r` trajectories agrees with the ranking with `2r` (Kendall tau at least 0.9) |

The cost is `2r (k + 1)` runs for `k` levers, however many goals there are. A lever with no declared range is refused. A
ranking that has not converged is reported as such: raise `--trajectories` before relying on it.

Tested against SALib on the same design (agreement to 1e-9) on the Ishigami function, and on additive and interacting
formula models (`benchmarks/b3/tests/`).

## F2 `importance`

`dyn importance FILE` (tool `importance`, `Model.importance()`). Pass the levers that survived `screen`.

- **Sobol indices** per goal and lever: `s1` (the share of the goal's variance the lever explains alone) and `st` (including
  its interactions), each with a bootstrap interval (`*_conf`, 95%). `sum_s1` well below 1 means interactions matter. `converged`
  compares the indices from N base samples with those from 2N (largest change at most 0.05). Cost: `2N (k + 2)` runs.
- **Loop dominance** (Loops That Matter, Schoenberg, Davidsen and Eberlein 2020), for the base run and tallied over an ensemble of
  parameter draws: per loop its polarity (`R`, `B`), the share of time it is the strongest and its mean relative score. Also
  `dyn dominance FILE [--csv OUT]` for the base run alone, with the relative score of every loop at every time as a CSV.
  Every result header (`dyn dominance`, and `loop_scores` in the importance JSON) carries `method`, `unscored_loops`,
  `integration` (always `euler`: with another integration method the scores are refused) and `cap_reached`.
  Limits: Euler integration only; the functions followed and the convention for cutting a loop at each are in
  [Loop scoring through stateful functions](#loop-scoring-through-stateful-functions); loops through any other stateful
  function (NPV, IRR, DEPRECIATE, queue ages...) are listed (`scored: false`) but not scored, and counted in `unscored_loops`.
- **How the loops are found (`method`).** `exhaustive`: every simple loop inside each strongly connected part of the model, when
  there are fewer than 20 000. Otherwise `approximate_strongest_path` (`cap_reached: true`): at 200 evenly spaced times, a
  best-first search from each stock over the link graph, ordered by the product of the link scores' magnitudes, keeps the
  strongest loops (per start: at most 30 loops, 10 000 queue pops, each node expanded at most 6 times); the union is scored over
  the whole run. This is our own best-first variant, **not** the published strongest-path algorithm (its source was not
  available), and it is approximate: it can miss weak loops, relative scores are relative to the loops found, `unscored_loops`
  is a lower bound, and no ensemble tally is made (the loop set differs between runs). On World3-03 (311 variables, over
  20 000 loops) `dyn dominance` finishes in under a second and finds 44 of the 46 loops that a much larger search finds
  dominant at some time. API: `loop_scores` (automatic), `loop_scores_exhaustive`, `loop_scores_strongest`.
- **Eigenvalues** (`--eigen-at T`): the model linearised at time T (numerical Jacobian of the state derivatives), its
  eigenvalues with period and time constant, and the elasticity `(x / λ) ∂λ/∂x` of each eigenvalue to each lever (Kampmann and
  Oliva 2006). Near-linear phases only.
- **removable**: variables with no causal path to any goal output.

Tested: Sobol agrees with SALib on the same sample to 1e-9 and with the analytic Ishigami indices within the intervals;
dominance on exponential growth (one reinforcing loop), goal seeking (one balancing loop) and S-shaped growth (the switch at
`S = K/2`); goal seeking through DELAY FIXED (one balancing loop, relative score exactly -1 once the delayed value moves), TREND
and a QUEUE FIFO (inflow and outflow shares from the closed form); strongest-path recall of 100% of the loops that are dominant at
some time in the exhaustive result on the small models and the workforce-inventory model; eigenvalues of a linear oscillator and their elasticities against the analytic derivative.

## F3 `reduce`

`dyn reduce FILE --out OUT [--log CSV] [--goal G]… [--samples 512] [--trajectories 20] [--ensemble 30] [--batch 4] [--st-threshold 0.01] [--morris-tau 0.9]`.
Makes a **new** model linked to its source (`manifest.reduced_from`); the source file is never touched. `reduce` is a **cut, verify,
add-back loop** behind hard gates.

### Gates (before any cut), per goal

1. **Morris**: the ranking of the parameters from `r` trajectories and from `2r` agree (Kendall tau at least 0.9; effects within 2% of the
   largest count as tied, since equal effects have no ranking). If not, `r` is doubled once; if it still fails, the goal is refused.
2. **Sobol**: total indices are reported with bootstrap intervals; they must lie in [0, 1] plus an estimator tolerance (the larger of 0.05
   and the interval), move by at most 0.05 when N doubles, and keep their ranking when N doubles (Kendall tau at least 0.9 over the
   parameters with an index of 0.01 or more; indices closer than their intervals are tied).
3. If a gate fails and the goal declares `transform: "log"` in the manifest, the goal is read through the natural logarithm (strictly positive
   values only) and the gates are rerun. With no transform declared, or if the gates still fail, **reduction is refused**: exit status 1, a
   `refused` list with the reason per goal, no file written, nothing cut. A refusal for a goal that is strictly positive and heavy tailed
   says to declare the log transform. `reduce` never applies a transform that was not declared.

`transform` is a goal field (`none` or `log`, default none); once declared it applies to every analysis that reads the goal (`screen`,
`importance`, `verify`). Declare `log` for goals that grow exponentially: variance-based importance of a heavy-tailed number is meaningless.

### Cuts, in rank order

1. **Unreachable elements**: no causal path to any goal output (structure).
2. **Pass-through auxiliaries**: a formula that is one other variable or one number (structure, lossless).
3. **Levers** whose total Sobol index is below `--st-threshold` for every goal, by ascending index: fixed at the current value and removed
   (a *parameter fixed*).
4. **Loops** that never dominate are listed as `loop_proposals`, never cut. A proposal is marked `proposed` only when every loop of the
   model is scored; with any unscored loop it is `blocked`: dominance cannot justify a cut across unscored structure.

Cuts are made in batches (all unreachable elements in one, levers `--batch` at a time). After each batch `verify` runs against the full model.
A failing batch is bisected; a single cut that fails is **re-admitted for good**. The loop ends when nothing more can be cut. Levers that
each look negligible but matter together are therefore kept: the cut passes in small groups and fails once enough are gone.

### Output

JSON (and `--log CSV`, one row per cut with a `category` column) separates:

- **`structure_removed`**: elements, stocks, links and loops removed, with every change and its reason;
- **`parameters_fixed`**: parameters fixed because they were inert in the scenarios tested, with `scenarios_tested` named (base run, the Sobol
  design over the declared ranges, the verification ensemble, protected policy scenarios); this is a statement about those scenarios only;
- `headline`: the reduction figure, **structure only** (parameters fixed are not counted);
- `readmitted` and `cuts_tried` (each cut with its batch and outcome), `verifications` (calls to `verify`) and `verification_runs` (model runs),
  `verdict` of the final model, `gates` (per goal), `refused`, `loop_proposals`, `before` and `after` counts, `sobol`.

Not done: aggregation of chains of identical delays, and removal of loops. The reduced model has passed `verify` inside the loop.

## F4 `verify_reduction`

`dyn verify FULL REDUCED [--ensemble 50] [--max-nrmse 0.05]` (exit status 1 on failure). On the declared goals, for the base run and
for an ensemble of draws of the levers both models share:

| Check | Pass when |
|---|---|
| error of each goal's output series | normalised RMSE (by the full model's range) at most 0.05 for the base run and on average; Theil's `um`, `us`, `uc` (bias, variance, covariance) say what kind of error |
| behaviour mode | the mode (flat, exponential growth, goal seeking, S-shaped, overshoot and settle or collapse, damped, sustained or growing oscillation) is the same in at least 90% of the runs |
| goal values | Kendall tau of the goal values over the ensemble at least 0.9 |
| policy ranking | for goals with `protected_ranking`, the ranking of the named scenarios is identical (tau 1) |
| checks | the reduced model's own checks hold |

Tested on a model built with a dead branch, an inert parameter and a pass-through auxiliary (exactly those are removed, four as structure and one as a parameter, and the
result verifies), on twenty small levers that add up (re-admitted), on an exponential-growth goal (refused without `transform`, reduced with `log`), and on a deliberately wrong reduction (a parameter that matters fixed at a wrong value), which is refused with the errors.

## F5 `recommend_structures`

`dyn recommend FILE --role flow=Orders [--role KEY=NAME]… [--structure supply_line]… [--ensemble 20]`. Library version 2; every
structure is a parameterised batch of operations tied to a real decision. **Nothing is applied**: the answer is a list of proposals.

**Roles.** Name only the decision flow (`--role flow=Orders`). The other roles are read off the structure of the model where possible,
and the answer's `roles` lists every role used with how it was found (`given` or `detected: …`):

| Role | Detected as |
|---|---|
| `stock` | the stock the flow ultimately feeds (following dependents through flows) and reads back; informational |
| `arrival`, `delay` | a `DELAY*` or `SMOOTH*` function whose input is on the path from the flow to that stock, or a stage (an outflow `Stock / Time` of a stock on the path): the variable holding it, and its named delay time |
| `time` | the constant in a `(gap involving the stock) / constant` pattern in the flow's equation |
| `target` | the constant in `target - stock` (or `stock - target`) inside that gap |
| `perceived`, `actual` | a variable read by the flow that is a `SMOOTH*`/`DELAY*` of another variable, and that other variable |

A role that cannot be detected (`input`, any pattern the flow's equation does not follow, a gap hidden in an auxiliary, a delay
time written as a literal) is asked for: the structure is skipped with the reason and the `--role` to pass. `--role` always overrides
detection. With several delay stages in a chain only the first one found on the path is reported.

| Structure | Decision it stands for | Roles | Source |
|---|---|---|---|
| `supply_line` | count what is already ordered or in transit before ordering more | `flow`, `arrival`, `delay`, `time` | Sterman (2000) *Business Dynamics*, ch. 17 |
| `forecast` | act on an expected value (smoothed trend projected ahead, `EXPECT`) instead of the last observation | `flow`, `input`, `time` | Sterman (2000), expectation formation / forecasting (chapter number to be verified) |
| `information_link` | give the decision-maker the actual value instead of the perceived one | `flow`, `perceived`, `actual` | to be recorded (`unsourced`) |
| `buffer` | aim above the target (safety stock, spare capacity) | `flow`, `target` | to be recorded (`unsourced`) |
| `delay_change` | make a process faster or slower | `delay` | to be recorded (`unsourced`) |

Every template carries a `source`; a structure without a recorded source says `to be recorded` and is flagged `unsourced: true` in the
output. The library is extended only with structures that have a recorded source.

**Improvement is scale-free.** For each goal and ensemble draw the change in the goal's score is divided by the spread (standard
deviation) of that goal's base-run scores over the ensemble, or by |target| when the direction is `target:VALUE`, not by the base
value (which breaks down near zero and changes when the output is shifted). The weight of each structure is searched on a grid over
an ensemble of draws of the manifest's levers to maximise the mean improvement over the goals (weighted when weights are declared).

**Several goals: a Pareto set, not one list.** Per proposal: the weight, the mean improvement of every goal, **robustness** (the
share of draws in which it beats the model as it is), the loops before and after (reinforcing, balancing), `plausible: false` when
the best weight lies outside what a real decision could be (for example a buffer above 50%), `source`/`unsourced`, and
`pareto: true` when no other proposal is at least as good on every goal and better on one (doing nothing counts as a proposal with
improvement 0). Proposals are listed Pareto set first; that order is a reading order, not a ranking. A single ranked list (`ranking`,
with `weighted_improvement` on each proposal) is returned **only when every goal in the manifest declares a `weight`** (a positive
number, optional, default none). Weights are the study's own statement of what matters more; the tool never invents them.

The `operations` of a proposal can be applied with `dyn ops` after a person has checked it.

Tested on a stock controlled by an order rule with a delivery delay: with only `--role flow=Orders` the arrival, delay, adjustment
time and target are detected; `supply_line` helps in every draw and adds a balancing loop; the two-goal tradeoff (a buffer wins the
trough and loses the peak, the supply line wins the peak) gives a Pareto set containing both; shifting a goal's output by an offset
does not change the improvements; a ranking appears only with weights.
Limits: the spread is estimated from the base-run ensemble (a goal that does not vary over the levers falls back to |mean|, then 1);
one weight per structure is reported (the best on the mean), so a structure's own tradeoff along its weight is not shown; the model
is assumed to start in balance (`supply_line`); `input` is never detected.
Tested on a stock controlled by an order rule with a delivery delay: `supply_line` comes first, adds a loop, helps in every draw.
Limits: the improvement measure is the relative change of each goal's score, so goals with very different scales weigh differently;
the model is assumed to start in balance (`supply_line`).

## Loop scoring through stateful functions

Link scores are defined for Euler integration. A function with state is turned into hidden stocks and plain links so that every
loop through it is scored; each loop is cut at the function as follows.

| Function | Convention |
|---|---|
| SMOOTH, SMOOTH3, DELAY1, DELAY3 | hidden stocks (one per stage): input link = the input's share of the change in the stage's net flow, the stage's own draining term is a negative self link; the last stage feeds the output |
| TREND(input, time, init) | one hidden stock (the average of the input, as SMOOTH); the output is computed from the input, the averaging time and the average, so it has a direct link from each of them and a link from the average stage. A loop through TREND therefore has both a direct and an averaged path |
| QUEUE FIFO(inflow, outflow, init) | the queue is a level: a hidden stock whose net flow is the inflow less the outflow that actually leaves. The variables read by the inflow are linked with their share of the change in that net flow (positive), those read by the outflow with the negative share. The cap on the outflow (what is waiting) is attributed to the outflow link |
| DELAY FIXED(input, delay, init) and the conveyor `DELAY(..., exact=True, leak=...)` | the loop is **cut at the delay**: the output at t is the input one delay earlier, so its whole change is attributed to the input with score +1 (the delay is a positive link), shared among the variables the input expression reads in proportion to their share of the change of the expression at the time the output changes. The score is 0 while the output does not change (before the first delay has passed). The timing lag itself is not in the score: a loop scored through a delay is credited when the output moves, not when the input moved |
| QUEUE AGE AVERAGE, QUEUE AGE OLDEST, QUEUE AGE IN RANGE | not followed (age depends on the whole history): loops through them are `unscored` |
| NPV, NPVE, IRR, DERIVN, DEPRECIATE STRAIGHTLINE, SAMPLE IF TRUE, INITIAL | not followed: `unscored` |

Known limits: if one variable feeds both the inflow and the outflow of a queue, or appears both directly and through a DELAY
FIXED in the same equation, only one link is kept for the pair (the first in the order inflow, outflow, direct, delayed); a
DELAY FIXED nested directly inside the input of another DELAY FIXED is not followed. The delay time, initial value and
averaging time of SMOOTH, DELAY and DELAY FIXED are not links of the loop scores (TREND's averaging time is).
