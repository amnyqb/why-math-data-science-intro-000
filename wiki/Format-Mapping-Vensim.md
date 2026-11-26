# Format mapping: Dynamist ↔ Vensim `.mdl`

How a model is carried between Dynamist and Vensim's classic text format. The reader is `dynamist_core::mdl::read`, the writer `dynamist_core::mdlw::write`. Both return a **loss report** (`dynamist_core::loss::LossReport`): every construct that was dropped, written in a different form, or kept only as opaque text. An empty report means nothing was lost.

> **The reader is checked against Vensim's own output; the writer is not.** Benchmark B2 ([Import fidelity](Import-Fidelity.md)) imports every model of two public test suites and compares each variable at every saved time with the results Vensim exported. It also compares every model of a Vensim install with PySD, an independent open-source engine for Vensim models. The writer is still tested only by writing and reading back with our own reader. Everything under "Unverified" below is about the writer and needs one person with Vensim to open a written file.

## Constructs

| Dynamist | Vensim | Loss |
|---|---|---|
| Stock (initial value, flows in and out) | `x = INTEG(in - out, init)` | none. On reading, `INTEG(a - b, init)` becomes a stock with flows `a` and `b` when they are plain variables (not constants, not already used by another stock on the same side); otherwise one `x net flow` carries the whole rate. |
| Flow (from / to a stock, or a cloud) | an ordinary variable that appears in `INTEG`; in the sketch a valve with pipes | none. Cloud positions are redrawn. |
| Constant (+ slider range) | `x = 5 ~ units [min,max,step]` | none |
| Variable (aux) | `x = formula` | none |
| Lookup used as a function | `x( [(xmin,ymin)-(xmax,ymax)], (x1,y1), … )` | none. `:INTERPOLATE:` is accepted; `:HOLD BACKWARD:`, `:LOOK FORWARD:`, `:RAW:` are read as straight-line interpolation and reported. |
| Lookup with an input | `x = WITH LOOKUP(input, ([…],(x1,y1),…))` | none |
| Units | the first `~` field; `[min,max]` becomes the slider | `**undefined**` ↔ empty |
| Comment | the second `~` field | none |
| Run settings | `INITIAL TIME`, `FINAL TIME`, `TIME STEP`, `SAVEPER` (units of FINAL TIME = time units) | `SAVEPER` other than the time step is reported and ignored: every step is kept. Expressions such as `1/4` are evaluated. |
| Integration method | not in the equations | Euler is assumed on reading. RK2/3/4 are **not written** (reported): Vensim then uses its default. |
| Set (list of items) | range `Dim: a, b, c`; numbered `(A1-A5)` | none |
| Set with a parent (each tank belongs to one region) | mapping `Tank: T1, T2, T3 -> (Region: R1, R1, R2)` | a mapping whose list does not match is dropped and reported. A positional `-> Region` of equal size becomes "belongs to the item in the same place" (reported). |
| Subrange (`Sub: B, C` inside `Dim`) | range | read as an independent set (reported). A formula over the larger set that reads the subrange, or the other way round, is written one item at a time, and each item resolves to itself. A variable defined in parts (`x[infant] = …`, `x[all but youngest] = …`) gets each item's own formula. |
| Dimension listing other dimensions (`dim: A, subdim1, B`) | range | the other dimensions' items are put in place, in order (nested a few levels deep). |
| Alias (`A <-> B`) | `<->` | read as a second set with the same items (reported). The alias may come before the dimension it copies. |
| Mapping between dimensions (`prev: infant, child -> older`) | `->` | a reference `x[prev]` in a formula over `older` reads the item in the same place: `x[infant]` for `child`. Such formulas are written item by item. A mapped dimension is never read as "the same item" just because it contains it. |
| A dimension or item name used as a number (`IF THEN ELSE(DimA = Selected A, 1, 0)`, `dim1 * Time`) | value | the item's place, counted from 1 in the dimension that first declares it (so `dim1 = dim3` compares items even when `dim3` lists them in another order), as Vensim does. Written item by item. |
| Array constants written as a table | `x[A, B] = 1, 2, 3; 4, 5, 6;` | one value per combination, the last subscript varying fastest (`,` between columns, `;` between rows); a fixed item such as `[A, B, Depth 1]` takes one table per item. |
| Variable per set, same formula for all | `x[Dim] = formula` | none |
| Variable per set, items differ | `x[A] = …`, `x[B] = …`, and `:EXCEPT:` | none. On reading, the first item's formula is the base and the others are overrides. |
| Variable over **two or more** dimensions | `x[A,B] = …` | read as one variable per combination named `x[a,b]` (reported, capped at 5000). A formula over one set that reads such a variable by dimension (`y[dim1] = x[dim1, D]`) is written item by item. Not written (a model never holds one). |
| `TOTAL(x)` in a plain formula | `SUM(x[Dim!])` | none |
| `AVERAGE(x)` | `SUM(x[Dim!]) / ELMCOUNT(Dim)` | none |
| `TOTAL(x, group)`, `TOTAL(x)` inside a per-region formula | the items written out: `(x[T1] + x[T2])` | none, but written item by item |
| `Name[item]` | `x[item]` | none |
| `SUM`, `PROD`, `VMIN`, `VMAX` over `Dim!`, `ELMCOUNT` | read: `SUM(x[Dim!])` in a plain formula → `TOTAL(x)`, anything else is written out as an explicit sum, product, `MIN(…)`, `MAX(…)` | reported as approximated (the formula is longer, not different) |
| A variable read through another dimension (`y[Region]` in a per-tank formula) | mapping | written item by item; reported on reading if no mapping exists |
| Your own function (Edit → My functions) | `:MACRO: … :END OF MACRO:` | read: a macro of plain equations (no `INTEG`) becomes a function. Written: the formula is substituted at each use (reported). A macro with a stock is dropped (reported). |
| Combine = average, never negative, key lever | no field | carried in the variable's comment as `[dynamist: combine=average; never-negative; lever=2]` and restored on reading. Vensim itself ignores the tag; it does **not** enforce never-negative (reported as preserved). |
| Data variable / `GET XLS|DIRECT …` / `:=` | data | read as a placeholder (a lookup over `Time` that is zero, or a constant 0 for `CONSTANTS`) with the original text kept in the comment as `[Vensim data: …]`; written back as the same data definition. The file's numbers are **not** imported (reported as dropped). |
| Modules (nested) | views | written as one sketch view per diagram sheet, named after the module; ports and descriptions are not written (reported). |
| Scenarios, model checks, notes, tables, results, dashboards, causal loop and optimisation sheets | — | dropped and reported |
| Arrow polarity marks | — | not written (reported) |
| Sketch views | `*View name` sections | read: all views are merged; each variable keeps its first position (reported). Arrows are read per view (their numbers restart in each view). |
| Sketch comments | type 12 objects other than clouds | not read (reported) |
| Run-settings block after the sketch (`:L<%^E!@ …`) | integration type, output options | not read (reported); not written |

## Functions

Built-ins we write unchanged (same name and meaning in Vensim): `IF THEN ELSE`, `ABS`, `SQRT`, `EXP`, `LN`, `LOG(x, base)`, `INTEGER`, `POWER`, `SIN`, `COS`, `TAN`, `ARCSIN`, `ARCCOS`, `ARCTAN`, `SINH`, `COSH`, `TANH`, `XIDZ`, `ZIDZ`, `STEP`, `PULSE`, `PULSE TRAIN`, `RAMP`, `SMOOTH`, `SMOOTHI`, `SMOOTH3`, `SMOOTH3I`, `DELAY1`, `DELAY1I`, `DELAY3`, `DELAY3I`, `DELAY FIXED`, `TREND`, `INITIAL`, `MODULO`, `RANDOM UNIFORM`, `RANDOM NORMAL`, `WITH LOOKUP`.

| Dynamist | Written for Vensim | Note |
|---|---|---|
| `MIN(a, b, c, …)`, `MAX(…)` (up to 16) | nested two-argument calls | exact |
| `ROUND(x)` | `IF THEN ELSE(x >= 0, INTEGER(x + 0.5), -INTEGER(-(x) + 0.5))` | exact; halves round away from zero |
| `FLOOR`, `CEILING` | `IF THEN ELSE` with `INTEGER` | exact |
| `SIGN`, `CBRT`, `HYPOT`, `CLAMP`, `MEAN` | spelled out with `IF THEN ELSE`, `^`, `MIN`/`MAX`, sums | exact |
| `RADIANS`, `DEGREES` | multiplied by π/180 (a number, not `PI`) | exact |
| `LOG10`, `LOG2` | `LOG(x, 10)`, `LOG(x, 2)` | exact |
| `PI()` | `3.141592653589793` | |
| `SINWAVE(a, p, start)`, `COSWAVE` | `a * SIN(2π (Time − start) / p)`; start defaults to `INITIAL TIME` | exact |
| `GRAPH(x, x0, dx, y1, y2, …)` | `WITH LOOKUP(x, ([…],(x0,y1),(x0+dx,y2),…))` when all are plain numbers | exact |
| `DELAYINF(x, d, n)` | `n` = 1: `SMOOTH`; 3: `SMOOTH3`; other `n`: `SMOOTH N(x, d, x, n)` | the `SMOOTH N` / `DELAY N` spelling is **unverified** (reported) |
| `DELAYMTR(x, d, n)` | `n` = 1: `DELAY1`; 3: `DELAY3`; other: `DELAY N(x, d, x, n)` | as above |
| `GAMMA(x)` | `EXP(GAMMA LN(x))` | Vensim documents only `GAMMA LN`, the log of the gamma function; equal to rounding error (reported as approximated) |
| `FACTORIAL(n)` | `EXP(GAMMA LN(n + 1))` | as above |
| `GRAPHSTEP`, `MEDIAN`, `ERF`, `ARCTAN2` | written unchanged | no Vensim function with this meaning that we know of; Vensim reports an unknown function (reported as preserved) |
| `&&`, `\|\|`, `!`, `!=`, `==` | `:AND:`, `:OR:`, `:NOT:`, `<>`, `=` | |

Read as Vensim means them (each checked against Vensim's exported output in benchmark B2):

| Vensim | Dynamist | Note |
|---|---|---|
| `ACTIVE INITIAL(x, init)` | `ACTIVE INITIAL(x, init)` | an engine function: `x` during the run, `init` only while initial values are computed. It breaks a loop among initial values, as World3-03 needs. |
| `SAMPLE IF TRUE(condition, input, initial)` | the same | an engine function that holds its value between true conditions |
| `QUANTUM(x, q)` | the same | `q × INTEGER(x / q)`; `x` when `q ≤ 0` |
| `GAME(x)` | `(x)` | outside Vensim's gaming mode the value is the formula (reported) |
| `SMOOTH N(x, t, init, n)` / `DELAY N(x, t, init, n)` | `DELAYINF(x, t, n[, init])` / `DELAYMTR(x, t, n[, init])` | every stage starts at `init`. The order may be a named constant; it is read once (reported). |
| Old DYNAMO-style table `name ( x1, …, xn, y1, …, yn )` | a lookup | the flat list is the x values, then the y values (World Dynamics, Market Growth) |
| `:NOT: a = b` | `!(a = b)` | `:NOT:` covers the whole comparison, unlike `!` |
| `NPV(stream, rate, init, factor)` | the same | an engine function with two hidden stocks (the discount factor and the discounted sum), as Vensim defines it |
| `DEPRECIATE STRAIGHTLINE(input, time, …)` | the same | each input written off evenly over the time |
| `GAMMA LN(x)`, `RANDOM EXPONENTIAL(…)` | the same | the random draws differ from Vensim's; the distribution is the same |
| `LOOKUP FORWARD`, `LOOKUP BACKWARD`, `LOOKUP INVERT`, `LOOKUP EXTRAPOLATE` | the same | the lookup is named as the first argument |
| `FORECAST(input, time, horizon[, trend])` | `input * (1 + TREND(input, time, trend) * horizon)` | Vensim's definition |
| `TIME BASE(t, dt)`, `REINITIAL(x)`, `DELAY INFORMATION(x, t, i)` | `t + dt * Time`, `INITIAL(x)`, `SMOOTHI(x, t, i)` | |
| `ALLOCATE AVAILABLE`, `ALLOCATE BY PRIORITY`, `VECTOR SORT ORDER`, `VECTOR RANK`, `VECTOR REORDER`, `VECTOR ELM MAP`, `VECTOR SELECT`, `INVERT MATRIX` | `… ITEM(k, n, every element…)` | written out element by element: each item of the result gets every element of the arrays as arguments, and the engine computes the item. `!` on the vector dimension is optional. `ALLOCATE AVAILABLE` takes profile types 1–4 (rectangular, triangular, normal, exponential). |
| `:NA:` | `-1.298074214633707e33` | Vensim's missing value |
| `ZIDZ(a, b)`, `XIDZ(a, b, x)` | the same | the divisor counts as zero below 1e-6 in magnitude, as Vensim |
| `RANDOM 0 1()` | `RANDOM UNIFORM(0, 1, 0)` | different numbers from Vensim's stream |
| bare `PI` | `PI()` | |

Vensim functions with no engine equivalent are **kept as written** on reading, so the model shows them as a problem and cannot run: `DELAY CONVEYOR`, `DELAY BATCH`, `QUEUE …`, `FIND MARKET PRICE`, `FIND ZERO`, `SIMULTANEOUS`, `GET DATA …`, `SHIFT IF TRUE`, `VECTOR LOOKUP`, `A FUNCTION OF` and any other not in our list (reported once each as preserved). The open ones are listed in the [backlog](BACKLOG.md) (B32).

Files saved by older Vensim versions in Windows-1252 or Latin-1 (not UTF-8) are read byte by byte, so names with é or ü survive.

### Where the same name means something slightly different

| Function | Dynamist | Vensim (as documented) | Effect |
|---|---|---|---|
| `STEP(h, t0)`, `PULSE`, `PULSE TRAIN` | switch on when `Time + dt/2 ≥ t0` | when `Time ≥ t0` | only when the time is not on the step grid: ours switches up to half a step earlier. `PULSE TRAIN` at its end time matches Vensim's output (the pulse is on at `t = end`); PySD differs there. |
| `PULSE(start, width)` | width shorter than a step counts as one step | width 0 = one step | the same on the grid |
| `RANDOM UNIFORM`, `RANDOM NORMAL` | own generator, one stream per run | seeded stream | the same distribution, different numbers; a run is not reproducible between the tools |
| `MODULO(x, y)` | `x mod y`, never negative for `y > 0` | remainder | may differ for negative `x` (**unverified**) |
| `INTEGER(x)` | truncates toward zero | truncates toward zero | same |
| `SMOOTH`, `SMOOTH3`, `DELAY1`, `DELAY3` | initial output equals the input at the start (steady state) | same convention | same; checked against Vensim's output (B2) |
| `TREND(x, t, init)` | `(x − avg) / (avg · t)` with the average smoothed over `t`, starting from `init` | same definition | same; not cross-checked |
| `DELAY FIXED(x, d, init)` | a recorded history; `init` until `d` has passed | a pipeline delay | same on the grid |
| Lookups outside their range | the end value | the end value (no extrapolation unless asked) | same; `:EXTRAPOLATE:` is not read |
| `IF THEN ELSE` | the condition is true when non-zero | the same | same |

## The sketch: what is assumed

Written to look like Vensim's own sketches; **none of it was checked in Vensim**.

- Header lines `\\\---/// Sketch information - do not modify anything except names` and `V300  Do not put anything below this section - it will be ignored`, then `*View name` and the `$192-192-192,…` style line.
- Variable: `10,id,name,x,y,40,20,shape,3,0,0,0,0,0,0`; shape 3 for a stock, 8 for everything else. Names containing commas or quotes are quoted.
- Flow: valve `11,id,0,x,y,6,8,34,3,0,0,1,0,0,0`, its label `10,id,name,x,y+20,49,8,40,3,0,0,-1,0,0,0`, a cloud `12,id,48,x,y,10,8,0,3,0,0,0,0,0,0` for each end not attached to a stock, and two pipes `1,id,valve,end,flag,0,0,22,0,192,0,-1--1--1,,1|(x,y)|`. Flag **4** goes to the end the flow runs *to*, flag **100** to the end it runs *from* (this is how Vensim's own teacup file looks).
- Information arrow: `1,id,from,to,0,0,0,0,0,64,0,-1--1--1,,1|(0,0)|`; an arrow into a flow ends at its valve; arrows into stocks are not written.
- Ids restart in each view. Arrows between variables on different views are not written (Vensim draws shadow variables; reported).
- No run-settings block (`:L<%^E!@ …`) is written. Vensim should open a file without it; if it does not, that block is the first thing to add.

## Unverified without Vensim

1. That Vensim opens a written file at all, and draws the sketch as intended (shapes, pipe flags, label offsets).
2. The mapping direction in `Tank: T1, T2 -> (Region: R1, R1)`: written the way Vensim documents mappings, but Vensim must accept `y[Region]` read in a per-tank formula. We avoid relying on it by writing such formulas item by item.
3. The spelling of `SMOOTH N` and `DELAY N` **as written** (reading them is checked against Vensim's output).
4. `MODULO` for negative numbers.
5. That Vensim keeps free text such as `[dynamist: …]` untouched in comments (it should; it is the comment field).
6. Whether Vensim needs the run-settings block.

## Using it

```rust
let (model, loss) = dynamist_core::mdl::read(&text)?;   // loss: what could not be carried over
let (text, loss)  = dynamist_core::mdlw::write(&model); // loss: what the file cannot hold
println!("{}", loss.summary());
```

`mdl::import(text)` is the same as `read` without the report (the desktop app uses it).

## Tests

**Against Vensim:** benchmark B2 ([Import fidelity](Import-Fidelity.md)) runs every model of two public test suites and compares it with Vensim's exported output. That gives a pass rate, and a fix list for whatever doesn't pass.

`core/src/mdl.rs` (reader, 15 tests: subscripts, `:EXCEPT:`, two dimensions, macros, data variables, Vensim-only functions, lookups, quoted names, views, a teacup as Vensim saves it, mappings, loss reports) and `core/src/mdlw.rs` (writer, 9 tests: round trips of the starter model and of a model with sets, parents, totals and per-item values; every function above; loss reports; the comment tags). `src/mdl_corpus_tests.rs` writes the fuel-supply model (57 tanks, 4 regions, 10 modules; about 120 KB, 3,200 lines), reads it back and checks that every series that exists in both agrees to 1e-9 at every step.
