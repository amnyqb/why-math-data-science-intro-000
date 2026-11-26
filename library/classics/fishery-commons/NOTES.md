# Fishery commons (open access)

## Question
Why does a fishery open to everyone end up with a small fish stock, a large fleet and no profit, and which rule keeps both fish and profit?

## Structure
- Stocks: Fish (logistic growth) and Boats.
- Catch = Catchability x Boats x Fish, capped by a Quota.
- Profit = Price x Catch - Cost Per Boat x Boats.
- Boats are bought in proportion to positive profit and retired in proportion to losses (Fleet Response). Purchases stop at a Fleet Limit.
- Loops: fish growth reinforces, catch balances it; profit draws in boats, boats raise catch, catch lowers the stock and so lowers profit per boat. The fleet keeps entering until profit is zero (the commons problem).
- Two levers, both off by default (set very large): `Quota` and `Fleet Limit`.

## Parameters (our own values)
| Name | Value | Units |
|---|---|---|
| Growth Rate (r) | 0.5 | 1/Year |
| Carrying Capacity (K) | 1000 | tonne |
| Catchability (q) | 0.001 | 1/boats/Year |
| Price (p) | 10 | kUSD/tonne |
| Cost Per Boat (c) | 2 | kUSD/boats/Year |
| Fleet Response | 0.05 | boats/kUSD |
| Quota | 100000 (off) | tonne/Year |
| Fleet Limit | 100000 (off) | boats |
| Initial Fish / Boats | 900 / 20 | tonne / boats |
| Time | 0 to 100, step 0.02, Euler | Year |

## Interpretation choices
- Fleet change is proportional to total profit (not profit per boat), which keeps the model one line and is stable with these numbers.
- Retirements are capped at the fleet size per step and purchases at the room under the Fleet Limit, so Boats cannot go negative or exceed the limit.
- Quota is a cap on catch per year (a total allowable catch), applied when the fleet could catch more.
- Price and cost are constants; no market or labour feedback.
- Maximum sustainable yield r K / 4 = 125 tonne/Year is shown as a variable, not used in any equation.

## Source
Gordon, H. S. (1954), The economic theory of a common-property resource: the fishery, Journal of Political Economy 62(2), 124-142. Structure only; the logistic growth form is Schaefer (1954).

## What check.py proved
Run with the server binary; all 14 checks pass.
- Open access: stock 201.4 against theory c/(p q) = 200 (tolerance 2); fleet 397.4 against (r/q)(1 - S/K) = 400 (tolerance 8); catch 80.0 against 80.0; profit 5.5 against revenue 800 (tolerance 10, still settling). Catch 80 is below MSY 125 and stock is below K/2.
- dyn Euler against an independent RK4 (step 0.01): base stock 201.39 vs 201.31, boats 397.41 vs 397.48.
- Quota 100 (below MSY): stock 723.6 against the closed-form root 723.6 (tolerance 3), catch 100.0, boats 500. Catch is 25 percent higher and the stock 3.6 times higher than open access.
- Fleet Limit 200: stock 600.0 against theory 600, catch 120 (96 percent of MSY), profit 800 kUSD/Year against 800.
- `dyn validate --strict-units`: 0 errors.

## Lessons
1. Individual boats see their own profit, not the cost they put on everyone else through a smaller stock, so entry continues until rent is zero. That is the commons problem, and the open-access catch (80) is below what the stock could give (125).
2. A quota fixes the fish but not the fleet: boats still pile in until profit is zero (500 boats chasing a 100 tonne quota). Compare with the fleet limit.
3. Limiting the fleet keeps both the stock (600 tonne) and the profit (800 kUSD/Year). The lever that works is the one that acts on the entry feedback itself.
