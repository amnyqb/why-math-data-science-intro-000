# Market growth and capital investment

## Question
Does a growing firm stall because it cannot deliver fast enough, and does adding capacity in response to long delivery delays restore growth, or only add swings?

## Source (structure only)
Forrester, J. W. (1968). "Market growth as influenced by capital investment." *Industrial Management Review* 9(2), 83-105 (citation details to be verified against the reprint in Forrester's Collected Papers). The structure follows that paper: sales force, orders, order backlog, delivery delay, production capacity. Equations, tables and numbers are our own.

## Structure
- Sales Force (Person) is moved toward a target set by budget: Budget Fraction * Revenue / Salary Cost, over Hiring Time.
- Order Rate = Sales Force * Orders per Salesman * Delay Effect Multiplier.
- Order Backlog (units) fills with orders and empties with Shipments = MIN(Capacity, Backlog / Minimum Delivery Time).
- Delivery Delay = Backlog / Shipments. Perceived Delay = SMOOTH(Delivery Delay, Delay Perception Time). Delay Ratio = Perceived Delay / Target Delivery Delay.
- Delay effect (lookup "Delay Effect on Orders"): a long delay lowers orders (ratio 1 gives 1.0, ratio 2 gives 0.65, ratio 4 gives 0.25).
- Capacity (units/Month) is added by Capacity on Order, a stock that empties into Capacity over Capacity Acquisition Time. New capacity is ordered at Capacity * fraction(Delay Ratio) (lookup "Capacity Order Fraction"; zero up to ratio 1, 0.1 per month at ratio 2).
- Loops: reinforcing sales loop (sales force, orders, shipments, revenue, budget, sales force); balancing delay-on-orders loop (more backlog, longer delay, fewer orders); capacity loop (long delay, more capacity ordered, shorter delay), which carries the acquisition delay. dyn loops lists these loops; its reinforcing/balancing labels looked wrong for some loops with goal-seeking flows, so the roles above are our own reading.
- Switches (constants, with sliders, 1 = on): Capacity Switch (capacity loop), Delay Effect Switch (delay discourages orders), Sales Budget Switch (sales force follows revenue; off holds it at its start).

## Parameters (all ours)
| Name | Value | Units |
|---|---|---|
| Initial Capacity | 100 | units/Month |
| Target Delivery Delay | 3 | Month |
| Minimum Delivery Time | 1 | Month |
| Initial Sales Force | 20 | Person |
| Orders per Salesman | 5 | units/Month/Person |
| Price | 10 | kUSD/units |
| Salary Cost | 2.5 | kUSD/Month/Person |
| Budget Fraction | 0.07 (0.05 = balance) | 1 |
| Hiring Time | 6 | Month |
| Delay Perception Time | 3 | Month |
| Capacity Acquisition Time | 18 | Month |
Time: 0 to 120 months, step 0.125, Euler.

## Interpretation choices
- Start in balance: 20 salesmen * 5 = 100 orders = capacity 100, backlog 300 = 3 months of shipments, and at Budget Fraction 0.05 the sales budget exactly pays 20 salesmen. We then use Budget Fraction 0.07 in the base run: the sales loop has a gain above 1, which makes the firm grow. Setting 0.05 gives the no-growth equilibrium used in the steady-state checks.
- Capacity is not depreciated, and the firm never sells capacity, as in a simple growth story.
- Capacity ordering ignores capacity already on order. This is deliberate: it produces the overreaction the paper is about, and the delay overshoot.
- The delivery delay both lowers orders (outside view) and triggers capacity (inside view), both through the same perceived delay. In the original these are separate perceptions.
- Capacity acquisition is a first-order delay (simpler than a pure transport delay).
- Revenue is shipments times price; the budget has no cost side or profit.
- Parameter values were tuned only so the base run shows a visible stall; they are not calibrated to any firm.

## What the checks prove (check.py, run against the server build of dyn; 19 checks, all pass)
- Strict units: 0 errors.
- Steady state at Budget Fraction 0.05: all four stocks drift by 0.0 (limit 1e-9) with all switches on, and with each switch off in turn.
- All loops on: capacity grows from 100 to 1365 units/month; orders peak at 121.1 in month 7, fall to 114.8 by month 16 (the stall), then grow to 1498. Delivery delay peaks at 4.97 months and settles at 4.03 (target 3).
- Capacity loop off: capacity stays at 100; orders peak at 121.0 and settle at 100.0; delivery delay ends at 5.52 months; the sales force stops at 28. Growth is limited by delivery delay.
- Both the capacity loop and delay effect off: nothing stops orders, so delay reaches 48.6 months and the backlog 4860 units and keeps growing.
- Delay effect off, capacity on: the delay peaks at 6.69 months, bounded.
- Accounting: backlog, capacity plus capacity on order, and sales force each equal start plus the sum of their flows; largest residuals 3.6e-12, 1.1e-12 and 5.7e-14.
- Halved step: capacity at month 120 is 1365.42, 1368.80, 1370.50 and the peak delay 4.9682, 4.9569, 4.9513 for steps 0.125, 0.0625, 0.03125; differences shrink.
Our base run shows a stall followed by growth with a delay overshoot, not a sustained oscillation. A longer Capacity Acquisition Time or a larger Budget Fraction gives larger swings; a learner can try them. We do not claim numerical agreement with the 1968 paper.

## Lessons for a learner
1. Growth feeds on itself (sales loop) until something it ignores pushes back: here the delivery delay, which acts through customers' orders. Turn the capacity switch off to see the limit.
2. Adding capacity is a balancing response with a long delay. Watch the delay overshoot while capacity on order builds, then settle above target (4.03 against 3), because the capacity rule only reacts to a gap.
3. Turn off the delay effect on orders and the firm grows much faster but customers wait far longer: a loop that looks like a nuisance is also a guard.
