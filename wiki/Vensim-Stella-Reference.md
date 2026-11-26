# Dynamist, Vensim and Stella side by side

For people who know Vensim or Stella: what each Dynamist term and function is called there, and where the same name means something different. It is checked against the vendors' own documentation (read on 19 November 2026; vendors change their pages, so a link is the authority, not this table). What could not be confirmed is listed at the end instead of being filled in from memory.

Sources: Vensim [summary list of functions](https://www.vensim.com/documentation/22300.html) and the function pages it links ([STEP](https://vensim.com/documentation/20485.html), [PULSE](https://www.vensim.com/documentation/fn_pulse.html), [PULSE TRAIN](https://www.vensim.com/documentation/fn_pulse_train.html), [SMOOTH](https://www.vensim.com/documentation/20480.html), [SMOOTH3](https://vensim.com/documentation/fn_smooth3.html), [DELAY3](https://www.vensim.com/documentation/fn_delay3.html), [DELAY FIXED](https://www.vensim.com/documentation/fn_delay_fixed.html), [TREND](https://www.vensim.com/documentation/fn_trend.html), [INTEGER](https://www.vensim.com/documentation/fn_integer.html), [RANDOM](https://www.vensim.com/documentation/fn_random.html), [XIDZ](https://www.vensim.com/documentation/fn_xidz.html), [macros](https://www.vensim.com/documentation/macros.html)); Stella [builtins overview](https://www.iseesystems.com/resources/help/v2/Content/08-Reference/07-Builtins/Overview_Builtins.htm) and its [delay](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Delay_builtins.htm), [test input](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Test_input_builtins.htm), [mathematical](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Mathematical_builtins.htm), [logical](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Logical_builtins.htm), [trigonometric](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Trigonometric_builtins.htm) and [array](https://www.iseesystems.com/resources/help/v10/Content/Reference/Builtins/Array_builtins.htm) pages.

How files are converted, construct by construct, is in [Format mapping: Vensim](Format-Mapping-Vensim.md) (and XMILE, for Stella files). This page is about reading and writing equations by hand.

## Terms

| Dynamist | Vensim | Stella |
|---|---|---|
| Stock | Level | Stock |
| Flow | Rate | Flow |
| Auxiliary, Constant | Auxiliary, Constant | Converter |
| Lookup (curve) | Lookup, `WITH LOOKUP` | Graphical variable, `LOOKUP(graph, x)` |
| Set / item | Subscript range / element | Dimension / element |
| `TOTAL(x)` over a set | `SUM(x[range!])` | `SUM(x[*])` |
| Module (a group of variables with ports) | View (drawing only) | Module (not described in the builtin pages read) |
| Module definition and instance | Macro (no subscripts inside a macro) | Module |
| Time, step, start, end | `TIME`, `TIME STEP`, `INITIAL TIME`, `FINAL TIME` | `TIME`, `DT`, `STARTTIME`, `STOPTIME` |
| Dimensionless unit | `dmnl` | not named in the pages read |
| Your own function | Macro | not described in the pages read |

Stella also has conveyors and queues (discrete stocks); Dynamist does not.

## Functions

"Same" means name, argument order and meaning agree. Anything else is described. Functions not listed (the common maths ones such as `ABS`, `SQRT`, `EXP`, `LN`, `SIN`, `COS`, `TAN`, `MIN`, `MAX`, `ARCSIN`, `ARCCOS`, `ARCTAN`) are the same in both products.

| Dynamist | Vensim | Stella | Differences that change results |
|---|---|---|---|
| `IF THEN ELSE(c, a, b)` | same | `IF c THEN a ELSE b` | none; Stella notes exact 0/1 switches need Euler under its Runge-Kutta methods |
| `LOG(x, base)` | `LOG(x, base)` | none: `LOG10(x)`, or `LN(x)/LN(b)` | |
| `POWER(x, y)` | same | the `^` operator | |
| `INTEGER(x)` | `INTEGER`: truncates toward zero | `INT`: largest integer ≤ x, so `INT(-8.9) = -9` | **Differs for negatives** between Vensim and Stella; Dynamist truncates like Vensim |
| `ROUND(x)` | not found in the pages read | `ROUND(x)` | |
| `MODULO(x, y)` | same | `x MOD y` | sign for negative operands not stated by either |
| `XIDZ(a, b, x)`, `ZIDZ(a, b)` | same, but the test is `ABS(b) < 1E-6`, not exactly zero | `SAFEDIV(a, b, onzero)`; also substitutes when the result is not finite | tiny non-zero divisors give the fallback in Vensim and Stella |
| `CEILING`, `FLOOR`, `SIGN` | not found | not found (`INT` is the floor) | |
| `SINH`, `COSH`, `TANH` | same | not found | |
| `PI()` | not found as a function | `PI` (no brackets) | |
| `GAMMA(x)` (the gamma function) | only `GAMMA LN(x)`, its logarithm | `GAMMALN(x)`; **`GAMMA(...)` there is a random-number generator** | writing our `GAMMA(x)` unchanged into Stella would silently be a random draw; use `EXP(GAMMALN(x))` |
| `FACTORIAL(n)` | not found | `FACTORIAL(n)` | |
| `ERF`, `CLAMP`, `MEDIAN` | not found | not found (`CLAMP` = `MIN(MAX(x, lo), hi)`) | |
| `SINWAVE(a, p, start)`, `COSWAVE` | not found | `SINWAVE(a, p)`, `COSWAVE(a, p)`: no start; amplitude taken as absolute | phase differs when `start` is not 0 |
| `GRAPH(x, x0, dx, y1, …)`, `GRAPHSTEP` | `WITH LOOKUP(x, ((x1,y1),…))` | a graphical variable with `LOOKUP` | |
| `WITH LOOKUP(x, ((x1,y1),…))` | same | `LOOKUP(graphical variable, x)` | |
| `STEP(height, start)` | same (`STEP(h, t)`) | same, plus optional duration and interval in newer versions | value exactly at `start` not stated |
| `PULSE(start, width)`, height 1 | `PULSE(start, width)`; a width of 0 counts as one `TIME STEP`; on only at step times (`Time + TIME STEP/2`) | **`PULSE(volume, first, interval)`**: the first argument is a volume, spread over one `DT` (value = volume ÷ `DT`); no width | **Different meaning of the same name** |
| `PULSE TRAIN(start, width, interval, end)` | same order | not found | width ≤ `TIME STEP` gives a single step in Vensim |
| `RAMP(slope, start, end)` | same: 0 until start, then slope, then constant after end | newer help: `RAMP(slope, [time])` (no end); older help: with a stop time | check what happens after `end` in the target version |
| `RANDOM UNIFORM(min, max, seed)` | same order; **third argument is a stream number**, usually 0; seed set elsewhere (`NOISE SEED`) | `UNIFORM(min, max, [seed], …)`; legacy `RANDOM(min, max, seed)` | numbers differ between tools; runs are not reproducible across them |
| `RANDOM NORMAL(min, max, mean, sd, seed)` | same order; values outside [min, max] are redrawn, not clipped | **`NORMAL(mean, stddev, [seed], [min], [max])`**: different order | clipped in Dynamist, redrawn in Vensim |
| `SMOOTH(x, t)` | same; starts at the input's initial value | `SMTH1(x, t, [initial])` | |
| `SMOOTHI`, `SMOOTH3I` | same (`SMOOTHI`, `SMOOTH3I`) | `SMTH1`/`SMTH3` with the third argument | |
| `SMOOTH3(x, t)` | same | `SMTH3(x, t, [initial])` | three stages of t/3; does not conserve material when t changes |
| `DELAY1`, `DELAY3` | same; no separate initial argument (the `I` forms take one) | same, with an optional initial argument | initial stock is input × delay (÷ 3 per stage); these conserve material when the delay changes |
| `DELAY1I`, `DELAY3I` | `DELAY3I` confirmed; `DELAY1I` is named on the Vensim pages but its signature was not read | `DELAY1`, `DELAY3` with the third argument | |
| `DELAY FIXED(x, t, initial)` | same order; only the *initial* value of `t` is used; minimum delay is `TIME STEP`; must stand alone in a level | `DELAY(x, t, [initial])` | later changes of `t` are ignored |
| `DELAYINF(x, t, n, [initial])` | `SMOOTH N(x, t, initial, n)` (reading checked against Vensim's output, B2) | `DELAYN(x, t, n, [initial])` is the nearest | reading Vensim is checked; writing `SMOOTH N` is not, nor the Stella side |
| `DELAYMTR(x, t, n, [initial])` | `DELAY N(x, t, initial, n)` (reading checked against Vensim's output, B2) | `DELAYN` | as above |
| `TREND(x, t, initial)` | same order; **no default** for the initial trend | `TREND(x, t, [initial])`, default 0 | |
| `INITIAL(x)` | same | `INIT(variable)`: a single variable only, not an expression | |
| `ACTIVE INITIAL(x, initial)` | same (checked against Vensim's output, B2) | no direct equivalent | `x` during the run; `initial` only while initial values are computed |
| `SAMPLE IF TRUE(c, x, initial)` | same (checked, B2) | no direct equivalent | holds its value while `c` is 0 |
| `QUANTUM(x, q)` | same (checked, B2) | `q * INT(x / q)` | `x` when `q ≤ 0` |
| `GAME(x)` is read as `x` | `GAME(x)` | — | the same outside Vensim's gaming mode |

## Where the same name is a trap

In order of how likely they are to change a model's numbers:

1. **`PULSE`** is a height-1 pulse of a given width in Vensim and Dynamist, and a *volume* spread over one step in Stella.
2. **`GAMMA`** is the gamma function here and a random generator in Stella.
3. **`INTEGER`/`INT`** of a negative number.
4. **`RANDOM NORMAL`** versus Stella's `NORMAL`: different argument order.
5. **`ZIDZ`/`XIDZ`**: Vensim and Stella treat a tiny divisor as zero; Dynamist only an exact zero.
6. **`SINWAVE`** with a start time other than 0.
7. **`RAMP`** after its end time.

## Units and arrays

- Vensim writes units as text in the equation editor and `dmnl` for dimensionless; function pages show signatures like `SMOOTH3(units, time) → units` and `TREND(units, time, 1/time) → 1/time`. Smoothing and delay times must be in the units of `TIME STEP`. Dynamist reads units as algebra (see the User guide) and treats `Dmnl`, `Fraction`, `Ratio` and `Yes/No` as dimensionless.
- Vensim sums over a subscript range with `SUM(x[range!])` (also `PROD`, `VMAX`, `VMIN`); Stella sums with `SUM(x[*])`, and `MIN`/`MAX`/`MEAN`/`STDDEV` take the same subset syntax. Dynamist uses `TOTAL(x)` and `AVERAGE(x)`, optionally for one group of a parent set.
- A Vensim macro cannot use subscripts inside its definition. A Dynamist module definition can use sets.

## Integration methods

Dynamist offers Euler, RK2, RK3 and RK4. Stella's pages refer to 2nd- and 4th-order Runge-Kutta without naming a default; Vensim's pages read here do not name its methods. Both Vensim (for `PULSE`, `PULSE TRAIN`, `DELAY FIXED`) and Stella (for exact integers and 0/1 switches) say some functions only behave exactly under Euler.

## What could not be confirmed

- Vensim: `ROUND`, `CEILING`, `FLOOR`, `SIGN`, `PI`, `ERF`, `FACTORIAL`, `CLAMP`, `SINWAVE`, `COSWAVE`, `GRAPH` and `GRAPHSTEP` were not found in its summary list. The pages for `DELAY N`, `DELAY INFORMATION`, `DELAY MATERIAL`, `MODULO`, `WITH LOOKUP`, `INITIAL`, `RAMP`, `RANDOM NORMAL`, `NOISE SEED`, the subscript chapter and the integration methods were not read.
- Stella: how dimensions and elements are declared, the dimensionless unit, the integration method names and the default method were not found in the pages read. One Stella page (statistical builtins, newest version) returned an error, so an older version of it was used.
- Nothing here was tried by running a model in Vensim or Stella.
