# SIR epidemic: one outbreak in a closed town

## Question it answers
How big does one outbreak get, when does it peak, and how far must contacts fall for it to stop?

## Structure
- Three stocks: Susceptible, Infectious, Recovered. Flows: Infection (Susceptible to Infectious) and Recovery (Infectious to Recovered). People are conserved.
- Infection = Contact Rate x Infection Chance x Susceptible x Infectious / Population (reinforcing loop on Infectious, balancing loop on Susceptible).
- Recovery = Infectious / Infectious Period (balancing loop).
- Check variables: Basic Reproduction Number R0 = Contact Rate x Infection Chance x Infectious Period; Effective Reproduction Number = R0 x Susceptible / Population; Herd Immunity Threshold = 1 - 1/R0; Immune Fraction.
- The main file is plain SIR; no exposed stage was added.

## Parameters (our own values)
| Name | Value | Units | Meaning |
|---|---|---|---|
| Population | 1000000 | person | closed town |
| Initial Infectious | 10 | person | seed of the outbreak |
| Contact Rate | 10 | 1/Day | contacts per person per day (lever 1 to 20) |
| Infection Chance | 0.05 | 1 | chance a contact with an infectious person infects (lever 0.01 to 0.1) |
| Infectious Period | 5 | Day | average time infectious |
| time | 0 to 300 | Day | step 0.01 day, Euler |

Base case R0 = 2.5, herd immunity threshold 60 percent.

## Interpretation choices
- Contacts are split into Contact Rate and Infection Chance so the lever "reduce contacts" is a single slider; their product is the usual transmission rate.
- Mixing is homogeneous; a recovered person is immune for good; no births or deaths; the town is closed.
- Susceptible starts at Population minus Initial Infectious, so the three stocks sum to Population at time 0.
- Euler with step 0.01 day; the step was chosen so that the reference differences below are small.

## Source
Kermack, W. O. and McKendrick, A. G. (1927). A contribution to the mathematical theory of epidemics. Proceedings of the Royal Society of London A, 115(772), 700-721. Only the structure is taken from the paper; wording, numbers and layout are ours.

## What the reference check proved
`check.py` solves the final size relation S_inf = S0 exp(-R0 (N - S_inf) / N) by bisection, independently of the simulator, and uses the peak condition (infectious peak when S = N/R0) and the peak size I_max = S0 + I0 - (N/R0)(1 + ln(R0 S0 / N)). Three contact rates (R0 = 2.5, 1.5, 3.75):
- Final size: difference between model and theory 1.3e-4, 6.1e-5, 9.7e-5 of the population (tolerance 5e-4). At R0 = 2.5, 89.3 percent are immune at the end, far above the 60 percent threshold (overshoot).
- Susceptible at the peak vs N/R0: within 4e-4 of N (tolerance 1e-3).
- Peak size: relative difference 6.4e-4, 2.8e-4, 9.3e-4 (tolerance 5e-3).
- Conservation: |S + I + R - N| below 3e-8 person.
- R0 below 1 (contact rate 3.2, R0 = 0.8): infectious never rises above the seed of 10.
- `dyn validate --strict-units`: 0 errors.

## Lessons for a learner
1. R0 alone decides whether an outbreak takes off: above 1 it grows, below 1 it dies, whatever the starting number of cases.
2. The epidemic does not stop at the herd immunity threshold; the momentum of the infectious people carries it past, so the final immune share (89 percent at R0 = 2.5) exceeds 1 - 1/R0 (60 percent).
3. A contact lever has a large effect on the peak. Halving the contact rate here (R0 = 1.25) cuts the peak from about 23 percent of the town to about 2 percent (theory: 1 - (1 + ln R0)/R0).
