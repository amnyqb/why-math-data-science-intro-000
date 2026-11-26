# Model doctor

> Generated from the rule table in `core/src/doctor/` (`cargo test -p dynamist-core --test doctor_catalogue` rewrites it when `DYN_WRITE_WIKI=1`). Do not edit by hand.

`dyn check FILE` runs the stages C1 to C4 and prints findings, each with exact fixes (`--fix 1,3` applies the first fix of findings 1 and 3, `--fix-all-safe` the first fixes that keep the numbers). `--experiments` adds stage C5, `--gate publish` runs everything and turns warnings into errors, `--stage C3` stops early, `--json` prints the report, `--lenient-time` lets a rate meet its amount.

| Stage | Content |
|---|---|
| C1 | parse: names |
| C2 | graphs: loops, double counting, initial values |
| C3 | units, signs and intervals |
| C4 | the base run and numerical health |
| C5 | automatic experiments (`--experiments`) |
| C6 | data and calibration (not implemented) |
| C7 | the report and the gate: Draft (errors), Review (no errors), Publish (no errors and no warnings) |

## Rules

Each finding names its rule and carries the source it comes from.

| Rule | Stage | What it catches |
|---|---|---|
| S1 | C1 | A name in an equation that no variable has (with the closest defined name as a suggestion). |
| S2 | C1 | A variable with an empty equation, or a stock with no initial value. |
| S3 | C1 | The same name defined twice. |
| S4 | C1 | An equation with a syntax error: where, and what was expected. |
| G1 | C2 | An algebraic loop: variables that depend on each other in the same step with no stock or delay between them. |
| G2 | C2 | A stock whose equation uses run variables instead of only its initial value (its changes must come through flows). |
| G3 | C2 | A flow attached to no stock at either end. |
| G4 | C2 | A flow read as information by another variable. |
| G5 | C2 | A flow that is just another stock's value. |
| G6 | C2 | A stock with no flows. |
| G7 | C2 | A variable that affects no stock, output or KPI. |
| G8 | C2 | An equation with more than 3 inputs. |
| G9 | C2 | An arrow that its equation does not use, or an equation input with no arrow. |
| G10 | C2 | A part of the model with no path to the declared KPIs. |
| G11 | C2 | Double counting: a stock called Total or All that starts as the sum of other stocks. |
| G12 | C2 | A causal loop diagram arrow whose sign differs from what the equations give. |
| U1 | C3 | Dimension: quantities of different kinds added or compared, or an equation that gives other units than declared (code unit-dimension-mismatch, unit-declared). |
| U2 | C3 | Scale: the same kind of quantity in different scales, with the exact conversion factor as the fix (code unit-scale-mismatch). |
| U3 | C3 | Percent and ppm: how a constant declared in % or ppm is read, or what units_scale_values would change (code unit-percent, unit-scale-change). |
| U4 | C3 | A flow times the time unit that is not its stock's unit (code unit-flow). |
| U5 | C3 | Real and nominal money, or two price years, added together (code money-real-nominal). |
| U6 | C3 | An ambiguous unit symbol (MB, ton, gallon): which meaning is meant (code unit-ambiguous). |
| U7 | C3 | A unit that cannot be read, with the closest known unit as a suggestion (code unit-parse). |
| U8 | C3 | A variable with no units whose equation gives them (code units-inferred). |
| U9 | C3 | Dated data (a lookup read against Time in a model with a start date) with gaps, repeated times or times going backwards. |
| U10 | C3 | A constant of value 1 that carries units only to make the units balance. |
| U11 | C3 | A lookup table whose input or output carries units. |
| U12 | C3 | A time constant (adjustment time, delay, lifetime) declared without a time unit. |
| F1 | C3 | An outflow of a never-negative stock that can ask for more than the stock holds in one step. |
| F2 | C3 | An outflow that can drain a stock below zero because nothing holds the stock at zero. |
| F3 | C3 | A division by something that can be zero. |
| F4 | C4 | A number other than 0 and 1 written into an equation (code magic-number). |
| F5 | C3 | IF THEN ELSE, MIN or MAX on a quantity that changes gradually in reality. |
| F6 | C3 | A lookup that is not monotonic. |
| F7 | C3 | A multiplier lookup that does not pass through 1 at a relative input of 1. |
| F8 | C3 | A lookup whose input can leave the range of its table. |
| F9 | C3 | Time used inside a behavioural equation. |
| F10 | C3 | An information delay on a material flow, or a material delay used for a perception. |
| F11 | C3 | DELAY FIXED with a delay time that varies. |
| F12 | C3 | A gap, error or shortfall in a reinforcing loop: it grows instead of closing. |
| F13 | C3 | A desired value computed from the stock it controls. |
| F14 | C3 | A ratio, share, fraction or probability that can leave 0 to 1. |
| F15 | C3 | A parameter value written in the documentation text. |
| N1 | C4 | A value that is not a number or is infinite in the base run. |
| N2 | C4 | The time step is more than a quarter of the fastest time constant. |
| N3 | C4 | Halving the time step changes a stock by more than 1% (code dt-sensitivity). |
| N4 | C4 | A Runge-Kutta method with step, pulse, fixed-delay or time-test functions that jump. |
| N5 | C4 | A time constant below the time step: the fastest loop of the model is faster than the clock (raised together with N2). |
| N6 | C4 | Time compared for equality with a time step that is not a power of 1/2 (code time-equality). |
| N7 | C4 | A net flow that changes sign on most steps: the run chatters. |
| N8 | C4 | A stock so slow that it hardly moves in the length of the run. |
| I1 | C2 | Initial values that depend on each other. |
| I2 | C4 | The model starts in equilibrium by its own statement, but its stocks are changing at the start. |
| I3 | C4 | The start-up transient dominates the first part of the run. |
| X1 | C5 | A constant at zero, ten times its value or an end of its range makes the model break an invariant; a constant with a declared range (a lever or a slider) is tried at the two ends of that range only. |
| X2 | C5 | A stock rises although every source inflow is zero. |
| X3 | C5 | An arrow whose drawn sign disagrees with what raising its constant does. |
| X4 | C5 | Cutting a feedback loop changes nothing: the loop is inert. |
| X5 | C5 | From equilibrium, a step in the largest input does not settle although every loop is balancing. |
| X6 | C5 | One of the modeller's own checks fails in an extreme case that passes in the base run; the worst breach is reported, a diverging run is named, and a check marked as a validity range or a promise gives information. |
| C1 | C6 | More than 5 parameters calibrated at once, where partitions by data series are possible. |
| C2 | C6 | Parameter estimates that are correlated above 0.9: they cannot be told apart. |
| C3 | C6 | A fit reported without Theil's decomposition of the error. |
| C4 | C6 | A calibrated value outside its plausible range. |
| C5 | C6 | A data series whose units or times do not fit the model. |
| R1 | C4 | Variables with no description. |
| R2 | C4 | Random functions with no recorded seed. |
| R3 | C4 | No run manifest: levers with ranges, outputs. |
| R4 | C4 | A model that cannot be exported with its equations. |
| P1 | C4 | No modelling brief: the question, the kind of model and the behaviour wanted. |
| P2 | C4 | A variable owned by two layers, or a link across layers with no declared contract. |
| P3 | C4 | A benchmark with a published source that the base run misses. |

The catalogue is `wiki/specs/Dynamist_Model_Doctor_Research_and_Rules.md` (Part B); all 72 rules are in the table above. Findings carry the rule id in `rule`, and the older codes of the unit and base checks are named in each rule's text.
