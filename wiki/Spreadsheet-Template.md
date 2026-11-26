# Spreadsheet template — describing a system so it becomes a model

> **Audience:** anyone (a person or an assistant such as Claude Desktop) turning a complicated spreadsheet into something Dynamist can import, and that the builder can read without guessing.
>
> **Result:** File → **Import model from spreadsheet…** builds the stock-and-flow model, its equations table, a results table, a causal loop diagram and the scenarios, and lists anything it skipped.

A ready-made workbook with every sheet filled in for the example model is at [`templates/Dynamist-Model-Template.xlsx`](templates/Dynamist-Model-Template.xlsx) (the same sheets are also in `templates/*.csv`). In the app: File → **Save blank spreadsheet template…**. The fastest way to see the format is to open the example model and use File → **Export model to spreadsheet…**.

---

## 1. The idea in one paragraph

A system dynamics model is made of **stocks** (things that accumulate: people, inventory, money, fuel in a tank), **flows** (rates that fill or drain stocks: deliveries per day, consumption per day), **parameters** (fixed inputs you might change: a price, a capacity, a delay time), **variables** (values calculated from others: a ratio, a gap, a desired rate), **lookups** (curves for effects that aren't straight lines), and **links** (what affects what, with a + or − sign). Put each kind on its own sheet, one row per item, and write every calculation as an equation that uses the other items' names.

## 2. Sheets

Every sheet is optional. Column order doesn't matter. Sheet and column names are matched loosely (case, spaces and the synonyms in brackets are accepted).

### Settings *(Simulation, Control)*

| Setting | Value |
|---|---|
| Title | Fuel supply |
| Initial time | 0 |
| Final time | 365 |
| Time step | 0.25 |
| Time units | Day |
| Method | Euler *(or RK4 for smoother, more accurate results)* |

**Time step** should be at most a quarter of the shortest delay or adjustment time in the model.

### Parameters *(Constants, Inputs, Assumptions)*

| Name | Value | Units | Min | Max | Description |
|---|---|---|---|---|---|
| Tank capacity | 50000 | Litre | 10000 | 100000 | Usable volume of the main tank |
| Delivery lead time | 3 | Day | 1 | 10 | Days between ordering and delivery |

- **Value** is a number (it may also be a formula of other parameters).
- **Min / Max** set the slider range used on dashboards, scenarios and optimization. If left empty, 0 to twice the value is used.
- Each parameter becomes a green *constant* on the diagram.

### Stocks *(Levels, Accumulations)*

| Name | Initial value | Units | Never negative | Description |
|---|---|---|---|---|
| Fuel in tank | 30000 | Litre | yes | |
| Orders in transit | 0 | Litre | yes | Ordered but not delivered |

- **Initial value** is a number or a formula (e.g. `Tank capacity * 0.6`).
- **Never negative**: `yes` stops the stock dropping below zero.
- A stock changes **only** through its flows — don't give it an equation of its own.

### Flows *(Rates)*

| Name | Equation | From stock | To stock | Units | Description |
|---|---|---|---|---|---|
| Orders | MAX(0, Desired fuel - Fuel in tank - Orders in transit) / Order adjustment time | | Orders in transit | Litre/Day | |
| Deliveries | DELAY3(Orders, Delivery lead time) | Orders in transit | Fuel in tank | Litre/Day | |
| Consumption | MIN(Demand, Fuel in tank / Minimum supply time) | Fuel in tank | | Litre/Day | |

- **From stock** = the stock the flow drains. **To stock** = the stock it fills. Leave one empty when the flow comes from, or goes to, *outside the model*.
- A flow from stock A to stock B moves material between them (e.g. *Orders in transit → Fuel in tank*).

### Variables *(Auxiliaries, Calculations, Formulas)*

| Name | Equation | Units | Description |
|---|---|---|---|
| Desired fuel | Demand * Days of cover | Litre | |
| Days of supply | ZIDZ(Fuel in tank, Demand) | Day | Protected against dividing by zero |

### Lookups *(Tables, Curves, Graphical functions)*

| Name | Input | Points | Units | Description |
|---|---|---|---|---|
| Effect of price on demand | Price ratio | (0.5,1.3) (1,1) (1.5,0.8) (2,0.7) | Dmnl | |
| Seasonality | Time | (0,1.2) (90,1) (180,0.8) (270,1) (365,1.2) | Dmnl | |
| Risk curve | | (0,0) (0.5,0.2) (1,1) | Dmnl | Used as a function: `Risk curve(x)` |

- **Points** are `(x,y)` pairs separated by spaces, with x increasing. Between points the curve is a straight line; beyond the ends it stays flat.
- **Input** filled → the lookup is a variable whose value is the curve read at that input.
- **Input** empty → the lookup is a *function*: write `Risk curve(Some value)` in any equation.

### Links *(Relationships, Arrows, CLD)*

| Cause | Effect | Polarity | Delay | Notes |
|---|---|---|---|---|
| Fuel in tank | Orders | - | | More fuel on hand → fewer orders |
| Orders | Deliveries | + | yes | |
| Supplier reliability | Delivery lead time | - | | A concept outside the model: goes on the causal loop diagram only |

- **Polarity** `+` = they move in the same direction; `-` = opposite. Leave empty if unsure.
- **Delay** `yes` marks a delay on the arrow (`||`).
- Links between model items become arrows on the diagram. Links naming things that aren't in the model become **causal loop diagram** words and arrows, so you can describe the wider system in words too.
- You don't need to list links that are already implied by equations — the importer adds an arrow for every name an equation uses. Listing them anyway is how you give them a **polarity**.

### Scenarios *(What-ifs, Cases)*

| Scenario | Variable | Value |
|---|---|---|
| Faster supplier | Delivery lead time | 1 |
| Bigger tank | Tank capacity | 80000 |
| Both | Delivery lead time | 1 |
| Both | Tank capacity | 80000 |

One row per change. Several rows with the same scenario name make one scenario. **Value** may be a formula, e.g. `STEP(10000, 100)`.

### TimeSeries *(Data, Historical data)*

| Time | Demand data | Price data |
|---|---|---|
| 0 | 1200 | 1.10 |
| 30 | 1350 | 1.12 |
| 60 | 1280 | 1.18 |

First column is time; every other column becomes a lookup over time with the column's name, which equations can use like any variable (`Demand data`).

### Sets, Values, ItemSeries *(for many things with the same structure — see [Sets](Sets.md))*

**Sets** — one row per item:

| Set | Item | Belongs to |
|---|---|---|
| Region | COA | |
| Region | WOA | |
| Tank | Hail 2 - Crude | COA |
| Tank | Tabuk PP2 - Diesel | WOA |

*Belongs to* names an item of another set (each tank's region). Leave it empty for a set that stands alone.

**Per** — an extra column on *Parameters, Stocks, Flows, Variables* and *Lookups*: the set the row exists once for (`Tank`). Empty = a single value. An optional **Combine** column (`Total`, the default, or `Average`) says how the items are shown as one number on the diagram and in charts.

**Values** — `Name`, `Item`, `Value`: an item whose number or formula differs from the row's default (`Capacity | Hail 2 - Crude | 506.3`).

**ItemSeries** — `Name`, `Item`, `Time`, `Value`: a curve or data series per item. The variable is a *Lookup* with *Input* `Time` and *Per* the set (`Consumption forecast | Hail 2 - Crude | 15 | 6.2`).

Formulas: another per-Tank variable means the same tank's value; `TOTAL(X)` adds up all items (inside a per-Region variable: that region's); `TOTAL(X, COA)`, `AVERAGE(X)`, `X[item]`. A **folder of CSV files** (one per sheet) imports too.

### Module *(an optional column on Parameters, Stocks, Flows, Variables and Lookups)*

Name the part of the system a row belongs to (`Optimal plan`, `Inputs & limits`). **Write a path to nest modules to any depth**: `Plans / Optimal plan / Trucks & skids` is a module inside a module inside a module, like blocks in a hardware design. Rows with the same label become **one module**: the *Model* sheet shows just those boxes with their inputs and outputs, and each module opens on its own diagram sheet. Leave it empty to keep everything on one sheet. A model of 40+ variables reads far better as 4–6 modules.

### Lever *(an optional column on Parameters)*

`yes` marks a parameter as a **key lever**: the few controls that drive the model. They get the sliders at the top of the side panel; other constants fold away.

### Modules *(optional: what each module is for)*

| Module | Description |
|---|---|
| Plans / Optimal plan | Delivers what is burnt plus an even share of the gap… |

One short sentence per module: it is shown on the module's card on the overview.

### Checks *(optional: limits the results must respect)*

| Name | Variable | Min | Max | When |
|---|---|---|---|---|
| Shortage is never negative | Network shortage | 0 | | always |
| Shortage at the end is small | Network shortage | | 1000 | end |

Leave Min or Max empty for no limit on that side. *When* is `always` (every time step) or `end` (last step). A broken check appears in the Problems tab.

### Functions *(optional: your own functions)*

| Name | Parameters | Body | Description |
|---|---|---|---|
| Effect of price | p | 1 - 0.3 * LN(p) | Demand falls as price rises |

See [Functions](Functions.md). Parameters are separated by commas.

### Any other sheet

Sheets with other names are reported and left out, so a workbook can keep its notes, charts and source data next to the template sheets.

---

## 3. Equations

- Use the **exact names** from the sheets; names may contain spaces and are not case-sensitive (`Fuel in tank`, `fuel in tank` and `Fuel_in_tank` are the same).
- Operators: `+ - * / ^`, brackets, comparisons `< <= > >= = <>`, logic `:AND: :OR: :NOT:`.
- Built-in time: `TIME`, `INITIAL TIME`, `FINAL TIME`, `TIME STEP`.
- Functions: `IF THEN ELSE`, `MIN`, `MAX`, `SMOOTH`, `DELAY1`, `DELAY3`, `DELAY FIXED`, `STEP`, `PULSE`, `RAMP`, `XIDZ`, `ZIDZ`, `RANDOM NORMAL` … — full list with examples in the [Function reference](Function-Reference.md).
- Excel formulas don't carry over: rewrite `=IF(B2>C2, …)` as `IF THEN ELSE(Stock > Capacity, …, …)` using names, not cell addresses.

## 4. Turning a complicated Excel workbook into the template — a checklist

For the assistant restructuring a workbook:

1. **List every quantity** the workbook uses. For each, decide its kind:
   - accumulates over time (carried forward row to row, `previous + in − out`) → **Stock**;
   - "per day / per month", added to or taken from a stock → **Flow**;
   - typed-in input someone might change → **Parameter**;
   - calculated from others in the same period → **Variable**;
   - a table that maps one value to another (bands, curves, `VLOOKUP` ranges) → **Lookup**;
   - a column of observed values over time → **TimeSeries**.
2. **Find the time step**: rows per day, week or month → Settings (Time units, Time step = 1 of that unit unless the workbook iterates more finely).
3. **Rewrite every formula with names** (no cell references). Replace running totals with a stock and its flows.
4. **Give units** to everything you can; mismatched units are the most common modelling mistake.
5. **Write the causal links** you know of, with signs, in Links — including ones about things the workbook doesn't calculate.
6. **Put the cases** the workbook compares (columns of alternatives, input sets) into Scenarios.
7. **Note anything uncertain** in the Description columns, and list open questions in [INBOX.md](INBOX.md).
8. **Check**: every name used in an equation exists on some sheet; every flow names real stocks or is left empty for outside; no stock has an equation other than its initial value.

## 5. Handing it over

1. Save the workbook into `app/wiki/inbox/` (or anywhere, and give its full path).
2. Add a message to [INBOX.md](INBOX.md): what the system is, what questions the model should answer, and the file name.
3. The builder imports it, fixes what doesn't fit, builds the causal loop diagram and dashboards, and replies in the inbox with what it did and any questions.
