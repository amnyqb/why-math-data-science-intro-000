# Dynamist tools inside sd-ai's agent: the with-and-without runs on development groups

Round 8, B73 and B74. 23 November 2026. Written failures and losses first, then the method, then the numbers.

## What did not work, and what is not finished

1. **The judge is DeepSeek, not sd-ai's Gemini, for every formulationCritique number and for the few judged extremeConditions rows, so none of these numbers is comparable with sd-ai's published ones.** The runs themselves were graded by sd-ai's judge (`gemini-3.8-flash`, through the Google key in the environment), which was on the free tier and out of quota: 28 of 40 formulationCritique rows per pass, and 2 extremeConditions rows, ended as "Evaluation error", so the first comparison (`compare-gemini-failed.md`) showed 0% for formulationCritique in both arms. That was the quota, not the agents. The stored responses were then re-graded without any agent call, both arms and all six files with the same judge, `deepseek-v4-flash` (DeepSeek's own API), by `evals/scripts/regradeRun.js` in the fork. Two caveats: (a) sd-ai's own DeepSeek route has no JSON-schema mode and names only the top-level fields, so DeepSeek answered the nested schema in the wrong shape (`{"issues":[1,3]}`), which first made every plant read as "not identified" (0 of 72 in both arms again: a second artifact); the script now spells the whole JSON Schema out in the judge's prompt and checks the reply against the category's own schema, and nothing else in the category's grading changed; (b) a different judge reads "identified in substance" differently, so the rates may be higher or lower than Gemini's (Merlin's published formulationCritique rate on Gemini is 0 of 12, here it is 7 of 12 in one pass). No evaluation error remains. DeepSeek key: held in the environment for these runs only, not stored anywhere in the repository, the fork or the logs. Judge cost: not measured (the script does not log it); the number of judge calls was about 180 per file set, small.
2. **The tools did not help on any hard group, and the differences are small against the noise.** extremeConditionsHard: 72.2% without the tools (26 of 36), 66.7% with them (24 of 36), -5.6 points (95% interval -25.8 to +15.3). formulationCritiqueHard: 77.1% (37 of 48) and 75.0% (36 of 48), -2.1 points (-18.9 to +14.9). Plain formulationCritique: 58.3% (42 of 72) in both arms, difference 0 (-15.7 to +15.7). Plain extremeConditions: 97.2% and 100% (+2.8, -2.7 to +9.6). Five tests did worse with the tools and four did better; the biggest loss is Merlin on the clean control of the rural dialysis hard case (2 of 3 to 0 of 3: the response reported a problem in a correct model), the biggest gain Merlin on the clean control of the regional distributor hard case (1 of 3 to 3 of 3). Every interval includes zero. These are three passes of 36 to 72 rows; they do not separate a loss, a gain or no change.
3. **The agents did use the tools, and on the hard group that replaced what-if runs.** The prompts of these tests name `run_scenario`, and the grader of the "found" criterion reads only `run_scenario` calls from the tool log, so a finding of `dynamist_extreme_tests` earns credit only if the agent then reproduces it with `run_scenario`. With the tools on, the agent called a Dynamist tool in 23 of 36 extremeConditions runs and 30 of 42 formulationCritique runs (counting a run once, over all three passes and both agents); there were 98 calls in all (57 `dynamist_check`, 41 `dynamist_extreme_tests`) and none returned an error. (The use table below counts a call once for every result row that shares its run, so its totals are larger. Merlin's calls are logged as `dynamist_check`, Socrates's as `client_dynamist_check`; an earlier version of the comparison script missed the second form and undercounted.) **Why the hard group got worse:** in its 12 runs per arm the agents called `run_scenario` 1.25 times per run without the tools and 0.58 with them, and edited 1.42 and 0.83 times per run, while calling `dynamist_check` 11 times and `dynamist_extreme_tests` 6 times. The grader credits a finding only from `run_scenario` calls, so an agent that let the doctor stand in for its own what-if runs loses the "found" test. On the plain group `run_scenario` calls did not fall (16.8 and 20.1 per run). The loss is concentrated in one case, the regional distributor, and is a few flipped single tests (finds "shipments…": Socrates 3/3 to 2/3, Merlin 1/3 to 0/3; fixes "orders…": 2/3 to 1/3 for both agents); that is a pattern worth testing, not proof.
4. **Not run, by design or by cost:** Opus 5.5 (waits for the owner's approval of the cost); every `*HeldOut` group (none was opened, run or read; the comparison script refuses a file that holds one); the public leaderboard. The held-out run and the leaderboard entry are the owner's decision.
5. **The first smoke attempt failed for a reason outside Dynamist:** PySD was not installed on the laptop, so the agent's `run_scenario` calls failed and the grader crashed ("The correct model failed to simulate"). It cost $0.22. PySD 3.14.0 went into a virtual environment (`.venv`, Python 3.12) in the sd-ai clone, which is outside this repository.
6. **No fork on GitHub.** The work is a branch (`dynamist-tools`) in a local clone of sd-ai outside this repository. Publishing it as a fork is the owner's step.

## What was built (B73)

- Two client tools for sd-ai's agents, `dynamist_check` (the doctor's stages C1 to C4: findings ranked, fixes in words and as data) and `dynamist_extreme_tests` (stage C5, rules X1 to X6), in the fork at `evals/utilities/dynamistTools.js`. They take no arguments and run on the agent's current model. The model is turned into XMILE by sd-ai's own converter (`utilities/SDJsonToXMILE.js`, the one the simulator uses; there is no second converter), written to a temporary file and given to the `dyn` binary named by `DYN_BIN`. Dynamist does not write the model back: the agent applies the fixes in SD-JSON itself. Each result carries the import's loss report.
- A flag, `dynamistTools`, built like `analysisTools`: an eval-only engine parameter on the three agent engines, read in `agent/utilities/AgentEvalRunner.js`. Default off.
- Tests in the fork (run on the server; all pass): with the flag absent, `false` or unset, the agent's client tool list is `[]` and the run's result is byte-identical; with `analysisTools` alone the list is exactly upstream's; with the flag on the two tools follow the analysis tools and the rest of the response is unchanged (`tests/agent/AgentEvalRunnerDynamist.test.js`). Five tests of the tools themselves, one of them with the real binary (`tests/evals/utilities/dynamistTools.test.js`).
- **Loss report of the XMILE import, for every dev test model of the two target categories** (8 distinct models, run through `evals/scripts/dynamistLossReport.js`): nothing the model computes was dropped. Every model lists one dropped item, the empty `<model_units>` block ("not used by Dynamist"); the two regional distributor models also list one approximated item, `STEP` ("Dynamist moves a step to the nearest time step; they agree when the start time falls on the time grid").
- Everything about running the harness here: the agent harness ran on the laptop (node v26, PySD 3.14.0 in a Python 3.12 virtual environment); `DYN_BIN` pointed at a small shell script that copies the model file to the Contabo server and runs the Linux `dyn` there (nothing heavy ran on the laptop; the models are the benchmark's public ones). Docker was not used.

## Method (B74)

- **Experiment files:** `evals/experiments/dynamistDev-off.json` and `dynamistDev-on.json` in the fork. They differ in one line per agent: `"dynamistTools": true` (checked by diffing the two files). Categories: `extremeConditions` (groups `extremeConditions`, `extremeConditionsHard`) and `formulationCritique` (`formulationCritique`, `formulationCritiqueHard`). No `HeldOut` group is named in them.
- **Agents and models:** Merlin and Socrates, both on Claude Sonnet 5.5 (the config of sd-ai's agent leaderboard, copied without change).
- **Passes:** three per arm, one run each (`npm run evals`), the arms run at the same time on the same machine.
- **Baseline:** the arm without the tools, run here at the same sd-ai commit; published results are not used.
- **Pins:** sd-ai commit of both arms `aa39e264df927b621abd4829c096aec937856a88` (branch `dynamist-tools` of the local clone; upstream base `58cfc86`). Dynamist binary built from commit `bb3f250` of branch `round-8`; sha256 of the binary `ceb85d28ae225eab63934870a11924311c71235c8a1e38059dd7dcb5f9d6d80d`. **Rule set at the freeze:** sha256 of `wiki/Model-Doctor.md` `5fd622a5fff363fec23f4beb5a55e34c7e681ef99d72507d7da4c4b3e0623ca6`; git tree of `core/src/doctor` `6aafab237ac25b99c33b6435d869b388f9acb6a4`. No rule was copied from sd-ai's graders and no rule was changed after the first run.
- **Costs** (from the cost files of the runs; USD): without the tools 5.5444, 5.1257 and 4.3401 per pass, 15.0102 in all; with the tools 5.5706, 4.7001 and 4.7961, 15.0668 in all. Smoke tests: 0.22 (failed attempt), 1.10 (flag off, Merlin, the 12 extremeConditions tests, 11 passed), 1.03 (flag on, the same 12 tests, 12 passed, a Dynamist tool called in 1 of the 4 cases). Total about 32.4. The estimate before the runs, from the published dev rows, was 4.64 per pass per arm; no single run was near the 50 limit, so no approval was needed.

## How to rerun the comparison

The result files are in `benchmarks/sdai/results/` (gzipped: `dev-off-pass1..3.json.gz` and `dev-on-pass1..3.json.gz` as the runs wrote them, `dev-*.regraded.json.gz` after the DeepSeek re-grade, and the cost files). Run `benchmarks/sdai/rerun-compare.sh` from the repository root (Python 3, standard library only). It writes `compare-gemini-failed.md` (the files as the runs wrote them) and `compare-deepseek.md` (the same responses re-graded with DeepSeek; the files `dev-*.regraded.json.gz`); the same files give the same text (a test in `scripts/test_sdai_compare.py` checks that, and that a file holding a held-out group is refused). To rerun the agents, check out the fork branch at the commit above, set `DYN_BIN` to a `dyn` built from `round-8`, and run `npm run evals -- -e evals/experiments/dynamistDev-off.json` and the same with `-on`, three times each; each file is one pass.

## The comparison (DeepSeek as the judge for the re-graded rows)

sd-ai commit: aa39e264df927b621abd4829c096aec937856a88
Passes: 3 without the tools, 3 with them. Pass rates pool the passes; the intervals treat every pass of every test as an independent trial, which they are not (a test repeated three times is not three tests), so read them as optimistic.

### Pass rate by category and group (losses first)

| Category | Group | Without: passed | Without: rate (Wilson 95%) | With: passed | With: rate (Wilson 95%) | Difference, points (95%) |
|---|---|---|---|---|---|---|
| extremeConditions | extremeConditionsHard | 26 of 36 | 72.2% (56.0% to 84.2%) | 24 of 36 | 66.7% (50.3% to 79.8%) | -5.6 (-25.8 to +15.3) |
| formulationCritique | formulationCritiqueHard | 37 of 48 | 77.1% (63.5% to 86.7%) | 36 of 48 | 75.0% (61.2% to 85.1%) | -2.1 (-18.9 to +14.9) |
| formulationCritique | formulationCritique | 42 of 72 | 58.3% (46.8% to 69.0%) | 42 of 72 | 58.3% (46.8% to 69.0%) | +0.0 (-15.7 to +15.7) |
| extremeConditions | extremeConditions | 70 of 72 | 97.2% (90.4% to 99.2%) | 72 of 72 | 100.0% (94.9% to 100.0%) | +2.8 (-2.7 to +9.6) |

### Tests that changed

5 test(s) did worse with the tools, 4 did better.

| Change | Engine config | Group | Test | Without | With |
|---|---|---|---|---|---|
| -67 points | merlin-agent-sonnet-5-5 | formulationCritiqueHard | Rural dialysis network (hard) (clean control): finds nothing to fix and leaves the clean model as it is | 2/3 | 0/3 |
| -33 points | socrates-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): finds "shipments keeps flowing after "inventory" runs out" with a what-if run, unprompted | 3/3 | 2/3 |
| -33 points | merlin-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): finds "shipments keeps flowing after "inventory" runs out" with a what-if run, unprompted | 1/3 | 0/3 |
| -33 points | merlin-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): fixes "orders loses the MAX that limits it to 0" without changing normal behavior | 2/3 | 1/3 |
| -33 points | socrates-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): fixes "orders loses the MAX that limits it to 0" without changing normal behavior | 2/3 | 1/3 |
| +67 points | merlin-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard) (clean control): finds nothing to fix and leaves the robust model as it is | 1/3 | 3/3 |
| +33 points | merlin-agent-sonnet-5-5 | formulationCritiqueHard | Rural dialysis network (hard): identifies and fixes desired chairs inlined into ordering chairs unprompted | 2/3 | 3/3 |
| +33 points | merlin-agent-sonnet-5-5 | extremeConditions | Endemic Infection: the revised model survives the extreme run | 2/3 | 3/3 |
| +33 points | merlin-agent-sonnet-5-5 | extremeConditions | Endemic Infection: fixes the formulation without changing normal behavior | 2/3 | 3/3 |

### Failures by type

| Category | Group | Failure type | Without | With |
|---|---|---|---|---|
| extremeConditions | extremeConditions | Formulation not fixed | 1 | 0 |
| extremeConditions | extremeConditions | Model not robust | 1 | 0 |
| extremeConditions | extremeConditionsHard | Correct model changed | 1 | 0 |
| extremeConditions | extremeConditionsHard | Formulation not fixed | 2 | 4 |
| extremeConditions | extremeConditionsHard | Problem not found by testing | 6 | 8 |
| extremeConditions | extremeConditionsHard | Problem reported in a correct model | 2 | 0 |
| formulationCritique | formulationCritique | Formulation not fixed | 30 | 30 |
| formulationCritique | formulationCritiqueHard | Formulation not fixed | 7 | 6 |
| formulationCritique | formulationCritiqueHard | Formulation not identified | 1 | 0 |
| formulationCritique | formulationCritiqueHard | Problem reported in a correct model | 4 | 6 |

### Use of the Dynamist tools

| Arm | Result rows | Rows whose agent called a Dynamist tool | Calls to dynamist_check | Calls to dynamist_extreme_tests |
|---|---|---|---|---|
| Without | 228 | 0 | 0 | 0 |
| With | 228 | 149 | 170 | 103 |

Rows that share one generation (the tests of one case) share its tool calls, so a call counts once for each of those rows.

### Cost

Without the tools: $15.01. With the tools: $15.07. (Rows that reused a sibling's generation are not counted twice.)

The earlier comparison on the runs' own grading, with 0% for formulationCritique because the Gemini judge was out of quota, is `benchmarks/sdai/compare-gemini-failed.md`; it is kept as the record of what the quota did.

## What this does and does not show

- It does not show that the tools help. No group shows a gain; the hard groups show small losses (-5.6 and -2.1 points) with intervals that include zero.
- It does not show that the tools hurt. Three passes of 36 and 72 rows are few, and the passes are not independent.
- It shows that the agents use the tools often (53 of 78 runs), that the tools run without errors on every model they were given, and that on the hard, unprompted group the tools displaced `run_scenario` calls, which is what the grader reads.
- A candidate change, on development groups only and a new freeze (`b74-freeze-2`): make the tools' result something the agent can reproduce directly with `run_scenario`, for example by returning the overrides that expose each finding ("set X to 0, read Y"). That would turn the tools' finding into the run the grader credits. Not done.
- The next step is the owner's: restore the judge quota and re-grade (no agent cost), then decide whether a change to Dynamist is worth making before the held-out run. The tag `b74-freeze` already exists and stays as the record of these runs: any change to Dynamist's rules or messages, or to the tools' output, means a second freeze, `b74-freeze-2`.

## Changes waiting for a second freeze (`b74-freeze-2`)

The enhancement session reports doctor changes on branch `doctor-fixes-3` (tip `3f172bd`, not merged; `b74-freeze`, `round-8` and `master` untouched). None was in the binary the dev runs used. If merged they change what the agent sees through `dynamist_check` and `dynamist_extreme_tests`: X1 and X6 try a constant with a declared range (a lever or a slider) at the two ends of the range only; X6 names a diverging run and reports the worst breached check; a check marked as a validity range or a promise gives information; day and week unit fixes end with "(assuming a 7-day week)"; a `teaching_draft` setting and the figure-model suffix turn X findings to information; the sanity sheet has 14 rows. The `Model-Doctor.md` table changes in the X1 and X6 rows. Any dev or held-out run with them must be reported as a second freeze with its own rule-set hash.
