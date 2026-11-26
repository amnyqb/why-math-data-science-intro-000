# Format mapping: XMILE

XMILE is the OASIS interchange format for system dynamics (version 1.0, 2015). In Dynamist it is a **bridge for reading Stella files** (and other XMILE files), not the format Dynamist works in. The reader is the main part. The writer is kept because it round-trips the starter model, the fuel-supply model and the small test models to within 1e-9, but it is best-effort: whatever it cannot express is listed in the loss report it returns, and in the tables below.

Code: `core/src/xmile.rs` (`read(text) → (Model, LossReport)`, `write(model) → (text, LossReport)`). Rules were checked against the OASIS specification text. For what Vensim and Stella call things, see [Vensim and Stella side by side](Vensim-Stella-Reference.md).

## What was tested

| Test | Result |
|---|---|
| 21 unit tests in `xmile.rs`: XML reader, both equation translators, precedence, every built-in, stocks/flows/clouds/lookups/units/ranges/arrows, sets (plain and with parents), macros, levers/scenarios/checks, groups, bad files, odd names | pass |
| Starter model → XMILE → Dynamist: every series the same within 1e-9 | pass |
| One formula per built-in (50 formulas) → XMILE → Dynamist: same values within 1e-9 | pass |
| Fuel-supply model (57 tanks, parent sets) → XMILE (flattened) → Dynamist: 1,524 series, same within 1e-9 (`src/xmile_corpus_tests.rs`) | pass |
| 67 `.xmile` files of the SDXorg *test-models* repository (MIT licence), read and run against the expected `output.csv` | 40 match; see "Real files" below |

Not tested: opening a file written by Dynamist in Stella, Vensim or PySD (none was available), and the reverse for anything not in the 67 files. "Same within 1e-9" means Dynamist wrote it and Dynamist read it; it says the translation is self-consistent, not that another tool will compute the same numbers.

## Constructs

| Dynamist | XMILE | Read | Written | Loss |
|---|---|---|---|---|
| Model title | `<header><name>` | yes | yes | none |
| Start, end, step, time units | `<sim_specs>` `<start>` `<stop>` `<dt>` (also `reciprocal="true"`), `time_units` | yes | yes | none; `pause` and `run` ignored (reported) |
| Euler, RK2, RK4 | `method="euler"`, `rk2`, `rk4` | yes; `"gear, rk4"` takes the first method we have; anything else becomes RK4 (reported) | yes | none |
| RK3 | no equivalent | | written as `rk4` | reported |
| Stock, initial value | `<stock>` `<eqn>` | yes | yes | none |
| Inflows, outflows | `<inflow>`, `<outflow>` | yes | yes | a flow named by two stocks at one end keeps the first (reported) |
| Cloud (flow with one open end) | flow with no stock on that side | yes | yes | none |
| Never negative (stock) | `<non_negative/>`, or `<behavior>` | yes | yes | Stella clamps outflows, Dynamist clamps the stock: results can differ (not reported per file) |
| Flow | `<flow>` `<eqn>` | yes | yes | |
| Non-negative flow | `<non_negative/>` on a flow | read as `MAX(0, equation)` (reported) | | Dynamist flows have no such setting |
| Auxiliary | `<aux>` | yes | yes | |
| Constant | `<aux>` whose equation is a number | yes (back to Constant) | yes | none |
| Slider range of a constant | `<range min max>` | yes | yes | none |
| Units | `<units>`: words joined by underscores, odd names quoted | yes | yes | `<model_units>` definitions are not kept (reported) |
| Note on a variable | `<doc>` (`\n`, `\t`, `\\` escapes) | yes | yes | |
| Lookup (curve) with an input | `<aux>` with `<eqn>` and `<gf>` | yes | yes | |
| Lookup used like a function | standalone `<gf name>`; an `<aux>` with `<gf>` and no equation when it has units or a note | yes | yes | |
| Graphical function points | `<xscale>` + `<ypts>`, or `<xpts>` + `<ypts>` (`sep` attribute) | yes | `xscale` when evenly spaced | none |
| `type="extrapolate"` | linear extrapolation outside the points | read as flat (reported) | | |
| `type="discrete"` (step function) | | read as `GRAPHSTEP` when evenly spaced and the aux has an input; otherwise read as a curve (reported) | `GRAPHSTEP` is written as a discrete gf | |
| Set (items) | `<dimensions>`: `<dim name><elem name/>` or `size` | yes | yes (plain sets) | |
| Variable per set | `<dimensions>` on the variable, one equation (apply-to-all) | yes | yes | |
| Different value for one item | `<element subscript>` with its own `<eqn>` or `<gf>` | yes | yes (all elements are written, as XMILE needs) | |
| Two or more dimensions on a variable | | **left out** (reported by name) | | Dynamist sets are one-dimensional |
| Subranges, mapping between dimensions | | not read | | |
| Set with a parent set | no equivalent | | the model is **written flattened**, one variable per item named `Name[item]` (reported); the roll-up totals shown on the diagram are not variables and are not written | cannot be read back as sets |
| `TOTAL(x)`, `AVERAGE(x)` | `SUM(x[*])`, `MEAN(x[*])` | yes | yes (plain sets) | `TOTAL(x, group)` has no form: set with parents are flattened |
| Constants per set with an all-items lever scale | | | scale multiplied into the values (reported) | |
| Your own function | `<macro>` `<parm>` `<eqn>` `<doc>` | yes (expression macros) | yes | macros with variables, parameter defaults: left out (reported) |
| Module (group of variables with ports) | `<group>` with `<entity>`s, named `Outer / Inner` | | written (reported) | ports and purposes are not kept |
| XMILE `<group>` | | read as a diagram group (frame) (reported) | | not as a module |
| XMILE submodel (`<module>` + `<model name>`) | | read by **flattening**: variables named `Module.Variable`, inputs become equations (reported) | | only submodels in the same file |
| Module definitions and instances | | | | not supported |
| Diagram: positions, flow ends and corners, arrows | `<views>` `<view>` with `<stock>`, `<flow><pts>`, `<aux>`, `<connector>` | yes (a stock with width and height is placed by its top-left corner; with `<shape>` by its centre) | yes: one view per diagram sheet | arrow shapes, weights, colours and turning points are not kept (reported) |
| Arrows | | missing arrows are added for every variable an equation uses | | |
| Key levers, roll-up setting, scenarios, model checks | no equivalent | | kept in an XML comment `<!--dynamist:meta …-->`; other tools ignore it | listed as "preserved" in the report |
| Dashboards, causal loop diagrams, tables, optimisation setups, notes | no equivalent | | left out (reported) | |
| Conveyors, queues, event posters, number formats, MathML | | not supported (reported) | | |
| Variables called `INITIAL TIME`, `FINAL TIME`, `TIME STEP` (Vensim exports) | | skipped: they are the run settings | | reported as preserved |

## Names

Spaces and underscores are the same thing in XMILE, and names ignore case. The writer puts the name in the attribute as words separated by spaces (`name="Water Tank"`) and in equations with underscores (`Water_Tank`); names with any other character (brackets, dots, a leading digit) are quoted. The reader turns underscores back into spaces, so a name that had an underscore in Dynamist comes back with a space. `\n` inside a name is a space. A quoted name is one name, dots and all; a bare name with dots is `Module.Variable`.

## Functions and operators

Dynamist to XMILE and back. "Exact" means: the XMILE expression written gives the same numbers as the Dynamist function, shown by running both inside Dynamist (the 50-formula test). It does not mean another tool was run.

| Dynamist | Written to XMILE | Exact? | Read from XMILE |
|---|---|---|---|
| `IF THEN ELSE(c, a, b)` | `IF c THEN a ELSE b` | yes | the same |
| `a && b`, `a \|\| b`, `!a` | `AND`, `OR`, `NOT` | yes | the same; `=`, `<>`, `<`, `<=`, `>`, `>=` unchanged |
| `MIN`, `MAX` (up to 16 arguments) | nested two-argument calls | yes | same |
| `ABS SQRT EXP LN SIN COS TAN ARCSIN ARCCOS ARCTAN LOG10` | same name | yes | same |
| `POWER(x, y)` | `x^y` | yes | `^` |
| `LOG(x, b)`, `LOG2(x)` | `LN(x)/LN(b)`, `LN(x)/LN(2)` | yes | |
| `INTEGER(x)` (cuts toward zero) | `IF x >= 0 THEN INT(x) ELSE -INT(-x)` | yes (exact whichever way `INT` rounds negatives) | `INT(x)` is read as `FLOOR(x)` as the specification says (rounds down); reported, because Vensim's `INTEGER`, which some converters write as `INT`, truncates |
| `FLOOR(x)` | `INT(x)` | yes (XMILE `INT` rounds down) | |
| `CEILING(x)` | `-INT(-x)` | yes | |
| `ROUND(x)` (half away from zero) | `IF x >= 0 THEN INT(x+0.5) ELSE -INT(-x+0.5)` | yes except for the last bit of values just under a half | |
| `MODULO(x, y)` (never negative) | `x MOD y` | yes when `y` is a positive number; otherwise reported: XMILE `MOD` follows the sign of the divisor | `MOD` is read as `MODULO`, same report |
| `SIGN`, `CLAMP`, `HYPOT`, `CBRT`, `RADIANS`, `DEGREES`, `SINH`, `COSH`, `TANH`, `ARCTAN2` | spelled out with `IF`, `MIN`, `MAX`, `EXP`, `SQRT`, `PI`, `ARCTAN` | yes (`CLAMP` is `MIN(MAX(x, MIN(lo,hi)), MAX(lo,hi))`, which also copes with `lo > hi`) | not recognised back; the written expression is read as it stands |
| `MEAN(a, b, …)` | `(a + b + …)/n` | yes | `MEAN(a, b)` is read as `MEAN` |
| `MEDIAN` of 2 or 3 values | spelled out | yes | |
| `MEDIAN` of 4 or more, `ERF` | written unchanged | no: **other tools will not run them** (reported as dropped) | |
| `GAMMA(x)` (the gamma function) | `EXP(GAMMALN(x))` | approximated (reported): `GAMMALN` is an isee function, not in XMILE 1.0. The name `GAMMA` is never written: in Stella `GAMMA(...)` is a random-number generator | `EXP(GAMMALN(x))` is read back as `GAMMA(x)`; `GAMMALN(x)` alone as `LN(ABS(GAMMA(x)))`; any other `GAMMA(...)` is reported as unsupported and will not compile |
| `FACTORIAL(n)` | unchanged | native in Stella, not in XMILE 1.0 (reported as preserved) | |
| `PI()` | `PI` | yes | `PI` is read as `PI()` |
| `Time`, `Initial Time`, `Final Time`, `Time Step` | `TIME`, `STARTTIME`, `STOPTIME`, `DT` | yes | also `INITIAL TIME`, `FINAL TIME`, `TIME STEP` (Vensim style names) |
| `STEP(h, t)` | `STEP(h, t)` | yes when `t` is on the time grid; otherwise reported (Dynamist moves the step to the nearest time step, XMILE to the first one at or after `t`) | same, same report |
| `PULSE(start, width)` (1 for a while) | `IF TIME+DT/2 >= start AND TIME+DT/2 < start+MAX(width, DT) THEN 1 ELSE 0` | yes: the same rule Dynamist uses, written out. **The name `PULSE` is not used**, because XMILE and Stella `PULSE(volume, first, interval)` is a volume spread over one step | `PULSE(volume, first[, interval])` is read as `volume/DT × (a one-step pulse at first, repeating every interval)` |
| `PULSE TRAIN(start, width, interval, end)` | `IF … THEN 0 ELSE IF … MOD … THEN 1 ELSE 0` | yes | |
| `RAMP(slope, start, end)` | `IF TIME > start THEN slope*(MIN(TIME, end) - start) ELSE 0` | yes | `RAMP(slope, start)` is read as `RAMP(slope, start, FINAL TIME)` |
| `SINWAVE(a, p, start)`, `COSWAVE` | `a*SIN(2*PI*(TIME - start)/p)` (`start` defaults to `STARTTIME`) | yes. Stella's own `SINWAVE(a, p)` has no start and uses absolute time, so it is not used | |
| `GRAPH(x, x0, dx, y1, …)`, `GRAPHSTEP` | a graphical function `fc_graph_N` / `fc_graphstep_N` called as `fc_graph_N(x)` | yes (the start, step and values must be plain numbers, otherwise reported) | such a call is read back as `GRAPH` / `GRAPHSTEP` |
| `WITH LOOKUP(x, ((x1,y1), …))` | a graphical function `fc_lookup_N` called as `fc_lookup_N(x)` | yes | read back as `WITH LOOKUP` |
| `XIDZ(a, b, x)`, `ZIDZ(a, b)` | `IF b = 0 THEN x ELSE a/b` | yes (tests for exactly zero, as Dynamist does; Vensim and Stella also treat tiny divisors as zero) | `SAFEDIV(a, b[, x])` is read as the same `IF`, reported: it also substitutes for tiny divisors and non-finite results |
| `SMOOTH`, `SMOOTHI`, `SMOOTH3`, `SMOOTH3I` | `SMTH1`, `SMTH3` (third argument = initial value) | yes | `SMTH1`, `SMTH3`; `SMTHN(x, t, n)` with n = 1 or 3 as those, otherwise `DELAYINF` |
| `DELAY1`, `DELAY1I`, `DELAY3`, `DELAY3I` | `DELAY1`, `DELAY3` | yes | same; `DELAYN(x, d, n)` with n = 1 or 3 as those, otherwise `DELAYMTR` |
| `DELAY FIXED(x, d, init)` | `DELAY(x, d, init)` | yes (Vensim uses only the first value of `d`; Dynamist uses the current one) | `DELAY(x, d)` is read as `DELAY FIXED(x, d, INITIAL(x))` |
| `DELAYINF(x, d, n)`, `DELAYMTR(x, d, n)` | `SMTHN(x, d, n)`, `DELAYN(x, d, n)` (n a whole number, 1 to 20) | approximated (reported): inside Dynamist they match `SMOOTH3` / `DELAY3` for n = 3; **not run in Stella** | |
| `TREND(x, t, initial)` | `TREND(x, t, initial)` | the same arguments; the meaning of the third argument was not checked in another tool | `TREND(x, t)` is read with initial 0, as XMILE says |
| `INITIAL(x)` | `INIT(x)` when `x` is a variable; otherwise an auxiliary `fc_init_N` holds the expression and `INIT(fc_init_N)` is written, because `INIT` takes a variable only | yes | read back as `INITIAL(expression)`; the helper is not a variable |
| `RANDOM UNIFORM(min, max, seed)` | `RANDOM(min, max, seed)` | approximated (reported): random numbers differ between tools | `RANDOM(min, max[, seed])` |
| `RANDOM NORMAL(min, max, mean, sd, seed)` | `MAX(min, MIN(max, NORMAL(mean, sd, seed)))` | approximated (reported): Dynamist clips, Vensim and Stella redraw values outside the limits; the random numbers differ as well | `NORMAL(mean, sd[, seed])`, and the five-argument Stella form `NORMAL(mean, sd, seed, min, max)` (clipped, reported) |
| a lookup called as `Name(x)`; your own function | `Name(x)` | yes | same |
| | | | `FORCST(x, t, h[, init])` is read as `x*(1 + TREND(x, t, init)*h)`; `INF` as `1e300` (reported) |
| | | | **not supported, reported:** `EXPRND`, `LOGNORMAL`, `POISSON`, `PREVIOUS`, `SELF`, `SIZE`, `RANK`, `STDDEV`, `PROD`, other `SUM` forms, subscripts that are not a single element |

Operator precedence follows XMILE (`^` above unary minus above `* / MOD` above `+ -` above `< <= > >=` above `= <>` above `AND` above `OR`); Dynamist treats all six comparisons as one level, so the writer adds brackets wherever that matters.

### Integration method and switches

Stella's documentation says exact 0/1 switches and exact integers are only reliable under Euler; under Runge-Kutta methods the in-between evaluations can see values that are not on the step grid. The same holds for Dynamist: `STEP`, `PULSE`, `INTEGER`, `ROUND` and `IF` on a time comparison are written exactly, but a model that relies on them should be run with Euler, in either tool.

## Vensim and Stella facts applied

From the coordinator's list, checked against [Vensim and Stella side by side](Vensim-Stella-Reference.md):

1. **PULSE**: the name is not written; the Dynamist rule is written out as an `IF` (exact, because it is the same rule). Stella's volume form is read as volume ÷ DT.
2. **INTEGER**: written so that the result is the same whichever way `INT` rounds negatives.
3. **GAMMA**: written as `EXP(GAMMALN(x))`, reported as approximated; Stella's random `GAMMA(...)` is refused on reading. *(Changed.)*
4. **RANDOM NORMAL**: written as `NORMAL(mean, sd, seed)` clipped to the limits, reported (clipping versus redrawing; random numbers differ). The five-argument Stella `NORMAL` is read. *(Report text changed, reading added.)*
5. **SINWAVE/COSWAVE**: written as an explicit sine with the start time, never as Stella's two-argument `SINWAVE`.
6. **ZIDZ/XIDZ**: written as an `IF` (exact for what Dynamist does); `SAFEDIV` is read and reported as approximated. *(Reading added.)*
7. **CLAMP, CEILING, FLOOR, SIGN**: spelled out. **ERF** and `MEDIAN` of 4 or more: reported as dropped rather than "preserved". *(Changed.)*
8. **DELAY FIXED → DELAY, SMOOTH → SMTH1, SMOOTH3 → SMTH3**: as asked. `DELAYINF` is written as `SMTHN` (a chain of smooths) and `DELAYMTR` as `DELAYN`, both reported as approximated because only the Dynamist side was run. *(Reports added.)*
9. **INITIAL(x)**: `INIT` with a bare variable; any expression goes through a helper auxiliary. *(Changed.)*
10. **Euler for switches and integers**: stated above.

## Real files (SDXorg test-models)

67 `.xmile` files, mostly Vensim models converted by *xmutil*, read and run against their expected output: **40 match**. The other 27 (25 run but differ, 2 cannot be read):

- **2 cannot be read** because the files themselves are not well-formed XML (a `<flow>` is never closed): the reader says which line.
- **Problems in the files, not in the reader (10):** equations that the converter left as `???` (the `subscript_*` files with `Rate A`), a macro called in an equation but never defined in the file (the four `macro_*` files), an expected output that belongs to another model (`smooth_and_stock`), expected values that follow the original Vensim file where the XMILE differs (`active_initial`), and numerical noise near zero (`exp`).
- **Dynamist does not do it (9):** variables with two dimensions or subranges (`subscript_mixed_assembly`, `subscript_updimensioning`, `subscript_subranges`, `subscript_subranges_equal`, `min_max_1arg`), and the Stella meaning of non-negative stocks and flows (`non_negative_all` ×2, `non_negative_stocks` ×2).
- **Different meaning of `INT` and `MOD` (1):** `rounding`: the file says `INT` and `MOD`, the expected numbers are Vensim's `INTEGER` (truncates) and `MODULO`. Both are reported on reading.
- **Not looked into (5):** `bpowers-hares_and_lynxes_modules` (submodels are flattened and the run starts right, but the numbers drift from the expected ones from the first step), `arithmetics_exp`, `zeroled_decimals`, `subscripted_trig` and `subscripted_flows` are partly explained by the points above but not examined one by one.

Run it yourself (the corpus is not in this repository):

```
XMILE_CORPUS=/path/to/test-models cargo test -p dynamist-core corpus_xmile -- --ignored --nocapture
```

## What could not be verified without another XMILE tool

- That Stella, Vensim or PySD open and run a file Dynamist writes (element order, `<views>` details and the `GAMMALN`, `SMTHN` and `DELAYN` calls are the likeliest trouble).
- The meaning of `TREND`'s third argument and of `DELAYN`/`SMTHN` initial values in Stella.
- Whether a named `<gf>` standing alone is called as `name(x)` by every tool.
- The `<!--dynamist:meta …-->` comment is legal XML but is not part of XMILE; a tool that rewrites the file will drop it, and the levers, scenarios and checks with it.
