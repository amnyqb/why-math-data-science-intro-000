# Tutorial: system dynamics in seven lessons

Seven short models, one idea each, in the order that builds the skills. Every lesson is a model you open in Dynamist (or run from the command line), a question, a guided exercise and a check with the answer. Every number in the pages was produced by running the model; [`library/build_tutorial.py`](../library/build_tutorial.py) rebuilds all the figures and the numbers (`wiki/tutorial/img/facts.json`) in a few seconds.

| # | Lesson | Level | You learn | Model |
|---|---|---|---|---|
| 1 | [The bathtub](tutorial/01-bathtub.md) | beginner | stock and flow: a stock changes by its net flow | `library/lessons/01-bathtub.sdm` |
| 2 | [Births and deaths](tutorial/02-population-loops.md) | beginner | feedback: reinforcing and balancing loops | `02-population-loops` |
| 3 | [A shipment delay](tutorial/03-delays.md) | beginner | delays: material in transit, first-order and fixed | `03-delays` |
| 4 | [Hiring and stock](tutorial/04-workforce-oscillation.md) | intermediate | oscillation: a balancing loop with a delay overshoots | `04-workforce-oscillation` |
| 5 | [Growth runs into a limit](tutorial/05-limits-to-growth.md) | intermediate | growth limits: S-shaped growth, loops trading dominance | `05-limits-to-growth` |
| 6 | [A fishery: testing a policy](tutorial/06-fishery-policy.md) | intermediate | policy testing: levers, scenarios, sweeps, a landscape | `06-fishery-policy` |
| 7 | [Which lever matters?](tutorial/07-workforce-what-matters.md) | advanced | finding what matters: screening, Sobol indices, tornado | `07-workforce-what-matters` |

Do them in order; each page lists what it assumes. The library index with each model's concepts, prerequisites, source and licence is [`library/index.json`](../library/index.json); [the library note](Model-Library.md) says how the app should present it.

## How to open a lesson

- **In the app:** File → **Open…** (⌘O), then choose the lesson's file in `library/lessons/`. Press **Run** (top right). Select any item to see its equation; drag a slider to change a constant and the whole model re-runs.
- **From the command line:** `dyn run library/lessons/01-bathtub.sdm --series "Water in tub"`, with `dyn` built by `cargo build --release -p dynamist-cli`. Add `--set "Drain rate=6"` to change a constant for that run.
- **From Python:** `import dynamist as dyn; m = dyn.Model("library/lessons/01-bathtub.sdm"); m.run(overrides={"Drain rate": 6}, series=["Water in tub"])`. Set `FC_BIN` to the `dyn` binary. The statistical charts in lessons 4 to 7 need `pip install -r python/requirements-plots.txt`.

Figures in the pages are drawn the way the book draws them: diagrams and loop diagrams by `dyn render`, plain charts by `dyn chart`, the others by [`dynamist.plots`](Charting-Ideas.md).

## Rebuild and test

```bash
cargo build --release -p dynamist-cli
python3 library/build_library.py          # the seven models and library/index.json
python3 library/build_tutorial.py         # every figure and facts.json
python3 -m pytest library/test_library.py # models load, validate, run; pages quote the facts; figures are current
```

## Sources and licences

All seven models are our own work, written from textbook structures. Lessons 4 and 7 follow the structure of the Workforce-Inventory diagram in Hayward and Roach (2023, Fig. 5): the diagram's structure is theirs, the formulas, names and numbers are ours, and no text, table or file of the paper is included. None of the models comes from a third party's file, and the Market Growth rebuild of a Stella model (no licence) is deliberately not in the library.
