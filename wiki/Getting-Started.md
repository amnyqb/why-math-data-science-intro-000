# Getting started

## Run the app

The program is `app/target/debug/dynamist` — double-click it in Finder. It opens with an example model, *Spread of an Illness*, already simulated.

To rebuild it after a change: in Terminal, `cd "~/Documents/Craft Anything/vensimple/app"` then `cargo build`.

## A five-minute tour

1. **The diagram** (Model sheet). Boxes are *stocks* (Susceptible, Infected, Recovered); the thick pipes with valves are *flows* (Infection Rate, Recovery Rate); green diamonds are *constants*; orange dots are *variables*; the pink curve is a *lookup*; thin curved arrows say what affects what (+ same direction, − opposite).
2. **Click any item.** The right panel shows its equation, units, notes and a small graph. Type in the equation box — suggestions pop up; press **F1** for the full function reference.
3. **Drag a slider** (select nothing to see all sliders in the right panel). The whole model re-runs instantly: the numbers on the diagram, the graphs and the dashboard update together.
4. **Press Run** (top right). The lines draw themselves over time, and the diagram's numbers tick along.
5. **Look at the sheets** along the top: *Equations* and *Results* (spreadsheet-like tables), *Dashboard* (cards, charts, sliders), *Scenarios* (what-ifs side by side), *Causal loops* (the loops and their R/B labels), *Optimize* (find the best settings).
6. **Copy a picture**: right-click any chart → *Copy chart as picture*, or **Copy as picture** at the top right for a whole sheet. Paste into Word or PowerPoint.

## Build your own model

- From scratch: File → **New model**, then pick tools on the left: **S** stock, **F** flow (drag from a stock to another or to empty space), **A** variable, **C** constant, **G** lookup, **K** arrow, **N** note. **V** or **Esc** goes back to selecting.
- From a spreadsheet: File → **Import model from spreadsheet…** — see [Spreadsheet template](Spreadsheet-Template.md).
- From an older tool's text model (`.mdl`): File → **Import classic model**.

Save with **⌘S** (`.sdm` files). Undo anything with **⌘Z**.
