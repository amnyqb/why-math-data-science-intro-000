# Bass diffusion: how a new product spreads

## Question it answers
How fast does a new durable product reach its market, and what shapes the S-curve: advertising (outside influence) or word of mouth (inside influence)?

## Structure
- Two stocks: Potential Adopters (starts at Market Size) and Adopters (starts at 0). One flow, Adoption, moves households from one to the other. Total is conserved.
- Adoption = Innovator Adoption + Imitator Adoption.
- Innovator Adoption = Advertising Effect x Potential Adopters (balancing loop: the pool drains).
- Imitator Adoption = Word of Mouth Effect x Adopters x Potential Adopters / Market Size (reinforcing loop through Adopters, balancing through Potential Adopters).
- `dyn loops` finds 3 loops: two balancing (one per source) and one reinforcing.

## Parameters (our own values)
| Name | Value | Units | Meaning |
|---|---|---|---|
| Market Size | 500000 | household | everyone who will eventually buy |
| Advertising Effect | 0.02 | 1/Year | fraction of the pool that adopts each year without contact with adopters |
| Word of Mouth Effect | 0.45 | 1/Year | adoptions per adopter per year if the whole market were still open |
| time | 0 to 20 | Year | step 1/1024 year, Euler |

Levers (sliders): Advertising Effect 0 to 0.1, Word of Mouth Effect 0 to 1.

## Interpretation choices
- Continuous time, one-time purchase, no repeat sales, no price, no supply limit, constant market size.
- Euler with a step of 1/1024 year: the discretisation error is first order, so the step was chosen small enough for the check below.
- Imitation Share uses MAX(Adoption, 0.000001) only to avoid dividing by zero at the start.

## Source
Bass, F. M. (1969). A new product growth model for consumer durables. Management Science, 15(5), 215-227. Only the structure (innovation and imitation coefficients) is taken from the paper; the wording, numbers and layout are ours.

## What the reference check proved
`check.py` compares Adopters with N x F(t), F(t) = (1 - e^{-(p+q)t}) / (1 + (q/p) e^{-(p+q)t}), for three (p, q) pairs, plus the theoretical time of peak adoption, ln(q/p)/(p+q).
- Worst relative error where F >= 0.01: 3.2e-4 (0.02, 0.45), 7.3e-5 (0.05, 0.2), 8.7e-4 (0.01, 0.8). Tolerance 1e-3. The worst error as a share of Market Size was 3.0e-4 or less (tolerance 1e-3).
- Peak adoption time: 6.625 y vs 6.625 y theory for the base case (tolerance 0.02 y).
- Adopters + Potential Adopters equals Market Size to 1e-6 relative.
- At a step of 1/128 year the error was 2.6e-3 to 6.8e-3, which is why the step is small: Euler error shrinks linearly with the step.
- `dyn validate --strict-units`: 0 errors.

## Lessons for a learner
1. Two feedbacks make the S: imitation reinforces while the pool is large, then the shrinking pool balances it. Growth shifts from reinforcing to balancing at the inflection point.
2. Advertising only starts the process; with Advertising Effect near zero the curve is still S-shaped but starts late. Word of mouth sets how steep it is.
3. A numerical method is an assumption: with a coarse Euler step the model is visibly off the exact answer. Always check a step against a smaller one.
