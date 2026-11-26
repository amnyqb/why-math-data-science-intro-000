# Predator and prey (Lotka-Volterra)

## Question
Why do predator and prey numbers rise and fall in regular cycles, and what changes when the prey also crowd themselves?

## Structure
- Two stocks: Prey and Predators.
- Prey Births (growth rate x prey, with an optional logistic brake) and Prey Eaten (attack rate x prey x predators) move Prey.
- Predator Births (conversion rate x prey x predators) and Predator Deaths (death rate x predators) move Predators.
- Loops: prey reinforce themselves; predation and predator growth close one balancing loop with a delay between the two stocks, which gives the cycle.
- Switch `Logistic On` (0 or 1) and `Prey Capacity` add the logistic limit. At 0 the model is the pure Lotka-Volterra system.

## Parameters (our own values)
| Name | Value | Units |
|---|---|---|
| Prey Growth Rate (a) | 0.8 | 1/Year |
| Attack Rate (b) | 0.04 | 1/predators/Year |
| Predator Death Rate (c) | 0.6 | 1/Year |
| Conversion Rate (d) | 0.01 | 1/prey/Year |
| Logistic On | 0 | 1 |
| Prey Capacity (K) | 200 | prey |
| Initial Prey / Predators | 40 / 15 | prey / predators |
| Time | 0 to 60, step 0.001, Euler | Year |

Equilibrium: prey = c/d = 60, predators = a/b = 20 (pure) or (a/b)(1 - 60/K) = 14 (logistic).

## Interpretation choices
- Predator births are proportional to prey eaten through a separate conversion rate d, not tied to b. This is the usual Volterra form.
- Units use the plain words prey and predators as dimensions.
- The step is 0.001 year with Euler because Euler spirals outward in this system; the small step keeps that drift tiny (see below). RK4 is available in dyn but Euler keeps the model plain.
- Lotka (1925) wrote the chemical and ecological forms, Volterra (1926) the fish-catch version. We only borrow the structure.

## Source
Lotka, A. J. (1925), Elements of Physical Biology, Williams and Wilkins, Baltimore. Volterra, V. (1926), Variazioni e fluttuazioni del numero d'individui in specie animali conviventi, Mem. R. Accad. Naz. Lincei 2, 31-113.

## What check.py proved
Run with the server binary; all 7 checks pass.
- Euler trajectory against an independent RK4 (step 0.001) over 60 years: max relative gap 0.9 percent (tolerance 10 percent).
- Conserved quantity V = d x - c ln x + b y - a ln y: RK4 drifts 2e-15; dyn Euler drifts 0.06 percent over 60 years (tolerance 3 percent).
- Small cycle period 9.070 years against 2 pi / sqrt(a c) = 9.069 (tolerance 1 percent).
- Base (large) cycle period 9.234 years against RK4 9.232 (tolerance 3 percent). Large cycles are a little longer than the formula.
- Logistic option: ends at prey 60.01, predators 14.00 against theory 60 and 14 (tolerance 0.5), and equals RK4 within 0.05.
- `dyn validate --strict-units`: 0 errors.

## Lessons
1. The cycle is the signature of a pair of coupled stocks, not of outside forcing. Nothing in the model is periodic.
2. Without a limit the amplitude is set by the starting point alone (V is conserved), so the system has no preferred cycle and any shock changes it for good.
3. A single logistic limit on the prey turns the endless cycle into damped oscillation that settles on the equilibrium. Check it by switching Logistic On to 1.
