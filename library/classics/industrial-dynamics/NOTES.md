# Industrial dynamics: a production-distribution chain

## Question
Why does a small, one-time step in customer demand cause much larger swings in orders and production further up the chain, and which policy settings make the swing smaller?

## Source (structure only)
Forrester, J. W. (1961). *Industrial Dynamics*. MIT Press and Wiley (the production-distribution system of the early chapters). This is our own formulation: names, equations, delays and numbers are ours. No file or text of any other model was used.

## Structure
Three stages in a row: Retailer, Distributor, Factory. Each stage has three stocks: Inventory (goods on hand), Backlog (orders received, not yet shipped) and Pipeline (goods ordered from the stage above, on the way; at the factory this is work in process).
- Goods flow: Pipeline, through Receipts (pipeline / delay, a first-order material delay), into Inventory, out by shipments to the next stage down (to customers for the retailer).
- Orders flow the other way: each stage places an order that adds to the backlog of the stage above; the factory's order is a production start.
- Ordering rule at every stage: expected demand + (desired inventory - inventory) / adjustment time + supply line weight * (desired pipeline - pipeline) / adjustment time, never below zero.
- Expected demand is the orders the stage receives, smoothed with SMOOTH (the information delay).
- Shipments = MIN(backlog, inventory) / order fill time (a stage cannot ship what it does not have).
- Customer demand is 100 units/week with a step of +10 at week 5.
- Accounting stocks: Cumulative Starts and Cumulative Delivered, and the aux Material in System.

## Parameters (all ours)
| Name | Value | Units | Meaning |
|---|---|---|---|
| Initial Demand | 100 | units/Week | Demand before the step |
| Step Size, Step Time | 10, 5 | units/Week, Week | The test input |
| Demand Averaging Time | 4 | Week | Information delay (SMOOTH) |
| Cover Time | 3 | Week | Desired inventory in weeks of expected demand |
| Adjustment Time | 4 | Week | Gap closing time, inventory and supply line |
| Supply Line Weight | 0.5 | 1 | Share of the supply-line gap corrected |
| Order Fill Time | 1 | Week | Backlog clearing time |
| Retailer / Distributor Transit Delay | 2 / 3 | Week | Material delays |
| Production Delay | 4 | Week | Material delay at the factory |
Time: 0 to 100 weeks, step 0.125, Euler.

## Interpretation choices
- Material delays are explicit first-order stocks (pipeline / delay), not DELAY3, so the goods in transit can be counted and conservation checked. Forrester's delays are often higher order; ours give less oscillation.
- The information delay is SMOOTH of the orders received. There is no separate order-transmission delay; the backlog and fill time play that part.
- No lost sales: unfilled orders wait in the backlog.
- The backlog is not part of the ordering rule (only inventory and pipeline gaps).
- All stages share the same policy constants, so the amplification comes only from structure and the delays, not from different behaviour.
- Supply Line Weight 0.5 is a middle value; the checks run 0, 0.5 and 1.
- The system starts in exact equilibrium, with inventory = cover * demand, backlog = fill time * demand, pipeline = delay * demand.

## What the checks prove (check.py, run against the server build of dyn)
- Strict units: 0 errors.
- Steady state: with Step Size 0, all 9 non-tally stocks drift by 0.0 (limit 1e-9); the tallies grow by exactly 100 units/week (10000 over 100 weeks).
- Amplification: peak orders rise at every stage up the chain. Supply line weight 0: 110.0, 119.2, 137.8, 168.7 (customer, retailer, distributor, factory starts). Weight 0.5: 110.0, 117.8, 132.2, 153.5. Weight 1: 110.0, 116.6, 128.1, 143.7. A larger weight gives a smaller factory peak, as published qualitative results suggest.
- New equilibrium: by week 100 all orders are 110 (to 1e-3) and inventories are 330 (cover * 110).
- Conservation: change in Material in System equals cumulative starts minus cumulative deliveries; largest residual 8.4e-12 over 801 steps.
- Halved step: factory peak 153.45, 152.45, 151.97 for steps 0.125, 0.0625, 0.03125. Differences 0.995 then 0.481: they shrink by half, the signature of first-order Euler convergence.
We do not claim to match a number from the book; the claim is the qualitative behaviour (amplification and oscillation up the chain) and exact bookkeeping.

## Lessons for a learner
1. Each stage sees only the orders of the stage below, smooths them, and corrects its own inventory. Every correction adds to the order, so a 10 percent step becomes a 40 to 70 percent surge at the factory. Amplification comes from structure, not from foolish people.
2. Ignoring goods already on the way (supply line weight 0) is the biggest single cause of overshoot. Set the slider to 1 and watch the factory peak drop.
3. Slower correction (a longer Adjustment Time) calms the chain but leaves inventory away from target longer: a trade-off to test with the lever sliders.
