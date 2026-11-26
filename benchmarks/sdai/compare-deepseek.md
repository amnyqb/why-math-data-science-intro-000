# With and without the Dynamist tools

sd-ai commit: aa39e264df927b621abd4829c096aec937856a88
Passes: 3 without the tools, 3 with them. Pass rates pool the passes; the intervals treat every pass of every test as an independent trial, which they are not (a test repeated three times is not three tests), so read them as optimistic.

## Pass rate by category and group (losses first)

| Category | Group | Without: passed | Without: rate (Wilson 95%) | With: passed | With: rate (Wilson 95%) | Difference, points (95%) |
|---|---|---|---|---|---|---|
| extremeConditions | extremeConditionsHard | 26 of 36 | 72.2% (56.0% to 84.2%) | 24 of 36 | 66.7% (50.3% to 79.8%) | -5.6 (-25.8 to +15.3) |
| formulationCritique | formulationCritiqueHard | 37 of 48 | 77.1% (63.5% to 86.7%) | 36 of 48 | 75.0% (61.2% to 85.1%) | -2.1 (-18.9 to +14.9) |
| formulationCritique | formulationCritique | 42 of 72 | 58.3% (46.8% to 69.0%) | 42 of 72 | 58.3% (46.8% to 69.0%) | +0.0 (-15.7 to +15.7) |
| extremeConditions | extremeConditions | 70 of 72 | 97.2% (90.4% to 99.2%) | 72 of 72 | 100.0% (94.9% to 100.0%) | +2.8 (-2.7 to +9.6) |

## Tests that changed

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

## Failures by type

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

## Use of the Dynamist tools

| Arm | Result rows | Rows whose agent called a Dynamist tool | Calls to dynamist_check | Calls to dynamist_extreme_tests |
|---|---|---|---|---|
| Without | 228 | 0 | 0 | 0 |
| With | 228 | 149 | 170 | 103 |

Rows that share one generation (the tests of one case) share its tool calls, so a call counts once for each of those rows.

## Cost

Without the tools: $15.01. With the tools: $15.07. (Rows that reused a sibling's generation are not counted twice.)
