# Units and calendar time

Dynamist reads every unit as an exact quantity: base dimensions plus a scale. The model doctor (`dyn check`) uses them to find the mistakes that are easiest to make and hardest to see: a stock in thousand barrels fed by a flow in barrels, a power in MW added to an energy in kWh, a time step that is not the unit the rate is written in.

## How a unit is read

A unit is `m, kg, s, A, K, mol, cd` raised to whole powers, plus `currency:USD` style dimensions for money, `count:truck` style dimensions for counted things, and a scale (the factor to the base unit). `kWh` is `kg m^2 s^-2` with scale 3.6e6; `bbl` is `m^3` with scale 0.158987294928.

Accepted writing: `a/b`, `a*b`, `a^n`, brackets (`kWh/(kW*Year)` is `kWh/kW/Year`), the word "per" (`Barrels per Truck`), plurals (`Barrels`), `Day of Month` (the words after "of" are dropped), and the dimensionless words (`dmnl`, `fraction`, `ratio`, `index`, `units`).

| Group | Atoms |
|---|---|
| SI prefixes (on metric atoms) | k, M, G, T, P, m, µ or u, n |
| Time | s, min, h, Day, Week, Month, Quarter, Year |
| Energy and power | J, Wh, W, Btu, MMBtu, toe, cal |
| Volume and mass | L, bbl, gal, cf, g, t (tonne), lb, ft, mile |
| Money | USD, EUR, GBP and 24 more ISO codes; `kUSD`, `MEUR`; `USD_2024` is 2024 dollars |
| Pure numbers | % (0.01), ppm (1e-6) |
| Anything else | a counted thing: `truck`, `crew`, `customer` |

A word that is close to a known unit but not one (`kwhh`) is an error with a suggestion (`kWh`). Case is forgiven where it cannot change the meaning (`kwh`, `DAY`); the capital M wins over milli (`mw` is MW).

## Symbols with more than one meaning

`MB` (thousand or million barrels), `MMB`, `ton` (metric, short or long), `gallon` (US or UK), `MCF` are ambiguous. The first time one is used the doctor raises `unit-ambiguous` with one fix per meaning. The choice is stored in the model (`settings.unit_choices`, set with `{"op":"set_setting","key":"unit_choice","value":"MB=thousand barrels"}`) and honoured from then on. Until a choice is made, the first meaning is used.

## What the checks require

* Sums, differences, comparisons, `MIN`, `MAX` and the branches of `IF THEN ELSE` need equal dimensions and a scale ratio of 1 within 1e-12.
* A flow times the time unit must be its stock's unit, scale included. A flow with no declared unit (the net flow the importers make) is expected in stock unit per time unit.
* A rate meeting an amount is an error. The old lenient rule (time ignored) is still available with `dyn check --lenient-time` for old models. `dyn validate` keeps its older lenient unit warnings.
* Money: `USD_2024` adds to `USD_2024`, not to `USD` (error `money-real-nominal`).

## Fixes, in order

1. A scale factor: a constant named "to per from" in a group called Conversions, with the exact factor, wrapped round the offending term. Example: "MB per barrel" = 0.001, "kWh per MW Day" = 24000.
2. A factor with a dimension taken from a variable of the model whose unit is the missing factor or its inverse.
3. A declaration change (fewest variables edited).
4. `CONVERT(x, "from", "to")` is not available yet.

Within a rank, fixes that keep the numbers come first. `dyn check --fix-all-safe` applies only first-ranked fixes that keep the numbers.

## Value scale: `units_scale_values`

With `units_scale_values` on, a constant that is a plain number and is declared in `%` or `ppm` carries that scale: `5 ~ %/Day` is 0.05 per Day, in the engine and in the checks. Other units do not rescale values (a stock in MB stays a number of MB). New models default to on. Models written before the setting, and every imported Vensim or Stella model, are off, so their numbers do not change. With it off the doctor lists each place where turning it on would change a value (`unit-scale-change`).

## Calendar time

Two conventions, in the model settings:

* **Abstract** (default): Year is `year_days_abstract` days (365.25), Month is Year/12. `dyn run` reports the convention.
* **Calendar**: with a `start_date` (ISO 8601, optional offset) and a time unit of Month or Year, each step is the real calendar month or year. A flow declared per Day (or per Hour) is multiplied by the real days (hours) of the step: 10 per Day over January and February 2027 adds 310 and then 280. The step length is one time unit.

Functions: `DAYS IN STEP()`, `HOURS IN STEP()`, `YEAR FRACTION(t1, t2, basis)` with basis 0 = 30/360 US, 1 = actual/actual, 2 = actual/360, 3 = actual/365, 4 = 30E/360 (Microsoft's YEARFRAC, including the February rules), `EXCELDATESERIAL(year, month, day)`.

**Excel serial dates.** For dates on or after 1900-03-01 the serial is the number of days since 1899-12-30 (2026-11-22 is 46,348). Excel counts a 29 February 1900 that never existed, so for 1900-01-01 to 1900-02-28 Excel's serial is one lower than days since 1899-12-30; `EXCELDATESERIAL` returns Excel's value.

**Dated data.** `calendar::resample` aligns an observed series to the model's steps: flows are summed (totals conserved), levels are sampled at the step start or averaged. `calendar::data_issues` reports gaps, duplicate times and times that go backwards.
