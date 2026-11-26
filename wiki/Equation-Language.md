# Equation language

## Names

- Names may contain letters, digits, spaces and underscores: `Contact Rate`, `Fuel in tank 2`.
- They are **not case-sensitive**, and spaces and underscores count as the same: `contact_rate` = `Contact Rate`.
- Names that hold other characters can be written in double quotes: `"Cost (USD)"`.
- Reserved names: `TIME`, `INITIAL TIME`, `FINAL TIME`, `TIME STEP` (also `dt`).

## What each kind of item's equation means

| Kind | Its equation is… |
|---|---|
| Constant | a number (or a formula of other constants) |
| Variable | a formula, re-calculated every time step |
| Flow | a formula giving a rate (amount per time unit) |
| Stock | its **initial value** only; it then changes by *inflows − outflows* every step |
| Lookup with an input | the input; the value is the curve read at that input |
| Lookup without an input | nothing; call it like a function: `Effect(x)` |

## Operators (highest priority first)

| Operator | Meaning |
|---|---|
| `( )` | grouping |
| `^` | power (`-x^2` is `-(x²)`) |
| `-x`, `:NOT: x` (`!x`) | negation, logical not |
| `*  /` | multiply, divide |
| `+  -` | add, subtract |
| `<  <=  >  >=  =  <>` | comparisons, giving 1 (true) or 0 (false) |
| `:AND:` (`&&`) | both true |
| `:OR:` (`||`) | either true |

## Functions

Logic (`IF THEN ELSE`), maths (`MIN`, `MAX`, `ABS`, `SQRT`, `EXP`, `LN`, `LOG`, `POWER`, `INTEGER`, `ROUND`, `MODULO`, `SIN`, `COS`, `TAN`, `ARCTAN`, `XIDZ`, `ZIDZ`), test inputs (`STEP`, `PULSE`, `PULSE TRAIN`, `RAMP`, `RANDOM UNIFORM`, `RANDOM NORMAL`), delays and smoothing (`SMOOTH`, `SMOOTHI`, `SMOOTH3`, `SMOOTH3I`, `DELAY1`, `DELAY1I`, `DELAY3`, `DELAY3I`, `DELAY FIXED`, `TREND`, `INITIAL`) and lookups (`WITH LOOKUP`, `Name(x)`).

Every function, with parameters, results, examples and tips: **[Function reference](Function-Reference.md)**.

## Sets

When a variable is *one for each* item of a set (see [Sets](Sets.md)): other per-set variables mean the same item; `TOTAL(X)`, `TOTAL(X, group)`, `AVERAGE(X)`, `AVERAGE(X, group)` add up or average over items; `X[item]` picks one. `CEILING(x)` rounds up (whole trucks, crews, batches).

## Units

Write units as text: `MB`, `Truck/Day`, `Barrels per Truck`, `1/Day`, `MB*Day`, with `^` for powers. They are read as algebra (see the User guide); `Day of Month` counts as a day, and `Dmnl`, `Fraction`, `Ratio` and `Yes/No` as dimensionless.

## Modules with ports

A reusable module's variables are written inside its definition; an instance's variables become `Instance.Variable`. To read one in an equation, quote it: `"North.Stock" / 20`.

## Simulation

- The run goes from *Initial time* to *Final time* in steps of *Time step*, using **Euler** (fast, the classic choice) or **Runge-Kutta 4** (more accurate for smooth models).
- Each step: every variable and flow is calculated in dependency order; then each stock changes by `time step × (inflows − outflows)`.
- A loop of variables with no stock in it ("circular logic") is reported as a problem; put a stock or a delay function in the loop.
- Random functions give the same sequence every run, so results are repeatable.
