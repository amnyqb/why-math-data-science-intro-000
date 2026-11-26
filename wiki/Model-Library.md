# Model library: how the app should present it

The library is `library/`: seven `.sdm` files in `library/lessons/`, one per tutorial lesson, and `library/index.json` with, for each model, its title, level, concepts taught, prerequisites, the question it answers, source, licence, file and tutorial page. `library/build_library.py` writes both from one place, so the index cannot drift from the files. The lessons are in [the tutorial](Tutorial.md).

## What is in it, and the rights to it

Every model is our own work, written from textbook structures; the index records this per model. Lessons 4 and 7 follow the structure of the Workforce-Inventory diagram of Hayward and Roach (2023); the formulas and numbers are ours and nothing of the paper is copied. The Market Growth rebuild of a Stella model (no licence) and any other third-party file are not in the library. Rule for adding one: a model goes in only with a `source` and a `licence` in its entry that allow shipping it.

## How the app should present it

1. **File → Open example ▸** a submenu, grouped by level (Beginner, Intermediate, Advanced), one item per model, each showing its title. Hovering shows the question and the concepts. Beside *Open…* and above *Import*.
2. **An Open example dialog** (also from the start screen and the command palette, ⌘K): a list of cards with title, level chip, concepts, prerequisites, and a *Lesson page* button that opens the wiki page. A model whose prerequisites have not been opened shows them first (not blocked: only ordered).
3. **Opening is copying.** The example loads as an unsaved model (title bar "Births and deaths (example)"); Save asks for a new name so the library file is never overwritten.
4. **Embed the files** in the binary with `include_str!` so the menu works without the repository on disk.
5. **Numbered lessons in the app:** a *Lesson* side panel that shows the page's steps (the question, what to click, the check) beside the model, with *Next lesson*. The pages are Markdown already.


## Searching and filtering the library (built)

The library is described by `library/catalog.json`, written by `library/make_catalog.py`. Search it with `dyn library`; check it with `dyn library --check` (vocabulary, scales, files that exist).

**Two kinds of tags.** *Generic* tags come from a fixed vocabulary in the catalogue, so every model is described with the same words: `category` (lesson, classic, textbook, benchmark, case-study), `domain` (population, epidemiology, ecology, economy, supply-chain, workforce, world, urban, ...), `skills` (stocks-and-flows, feedback-loops, delays, oscillation, growth-limits, policy-design, sensitivity, loop-dominance, ...) and `features` (lookup, delay-fixed, levers, units, multiple-sectors, ...). *Special* tags (`special`) are free words for a model's own ideas ("bullwhip", "overshoot and collapse", "tragedy of the commons"). A word that is not in the vocabulary is an error for a generic tag and fine for a special one.

**Other fields.** `title`, `question` (what the model answers), `lessons`, `level` (beginner, intermediate, advanced), `simplicity` and `difficulty` (each 1 to 5: how small the model is, how hard it is to learn), `authors`, `year`, `paper` (citation), `status` (`built` has a file, `planned` does not), `file`, `source`, `licence`.

```
dyn library --domain supply-chain --skill delays --max-simplicity 3
dyn library --tag bullwhip
dyn library --author sterman --year-from 1980 --status built
dyn library --text "carrying capacity" --level beginner
dyn library --facets           # how many models carry each tag
dyn library --stats            # adds stocks, flows, lookups, levers read from the model file
dyn library --check            # validate the catalogue itself
```

Filters combine (all must hold); `--tag` matches any generic or special word; `--text` looks in title, question, lessons, authors, tags and the paper citation.

## The classics (built)

Each is our own implementation of the published structure, with our own parameter values, in `library/classics/<slug>/`: `make.py` (builds the model from the operations language), `<slug>.sdm`, `check.py` (checks against an independent reference) and `NOTES.md` (question, structure, parameters, interpretation choices, source, lessons).

| Model | Source of the structure | Checked against |
|---|---|---|
| `bass-diffusion` | Bass 1969 | the closed-form S-curve (within 0.1% for three parameter pairs), the peak time |
| `sir-epidemic` | Kermack and McKendrick 1927 | final-size equation, peak at S = N/R0, conservation |
| `predator-prey` | Lotka 1925, Volterra 1926 | the conserved quantity, the cycle period, an independent RK4 run |
| `fishery-commons` | Gordon 1954 | open-access equilibrium, maximum sustainable yield, quota and fleet-limit outcomes |
| `industrial-dynamics` | Forrester 1961 | steady state, conservation, amplification up the chain, step convergence |
| `market-growth` | Forrester 1968 | steady state, accounting identities, switch-off behaviours, step convergence |

Also in the library: the beer game and workforce-inventory (benchmarks B1 and B3). World2, Urban Dynamics and World3 are planned; they are large.

## Bringing models in (built)

`dyn library --ingest FOLDER --out DIR --source TEXT --licence TEXT [--json OUT]` imports every Vensim, Stella, XMILE or `.sdm` file under FOLDER, saves each as `.sdm` in DIR, runs it, and prints a summary (files, imported, run) and the files that failed with the reason. With `--json` it writes one catalogue entry per file with its size, loss-report count, validation errors and run result. Nothing is invented: tags, question, authors and year stay empty and the entry says what it needs. `--source` and `--licence` are required so a file never enters the library without a stated origin (write "unverified" if you do not know).

Rights: models whose licence allows it (for example the MIT-licensed SDXorg test models and SDEverywhere models) can be kept in the library. Files from software installs or books with restrictive terms are imported for analysis and checking only.

Excel: the spreadsheet adapter reads a workbook that follows the [spreadsheet template](Spreadsheet-Template.md). A converter for free-form Excel sheets (formulas as variables, rows as time) is not built.

## Status

Not built in the GUI. The submenu alone is small, but `src/app.rs` is being changed by other sessions and the app needs a full GUI build to check; the work is in [BACKLOG](BACKLOG.md) as B71 (menu and dialog) and B72 (lesson panel). Until then, open a lesson with File → Open… and the file in `library/lessons/`; the tutorial pages say so.
