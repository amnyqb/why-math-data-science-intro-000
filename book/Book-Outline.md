# The book: from first model to expert, with Dynamist

*Working outline. Last updated 20 November 2026. This file is the plan for the book and the place to collect topics, cases and examples. Edit it freely.*

## 1. What the book is for

A reader who has never built a system dynamics model should be able to finish this book and do **expert work** with Dynamist: model a supply chain, a shortage, an investment, an ageing population; trust the result; compare it with data; bring in models from Vensim and Stella; and have an AI assistant build and check models with them.

The book does not teach a catalogue of functions. It teaches **cases**. Each chapter starts from a real question, builds the model, shows the numbers, and uses an expert technique in one line, where other tools need a page of formulas.

## 2. Principles

1. **One question per chapter.** "Why does the shop run out when the factory ships more?" is a chapter. "Delay functions" is not.
2. **The expert way in one line.** Where Vensim or Stella needs five functions, we show the Vensim way once, briefly, and our way (`DELAY(Orders, 6, exact=True)`).
3. **Every number is real.** Every example is a model file in [`examples/`](examples/) that runs, and every figure quoted in the text comes from running it. A test re-runs the examples and checks the numbers quoted in the text, so the book cannot drift from the software (`the_books_examples_run_and_give_the_numbers_the_text_quotes` in `core/src/mdl.rs`; add each new example and its numbers there).
4. **Show, don't only tell.** Every chapter has screenshots of the app and of the result. Where a screenshot is needed, the text says so with a note (see section 3).
5. **Say what is not built.** A technique the app does not have yet is marked ⬜ and the chapter is written around what exists today.
6. **Trust is a topic.** How we know a model is right (checks, units, conservation, comparing with data, matching Vensim exactly) is taught as part of modelling, not as an appendix.
7. **No oversimplifying, no atlas.** A handful of expert functions with readable options. The book explains what each one means in words, then in numbers.

## 3. Conventions in this file

**Screenshot notes.** Wherever a picture is needed, write a note in this form, so that they can be collected and taken in one session:

> 📷 **SCREENSHOT** `images/ch03-results-panel.png`: *What to show, which sheet and panel, what to select first, what to leave out.*

Number the files by chapter. Keep the window the same size for all screenshots (suggested: 1440×900, light theme, with the dark theme version of the main ones for the cover and the dark-mode box). Names of menus and buttons in the text must match the app exactly (see [User guide](../wiki/User-Guide.md)).

**Status of a feature or technique.**

| Mark | Meaning |
|---|---|
| ✅ | exists and is tested |
| 🔶 | exists in part; the gap is named |
| ⬜ | planned; the chapter works around it or waits for it |

**Boxes** used in chapters: *Try it* (do this in the app), *In Vensim* (how the same is written there), *Check* (how to know it worked), *Common mistake*, *Going further*.

## 4. How a chapter is built

Every chapter has the same parts, in this order:

1. **The question**: two or three sentences a manager would ask.
2. **The picture first**: the stock-and-flow diagram, built step by step. 📷
3. **The first run**: the simplest version, and what it shows.
4. **The expert step**: the technique that makes it realistic, in one line, with the numbers before and after.
5. **Check it**: the checks and units that would catch a mistake; one deliberate mistake and how the app reports it.
6. **Ask what-if**: scenarios and levers; one chart that answers the question.
7. **In Vensim / Stella**: how the same model reads in those tools, and how to bring it over.
8. **Exercises** with answers, and the model file to start from.

## 5. Contents

### Part I. The basics: a first model you trust

| # | Chapter | The question | Teaches | Status |
|---|---|---|---|---|
| 1 | **A bath that fills and drains** | Why does the water level keep changing after I turn the tap down? | stocks, flows, the diagram sheet, running, the results panel | ✅ |
| 2 | **Writing equations** | How do I say "10% of the stock leaves each week"? | equations, names, units, the equation editor with autocomplete, `IF THEN ELSE`, `STEP`, `TIME` | ✅ |
| 3 | **Reading the results** | Is this good news or bad? | charts, tables, the Showing bar, kept runs, pictures for Office | ✅ |
| 4 | **Levers and scenarios** | What if the tap is turned down at week 10? | levers (circles on the diagram), the Scenarios sheet, comparing runs | ✅ |
| 5 | **Loops that drive behaviour** | Why does the stock oscillate? | feedback, balancing and reinforcing loops, causal loop sheets, the CLD | ✅ / 🔶 (the CLD is flat: B10) |
| 6 | **Many similar things written once** | Four regions and fifty tanks, one equation | sets, `TOTAL`, `AVERAGE`, items, parents (a tank belongs to a region) | ✅ |
| 7 | **Modules: models inside models** | How do I reuse a warehouse? | levels, module definitions and instances with typed ports | 🔶 (engine and API; no diagram box yet) |
| 8 | **Is the model right?** | How do I know I did not make a mistake? | units (strict), checks of kinds bounds, nonneg, reference and balance; conservation; `dyn validate`; the Checks sheet | ✅ |

> 📷 **SCREENSHOT** `images/ch01-first-diagram.png`: *An empty diagram sheet with one stock ("Bath"), one inflow ("Tap") and one outflow ("Drain"), drawn and labelled, results panel closed.*
> 📷 **SCREENSHOT** `images/ch01-first-run.png`: *The same, after Run, with the results panel showing Bath over time.*
> 📷 **SCREENSHOT** `images/ch02-equation-editor.png`: *The right panel while editing a variable: the equation, units, and the autocomplete list open on a few letters.*
> 📷 **SCREENSHOT** `images/ch04-lever-circle.png`: *A lever drawn as a circle on the diagram, mid-drag, with the chart updating.*
> 📷 **SCREENSHOT** `images/ch06-set-expanded.png`: *A set variable on the diagram, with its items listed, and the outline panel showing the tree.*
> 📷 **SCREENSHOT** `images/ch08-checks-failing.png`: *The Checks sheet with one check failing (a stock that went negative) and the message.*

### Part II. Expert cases: one problem, one expert function

These chapters are the heart of the book. Each adds one expert technique, and one function from the app.

| # | Chapter | The question | Expert technique and function | Status |
|---|---|---|---|---|
| 9 | **The factory and the shop: the beer game** | Why do orders swing more and more as we go upstream? | the bullwhip effect; four stages; a human-like ordering rule against base-stock; comparing policies with paired statistics | ✅ (benchmark B1; Sterman's numbers reproduced exactly) |
| 10 | **A shipment pipeline** | Why does stock run out when shipping was increased? | **`DELAY`**: smooth, `order=`, `exact=True` pipeline, `kind=information` (a belief catching up with a fact) | ✅ (verb built; hover explanation ⬜) |
| 11 | **An emergency room** | How long do people wait when more arrive than leave? | **`QUEUE`** and **`WAIT`**: a waiting line, the longest wait, the average wait, how many waited too long | ✅ (rules checked by hand; Vensim's own numbers not available) |
| 12 | **A water shortage** | Who gets water when there is not enough? | **`ALLOCATE`**: proportional, strict priority, smooth priority curve | ✅ |
| 13 | **A population that ages** | How many people are 65 or older in 2040? | cohorts moving up as years turn over; **`AGE`** | ✅ (the stock steps once per period: read it at the moves) |
| 14 | **An investment decision** | Is the plant worth it at 1% a month? | **`VALUE`**: present value, discount rate, the spreadsheet convention | ✅ (IRR and depreciation ⬜) |
| 15 | **What people believe** | Why do managers keep ordering too much? | perception and expectation: **`EXPECT`** (level, trend, forecast) | ✅ |
| 16 | **Starting in balance** | Why does my model jump in the first weeks? | initial values, equilibrium, `ACTIVE INITIAL`; **`SOLVE`** for balancing | 🔶 (`ACTIVE INITIAL` ✅; `SOLVE` ⬜) |
| 17 | **Curves and effects** | How strongly does price change demand? | lookups: reading a curve, inverse, slope, area; **`LOOK`** | ✅ |
| 18 | **Using real data** | Does my model match what happened? | Compare with data: held-out scoring against last-value, trend, seasonal and smoothed-trend guesses; the data bindings | ✅ |
| 19 | **Tuning with a purpose** | What ordering rule minimises cost? | Optimize: levers, bounds, limits, the effort level; avoiding overfitting (train and test windows) | ✅ |
| 20 | **How sure are we?** | What range of outcomes should I expect? | sensitivity (Monte Carlo): vary levers at random, bands of outcomes | ✅ |

For every chapter in this part, each case lists, in its own box, the Vensim way. The pairs that already exist:

| Our line | Vensim equivalent (shown once, in the *In Vensim* box) |
|---|---|
| `DELAY(Orders, 6, exact=True)` | `DELAY FIXED(Orders, 6, Orders)` |
| `DELAY(Orders, 6, order=3)` | `DELAY3(Orders, 6)` |
| `DELAY(Sales, 3, kind=information)` | `SMOOTH(Sales, 3)` |
| `ALLOCATE(Demand, Supply, by=Priority)` | `ALLOCATE BY PRIORITY(Demand[region!], Priority[region!], ELMCOUNT(region), 0.000000001, Supply)` |
| `ALLOCATE(Demand, Supply, by=Priority, curve=normal)` | `ALLOCATE AVAILABLE` with a table of profiles |
| `VALUE(Cash Flow, 0.01)` | `NPV(Cash Flow, 0.01, 0, 1)` |

> 📷 **SCREENSHOT** `images/ch10-pipeline-chart.png`: *The chart of `ch10-shipment-pipeline.mdl`: Orders, Smooth arrival, Bunched arrival, Pipeline arrival on one chart, weeks 0 to 40. This one picture shows what the options mean.*
> 📷 **SCREENSHOT** `images/ch10-delay-help.png`: *Help → function reference for `DELAY`, showing the options and the plain-words explanation.*
> 📷 **SCREENSHOT** `images/ch11-queue-chart.png`: *`ch11-emergency-room.mdl`: Arrivals against Treatment capacity, with Waiting and Longest wait on a second chart, hours 0 to 48; mark the peak at hour 18 and the hour it empties.*
> 📷 **SCREENSHOT** `images/ch13-age-classes.png`: *`ch13-ageing-population.mdl`: the Population of all fourteen classes at 2020, 2040 and 2070 as a bar chart (a population pyramid on its side), and the share aged 65 or over over time.*
> 📷 **SCREENSHOT** `images/ch12-sets-diagram.png`: *`ch12-water-shortage.mdl` on the diagram: the set Region, Demand, Priority, Supply and the three sharing variables.*
> 📷 **SCREENSHOT** `images/ch12-three-ways.png`: *A bar chart at week 1 (supply 50) of the three ways of sharing, by region.*
> 📷 **SCREENSHOT** `images/ch14-npv-chart.png`: *`ch14-investment.mdl`: cash flow and present value over 60 months, with the month where it turns positive marked.*

**Numbers already verified for the examples** (from running the files in [`examples/`](examples/)):

- *Water shortage* (`ch12`), supply 50 at week 1, North / Valley / Coast: equal share 20, 12.5, 17.5; by priority 0, 25, 25 (the Valley first, then the Coast, the North last); softened priority 8.1, 22.0, 19.9. From week 8 (supply 80) the strict priority gives 20, 25, 35: everyone is served except the North, who gets 20 of 40.
- *Emergency room* (`ch11`): 8 patients an hour arrive, 12 an hour between hours 6 and 18, against a capacity of 10. The waiting room grows to **24 at hour 18**, the longest wait reaches **2.0 hours** (average 1.12), **3** people have waited more than 2 hours, and it is empty again by **hour 30**.
- *Ageing population* (`ch13`), 14 classes of five years, 828 thousand people in 2020 (60 aged 65 or over, 7.2%): **81.0** aged 65 or over in 2025, **149.4 in 2040 (15.7% of 948.9)**, 236.5 in 2070. Read the classes at the years they move (2025, 2030, …): between moves the old class loses people to deaths and then jumps when the next class joins it, a sawtooth that comes from moving five-year classes in one step.
- *Beliefs* (`ch15`), demand rising 2 a week from week 10 to 40, noticed over 6 weeks: at week 20 demand is **120**, the manager believes **110.2** and expects **134.2** in 8 weeks (a trend of 1.5% a week); at week 40 the belief is 148.1 against a true 160. After the growth stops the trend belief fades (0.23% at week 50).
- *Price effect* (`ch17`), one curve read five ways at a relative price of 1.2: effect **0.84**; the price that gives 0.7 is **1.375**; slope **−0.8**; as steps 0.6; extrapolated to 2.5, 0.2.
- *Investment* (`ch14`): present value −400 at month 0, −388.7 at month 6, −148.5 at month 30, **+81.2 at month 60**: the plant pays back in about five years at 1% a month.
- *Shipment pipeline* (`ch10`), orders step from 100 to 160 at week 10, delay 6: the **pipeline** arrives all at once at week 16; the **bunched** line (order 3) reaches halfway at week 15.5 and 90% at week 20.25; the **smooth** line reaches halfway at week 14.25 and 90% only at week 23.75. The **belief** line (`kind=information`) has the same shape as the smooth one: the same arithmetic, read as a perception instead of as material. All four end at 160 (159.6 for the smooth ones at week 40, still catching up).

### Part III. Working with other tools and other people

| # | Chapter | The question | Teaches | Status |
|---|---|---|---|---|
| 21 | **Bringing in a Vensim model** | I have fifty models in Vensim | opening `.mdl`, the loss report, what is read exactly, what is kept as written; the sketch | ✅ |
| 22 | **Bringing in a Stella model** | and some in Stella | `.stmx` and XMILE, arrays, what to expect; older Stella files | 🔶 (older Stella files ⬜: B30) |
| 23 | **How we know it gives the same numbers** | Can I trust an imported model? | the import benchmark: Vensim's own output against ours, variable by variable; the classics (World3, Urban Dynamics, World Dynamics) | ✅ (benchmark B2) |
| 24 | **Sharing results** | The board wants a picture and a table | dashboards, pictures for Office, spreadsheets in and out | ✅ |
| 25 | **A spreadsheet as the source** | The model lives in Excel | the spreadsheet template: describing a system in a workbook so it becomes a model | ✅ |
| 26 | **Models as scripts** | Run it a thousand times, overnight | `dyn run`, `dyn optimize`, `dyn sens`, the operations language, the audit log, the Python wrapper | ✅ |
| 27 | **Building with an AI assistant** | Can an assistant build and check a model? | the MCP server, the operations language, the procedure from question to report, what to check by hand | 🔶 (the procedure: B46) |
| 28 | **What matters in this model** | Fifty parameters, three loops: where do I look? | declare the goal; `screen` (which parameters move it); `importance` (how much, and which loop drives it when); `reduce` and `verify` (a smaller model you can trust); `recommend` (structure worth trying). The Workforce-Inventory model as the running example | ✅ (benchmark B3) |

> 📷 **SCREENSHOT** `images/ch21-loss-report.png`: *Opening a Vensim model: the report of what was read, approximated or kept as written.*
> 📷 **SCREENSHOT** `images/ch23-world3.png`: *World3-03 imported and run: population, industrial output, food per capita, pollution. This is the picture of the classic run.*
> 📷 **SCREENSHOT** `images/ch24-dashboard.png`: *A dashboard sheet with a lever, a chart and a table.*
> 📷 **SCREENSHOT** `images/ch28-loop-dominance.png`: *Workforce-Inventory: workforce and inventory after the demand step, with a band below showing which loop dominates when (inventory adjustment at the turning points, workforce adjustment between them).*
> 📷 **SCREENSHOT** `images/ch28-screening.png`: *The screening table: each parameter with its importance, its class and whether the ranking has converged.*
> 📷 **SCREENSHOT** `images/ch27-assistant.png`: *An assistant's conversation that builds a small model through the operations language, with the History window showing each accepted and refused operation.*

**Numbers checked for chapter 28** (`benchmarks/b3`, model `b2-1-workforce-inventory-core`): after the step from 100 to 110 at month 5 the oscillation has a period of **19.0 months** and decays with a time constant of **8 months** (eigenvalues -0.125 ± 0.3307 i); the workforce peaks at **57.7** and the inventory falls to **75.2**; Hiring Time alone sets the damping, and the inventory correction time has the larger effect on the period (0.189 against 0.142); the inventory-adjustment loop is the strongest at all five turning points of the workforce.

### Part IV. Building your own practice

| # | Chapter | Content |
|---|---|---|
| 28 | **From question to model: a procedure** | The steps professionals follow: question, boundary, structure, sanity tests, data, calibrate, scenarios, compare policies, report. Which of the app's features serves each step. |
| 29 | **Mistakes everyone makes** | Units that hide errors; a stock that goes negative; confusing a rate with a level; calibrating on the test data; trusting a smooth fit; delays that are too short. Each with the model, the symptom and how the app shows it. |
| 30 | **Writing the report** | How to present a model to people who will not open it: the question, the picture, three numbers, the limits. Validity ranges in the manifest. |
| 31 | **What to read next** | Sources, courses, the classic models; how this book's cases relate to them. |

### Appendices

| | Appendix | Content |
|---|---|---|
| A | **The expert functions on one page** | A table of the verbs with their options and one-line meanings: `DELAY`, `ALLOCATE`, `VALUE` and, as they arrive, `QUEUE`, `AGE`, `EXPECT`, `LOOK`, `SOLVE`. |
| B | **Vensim and Stella names → ours** | Every function, with the one that replaces it; the full table is generated from the app's help. |
| C | **Glossary** | Stock, flow, level, rate, loop, delay, set, lever, scenario, check, unit… in plain words. |
| D | **Keyboard shortcuts and menus** | From the app. |
| E | **The classic models** | World3, Urban Dynamics, Market Growth, the beer game: where they come from, how to run them, what they show. |

## 6. The expert functions, as a design rule

The book is also the test of the function design. A verb belongs in the app only if a chapter can explain it in one short paragraph and one line of code.

| Verb | Replaces | Chapter | Status |
|---|---|---|---|
| `DELAY(x, time, order=, init=, kind=, exact=)` | `DELAY1`, `DELAY3`, `DELAYMTR`, `DELAYINF`, `SMOOTH`, `SMOOTH3`, `DELAY FIXED` | 10, 15 | ✅ |
| `ALLOCATE(requests, supply, by=, overlap=, curve=, spread=)` | `ALLOCATE BY PRIORITY`, `ALLOCATE AVAILABLE` | 12 | ✅ |
| `VALUE(stream, rate, init=, factor=, timing=)`, `measure=irr`, `writeoff=` | `NPV`, `NPVE`, `INTERNAL RATE OF RETURN`, `DEPRECIATE STRAIGHTLINE` | 14 | ✅ |
| `QUEUE(arrivals, served, init=)` and `WAIT(queue, measure=, among=, older_than=, younger_than=)` | `QUEUE FIFO`, `QUEUE AGE AVERAGE`, `QUEUE AGE OLDEST`, `QUEUE AGE IN RANGE` | 11 | ✅ (`QUEUE ATTRIB …` ⬜) |
| `AGE(stock, every=, entrants=, open_ended=)` | `SHIFT IF TRUE`, cohort chains | 13 | ✅ |
| `EXPECT(x, time, ahead=, init=, measure=)` | `SMOOTH`, `TREND`, `FORECAST` | 15 | ✅ |
| `LOOK(curve, x, mode=, start=, outside=)` | the lookup functions | 17 | ✅ |
| `SOLVE(…)` | `FIND ZERO`, `SIMULTANEOUS` | 16 | ⬜ (needs the engine to re-evaluate part of the model inside a step: see backlog B47) |

Things the app should do for these chapters, that make the writing easier (backlog B44 to B46):

- say in plain words what a call means when the modeler hovers over it, for example "pipeline delay: material arrives exactly 6 weeks after it leaves; nothing is lost" ⬜
- suggest options as the modeler types `DELAY(x, 6, ` ⬜
- keep the *In Vensim* mapping next to each verb in the help ✅ (the function reference has it)

## 7. Production plan

1. **Examples first.** Each chapter's model is written and run before its text. Files live in [`examples/`](examples/), named `chNN-topic.mdl` (Vensim text format, readable and tool-neutral) with a `.sdm` beside it when the chapter needs diagram positions. Done so far: chapters 10 to 15 and 17: `ch10-shipment-pipeline.mdl`, `ch11-emergency-room.mdl`, `ch12-water-shortage.mdl`, `ch13-ageing-population.mdl`, `ch14-investment.mdl`, `ch15-beliefs.mdl`, `ch17-price-effect.mdl`.
2. **A test for every example.** ✅ for the first seven: `the_books_examples_run_and_give_the_numbers_the_text_quotes` runs each example and checks the numbers quoted in the text, so a change to the engine that moves a number fails the build. Add every new example and its quoted numbers to it.
3. **Screenshots in one session per part.** Collect the 📷 notes with `grep -n SCREENSHOT Book-Outline.md`; take them all with the same window size and theme; save under `images/`.
4. **Exactness claims need evidence.** Any sentence of the form "this gives the same numbers as Vensim" must cite the benchmark that proves it (B2), with the file and the figure.
5. **Review by a novice.** Each part is read by someone who has never modelled, and every place they stop is rewritten.

## 8. Open questions for the author

- **Audience level**: who is the reader on page 1 (a manager, a student, an analyst who knows spreadsheets)? The draft assumes an analyst who knows spreadsheets and has never seen a stock-and-flow diagram.
- **Length and form**: a printed book, an online book that runs the examples in the browser, or both? Part II would gain a lot from runnable examples next to the text.
- **Which cases are yours**: the cases above are suggestions. Replace any with a case from your own work; real models make better chapters.
- **How much Vensim**: the *In Vensim* boxes assume the reader may switch tools. If the book is for people who will never meet Vensim, they can be cut to the appendix.
- **Naming**: are `DELAY`, `ALLOCATE`, `VALUE` the names you want a novice to see? The verb names are cheap to change now and expensive later.
