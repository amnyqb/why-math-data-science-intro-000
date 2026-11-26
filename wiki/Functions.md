# Functions: built-in, your own, and from Python

Three layers, from simplest to most open. All of them work in any equation.

## 1. The built-in library
About 60 functions: logic, maths, test inputs, delays, lookups, sets. **Help → Function reference** (F1) or [Function-Reference.md](Function-Reference.md) lists each with examples. The maths part now covers what Python's `math` module offers for everyday modelling: `SINH COSH TANH ARCSIN ARCCOS ARCTAN2 FLOOR CEILING SIGN LOG10 LOG2 CBRT HYPOT RADIANS DEGREES CLAMP MEAN MEDIAN PI() ERF GAMMA FACTORIAL`, plus the earlier `MIN MAX ABS SQRT EXP LN LOG POWER INTEGER ROUND MODULO SIN COS TAN ARCTAN XIDZ ZIDZ`.

`ERF` is good to about 1.5e-7 and `GAMMA` to about 1e-13; both are checked against Python in the tests. Need more digits or a function we don't have? Use layer 3.

`SINWAVE`, `COSWAVE` (amplitude, period, optional start), `GRAPH`/`GRAPHSTEP` (an inline curve from evenly spaced values) and `DELAYINF`/`DELAYMTR` (information and material delays of a chosen order, 1–20) follow Powersim's rules; `DELAYMTR(x, d, 3)` equals `DELAY3(x, d)` and `DELAYINF(x, d, 3)` equals `SMOOTH3(x, d)`. Integration methods: Euler, RK2 (Heun), RK3, RK4.

### Verbs: one function with options
Instead of learning a family of functions, say what you mean with an option, the way Python does. **`DELAY`** is the first:

| You write | You mean | Same as (Vensim) |
|---|---|---|
| `DELAY(Shipments, 6)` | material in transit for about 6, spread out | `DELAY1(x, 6)` |
| `DELAY(Shipments, 6, order=3)` | the same, more bunched around 6 | `DELAY3`, or `DELAYMTR(x, 6, n)` for another order |
| `DELAY(Shipments, 6, exact=True)` | a pipeline: arrives exactly 6 later | `DELAY FIXED(x, 6, x)` |
| `DELAY(Sales, 3, kind=information)` | a belief catching up with a fact (nothing conserved) | `SMOOTH(x, 3)`, `SMOOTH3`, `DELAYINF` |
| `DELAY(Orders, 4, init=2)` | start with 2 coming out | `DELAY1I(x, 4, 2)` |

**`ALLOCATE`** shares a short supply among the items of a set, a different case with the same idea:

| You write | You mean | Same as (Vensim) |
|---|---|---|
| `ALLOCATE(Demand, Supply)` | everyone gets a share in proportion to what they ask (all of it if there is enough) | a formula; Vensim has no function for this |
| `ALLOCATE(Demand, Supply, by=Priority)` | the highest priority is served first | `ALLOCATE BY PRIORITY` |
| `ALLOCATE(Demand, Supply, by=Priority, overlap=0.5)` | items whose priorities are within 0.5 share | `ALLOCATE BY PRIORITY` with that width |
| `ALLOCATE(Demand, Supply, by=Priority, curve=normal, spread=2)` | a smooth fall-off around each priority: lower priorities get a little before the higher ones are full | `ALLOCATE AVAILABLE` with the normal profile |

Write it in a formula that is per the same set as the requests: `Delivered[Region] = ALLOCATE(Demand, Supply, by=Priority)`. Each item gets its own share. The curves are `rectangular`, `triangular`, `normal` and `exponential`. A file exported to Vensim contains `ALLOCATE BY PRIORITY(...)`; `curve=` has no one-line Vensim form (Vensim needs a table of profiles), so it is kept as written and reported.

**`QUEUE` and `WAIT`** are for waiting lines where *who* waits longest matters:

| You write | You mean | Same as (Vensim) |
|---|---|---|
| `Waiting = QUEUE(Arrivals, Treated)` | a waiting room: people join at the arrival rate and leave, oldest first, at the treated rate; `Waiting` is how many | `QUEUE FIFO(Arriving, Servicing, profile, initial, range)` |
| `WAIT(Waiting)` | how long those waiting have waited, on average | `QUEUE AGE AVERAGE(Waiting)` |
| `WAIT(Waiting, measure=oldest)` | the longest wait | `QUEUE AGE OLDEST(Waiting)` |
| `WAIT(Waiting, measure=count, older_than=2)` | how many have waited more than 2 time units | `QUEUE AGE IN RANGE(Waiting, 2, …)` |

Never more leave than are waiting; a negative flow counts as zero. `init=` sets the number waiting at the start (they count as one time step old). **Not the same as Vensim:** Vensim's `QUEUE FIFO` spreads the starting crowd over ages with a profile; here they start one step old. **Not yet checked against Vensim's own numbers** (queues are a DSS function, so no output is available); the rules were checked by hand against a case worked out on paper (see the test `queue_and_wait_follow_first_in_first_out`). Queues of attributes (`QUEUE FIFO ATTRIB`) are not built.

**`AGE`** moves people up through age classes:

| You write | You mean | Same as (Vensim) |
|---|---|---|
| `Change[age] = AGE(Population, every=5, entrants=Births) - Deaths[age]` | every five years each class moves up to the next, the youngest is refilled with five years of births, deaths take people in every class | `SHIFT IF TRUE` and a hand-built chain |
| `AGE(Population, every=5, open_ended=False)` | the oldest leave the model at each move | |

Write it in a formula that is per the age set (a set whose items run from the youngest up), and use the sum as the stock's change. The classes move together, so the stock changes in steps once per period: **read it at the years the classes move** (every 5 years here). Use Euler integration. The Vensim writer gives each class's ageing flow as a formula, since `SHIFT IF TRUE` changes the stock itself.

**`EXPECT`**, **`LOOK`** and the extra options of `VALUE` and `DELAY`:

| You write | You mean | Same as (Vensim) |
|---|---|---|
| `EXPECT(Demand, 6)` | what people believe demand is: the fact, noticed over about 6 weeks | `SMOOTH(x, 6)` |
| `EXPECT(Demand, 6, measure=trend)` | the growth they think they see | `TREND(x, 6, 0)` |
| `EXPECT(Demand, 6, ahead=8)` | where they expect demand to be in 8 weeks, extrapolating the trend | `FORECAST(x, 6, 8)` |
| `LOOK(Effect, Price)` | read the curve normally | `Effect(Price)` |
| `LOOK(Effect, 0.7, mode=inverse)` | the price at which the effect is 0.7 (also for a falling curve) | `LOOKUP INVERT` |
| `LOOK(Effect, x, mode=forward\|backward\|extrapolate\|slope\|area)` | as steps, beyond the ends, how steep, the area (`start=`, `outside=`) | `LOOKUP FORWARD`, `BACKWARD`, `EXTRAPOLATE`, `SLOPE`, `AREA` |
| `VALUE(Cash Flow, measure=irr)` | the rate of return that makes the stream worth nothing | `INTERNAL RATE OF RETURN` |
| `VALUE(Investment, writeoff=10)` | write each amount off evenly over 10 | `DEPRECIATE STRAIGHTLINE` |
| `DELAY(x, 4, exact=True, leak=0.1)` | a conveyor that loses 10% a step: 100 enters, 65.6 arrives after 4 steps | `DELAY CONVEYOR` with leakage |

The rate of return and the conveyor follow the Vensim help; there is no Vensim output to compare them with. `DELAY` with `batch=` or `profile=` is not built.

Options are named, come after the plain arguments, and are checked: a misspelt option says what the options are (`DELAY has no option 'ordr' — did you mean 'order'?`). The call is rewritten into the engine's own functions when the equation is read and when a model is exported, so a file exported to Vensim contains `SMOOTH3I(x, 6, 1)`, and an imported Vensim file keeps working. Each combination is tested to give exactly the numbers of the function it replaces. More verbs (`ALLOCATE`, `VALUE`, `QUEUE`) are planned in the [backlog](BACKLOG.md) (B42).

## 2. Your own functions (Edit → My functions…)
Write a formula once, with parameters, and use it like a built-in:

| Name | Parameters | Formula |
|---|---|---|
| `Effect of price` | `p` | `1 - 0.3 * LN(p)` |
| `Capped` | `x, top` | `MIN(CLAMP(x, 0, top), top)` |

Then: `Demand * Effect of price(Price / Base Price)`.

- A call is replaced by the formula, each parameter by its argument, *before* the equation is read. So a function can use other functions (up to 12 levels; it cannot call itself), variables, `TIME`, and `TOTAL(...)` of set variables, and costs nothing at run time.
- The window says "✓ reads correctly" or what is wrong. Names that clash with a built-in are refused.
- They are saved with the model, and in a spreadsheet as a **Functions** sheet (`Name`, `Parameters`, `Body`, `Description`).
- Not yet: the equation editor's autocomplete does not suggest your functions.

## 3. A function from Python (Edit → Function from Python…)
Anything Python's standard library can compute — `math.erf(x)`, `statistics.NormalDist(0, 1).cdf(x)`, `math.gamma(x + 1)`, `sum(1 / k for k in range(1, 6)) * x` — is **sampled once over a range into a curve** (a lookup) that equations use like any lookup: `Normal curve(Z)`. Choose the name, the expression in `x`, the range and the number of points (2–1000). *Try it* shows the result; *Create the curve* stores it (re-running with the same name updates it). The expression is kept in the curve's note.

**Why a curve and not live Python:** the model runs thousands of evaluations per second; calling Python each time would be slow and would make every model file a program. A curve is fast, saved with the model, and works without Python afterwards. Between the sampled points it is linear, so use enough points for smooth functions.

**Safety:** Python runs only when you press a button in this window, on your computer, on what you typed, with `python3 -I`, only `math`, `statistics` and a few harmless built-ins (no `open`, no `import`), and a 15 second limit. Model files and spreadsheets contain no code and never run any: opening or importing a model cannot run Python. (This is a guard against slips, not a hardened sandbox: don't paste expressions from people you don't trust.) Python 3 must be installed (`python3` on the path).

## Which to use?
Simple arithmetic you will reuse → **your own function**. A standard curve (normal, gamma, a physics law) → **built-in** if listed, otherwise **from Python**. Something that needs real code or data → compute it outside, and bring the result in as a **lookup** or a TimeSeries/ItemSeries sheet.
