# Import fidelity: do Vensim and Stella models give the same numbers in Dynamist?

A model you already have, in Vensim `.mdl` or Stella `.stmx`/XMILE, should import into Dynamist, run without
problems, and give **the same numbers** as the tool it was built in. Benchmark **B2** (`benchmarks/b2/`) measures
this. Its full, generated report is [`benchmarks/b2/B2_report.md`](../benchmarks/b2/B2_report.md).

## What is compared

| Source | Reference | How strong |
|---|---|---|
| [SDXorg/test-models](https://github.com/SDXorg/test-models) (MIT): single-feature tests and sample models, many in `.mdl`, `.xmile` and `.stmx` | the output **exported from Vensim** (and from Stella, where given) | proof |
| [SDEverywhere](https://github.com/climateinteractive/SDEverywhere)'s test models (MIT) | Vensim's own `.dat` output | proof |
| Every `.mdl` of a local Vensim PLE install, including World3-03, World Dynamics, Urban Dynamics and the MARKET sample | **PySD**, the open-source Vensim engine of SDXorg, run on the same file | strong evidence: PySD is itself tested against Vensim, but it is not Vensim |

Each model is imported with `dyn run` and run with its own time settings. Every variable the reference has is
compared at every time the reference has. **Exact** means equal to the reference's printed precision: within
1e-5 relative, plus 1e-6 of the variable's largest value. The second term is there because Vensim computes in
single precision, so a stock that should be 0 can print as −2.5e-6.

Vensim keeps its own runs in binary `.vdf` files, and Vensim PLE cannot export them. That is why the Vensim sample
library is compared with PySD and not with Vensim.

## Results (20 November 2026, after R31)

| Measure | Result |
|---|---|
| Test-suite model files with the original tool's output that Dynamist matches **exactly** | **219 of 335** (65%); 2 more within 1e-3 |
| …of which XMILE/Stella files | 84 exact (25 of the rest are broken test files: `???` left by Vensim's exporter, or a macro that is not in the file) |
| Vensim sample models that import and run | 285 of 367 (none crash) |
| …of those PySD can also run, matched **exactly** | **175 of 185** (95%) |

The 10 samples that differ from PySD:

| Model | Cause |
|---|---|
| 6 models with random numbers | each engine has its own random generator (expected) |
| `GET VDF` example | reads a binary `.vdf` file, which Dynamist does not import |
| Batch delay example (`PULSE TRAIN`) | at the end time Dynamist has the pulse on and PySD does not. Vensim's own output has it on, so **Dynamist is right** |
| World3-03, in three copies | see below |

### The classic models

| Model | Imports and runs | Against PySD |
|---|---|---|
| World Dynamics (Forrester 1971) | yes | exact, 81 variables |
| Urban Dynamics (Forrester 1969) | yes | exact, 251 variables |
| Market (Vensim sample MARKET; not Forrester's Market Growth) | yes | exact, 50 variables |
| Epidemic; Rabbits and foxes (Vensim samples) | yes | exact |
| World3-03 (Meadows et al. 2004) | yes | identical at 1900, then 0.004% to 0.17% apart (open: [B36](BACKLOG.md)) |

World3-03 first diverges at the first step, in the two `SMOOTH`s fed by `ACTIVE INITIAL` variables. PySD leaves
them at 1 for that step, although its own reported inputs are 1.17 and 1.34; Dynamist moves them by one Euler step.
Two minimal models with the same pattern agree in both engines, so the cause is not found yet. One look at Vensim's
own run settles which engine is right: in Vensim PLE, open `wrld3-03`, run it, and read **Perceived Food Ratio** at
1900.5. **1.0427** means Dynamist matches Vensim; **1.0** means PySD does.

## What made the difference

Fixed while running B2. Each fix is checked against Vensim's own output; see [Format mapping: Vensim](Format-Mapping-Vensim.md):

- array constants written as tables (`1, 2; 3, 4;`);
- old DYNAMO-style lookup tables;
- `ACTIVE INITIAL`, `SAMPLE IF TRUE`, `QUANTUM`, `GAME`;
- `DELAY N` / `SMOOTH N` with their own initial value;
- subranges, mappings between dimensions, dimension and item names used as numbers, dimensions that list other
  dimensions, `:NOT:`;
- files not in UTF-8.

One silent error was found and fixed. When a formula had to be written item by item, every item after the first
was skipped, so all items took item 1's formula, with no error.

## What is still open

The fix list is in the [backlog](BACKLOG.md), items **B29 to B37**, with how many test models each one affects.

## Run it yourself

```
cd benchmarks/b2
git clone --depth 1 https://github.com/SDXorg/test-models external/test-models
git clone --depth 1 --filter=blob:none --sparse https://github.com/climateinteractive/SDEverywhere external/sdeverywhere
(cd external/sdeverywhere && git sparse-checkout set models)
cargo build --release -p dynamist-cli
uv venv --python 3.12 build/pysd-venv && uv pip install --python build/pysd-venv/bin/python pysd   # for the PySD comparison
python3 report_b2.py                                 # runs every model of the local Vensim install; copies those that run
python3 pysd_all.py external/vensim_samples          # PySD on each of them
python3 run_b2.py && python3 report_b2.py            # compare and write B2_report.md
```

The downloaded suites, the copies of Vensim's samples and the PySD runs stay out of git.
