# Function reference

> Generated from the app (`src/docs.rs`) — edit there, then regenerate with `dynamist --write-docs wiki`. Every entry here is also in the app: **Help → Function reference** or **F1**.

## Logic

### IF THEN ELSE

Chooses between two values depending on a condition.

```
IF THEN ELSE(condition, a, b)
```

**In Vensim:** `IF THEN ELSE(c, a, b)` · **In Stella:** `IF c THEN a ELSE b`

| Parameter | Meaning |
|---|---|
| `condition` | Anything that is true (not zero) or false (zero), e.g. Stock > 100 |
| `a` | Value when the condition is true |
| `b` | Value when the condition is false |

**Result:** a when the condition holds, otherwise b.

The condition is checked again at every time step, so the result can switch back and forth during a run. Combine conditions with :AND: and :OR:.

**Example:** `IF THEN ELSE(Inventory < Reorder Point, Order Size, 0)`

- Tip: Comparisons give 1 (true) or 0 (false), so you can also multiply by them: (Time > 10) * 5.
- Tip: Avoid switching on exact equality of decimal numbers; use < or > instead.

See also: MIN, MAX, STEP

---

### AND

1 when every argument is true.

```
AND(a, b, …)
```

**Result:** A number.

Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.

**Example:** `AND(Open, Staffed)`


---

### OR

1 when any argument is true.

```
OR(a, b, …)
```

**Result:** A number.

Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.

**Example:** `OR(Open, Staffed)`


---

### NOT

1 when x is zero.

```
NOT(x)
```

**Result:** A number.

Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.

**Example:** `NOT(Open)`


---

### BETWEEN

1 if low <= x <= high.

```
BETWEEN(x, low, high)
```

**Result:** A number.

Calendar functions need a start date in the model settings and a time unit of Hour, Day, Week, Month or Year.

**Example:** `BETWEEN(Level, 10, 20)`


---

## Math

### MIN

The smallest of two or more values.

```
MIN(a, b, …)
```

**In Vensim:** `MIN(a, b)` · **In Stella:** `MIN(a, b)`

| Parameter | Meaning |
|---|---|
| `a, b, …` | Two to sixteen values or expressions |

**Result:** The smallest value.

Useful to cap a flow so it can never be larger than what is available, e.g. shipments limited by inventory.

**Example:** `MIN(Desired Shipments, Inventory / Minimum Delivery Time)`

- Tip: MIN(x, 0) keeps only the negative part of x.

See also: MAX, IF THEN ELSE

---

### MAX

The largest of two or more values.

```
MAX(a, b, …)
```

**In Vensim:** `MAX(a, b)` · **In Stella:** `MAX(a, b)`

| Parameter | Meaning |
|---|---|
| `a, b, …` | Two to sixteen values or expressions |

**Result:** The largest value.

Often used to keep a value from going below a floor, such as zero.

**Example:** `MAX(0, Desired Production - Capacity)`

- Tip: MAX(x, 0) keeps a flow from becoming negative.

See also: MIN, ABS

---

### ABS

The size of a number, without its sign.

```
ABS(x)
```

**In Vensim:** `ABS(x)` · **In Stella:** `ABS(x)`

| Parameter | Meaning |
|---|---|
| `x` | Any value |

**Result:** x if x is positive, −x if it is negative.

**Example:** `ABS(Target - Actual)`


See also: MAX, MIN

---

### SQRT

Square root.

```
SQRT(x)
```

**In Vensim:** `SQRT(x)` · **In Stella:** `SQRT(x)`

| Parameter | Meaning |
|---|---|
| `x` | A value of zero or more |

**Result:** The number that multiplied by itself gives x.

Negative inputs give an invalid result (shown as –).

**Example:** `SQRT(Area)`


See also: POWER, EXP

---

### EXP

e (≈2.718) raised to the power x.

```
EXP(x)
```

**In Vensim:** `EXP(x)` · **In Stella:** `EXP(x)`

| Parameter | Meaning |
|---|---|
| `x` | The exponent |

**Result:** eˣ.

Natural growth: a quantity growing at a constant fractional rate r follows Initial × EXP(r × Time).

**Example:** `Initial Value * EXP(Growth Rate * Time)`


See also: LN, POWER

---

### LN

Natural logarithm (the inverse of EXP).

```
LN(x)
```

**In Vensim:** `LN(x)` · **In Stella:** `LN(x)`

| Parameter | Meaning |
|---|---|
| `x` | A value greater than zero |

**Result:** The power to which e must be raised to give x.

**Example:** `LN(2) / Growth Rate`

- Tip: LN(2) / rate is the doubling time of exponential growth.

See also: EXP, LOG

---

### LOG

Logarithm of x in any base.

```
LOG(x, base)
```

**In Vensim:** `LOG(x, base)` · **In Stella:** `LOG10(x), or LN(x) / LN(base)`

| Parameter | Meaning |
|---|---|
| `x` | A value greater than zero |
| `base` | The base, e.g. 10 or 2 |

**Result:** The power to which base must be raised to give x.

**Example:** `LOG(Population, 10)`


See also: LN, POWER

---

### POWER

x raised to the power y.

```
POWER(x, y)
```

**In Vensim:** `POWER(x, y)` · **In Stella:** `x ^ y`

| Parameter | Meaning |
|---|---|
| `x` | The base |
| `y` | The exponent |

**Result:** xʸ — the same as writing x ^ y.

**Example:** `POWER(1 + Interest Rate, Years)`


See also: EXP, SQRT

---

### INTEGER

The whole-number part of x.

```
INTEGER(x)
```

**In Vensim:** `INTEGER(x): truncates toward zero` · **In Stella:** `INT(x): rounds down, so INT(-8.9) = -9` — Differs for negative numbers. Dynamist truncates, like Vensim.

| Parameter | Meaning |
|---|---|
| `x` | Any value |

**Result:** x with its fraction removed, towards zero (2.7 → 2, −2.7 → −2).

**Example:** `INTEGER(Time / 7)`

- Tip: Use ROUND for the nearest whole number instead.

See also: ROUND, MODULO

---

### ROUND

x rounded to the nearest whole number.

```
ROUND(x)
```

**In Vensim:** `not found in the pages read` · **In Stella:** `ROUND(x)`

| Parameter | Meaning |
|---|---|
| `x` | Any value |

**Result:** The nearest whole number; halves round away from zero.

**Example:** `ROUND(Staff Needed)`


See also: INTEGER

---

### MODULO

The remainder after dividing x by y.

```
MODULO(x, y)
```

**In Vensim:** `MODULO(x, y)` · **In Stella:** `x MOD y` — Neither says what a negative operand gives.

| Parameter | Meaning |
|---|---|
| `x` | The number divided |
| `y` | The divisor |

**Result:** A value from 0 up to (not including) y.

Handy for repeating patterns such as days of the week or seasons.

**Example:** `IF THEN ELSE(MODULO(Time, 7) >= 5, Weekend Demand, Weekday Demand)`


See also: INTEGER, PULSE TRAIN

---

### SIN

Sine of an angle in radians.

```
SIN(x)
```

**In Vensim:** `SIN(x), radians` · **In Stella:** `SIN(x), radians`

| Parameter | Meaning |
|---|---|
| `x` | Angle in radians (a full circle is 2π ≈ 6.283) |

**Result:** A value between −1 and 1.

Good for seasonal cycles: one full wave every Period time units.

**Example:** `Average Demand * (1 + 0.2 * SIN(2 * 3.14159 * Time / 12))`


See also: COS, TAN

---

### COS

Cosine of an angle in radians.

```
COS(x)
```

**In Vensim:** `COS(x)` · **In Stella:** `COS(x), radians`

| Parameter | Meaning |
|---|---|
| `x` | Angle in radians |

**Result:** A value between −1 and 1.

**Example:** `COS(2 * 3.14159 * Time / Period)`


See also: SIN, TAN

---

### TAN

Tangent of an angle in radians.

```
TAN(x)
```

**In Vensim:** `TAN(x), radians` · **In Stella:** `TAN(x), radians`

| Parameter | Meaning |
|---|---|
| `x` | Angle in radians |

**Result:** sin(x) / cos(x).

**Example:** `TAN(Slope Angle)`


See also: ARCTAN

---

### ARCTAN

The angle whose tangent is x.

```
ARCTAN(x)
```

**In Vensim:** `ARCTAN(x)` · **In Stella:** `ARCTAN(x), one argument`

| Parameter | Meaning |
|---|---|
| `x` | Any value |

**Result:** An angle in radians between −π/2 and π/2.

**Example:** `ARCTAN(Rise / Run)`


See also: TAN

---

### XIDZ

a divided by b, or x when b is zero.

```
XIDZ(a, b, x)
```

**In Vensim:** `XIDZ(a, b, x): the test is ABS(b) < 1E-6` · **In Stella:** `SAFEDIV(a, b, onzero)` — Vensim and Stella treat a tiny divisor as zero; Dynamist only an exact zero.

| Parameter | Meaning |
|---|---|
| `a` | The numerator |
| `b` | The divisor |
| `x` | What to use instead when b is zero |

**Result:** a / b, or x if b = 0.

Protects a ratio from dividing by zero, for instance when a stock empties.

**Example:** `XIDZ(Orders, Capacity, 1)`


See also: ZIDZ

---

### ZIDZ

a divided by b, or 0 when b is zero.

```
ZIDZ(a, b)
```

**In Vensim:** `ZIDZ(a, b): the test is ABS(b) < 1E-6` · **In Stella:** `SAFEDIV(a, b)` — As XIDZ.

| Parameter | Meaning |
|---|---|
| `a` | The numerator |
| `b` | The divisor |

**Result:** a / b, or 0 if b = 0.

**Example:** `ZIDZ(Infected, Total Population)`


See also: XIDZ

---

### SINH

Hyperbolic sine.

```
SINH(x)
```

**In Vensim:** `SINH(x)` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `SINH(Growth Rate * Time)`


See also: CEILING

---

### COSH

Hyperbolic cosine.

```
COSH(x)
```

**In Vensim:** `COSH(x)` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `COSH(x)`


See also: CEILING

---

### TANH

Hyperbolic tangent: an S-shaped curve from −1 to 1, handy as a smooth switch.

```
TANH(x)
```

**In Vensim:** `TANH(x)` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `TANH((Stock - Target) / Range)`


See also: CEILING

---

### ARCSIN

The angle (radians) whose sine is x; x must be between −1 and 1.

```
ARCSIN(x)
```

**In Vensim:** `ARCSIN(x)` · **In Stella:** `ARCSIN(x)`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `ARCSIN(Ratio)`


See also: CEILING

---

### ARCCOS

The angle (radians) whose cosine is x; x must be between −1 and 1.

```
ARCCOS(x)
```

**In Vensim:** `ARCCOS(x)` · **In Stella:** `ARCCOS(x)`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `ARCCOS(Ratio)`


See also: CEILING

---

### ARCTAN2

The angle (radians) of the point (x, y), in the right quadrant.

```
ARCTAN2(y, x)
```

| Parameter | Meaning |
|---|---|
| `y` | A value or expression |
| `x` | A value or expression |

**Result:** The result.

**Example:** `ARCTAN2(Rise, Run)`


See also: CEILING

---

### FLOOR

Rounds down to the whole number below.

```
FLOOR(x)
```

**In Vensim:** `not found` · **In Stella:** `not found (INT rounds down)`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `FLOOR(Days / 7)`


See also: CEILING

---

### SIGN

−1, 0 or 1 according to the sign of x.

```
SIGN(x)
```

**In Vensim:** `not found` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `SIGN(Gap)`


See also: CEILING

---

### LOG10

Logarithm to base 10.

```
LOG10(x)
```

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `LOG10(Population)`


See also: CEILING

---

### LOG2

Logarithm to base 2.

```
LOG2(x)
```

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `LOG2(Size)`


See also: CEILING

---

### CBRT

Cube root.

```
CBRT(x)
```

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `CBRT(Volume)`


See also: CEILING

---

### HYPOT

√(a² + b²): the length of the hypotenuse.

```
HYPOT(a, b)
```

| Parameter | Meaning |
|---|---|
| `a` | A value or expression |
| `b` | A value or expression |

**Result:** The result.

**Example:** `HYPOT(Dx, Dy)`


See also: CEILING

---

### RADIANS

Converts degrees to radians.

```
RADIANS(degrees)
```

| Parameter | Meaning |
|---|---|
| `degrees` | A value or expression |

**Result:** The result.

**Example:** `SIN(RADIANS(Angle))`


See also: CEILING

---

### DEGREES

Converts radians to degrees.

```
DEGREES(radians)
```

| Parameter | Meaning |
|---|---|
| `radians` | A value or expression |

**Result:** The result.

**Example:** `DEGREES(ARCTAN2(y, x))`


See also: CEILING

---

### CLAMP

Keeps x between a lower and an upper limit.

```
CLAMP(x, low, high)
```

**In Vensim:** `not found` · **In Stella:** `not found` — MIN(MAX(x, low), high) in both.

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |
| `low` | A value or expression |
| `high` | A value or expression |

**Result:** The result.

**Example:** `CLAMP(Demand, 0, Capacity)`


See also: CEILING

---

### MEAN

The average of two or more values.

```
MEAN(a, b, …)
```

| Parameter | Meaning |
|---|---|
| `a` | A value or expression |
| `b` | A value or expression |
| `…` | A value or expression |

**Result:** The result.

**Example:** `MEAN(Plan A, Plan B, Plan C)`


See also: CEILING

---

### MEDIAN

The middle value of two or more values.

```
MEDIAN(a, b, …)
```

**In Vensim:** `not found` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `a` | A value or expression |
| `b` | A value or expression |
| `…` | A value or expression |

**Result:** The result.

**Example:** `MEDIAN(Quote 1, Quote 2, Quote 3)`


See also: CEILING

---

### PI

The number π (3.14159…). Write it with brackets: PI().

```
PI()
```

**In Vensim:** `not found as a function` · **In Stella:** `PI (no brackets)`

**Result:** The result.

**Example:** `2 * PI() * Radius`


See also: CEILING

---

### ERF

The error function (accuracy about 1e-7); NormalCDF(x) = (1 + ERF(x / SQRT(2))) / 2.

```
ERF(x)
```

**In Vensim:** `not found` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `(1 + ERF(Z / SQRT(2))) / 2`


See also: CEILING

---

### GAMMA

The gamma function (a continuous factorial: GAMMA(n + 1) = n!).

```
GAMMA(x)
```

**In Vensim:** `GAMMA LN(x): only the logarithm` · **In Stella:** `GAMMALN(x). GAMMA(...) there is a random-number generator` — Write EXP(GAMMALN(x)) in Stella.

| Parameter | Meaning |
|---|---|
| `x` | A value or expression |

**Result:** The result.

**Example:** `GAMMA(Shape)`


See also: CEILING

---

### FACTORIAL

n! for a whole number n (up to 170).

```
FACTORIAL(n)
```

**In Vensim:** `not found` · **In Stella:** `FACTORIAL(n)`

| Parameter | Meaning |
|---|---|
| `n` | A value or expression |

**Result:** The result.

**Example:** `FACTORIAL(Choices)`


See also: CEILING

---

### CEILING

Rounds up to the next whole number.

```
CEILING(x)
```

**In Vensim:** `not found` · **In Stella:** `not found (INT rounds down)`

| Parameter | Meaning |
|---|---|
| `x` | Any value |

**Result:** The smallest whole number that is not below x.

Useful when things come in whole units: trucks, crews, batches.

**Example:** `CEILING(Required MB * 1000 / Barrels per truck)`

- Tip: CEILING(2.1) is 3 and CEILING(3) is 3.

See also: INTEGER, ROUND

---

### EXCELDATESERIAL

The Excel serial number of a date.

```
EXCELDATESERIAL(year, month, day)
```

**In Vensim:** `EXCELDATESERIAL(year, month, day)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `year` | The year |
| `month` | 1–12 |
| `day` | 1–31 |

**Result:** Days counted the way Excel does: 39448 is 1 January 2008.

Time spans between dates, or adding a span to a date.

**Example:** `EXCELDATESERIAL(2008, 1, 1)`


See also: EXCELDATESERIAL YEAR

---

### EXCELDATESERIAL YEAR

The year of an Excel serial date.

```
EXCELDATESERIAL YEAR(serial)
```

**In Vensim:** `EXCELDATESERIAL YEAR(serial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `serial` | An Excel date number |

**Result:** The year.

Taking a date apart.

**Example:** `EXCELDATESERIAL YEAR(39448)`


See also: EXCELDATESERIAL

---

### EXCELDATESERIAL MONTH OF YEAR

The month of an Excel serial date.

```
EXCELDATESERIAL MONTH OF YEAR(serial)
```

**In Vensim:** `EXCELDATESERIAL MONTH OF YEAR(serial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `serial` | An Excel date number |

**Result:** 1 to 12.

Taking a date apart.

**Example:** `EXCELDATESERIAL MONTH OF YEAR(39448)`


See also: EXCELDATESERIAL

---

### EXCELDATESERIAL DAY OF MONTH

The day of the month of an Excel serial date.

```
EXCELDATESERIAL DAY OF MONTH(serial)
```

**In Vensim:** `EXCELDATESERIAL DAY OF MONTH(serial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `serial` | An Excel date number |

**Result:** 1 to 31.

Taking a date apart.

**Example:** `EXCELDATESERIAL DAY OF MONTH(39448)`


See also: EXCELDATESERIAL

---

### EXCELDATESERIAL DAY OF YEAR

The day of the year of an Excel serial date.

```
EXCELDATESERIAL DAY OF YEAR(serial)
```

**In Vensim:** `EXCELDATESERIAL DAY OF YEAR(serial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `serial` | An Excel date number |

**Result:** 1 to 366.

Taking a date apart.

**Example:** `EXCELDATESERIAL DAY OF YEAR(39448)`


See also: EXCELDATESERIAL

---

### GAMMA LN

The natural logarithm of the gamma function.

```
GAMMA LN(x)
```

**In Vensim:** `GAMMA LN(x)` · **In Stella:** `GAMMALN(x)`

| Parameter | Meaning |
|---|---|
| `x` | A positive number |

**Result:** ln Γ(x); Γ(n) = (n − 1)! for whole numbers.

Factorials and binomial coefficients of large numbers without overflow.

**Example:** `EXP(GAMMA LN(n + 1))`


See also: EXP

---

### QUANTUM

x cut down toward zero to a whole multiple of q.

```
QUANTUM(x, q)
```

**In Vensim:** `QUANTUM(x, q)` · **In Stella:** `no direct equivalent: q * INT(x / q)`

| Parameter | Meaning |
|---|---|
| `x` | The value |
| `q` | The step it is cut to; 0 or less leaves x unchanged |

**Result:** q × INTEGER(x / q), or x when q is 0 or less.

Orders placed in whole batches or containers.

**Example:** `QUANTUM(Desired Order, 12)`


See also: INTEGER

---

## Sets

### TOTAL

Adds up a variable over all the items of its set.

```
TOTAL(name) or TOTAL(name, group)
```

**In Vensim:** `SUM(x[range!])` · **In Stella:** `SUM(x[*])`

| Parameter | Meaning |
|---|---|
| `name` | A variable that is per a set (for example per Tank) |
| `group` | Optional: only the items that belong to this group, e.g. a region name |

**Result:** One number: the sum over the items.

In a formula that is itself per Region, TOTAL(Stock) adds up only the tanks of that region. Anywhere else it adds up every item.

**Example:** `TOTAL(Stock, COA)`

- Tip: TOTAL(Stock) in a per-Region variable gives each region its own tanks.
- Tip: Stock[Hail 2 - Crude] picks one item.

See also: AVERAGE

---

### AVERAGE

The average of a variable over the items of its set.

```
AVERAGE(name) or AVERAGE(name, group)
```

**In Vensim:** `SUM(x[range!]) / ELMCOUNT(range)` · **In Stella:** `MEAN(x[*])`

| Parameter | Meaning |
|---|---|
| `name` | A variable that is per a set |
| `group` | Optional: only the items that belong to this group |

**Result:** One number: the mean over the items.

Works like TOTAL, but divides by the number of items.

**Example:** `AVERAGE(Stock Percent, WOA)`


See also: TOTAL

---

### ALLOCATE AVAILABLE ITEM

One requester's share of what is available, when supply is short.

```
ALLOCATE AVAILABLE ITEM(k, n, request…, profile…, available)
```

**In Vensim:** `ALLOCATE AVAILABLE(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** One requester's share of what is available, when supply is short.

Vensim's ALLOCATE AVAILABLE: each requester has a priority curve (rectangular, triangular, normal or exponential); the priority level is found where the curves add up to what is available. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `ALLOCATE AVAILABLE(Demand[region], Profile[region, ptype], Supply)`


See also: TOTAL

---

### ALLOCATE BY PRIORITY ITEM

One requester's share when the highest priority is served first.

```
ALLOCATE BY PRIORITY ITEM(k, n, request…, priority…, width, supply)
```

**In Vensim:** `ALLOCATE BY PRIORITY(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** One requester's share when the highest priority is served first.

Vensim's ALLOCATE BY PRIORITY: requesters whose priorities are within the width of each other share in proportion to their requests. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `ALLOCATE BY PRIORITY(Demand[region], Priority[region], ELMCOUNT(region), 1, Supply)`


See also: TOTAL

---

### VECTOR SORT ORDER ITEM

The index (from 0) of the element at place k once the values are sorted.

```
VECTOR SORT ORDER ITEM(k, direction, x…)
```

**In Vensim:** `VECTOR SORT ORDER(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** The index (from 0) of the element at place k once the values are sorted.

Vensim's VECTOR SORT ORDER; direction 1 sorts up, 0 or less down. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `VECTOR SORT ORDER(Sales[store], 1)`


See also: TOTAL

---

### VECTOR RANK ITEM

The rank (from 1) of element k.

```
VECTOR RANK ITEM(k, direction, x…)
```

**In Vensim:** `VECTOR RANK(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** The rank (from 1) of element k.

Vensim's VECTOR RANK. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `VECTOR RANK(Sales[store], 1)`


See also: TOTAL

---

### VECTOR REORDER ITEM

x at the index the order gives for place k.

```
VECTOR REORDER ITEM(k, n, x…, order…)
```

**In Vensim:** `VECTOR REORDER(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** x at the index the order gives for place k.

Vensim's VECTOR REORDER, usually with an order from VECTOR SORT ORDER. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `VECTOR REORDER(Sales[store], Order[store])`


See also: TOTAL

---

### VECTOR ELM MAP ITEM

The element offset places after start.

```
VECTOR ELM MAP ITEM(start, offset, x…)
```

**In Vensim:** `VECTOR ELM MAP(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** The element offset places after start.

Vensim's VECTOR ELM MAP. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `VECTOR ELM MAP(Price[first], Shift)`


See also: TOTAL

---

### VECTOR SELECT ITEMS

Sums, multiplies, or takes the minimum, maximum or mean of the selected values.

```
VECTOR SELECT ITEMS(n, selection…, value…, missing, action)
```

**In Vensim:** `VECTOR SELECT(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** Sums, multiplies, or takes the minimum, maximum or mean of the selected values.

Vensim's VECTOR SELECT; action 0 sum, 1 product, 2 min, 3 max, 4 mean (5–10 as Vensim). Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `VECTOR SELECT(Chosen[store!], Sales[store!], 0, 0, 0)`


See also: TOTAL

---

### INVERT MATRIX ITEM

One element of the inverse of a square matrix.

```
INVERT MATRIX ITEM(i, j, n, m…)
```

**In Vensim:** `INVERT MATRIX(…)` · **In Stella:** `no direct equivalent` — written out element by element when a Vensim model is read

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads a Vensim model: every element of the arrays, one argument each |

**Result:** One element of the inverse of a square matrix.

Vensim's INVERT MATRIX. Use the Vensim spelling in a Vensim file; Dynamist writes this form when it reads one.

**Example:** `INVERT MATRIX(A[row, col], 3)`


See also: TOTAL

---

### MAX OF

The largest value of a variable that is per a set.

```
MAX OF(name, group)
```

| Parameter | Meaning |
|---|---|
| `name` | A variable per a set |
| `group` | Optional: only the items of this group |

**Result:** One number.

Vensim's VMAX.

**Example:** `MAX OF(Level)`


See also: TOTAL

---

### MIN OF

The smallest value of a variable that is per a set.

```
MIN OF(name, group)
```

| Parameter | Meaning |
|---|---|
| `name` | A variable per a set |
| `group` | Optional: only the items of this group |

**Result:** One number.

Vensim's VMIN.

**Example:** `MIN OF(Level)`


See also: TOTAL

---

### COUNT OF

The number of items of value of a variable that is per a set.

```
COUNT OF(name, group)
```

| Parameter | Meaning |
|---|---|
| `name` | A variable per a set |
| `group` | Optional: only the items of this group |

**Result:** One number.

Vensim's ELMCOUNT and SIZE.

**Example:** `COUNT OF(Level)`


See also: TOTAL

---

### VECTOR LOOKUP ITEMS

A lookup whose y values are the elements of an array.

```
VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)
```

**In Vensim:** `VECTOR LOOKUP ITEMS(n, y…, x, xmin, xmax, mode)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `…` | Written out by Dynamist when it reads Vensim's VECTOR LOOKUP: every element, then x, xmin, xmax and the mode |

**Result:** The lookup read in the mode asked for (0 normal, 1 area, 2 backward, 3 extrapolate, 4 forward, 5 invert, 6 slope, 7–9 with extrapolation).

A curve given as an array of values over equal steps.

**Example:** `VECTOR LOOKUP(Curve[first], X, 0, 10, 0)`


See also: LOOKUP SLOPE

---

### AGE

Moves people up through age classes. Gives one class its net change from ageing: the class before it moves in, and its own people move on.

```
AGE(stock, every=, entrants=, open_ended=True|False)
```

**In Vensim:** `AGE(stock, every=, entrants=, open_ended=)` · **In Stella:** `SHIFT IF TRUE in Vensim (changes the stock itself); no direct equivalent in Stella (a conveyor or a chain)` — Dynamist writes the class's ageing flow as a formula when a model is exported.

| Parameter | Meaning |
|---|---|
| `stock` | A variable with one value per age class, in order from the youngest |
| `every=` | How often the classes move up, in time units (default 1). The whole class moves at once, at the end of each period |
| `entrants=` | People joining the youngest class each time unit; the youngest class is refilled with a period's worth at each move (default none) |
| `open_ended=` | True (default): the oldest class keeps its people, like '65 and over'. False: they leave the model when the classes move up |

**Result:** The change in this class from ageing: people arriving from the class before, minus its own people moving on.

Write it in a formula that is per the age set, add deaths and anything else, and use the sum as the stock's change: Change[age] = AGE(Population, every=5, entrants=Births) - Deaths[age]. Classes move together, so the stock rises and falls in steps of one period; read it at the moves (every 5 years here). Replaces SHIFT IF TRUE and hand-built ageing chains. Use Euler integration.

**Example:** `AGE(Population, every=5, entrants=Births per year)`


See also: QUEUE, DELAY

---

### ALLOCATE

Shares a limited supply among the items of a set when it is short. Each item gets its own share.

```
ALLOCATE(requests, supply, by=, overlap=, curve=rectangular|triangular|normal|exponential, spread=)
```

**In Vensim:** `ALLOCATE(requests, supply, by=, overlap=, curve=, spread=)` · **In Stella:** `ALLOCATE BY PRIORITY (strict order) and ALLOCATE AVAILABLE (curves) in Vensim; no direct equivalent in Stella` — Dynamist writes the Vensim function when a model is exported.

| Parameter | Meaning |
|---|---|
| `requests` | What each item asks for (a variable with one value per item) |
| `supply` | What there is to share (one number) |
| `by=` | Priorities (a variable per the same set). The highest is served first. Without it, everyone is served in proportion to what they ask |
| `overlap=` | Items whose priorities are this close share between them (default: none, strict order) |
| `curve=` | Instead of a strict order, a smooth fall-off around each priority: rectangular, triangular, normal or exponential. Needs by= |
| `spread=` | How wide the curve is (default 1) |

**Result:** This item's share. When the supply covers all requests, everyone gets what they asked.

Use it in a formula that is per the same set as the requests. Replaces Vensim's ALLOCATE AVAILABLE and ALLOCATE BY PRIORITY: ALLOCATE(Demand, Water) shares in proportion; ALLOCATE(Demand, Water, by=Priority) serves the highest priority first; ALLOCATE(Demand, Water, by=Priority, curve=normal, spread=2) lets the lower priorities get a little before the higher ones are full.

**Example:** `Delivered[Region] = ALLOCATE(Demand, Supply, by=Priority)`


See also: TOTAL, DELAY

---

## Test inputs

### SINWAVE

A repeating sine wave.

```
SINWAVE(amplitude, period, start)
```

**In Vensim:** `not found` · **In Stella:** `SINWAVE(amplitude, period): no start; amplitude taken as absolute` — The phase differs when start is not 0.

| Parameter | Meaning |
|---|---|
| `amplitude` | Height of the wave |
| `period` | Time for one full cycle |
| `start` | Optional: where the wave is at zero and rising (default: the start of the run) |

**Result:** amplitude × sin(2π (TIME − start) / period).

Seasonal demand, daily cycles.

**Example:** `Base * (1 + SINWAVE(0.2, 365))`


See also: COSWAVE, SIN

---

### COSWAVE

A repeating cosine wave.

```
COSWAVE(amplitude, period, start)
```

**In Vensim:** `not found` · **In Stella:** `COSWAVE(amplitude, period): no start` — As SINWAVE.

| Parameter | Meaning |
|---|---|
| `amplitude` | Height of the wave |
| `period` | Time for one full cycle |
| `start` | Optional: where the wave peaks (default: the start of the run) |

**Result:** amplitude × cos(2π (TIME − start) / period).

Peaks at the start time unless you give another.

**Example:** `Base * (1 + COSWAVE(0.2, 12, 6))`


See also: SINWAVE, COS

---

### STEP

Jumps from 0 to a new level at a given time and stays there.

```
STEP(height, start)
```

**In Vensim:** `STEP(height, time)` · **In Stella:** `STEP(height, time), optional duration and interval in newer versions`

| Parameter | Meaning |
|---|---|
| `height` | The size of the jump |
| `start` | When the jump happens |

**Result:** 0 before start, height from start onwards.

The classic test input: see how the system responds to a sudden, permanent change.

**Example:** `Base Demand + STEP(20, 10)`

- Tip: Add it to a constant to model a policy change from a certain date.

See also: PULSE, RAMP

---

### PULSE

A one-off burst: 1 for a while, otherwise 0.

```
PULSE(start, width)
```

**In Vensim:** `PULSE(start, width); a width of 0 counts as one TIME STEP` · **In Stella:** `PULSE(volume, first, interval): a volume spread over one DT` — Different meaning in Stella: its first argument is a volume, not a start time.

| Parameter | Meaning |
|---|---|
| `start` | When the pulse begins |
| `width` | How long it lasts (at least one time step) |

**Result:** 1 from start until start + width, otherwise 0.

Multiply by an amount to add a one-time shock, such as a single large order.

**Example:** `Normal Orders + 500 * PULSE(12, 1)`


See also: PULSE TRAIN, STEP

---

### PULSE TRAIN

Repeating pulses at regular intervals.

```
PULSE TRAIN(start, width, interval, end)
```

**In Vensim:** `PULSE TRAIN(start, width, interval, end)` · **In Stella:** `not found`

| Parameter | Meaning |
|---|---|
| `start` | When the first pulse begins |
| `width` | How long each pulse lasts |
| `interval` | Time between the starts of pulses |
| `end` | No pulses after this time |

**Result:** 1 during each pulse, otherwise 0.

**Example:** `Shipment Size * PULSE TRAIN(0, 1, 4, 100)`


See also: PULSE, MODULO

---

### RAMP

Rises in a straight line between two times.

```
RAMP(slope, start, end)
```

**In Vensim:** `RAMP(slope, start, end): constant after the end` · **In Stella:** `RAMP(slope, [time]) in newer versions; older ones also take a stop time` — Check what happens after the end in the version you use.

| Parameter | Meaning |
|---|---|
| `slope` | Increase per time unit |
| `start` | When it starts rising |
| `end` | When it stops rising and levels off |

**Result:** 0 before start, slope × (time − start) while rising, then flat.

**Example:** `Base Price + RAMP(0.5, 10, 30)`


See also: STEP

---

### RANDOM UNIFORM

A new random number every time step, evenly spread between min and max. The seed picks the stream: the same seed repeats, another seed gives other numbers.

```
RANDOM UNIFORM(min, max, seed)
```

**In Vensim:** `RANDOM UNIFORM(min, max, stream): third argument is a stream number, usually 0` · **In Stella:** `UNIFORM(min, max, [seed])` — Numbers differ between tools; runs are not reproducible across them.

| Parameter | Meaning |
|---|---|
| `min` | Lowest possible value |
| `max` | Highest possible value |
| `seed` | Kept for compatibility; every run uses the same sequence so results are repeatable |

**Result:** A random value in [min, max).

Use it to add noise to demand or test how robust a policy is.

**Example:** `Average Demand * RANDOM UNIFORM(0.9, 1.1, 1)`

- Tip: Smoothing noisy inputs with SMOOTH makes them more realistic.

See also: RANDOM NORMAL, SMOOTH

---

### RANDOM NORMAL

Bell-curve random numbers, kept between min and max. The seed picks the stream: the same seed repeats, another seed gives other numbers.

```
RANDOM NORMAL(min, max, mean, sd, seed)
```

**In Vensim:** `RANDOM NORMAL(min, max, mean, sd, stream): values outside the range are redrawn` · **In Stella:** `NORMAL(mean, stddev, [seed], [min], [max])` — Different argument order in Stella; Dynamist clips where Vensim redraws.

| Parameter | Meaning |
|---|---|
| `min` | Lowest value allowed |
| `max` | Highest value allowed |
| `mean` | Average value |
| `sd` | Standard deviation (spread) |
| `seed` | Kept for compatibility; runs are repeatable |

**Result:** A normally distributed random value, clipped to [min, max].

**Example:** `RANDOM NORMAL(0, 200, 100, 15, 1)`


See also: RANDOM UNIFORM

---

### RANDOM EXPONENTIAL

A new exponentially distributed random number every time step.

```
RANDOM EXPONENTIAL(min, max, shift, stretch, seed)
```

**In Vensim:** `RANDOM EXPONENTIAL(min, max, shift, stretch, seed)` · **In Stella:** `EXPRND(mean, [seed])` — Different streams; same distribution.

| Parameter | Meaning |
|---|---|
| `min` | Lowest value kept |
| `max` | Highest value kept |
| `shift` | Added to the draw |
| `stretch` | Multiplies the draw (the mean of the unshifted draw) |
| `seed` | Picks the stream |

**Result:** shift + stretch × an exponential draw with mean 1, kept within min and max.

Times between arrivals. The numbers differ from Vensim's; the distribution is the same.

**Example:** `RANDOM EXPONENTIAL(0, 100, 0, 5, 1)`


See also: RANDOM UNIFORM

---

## Delays

### DELAYINF

Information delay: the input, smoothed by a chain of stages.

```
DELAYINF(input, delay, order, [initial])
```

**In Vensim:** `no equivalent: nearest are DELAY INFORMATION(x, t, i) and DELAY N(x, t, i, n)` · **In Stella:** `DELAYN(x, t, n, [initial]) is the nearest` — Order and meaning need checking.

| Parameter | Meaning |
|---|---|
| `input` | What is delayed |
| `delay` | Average delay time |
| `order` | How many stages (a whole number, 1–20; default 1). More stages = a sharper delay |
| `initial` | Optional: the value every stage starts at. Without it the output starts at the input |

**Result:** The output of the last stage; each stage has delay ÷ order.

For perceptions and decisions: no material is conserved. DELAYINF(x, d, 3) equals SMOOTH3(x, d); with an initial value it is Vensim's SMOOTH N(x, d, initial, n).

**Example:** `DELAYINF(Orders, 5, 3)`


See also: DELAYMTR, SMOOTH

---

### DELAYMTR

Material delay: what goes in comes out later, nothing is lost.

```
DELAYMTR(input, delay, order, [initial])
```

**In Vensim:** `no equivalent: nearest is DELAY MATERIAL(x, t, i, m)` · **In Stella:** `DELAYN(x, t, n, [initial]) is the nearest` — As DELAYINF.

| Parameter | Meaning |
|---|---|
| `input` | The inflow |
| `delay` | Average delay time |
| `order` | How many stages (a whole number, 1–20; default 1) |
| `initial` | Optional: the outflow at the start; every stage starts full enough to give it. Without it the outflow starts at the input |

**Result:** The outflow of the last stage; each stage has delay ÷ order.

For things in transit: shipments, construction. DELAYMTR(x, d, 3) equals DELAY3(x, d); with an initial value it is Vensim's DELAY N(x, d, initial, n).

**Example:** `DELAYMTR(Shipments, 6, 3)`


See also: DELAYINF, DELAY3

---

### SMOOTH

An average that catches up with its input gradually (first-order exponential smoothing).

```
SMOOTH(input, delay time)
```

**In Vensim:** `SMOOTH(x, t)` · **In Stella:** `SMTH1(x, t, [initial])`

| Parameter | Meaning |
|---|---|
| `input` | The value being averaged |
| `delay time` | How long it takes to adjust, on average |

**Result:** The smoothed value; it starts equal to the input.

Models perceptions and expectations: people notice change only gradually. After one delay time it has closed about 63% of a gap.

**Example:** `SMOOTH(Actual Sales, 3)`

- Tip: Use SMOOTHI to start from a different value.
- Tip: SMOOTH3 responds with an S-shape rather than an immediate start.

See also: SMOOTHI, SMOOTH3, DELAY1

---

### SMOOTHI

SMOOTH with a chosen starting value.

```
SMOOTHI(input, delay time, initial)
```

**In Vensim:** `SMOOTHI(x, t, initial)` · **In Stella:** `SMTH1 with the third argument`

| Parameter | Meaning |
|---|---|
| `input` | The value being averaged |
| `delay time` | Adjustment time |
| `initial` | Value at the start of the run |

**Result:** The smoothed value.

**Example:** `SMOOTHI(Actual Sales, 3, 100)`


See also: SMOOTH

---

### SMOOTH3

Third-order smoothing: a slower start, then a quicker catch-up.

```
SMOOTH3(input, delay time)
```

**In Vensim:** `SMOOTH3(x, t)` · **In Stella:** `SMTH3(x, t, [initial])`

| Parameter | Meaning |
|---|---|
| `input` | The value being averaged |
| `delay time` | Total adjustment time |

**Result:** The smoothed value; it starts equal to the input.

Built from three SMOOTHs in a row, each a third of the delay time.

**Example:** `SMOOTH3(Price, 6)`


See also: SMOOTH, SMOOTH3I

---

### SMOOTH3I

SMOOTH3 with a chosen starting value.

```
SMOOTH3I(input, delay time, initial)
```

**In Vensim:** `SMOOTH3I(x, t, initial)` · **In Stella:** `SMTH3 with the third argument`

| Parameter | Meaning |
|---|---|
| `input` | The value being averaged |
| `delay time` | Total adjustment time |
| `initial` | Value at the start |

**Result:** The smoothed value.

**Example:** `SMOOTH3I(Price, 6, 10)`


See also: SMOOTH3

---

### DELAY1

A first-order material delay: what goes in comes out later, spread out over time.

```
DELAY1(input, delay time)
```

**In Vensim:** `DELAY1(x, t)` · **In Stella:** `DELAY1(x, t, [initial])`

| Parameter | Meaning |
|---|---|
| `input` | The flow going into the delay |
| `delay time` | Average time spent inside |

**Result:** The outflow of the delay; it starts equal to the input.

Unlike SMOOTH, nothing is lost: everything that enters eventually leaves. Typical for shipping, construction or ageing.

**Example:** `DELAY1(Orders Placed, Lead Time)`

- Tip: To see how much is inside the delay, model it with a stock and flows instead.

See also: DELAY3, DELAY FIXED, SMOOTH

---

### DELAY1I

DELAY1 with a chosen starting outflow.

```
DELAY1I(input, delay time, initial)
```

**In Vensim:** `DELAY1I (signature not read)` · **In Stella:** `DELAY1 with the third argument`

| Parameter | Meaning |
|---|---|
| `input` | The inflow |
| `delay time` | Average time inside |
| `initial` | Outflow at the start |

**Result:** The outflow.

**Example:** `DELAY1I(Orders Placed, Lead Time, 50)`


See also: DELAY1

---

### DELAY3

A third-order material delay: outflow rises in an S-shape after the input changes.

```
DELAY3(input, delay time)
```

**In Vensim:** `DELAY3(x, t)` · **In Stella:** `DELAY3(x, t, [initial])`

| Parameter | Meaning |
|---|---|
| `input` | The inflow |
| `delay time` | Average time inside |

**Result:** The outflow; it starts equal to the input.

More realistic than DELAY1 when most items take roughly the delay time, like a pipeline with some spread.

**Example:** `DELAY3(Construction Starts, 18)`


See also: DELAY1, DELAY FIXED

---

### DELAY3I

DELAY3 with a chosen starting outflow.

```
DELAY3I(input, delay time, initial)
```

**In Vensim:** `DELAY3I(x, t, initial)` · **In Stella:** `DELAY3 with the third argument`

| Parameter | Meaning |
|---|---|
| `input` | The inflow |
| `delay time` | Average time inside |
| `initial` | Outflow at the start |

**Result:** The outflow.

**Example:** `DELAY3I(Construction Starts, 18, 10)`


See also: DELAY3

---

### DELAY FIXED

An exact pipeline: the output is the input from exactly delay time ago.

```
DELAY FIXED(input, delay time, initial)
```

**In Vensim:** `DELAY FIXED(x, t, initial)` · **In Stella:** `DELAY(x, t, [initial])` — Only the initial value of the delay time is used.

| Parameter | Meaning |
|---|---|
| `input` | The value fed in |
| `delay time` | How far back to look |
| `initial` | Output before anything has made it through |

**Result:** The input from delay time ago, or initial early in the run.

Use for processes with a fixed duration, such as a 9-month gestation or a fixed-term contract.

**Example:** `DELAY FIXED(Hires, 3, 0)`


See also: DELAY3, DELAY1

---

### TREND

The fractional rate at which a value has been changing.

```
TREND(input, average time, initial trend)
```

**In Vensim:** `TREND(x, t, initial trend): no default` · **In Stella:** `TREND(x, t, [initial]): default 0`

| Parameter | Meaning |
|---|---|
| `input` | The value to watch |
| `average time` | Over how long the trend is judged |
| `initial trend` | Trend assumed at the start (fraction per time unit) |

**Result:** Fractional change per time unit, e.g. 0.05 = growing 5% per period.

Models how people extrapolate growth from recent experience.

**Example:** `TREND(Sales, 6, 0)`


See also: SMOOTH

---

### NPVE

Net present value with a discrete period rate, valued at the end of the period.

```
NPVE(stream, discount rate, initial value, factor)
```

**In Vensim:** `NPVE(stream, discount rate, initial value, factor)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `stream` | Money per time unit |
| `discount rate` | Per period |
| `initial value` | The value at the start |
| `factor` | Multiplies the result |

**Result:** The discounted sum, as Excel computes it.

Comparing with a spreadsheet. For time steps below 1, NPV is the better choice.

**Example:** `NPVE(Cash Flow, 0.01, 0, 1)`


See also: NPV

---

### DERIVN

The nth time derivative of x, from its past values.

```
DERIVN(x, order)
```

**In Vensim:** `DERIVN(x, order)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `x` | The value to differentiate |
| `order` | 1 for the rate of change, 2 for its rate of change, … |

**Result:** The nth backward difference divided by the time step to the nth; 0 until there is enough history.

Speed and acceleration of a variable.

**Example:** `DERIVN(Position, 1)`


See also: TREND

---

### NPV AT

Present value with period timing: the engine form of VALUE(timing=period_start or mid_period).

```
NPV AT(stream, rate, init, factor, offset)
```

**In Vensim:** `no direct equivalent: NPV with an offset in periods` · **In Stella:** `no direct equivalent`

**Result:** A number.

A rate with a unit per Year in a monthly model is read per Year: its period is the unit of the rate. In Excel: no single function; see PMT, NPV and RATE. In Vensim: VALUE, NPV. In Stella: no equivalent.

**Example:** `NPV AT(Cash, 0.1, 0, 1, 0)`


---

### EXPECT

What people believe about x: how they perceive it, the trend they see, or where they expect it to be.

```
EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)
```

**In Vensim:** `EXPECT(x, time, ahead=, init=, measure=level|trend|forecast)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `x` | The real thing |
| `time` | How long people take to notice a change |
| `ahead=` | How far ahead they expect. Gives a forecast: the level now plus the trend they see times this horizon |
| `init=` | What they believe at the start (default: the true value) |
| `measure=` | level (default): the belief about now. trend: the growth they think they see, per time unit. forecast: where they expect it to be (the default when ahead= is given) |

**Result:** A belief: the true value, noticed late, extrapolated if you ask.

Replaces SMOOTH, SMOOTHI, TREND and FORECAST. People rarely act on facts; they act on what they believe the facts are. EXPECT(Demand, 6) is demand as noticed over about six weeks; EXPECT(Demand, 6, ahead=8) is where they expect it to be in eight weeks. A trend extrapolation overshoots at turning points.

**Example:** `EXPECT(Demand, 6, ahead=8)`


See also: DELAY, TREND

---

### INTERNAL RATE OF RETURN

The rate of return that makes a stream of money worth nothing at the start.

```
INTERNAL RATE OF RETURN(stream, period, initial)
```

**In Vensim:** `INTERNAL RATE OF RETURN(stream, period, initial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `stream` | Money per time unit, spending negative, income positive |
| `period` | The length of the period the rate is per (default 1 time unit) |
| `initial` | Money at the start (default 0) |

**Result:** The rate per period, from the money so far; not available until the stream has both spending and income.

Vensim's name for VALUE(stream, measure=irr). The rate that turns the net present value of the stream to zero.

**Example:** `INTERNAL RATE OF RETURN(Cash Flow, 12, 0)`


See also: VALUE

---

### QUEUE

A waiting line: a level that remembers how long each has waited. The oldest leaves first.

```
QUEUE(arrivals, served, init=)
```

**In Vensim:** `QUEUE(arrivals, served, init=)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `arrivals` | How many join each time unit |
| `served` | How many leave each time unit; never more than are waiting |
| `init=` | How many are waiting at the start (default 0); they count as one time step old |

**Result:** The number waiting.

Use it when who waits longest matters: patients, orders, support tickets. Read how long they have waited with WAIT. Replaces QUEUE FIFO.

**Example:** `Waiting = QUEUE(Arrivals, Treated)`


See also: WAIT

---

### WAIT

How long the people in a queue have waited.

```
WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)
```

**In Vensim:** `WAIT(queue, measure=average|oldest|count, among=, older_than=, younger_than=)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `queue` | The name of a variable defined with QUEUE |
| `measure=` | average (default): the mean wait of those waiting. oldest: the longest wait. count: how many have waited within a range |
| `among=` | With average: only the n who have waited longest |
| `older_than=` | With count: waited at least this long |
| `younger_than=` | With count: waited no longer than this |

**Result:** A time (average, oldest) or a number of people (count).

Replaces QUEUE AGE AVERAGE, QUEUE AGE OLDEST and QUEUE AGE IN RANGE. WAIT(Waiting, measure=count, older_than=2) is how many have waited more than 2 hours.

**Example:** `WAIT(Waiting, measure=oldest)`


See also: QUEUE

---

### QUEUE FIFO

The queue function Vensim writes. Dynamist writes QUEUE(...) in its place.

```
QUEUE FIFO(inflow, outflow, initial)
```

**In Vensim:** `QUEUE FIFO(inflow, outflow, initial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `inflow` | How many join |
| `outflow` | How many leave |
| `initial` | How many are waiting at the start |

**Result:** The number waiting.

In Vensim the call also gives a profile that spreads the starting crowd over ages; that part is not read.

**Example:** `QUEUE FIFO(Arriving, Servicing, 0)`


See also: QUEUE

---

### QUEUE AGE AVERAGE

The average time those waiting have waited.

```
QUEUE AGE AVERAGE(queue, [n])
```

**In Vensim:** `QUEUE AGE AVERAGE(queue, [n])` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `queue` | A variable defined with QUEUE FIFO |
| `n` | Only the n who have waited longest (default everyone) |

**Result:** A time.

Vensim's name for WAIT(queue).

**Example:** `QUEUE AGE AVERAGE(Waiting)`


See also: WAIT

---

### QUEUE AGE OLDEST

How long the one who has waited longest has waited.

```
QUEUE AGE OLDEST(queue)
```

**In Vensim:** `QUEUE AGE OLDEST(queue)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `queue` | A variable defined with QUEUE FIFO |

**Result:** A time.

Vensim's name for WAIT(queue, measure=oldest).

**Example:** `QUEUE AGE OLDEST(Waiting)`


See also: WAIT

---

### QUEUE AGE IN RANGE

How many waiting have waited between two times.

```
QUEUE AGE IN RANGE(queue, min age, max age)
```

**In Vensim:** `QUEUE AGE IN RANGE(queue, min age, max age)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `queue` | A variable defined with QUEUE FIFO |
| `min age` | At least this long |
| `max age` | No longer than this |

**Result:** A number of people.

Vensim's name for WAIT(queue, measure=count, …).

**Example:** `QUEUE AGE IN RANGE(Waiting, 2, 1000)`


See also: WAIT

---

### VALUE

The present value of a stream of money: what it is worth at the start of the run, discounted at a rate, accumulated up to now.

```
VALUE(stream, rate, init=, factor=, timing=continuous|period_end) · VALUE(stream, measure=irr) · VALUE(stream, writeoff=time)
```

**In Vensim:** `VALUE(stream, rate, init=, factor=, timing=)` · **In Stella:** `NPV(stream, rate, init, factor) and NPVE for period_end` — Dynamist writes the matching function when a model is exported.

| Parameter | Meaning |
|---|---|
| `stream` | Money per time unit: income positive, spending negative |
| `rate` | The discount rate per time unit, for example 0.01 per month |
| `init=` | A value to start from (default 0) |
| `factor=` | Multiplies the result (default 1) |
| `timing=` | continuous (default, small steps) or period_end (the way a spreadsheet's NPV counts, with each period's money at its end) |

**Result:** The discounted sum so far.

Use it to compare investments: put the spending in the stream as a negative and the income as positive, and read VALUE at the end of the run. VALUE(stream, measure=irr) gives the rate of return that makes the stream worth nothing; VALUE(stream, writeoff=time) writes each amount off evenly over that time. Replaces NPV, NPVE, INTERNAL RATE OF RETURN and DEPRECIATE STRAIGHTLINE.

**Example:** `VALUE(Revenue - Costs, 0.01)`


See also: DELAY

---

### DELAY

Delays x by a time. One function for every kind of delay: say what you mean with an option.

```
DELAY(x, time, [order], init=, kind=material|information, exact=True|False, leak=)
```

**In Vensim:** `DELAY(x, time, [order], init=, kind=, exact=)` · **In Stella:** `DELAY1 / DELAY3 / DELAYN / DELAY FIXED (material), SMOOTH / SMOOTH3 / SMOOTH N (information)` — Dynamist writes the matching function when a model is exported.

| Parameter | Meaning |
|---|---|
| `x` | What is delayed |
| `time` | The average delay time (the transit time when exact=True) |
| `order` | How many stages, 1 to 20 (default 1). More stages = a sharper, more bunched delay |
| `init=` | What comes out at the start (default: the input at the start, a steady state) |
| `kind=` | material (default): what goes in comes out later, nothing is lost. information: a smoothed perception; nothing is conserved |
| `exact=` | True: a pipeline. Everything arrives exactly `time` after it entered (default False: smooth) |
| `leak=` | With exact=True: a conveyor that loses this fraction of what is on it each time step, so less arrives than entered |

**Result:** The delayed value: smooth by default, or exactly `time` late with exact=True.

Replaces DELAY1, DELAY3, DELAYMTR, DELAYINF, SMOOTH, SMOOTH3 and DELAY FIXED. In words: DELAY(x, 6) is material in transit for about 6 time units, spread out. DELAY(x, 6, order=3) is the same, more bunched around 6. DELAY(x, 6, exact=True) is a pipeline of exactly 6. DELAY(x, 6, kind=information) is how a belief catches up with a fact.

**Example:** `DELAY(Shipments, 6, order=3)`


See also: SMOOTH, DELAY FIXED

---

### NPV

Net present value of a stream of money, accumulated over the run.

```
NPV(stream, discount rate, initial value, factor)
```

**In Vensim:** `NPV(stream, discount rate, initial value, factor)` · **In Stella:** `NPV(stream, discount rate, [initial], [factor])` — Stella's NPV discounts per period; check the timing.

| Parameter | Meaning |
|---|---|
| `stream` | Money per time unit (negative for spending) |
| `discount rate` | Per time unit, e.g. 0.01 per month |
| `initial value` | The value at the start |
| `factor` | Multiplies the result (1 for none) |

**Result:** The discounted sum of the stream so far, times the factor.

Compare investments: a pulse of spending and a stream of revenue, discounted to the start of the run. Same definition as Vensim.

**Example:** `NPV(Revenue - Costs, 0.01, 0, 1)`


See also: SMOOTH

---

### DEPRECIATE STRAIGHTLINE

Writes each input off evenly over the depreciation time.

```
DEPRECIATE STRAIGHTLINE(input, depreciation time, [fiscal], [initial])
```

**In Vensim:** `DEPRECIATE STRAIGHTLINE(input, time, fiscal, initial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `input` | What is added this step (an investment, say) |
| `depreciation time` | How long each input takes to write off |
| `fiscal` | Optional; read but not used |
| `initial` | Optional; read but not used |

**Result:** The amount written off this step: each recent input divided by the depreciation time.

Straight-line depreciation of investments, as Vensim's DEPRECIATE STRAIGHTLINE.

**Example:** `DEPRECIATE STRAIGHTLINE(Investment, 10)`


See also: DELAY FIXED

---

### SAMPLE IF TRUE

Takes the input while the condition holds, and keeps the last value taken otherwise.

```
SAMPLE IF TRUE(condition, input, initial)
```

**In Vensim:** `SAMPLE IF TRUE(condition, input, initial)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `condition` | Non-zero means take the input now |
| `input` | The value sampled |
| `initial` | The value held until the condition first holds |

**Result:** The input when the condition holds; otherwise the value last taken.

Models a value read only at certain moments: a quarterly report, a reading taken when a switch is on.

**Example:** `SAMPLE IF TRUE(MODULO(Time, 3) = 0, Sales, 0)`


See also: INITIAL

---

### ACTIVE INITIAL

x during the run; a separate value used only while initial values are computed.

```
ACTIVE INITIAL(x, initial value)
```

**In Vensim:** `ACTIVE INITIAL(x, initial value)` · **In Stella:** `no equivalent: x with a separate initial value` — Stella has no direct counterpart.

| Parameter | Meaning |
|---|---|
| `x` | The formula used during the run |
| `initial value` | The value used while the model's starting values are worked out |

**Result:** x at every step of the run; the initial value only at initialisation.

Breaks a loop among starting values, as World3-03 does: food per capita needs food, which needs land yield, which needs food per capita.

**Example:** `ACTIVE INITIAL(food / population, 230)`


See also: INITIAL

---

### INITIAL

The value x had at the start of the run, held constant.

```
INITIAL(x)
```

**In Vensim:** `INITIAL(x)` · **In Stella:** `INIT(variable): a single variable only`

| Parameter | Meaning |
|---|---|
| `x` | Any value or expression |

**Result:** x evaluated once, at the initial time.

Handy for comparing against the starting point: e.g. growth relative to the first year.

**Example:** `Population / INITIAL(Population)`


See also: SMOOTH

---

## Lookups

### GRAPH

A curve written inline from evenly spaced values.

```
GRAPH(x, x0, dx, y1, y2, …)
```

**In Vensim:** `WITH LOOKUP(x, ((x1, y1), …))` · **In Stella:** `a graphical variable, read with LOOKUP(graph, x)`

| Parameter | Meaning |
|---|---|
| `x` | The input |
| `x0` | The x of the first value |
| `dx` | The distance between values |
| `y1, y2, …` | The values |

**Result:** Linear between values; the first value below x0 and the last beyond the end.

Handy for a short table without making a lookup variable. From Powersim.

**Example:** `GRAPH(Price, 0, 10, 100, 80, 55, 30, 10)`


See also: GRAPHSTEP, WITH LOOKUP

---

### GRAPHSTEP

Like GRAPH, but stepwise.

```
GRAPHSTEP(x, x0, dx, y1, y2, …)
```

**In Vensim:** `WITH LOOKUP (no stepwise form found)` · **In Stella:** `a graphical variable, read with LOOKUP`

| Parameter | Meaning |
|---|---|
| `x` | The input |
| `x0` | The x of the first value |
| `dx` | The distance between values |
| `y1, y2, …` | The values |

**Result:** y1 for x in [x0, x0+dx), y2 for the next stretch, and so on.

Use for tariffs, tax bands, price breaks.

**Example:** `GRAPHSTEP(Volume, 0, 100, 5, 4.5, 4, 3.5)`


See also: GRAPH

---

### LOOKUP SLOPE

The slope of a lookup at x.

```
LOOKUP SLOPE(lookup, x, m)
```

**In Vensim:** `LOOKUP SLOPE(lookup, x, m)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup |
| `x` | Where to read |
| `m` | Outside the points: 0 gives 0, 1 the nearest segment's slope, −1 gives :NA: |

**Result:** Change in y over change in x for the segment; at a point, the mean of the slopes either side.

How steeply an effect changes, for example to see where a curve flattens.

**Example:** `LOOKUP SLOPE(Effect Curve, Fraction, 0)`


See also: LOOKUP AREA

---

### LOOKUP AREA

The area under a lookup between two inputs.

```
LOOKUP AREA(lookup, start, end)
```

**In Vensim:** `LOOKUP AREA(lookup, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup |
| `start` | Where to start |
| `end` | Where to stop |

**Result:** The area between the curve and the x axis, flat beyond the end points.

Normalising an effort profile so that its total over the whole range is a known amount.

**Example:** `LOOKUP AREA(Effort Profile, 0, Fraction Done)`


See also: LOOKUP SLOPE

---

### GET DATA AT TIME

The data value at exactly that time.

```
GET DATA AT TIME(data, time)
```

**In Vensim:** `GET DATA AT TIME(data, time)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable (a lookup over Time) |
| `time` | When |

**Result:** The value, or :NA: when there is no point at that time.

Reading a series at one chosen moment.

**Example:** `GET DATA AT TIME(Sales Data, 20)`


See also: GET DATA BETWEEN TIMES

---

### GET DATA BETWEEN TIMES

The data value at a time that may fall between points.

```
GET DATA BETWEEN TIMES(data, time, mode)
```

**In Vensim:** `GET DATA BETWEEN TIMES(data, time, mode)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `time` | When |
| `mode` | 0 interpolates, 1 takes the later point, −1 the earlier |

**Result:** The value, or :NA: before the first or after the last point.

Reading a series at times that are not on its grid.

**Example:** `GET DATA BETWEEN TIMES(Sales Data, Time + 1, 0)`


See also: GET DATA AT TIME

---

### GET DATA FIRST TIME

The time of the first data point.

```
GET DATA FIRST TIME(data)
```

**In Vensim:** `GET DATA FIRST TIME(data)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |

**Result:** The first time in the series.

Finding where a series starts.

**Example:** `GET DATA FIRST TIME(Sales Data)`


See also: GET DATA LAST TIME

---

### GET DATA LAST TIME

The time of the last data point.

```
GET DATA LAST TIME(data)
```

**In Vensim:** `GET DATA LAST TIME(data)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |

**Result:** The last time in the series.

Finding where a series ends.

**Example:** `GET DATA LAST TIME(Sales Data)`


See also: GET DATA FIRST TIME

---

### GET DATA MAX

The largest data value between two times.

```
GET DATA MAX(data, start, end)
```

**In Vensim:** `GET DATA MAX(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The maximum, or :NA: when there are no points in the range.

Summaries of a series, usually inside INITIAL.

**Example:** `INITIAL(GET DATA MAX(Sales Data, INITIAL TIME, FINAL TIME))`


See also: GET DATA MIN, GET DATA MEAN

---

### GET DATA MIN

The smallest data value between two times.

```
GET DATA MIN(data, start, end)
```

**In Vensim:** `GET DATA MIN(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The minimum, or :NA: when there are no points in the range.

Summaries of a series.

**Example:** `GET DATA MIN(Sales Data, 0, 12)`


See also: GET DATA MAX

---

### GET DATA MEAN

The mean of the data values between two times.

```
GET DATA MEAN(data, start, end)
```

**In Vensim:** `GET DATA MEAN(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The average of the points in the range.

Summaries of a series.

**Example:** `GET DATA MEAN(Sales Data, 0, 12)`


See also: GET DATA MEDIAN

---

### GET DATA MEDIAN

The median of the data values between two times.

```
GET DATA MEDIAN(data, start, end)
```

**In Vensim:** `GET DATA MEDIAN(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The middle value of the points in the range.

Summaries of a series that ignore outliers.

**Example:** `GET DATA MEDIAN(Sales Data, 0, 12)`


See also: GET DATA MEAN

---

### GET DATA STDV

The standard deviation of the data values between two times.

```
GET DATA STDV(data, start, end)
```

**In Vensim:** `GET DATA STDV(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The sample standard deviation of the points in the range.

How much a series varies.

**Example:** `GET DATA STDV(Sales Data, 0, 12)`


See also: GET DATA MEAN

---

### GET DATA TOTAL

The sum of the data values between two times.

```
GET DATA TOTAL(data, start, end)
```

**In Vensim:** `GET DATA TOTAL(data, start, end)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `data` | The name of a data variable |
| `start` | From |
| `end` | To |

**Result:** The sum of the points in the range.

Totals of a series.

**Example:** `GET DATA TOTAL(Sales Data, 0, 12)`


See also: GET DATA MEAN

---

### LOOK

Reads a curve (a lookup) in the way you ask for.

```
LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)
```

**In Vensim:** `LOOK(curve, x, mode=lookup|forward|backward|inverse|extrapolate|slope|area, start=, outside=)` · **In Stella:** `no direct equivalent`

| Parameter | Meaning |
|---|---|
| `curve` | The name of a lookup |
| `x` | Where to read it (for mode=inverse, the value to find) |
| `mode=` | lookup (default): normal reading. forward / backward: as steps, the next or previous point. inverse: the input that gives this value. extrapolate: continue the end slopes beyond the ends. slope: how steep it is here. area: the area under it |
| `start=` | With mode=area: where to start (default 0); the area runs from start to x |
| `outside=` | With mode=slope: outside the points, 0 (default), 1 the nearest slope, −1 not available |

**Result:** The value, the input, the slope or the area, depending on the mode.

Replaces the lookup functions LOOKUP FORWARD, BACKWARD, INVERT, EXTRAPOLATE, SLOPE and AREA. LOOK(Effect, Price) is the same as Effect(Price).

**Example:** `LOOK(Effect of price, 0.7, mode=inverse)`


See also: LOOKUP SLOPE, WITH LOOKUP

---

### LOOKUP FORWARD

Reads a lookup as steps: the y of the next point at or after x.

```
LOOKUP FORWARD(lookup, x)
```

**In Vensim:** `LOOKUP FORWARD(lookup, x)` · **In Stella:** `a discrete graphical function`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup |
| `x` | Where to read |

**Result:** The y of the first point whose x is at or above x; no interpolation.

For values that hold until the next point, such as a price list.

**Example:** `LOOKUP FORWARD(Price Table, Time)`


See also: LOOKUP BACKWARD, WITH LOOKUP

---

### LOOKUP BACKWARD

Reads a lookup as steps: the y of the last point at or before x.

```
LOOKUP BACKWARD(lookup, x)
```

**In Vensim:** `LOOKUP BACKWARD(lookup, x)` · **In Stella:** `a discrete graphical function`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup |
| `x` | Where to read |

**Result:** The y of the last point whose x is at or below x; no interpolation.

For values that hold from one point until the next.

**Example:** `LOOKUP BACKWARD(Tax Bands, Income)`


See also: LOOKUP FORWARD

---

### LOOKUP INVERT

Reads a lookup the other way: the x at which it reaches y.

```
LOOKUP INVERT(lookup, y)
```

**In Vensim:** `LOOKUP INVERT(lookup, y)` · **In Stella:** `LOOKUPINV(gf, y)`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup that only rises or only falls |
| `y` | The value to find |

**Result:** The x, interpolated, where the lookup equals y.

Turning a target value back into the input that gives it.

**Example:** `LOOKUP INVERT(Yield Curve, Target Yield)`


See also: LOOKUP FORWARD

---

### LOOKUP EXTRAPOLATE

Like the lookup, but continues in straight lines beyond the first and last points.

```
LOOKUP EXTRAPOLATE(lookup, x)
```

**In Vensim:** `LOOKUP EXTRAPOLATE(lookup, x)` · **In Stella:** `an extrapolating graphical function`

| Parameter | Meaning |
|---|---|
| `lookup` | The name of a lookup |
| `x` | Where to read |

**Result:** The interpolated y; beyond the ends, the line through the two end points continued.

When the trend at the edge of the data should carry on instead of staying flat.

**Example:** `LOOKUP EXTRAPOLATE(Demand Curve, Price)`


See also: LOOKUP FORWARD

---

### WITH LOOKUP

A graphical function written directly into the equation.

```
WITH LOOKUP(input, ((x1,y1), (x2,y2), …))
```

**In Vensim:** `WITH LOOKUP(x, ((x1, y1), …))` · **In Stella:** `LOOKUP(graphical variable, x)`

| Parameter | Meaning |
|---|---|
| `input` | The value looked up along the x axis |
| `((x,y), …)` | Points of the curve, in increasing x |

**Result:** The y value on the line through the points at the input's x; flat beyond the first and last points.

Most of the time a Lookup variable is easier: draw the curve with the mouse and call it like Effect(x).

**Example:** `WITH LOOKUP(Time, ((0,0), (10,5), (20,8)))`


See also: Calling a lookup

---

### Calling a lookup

Use a Lookup variable like a function.

```
Lookup Name(x)
```

| Parameter | Meaning |
|---|---|
| `x` | The input value, read along the curve's x axis |

**Result:** The curve's y value at x, joining points with straight lines and staying flat outside the first and last points.

Create a Lookup with the curve tool, leave its Input empty, and write its name followed by the input in brackets in any equation.

**Example:** `Effect of Awareness(Fraction Infected)`

- Tip: Lookups express effects that aren't straight lines: saturation, thresholds, diminishing returns.

See also: WITH LOOKUP

---

## Time

### TIME

The current simulation time.

```
TIME
```

**Result:** Starts at the initial time and goes up by the time step.

**Example:** `IF THEN ELSE(TIME > 20, New Policy, Old Policy)`


See also: INITIAL TIME, FINAL TIME, TIME STEP, STEP

---

### INITIAL TIME

When the run starts.

```
INITIAL TIME
```

**Result:** The value set in the toolbar's “Time … to …” fields.

**Example:** `TIME - INITIAL TIME`


See also: TIME, FINAL TIME

---

### FINAL TIME

When the run ends.

```
FINAL TIME
```

**Result:** The end of the run.

**Example:** `STEP(10, FINAL TIME / 2)`


See also: TIME, INITIAL TIME

---

### TIME STEP

The integration step (dt).

```
TIME STEP
```

**Result:** How far time moves each calculation step.

Smaller steps are more accurate but slower. A good rule: at most a quarter of the shortest delay in the model.

**Example:** `Stock / TIME STEP`

- Tip: Halve the time step: if results barely change, it was small enough.

See also: TIME

---

## Operators

### +  −  ×  ÷

Add, subtract, multiply and divide.

```
a + b   a - b   a * b   a / b
```

**Result:** The usual arithmetic; * and / come before + and −.

Use brackets to change the order: (a + b) * c.

**Example:** `(Births - Deaths) / Population`

- Tip: Dividing by zero gives an invalid value; use XIDZ or ZIDZ to guard against it.

See also: ^, XIDZ

---

### ^

Raise to a power.

```
a ^ b
```

**Result:** a to the power b; −x^2 means −(x²).

**Example:** `Radius ^ 2 * 3.14159`


See also: POWER

---

### Comparisons

Compare two values.

```
a < b   a <= b   a > b   a >= b   a = b   a <> b
```

**Result:** 1 when true, 0 when false.

Mostly used inside IF THEN ELSE.

**Example:** `IF THEN ELSE(Stock >= Capacity, 0, Inflow)`


See also: IF THEN ELSE, :AND:  :OR:  :NOT:

---

### :AND:  :OR:  :NOT:

Combine conditions (also written && || !).

```
a :AND: b   a :OR: b   :NOT: a
```

**Result:** 1 when true, 0 when false.

**Example:** `IF THEN ELSE(Time > 10 :AND: Stock < 50, Order, 0)`


See also: Comparisons, IF THEN ELSE

---

