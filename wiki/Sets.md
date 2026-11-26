# Sets — many similar things, written once

Fifty tanks, four regions, three products: things that all work the same way. Instead of drawing and typing the same boxes fifty times, make a **set** (other tools call this an *array* or *subscript*) and say that a variable is **one for each** item of it.

## Why

- The diagram stays small: one box *Stock optimal plan* stands for all 57 tanks. It is drawn as a **stack of cards** with a badge **Tank ×57**.
- Every formula is written once: `Required MB = MAX(0, Consumption forecast + Gap per day)` works for every tank.
- Numbers that differ by tank (capacity, limits, opening stock, a daily curve) are filled in per item.
- Totals are one word: `TOTAL(Stock optimal plan)`.

## Making a set

- **In a spreadsheet:** the *Sets* sheet — `Set`, `Item`, `Belongs to` (see [Spreadsheet template](Spreadsheet-Template.md)).
- **By hand:** select a variable, and under *Details → One for each* pick the set it exists for. The *Differs by …* list under it holds the item-specific values.

A set can **belong to another set**: each tank belongs to a region. That is what lets totals follow the grouping.

## Rules, in plain words

1. In a formula for a per-Tank variable, another per-Tank variable means **the same tank's** value.
2. A per-Tank formula can read a per-Region variable: it gets **its own region's** value.
3. `TOTAL(X)` adds up X over **all** items. Inside a per-Region variable it adds up **only that region's** tanks.
4. `TOTAL(X, COA)` adds up the items that belong to *COA* (or the single item with that name). `AVERAGE(X)` / `AVERAGE(X, COA)` average.
5. `X[Hail 2 - Crude]` picks one item (use quotes if the name has odd characters: `X["Rabigh CC + Steam - Diesel"]`).
6. A plain (not per-set) formula cannot use a per-set name without `TOTAL`, `AVERAGE` or `[item]` — the app says so.

## The Showing bar: look inside one item

A diagram sheet with sets has a **Showing** bar with a dropdown per set (*Region*, *Tank*). With *All* every box shows the total of its items. Pick a tank and the **whole model is about that tank**: the numbers on the diagram, the Trends graph, tables, the side panel and its sliders. Pick a region and per-tank boxes show that region's tanks together. Choosing a tank also shows its region; the tank list narrows to the region you chose; ✕ goes back to totals. In the side panel, *Sliders for the selection* change only the chosen tank (they are saved as that tank's own value).

## What you see

- **Diagram:** a set variable shows `Σ total` (or `avg`) of its items, and its graph is the total (average). *Combine = Average* is for ratios and percentages.
- **Charts and tables:** the variable name gives the total; `Name[item]` gives one item — e.g. `Closing fill current[Jazan - Diesel]`.
- **Scenarios:** `Max trucks per day` changes every item; `Max trucks per day[Jazan - Diesel]` changes just one.
- **Inside the engine:** before a run, each set variable is expanded into plain copies (`Stock[Hail 2 - Crude]`, …). Nothing else in the app needs to know.

## Example: fuel supply

[inbox/fuel-supply/](inbox/fuel-supply/) is the *Fuel Supply Planning Simulator* workbook restructured with sets: 57 tanks in 4 regions, an *optimal plan* and the *current plan* per tank, region and network totals. It reproduces the original workbook's results for every tank that has a plan (checked by the test `fuel_supply_model_matches_the_workbook`). See [Fuel supply example](Fuel-Supply-Example.md).

## Limits

- A set can belong to one other set (tank → region), not several at once (tank → region *and* fuel).
- A flow between per-set stocks must be per the same set as both its stocks (or the stock's parent set).
- To chart one item: *+ Add → ▸ one Tank at a time*, or type `Name[item]` in Customize chart.
- Items cannot be added while a run plays; edit, and the run refreshes.
