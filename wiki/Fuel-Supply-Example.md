# Fuel supply example — from a workbook to a model

**Source:** `Fuel Supply Planning Simulator.xlsx` (Fuel Transportation & Logistics; as-of 14 Nov 2026). **Model:** [inbox/fuel-supply/](inbox/fuel-supply/) (CSV sheets in the [template](Spreadsheet-Template.md)), also as [inbox/Fuel-Supply-Model.xlsx](inbox/Fuel-Supply-Model.xlsx). Made by [inbox/restructure_fuel_workbook.py](inbox/restructure_fuel_workbook.py).

## What the workbook does

| Workbook sheet | What it holds | In the model |
|---|---|---|
| Stock_Limits | max filling level per plant & fuel, why | `Max fill` per tank |
| Unloading_Skids | skids in service → trucks a plant can unload per day | `Max trucks per day` per tank |
| Plan_Overview | one row per plant & fuel: capacity, stock, minimum, region | the **Tank** set (57) and **Region** set (4); `Capacity`, `Opening stock`, `Min fill`, `Ready` |
| Data | per day & tank: plan trucks, consumption, stock | lookups `Consumption forecast`, `Current plan trucks` per tank |
| Plan_Engine | the *optimal plan*: deliver what is burnt plus an even share of the gap to target | `Gap per day`, `Required MB`, `Trucks needed`, `Trucks planned`, `Delivery optimal plan` |
| Plan_Overview (projection) | the *current plan* run forward | `Stock current plan`, `Closing stock current`, `Shortage vs target`, `Below minimum` |
| Summary, region sheets, Dashboard | totals per region and overall | `Region …` (per Region) and `Network …` variables |
| Plan_Settings, Guide | target day (25th), barrels per truck (215) | `Target day`, `Barrels per truck` |
| Justification sheets, Mapping, Actuals_Q, Plan_Q, Lists, hidden helpers | reporting and data plumbing | not modelled (see below) |

The model starts the day after the as-of date (15 Nov) from each tank's actual stock, one step per day, time = day of November.

## The model

- **Stocks per tank:** `Stock optimal plan`, `Stock current plan`.
- **Flows per tank:** `Delivery …` (trucks × 215 bbl ÷ 1000, MB per day) in; `Burn …` (forecast consumption) out.
- **Optimal plan rule:** `Required MB = MAX(0, Consumption forecast + (Target stock − Stock) ÷ Days left)`; whole trucks (`CEILING`), at most `Max trucks per day`; no trucks for tanks that are not `Ready`.
- **Totals:** per region (`TOTAL(...)` inside a per-Region variable) and for the whole network.
- **Three levels:** *Model* shows Inputs & limits → Plans → Totals. *Plans* holds Optimal plan and Current plan; *Optimal plan* holds Tank balance, Target & gap and Trucks & skids — and at that level the feedback is plain to see: the tank's stock feeds the gap, the gap sets the trucks, the trucks refill the tank.
- **Levels (overview):** the *Model* sheet is a design overview: five cards with a sentence each on what they are for, coloured by role, with labelled arrows. It is an overview of five modules — *Inputs & limits*, *Optimal plan*, *Current plan*, *Region totals*, *Network totals* — with a handful of arrows between them. Each opens on its own diagram. The *Showing* bar picks a region or tank for the numbers everywhere.
- **Causal loop diagram:** the arrows of the model plus concepts from the workbook that it does not calculate (skid capacity, plant tank limits, facility readiness, expired waybills, truck availability). The stock-filling loop is a *balancing* loop.
- **Scenarios:** bigger trucks (250 bbl), earlier target (day 20), double Jazan diesel skids (`Max trucks per day[Jazan - Diesel]`), and a combination.

## Checks and units

- **Model checks** (Edit → Model checks…): shortage is never negative; fill levels stay between 0 and 1; shortage at the end stays under 1000 MB (an *example* limit, replace it with a real one). They live in a *Checks* sheet.
- **Units are checked as algebra.** That check found a mistake in the first version of the restructured workbook: *Barrels per truck* was declared as barrels, not barrels per truck. It is now `Barrel/Truck`. MB, Mbbl and barrel count as one kind of thing (scale is not tracked).
- **Command line:** `dynamist --convert wiki/inbox/fuel-supply fuel.sdm` writes the native model; then `dyn validate fuel.sdm` (clean), `dyn test fuel.sdm` and `dyn sens fuel.sdm --var "Network shortage" --runs 100 --spread 0.1` work on it. The sensitivity band is wide: with ±10% on the four key levers (100 runs, seed 1), shortage on the last day is about 240 MB at the 5th percentile, 670 at the median and 1600 at the 95th, against 612 as the model is set.
- The workbook holds no actuals, so the model has not been compared with real outcomes (*Edit → Compare with data* needs a data series).

## Check against the workbook

For the 49 tanks that are ready, the model's trucks to month end, fill level on the target day (optimal and current plan) and shortage match the workbook's Plan_Overview values (tolerances: trucks ±0.5, fill ±1e-6, shortage ±1e-4 MB). Run `cargo test fuel_supply`.

## Not modelled (and why)

- **Past days** (1–14 Nov): used only to get today's stock and consumption; the model starts at the as-of date.
- **Scheduling, loading and expiry reporting** (waybill status, justifications, tolerance flags): that is reporting about the past, not system behaviour. The *expiry recovery* and *stock uplift* splits in Plan_Engine only divide the same total, so they are not modelled separately.
- **8 tanks marked not ready** (missing capacity, max stock or stock — e.g. Qassim, PP11): kept in the Tank set with `Ready = 0`, so they get no plan.
- **Excel's volatile `TODAY()` and the AS_OF date**: fixed at import. To re-run for another date, run the script on a workbook saved as of that date.
