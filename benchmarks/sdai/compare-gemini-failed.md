# With and without the Dynamist tools

sd-ai commit: aa39e264df927b621abd4829c096aec937856a88
Passes: 3 without the tools, 3 with them. Pass rates pool the passes; the intervals treat every pass of every test as an independent trial, which they are not (a test repeated three times is not three tests), so read them as optimistic.

## Pass rate by category and group (losses first)

| Category | Group | Without: passed | Without: rate (Wilson 95%) | With: passed | With: rate (Wilson 95%) | Difference, points (95%) |
|---|---|---|---|---|---|---|
| extremeConditions | extremeConditionsHard | 22 of 36 | 61.1% (44.9% to 75.2%) | 18 of 36 | 50.0% (34.5% to 65.5%) | -11.1 (-32.1 to +11.4) |
| formulationCritique | formulationCritique | 0 of 72 | 0.0% (0.0% to 5.1%) | 0 of 72 | 0.0% (0.0% to 5.1%) | +0.0 (-5.1 to +5.1) |
| formulationCritique | formulationCritiqueHard | 0 of 48 | 0.0% (0.0% to 7.4%) | 0 of 48 | 0.0% (0.0% to 7.4%) | +0.0 (-7.4 to +7.4) |
| extremeConditions | extremeConditions | 70 of 72 | 97.2% (90.4% to 99.2%) | 72 of 72 | 100.0% (94.9% to 100.0%) | +2.8 (-2.7 to +9.6) |

## Tests that changed

4 test(s) did worse with the tools, 2 did better.

| Change | Engine config | Group | Test | Without | With |
|---|---|---|---|---|---|
| -33 points | socrates-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): finds "shipments keeps flowing after "inventory" runs out" with a what-if run, unprompted | 3/3 | 2/3 |
| -33 points | merlin-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): finds "shipments keeps flowing after "inventory" runs out" with a what-if run, unprompted | 1/3 | 0/3 |
| -33 points | merlin-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): fixes "orders loses the MAX that limits it to 0" without changing normal behavior | 2/3 | 1/3 |
| -33 points | socrates-agent-sonnet-5-5 | extremeConditionsHard | Regional distributor inventory (hard): fixes "orders loses the MAX that limits it to 0" without changing normal behavior | 2/3 | 1/3 |
| +33 points | merlin-agent-sonnet-5-5 | extremeConditions | Endemic Infection: the revised model survives the extreme run | 2/3 | 3/3 |
| +33 points | merlin-agent-sonnet-5-5 | extremeConditions | Endemic Infection: fixes the formulation without changing normal behavior | 2/3 | 3/3 |

## Failures by type

| Category | Group | Failure type | Without | With |
|---|---|---|---|---|
| extremeConditions | extremeConditions | Formulation not fixed | 1 | 0 |
| extremeConditions | extremeConditions | Model not robust | 1 | 0 |
| extremeConditions | extremeConditionsHard | Correct model changed | 1 | 0 |
| extremeConditions | extremeConditionsHard | Evaluation error | 6 | 6 |
| extremeConditions | extremeConditionsHard | Formulation not fixed | 2 | 4 |
| extremeConditions | extremeConditionsHard | Problem not found by testing | 6 | 8 |
| formulationCritique | formulationCritique | Evaluation error | 42 | 42 |
| formulationCritique | formulationCritique | Formulation not fixed | 30 | 30 |
| formulationCritique | formulationCritiqueHard | Evaluation error | 42 | 42 |
| formulationCritique | formulationCritiqueHard | Formulation not fixed | 7 | 6 |

## Use of the Dynamist tools

| Arm | Result rows | Rows whose agent called a Dynamist tool | Calls to dynamist_check | Calls to dynamist_extreme_tests |
|---|---|---|---|---|
| Without | 228 | 0 | 0 | 0 |
| With | 228 | 149 | 170 | 103 |

Rows that share one generation (the tests of one case) share its tool calls, so a call counts once for each of those rows.

## Cost

Without the tools: $15.01. With the tools: $15.07. (Rows that reused a sibling's generation are not counted twice.)
