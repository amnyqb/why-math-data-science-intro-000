# Specs and Vision v0.1 against the app

*19 November 2026, updated after the first round of fixes (R24). A check of `Flowcanvas_Specs_v0.1.md` and `Flowcanvas_Vision_v0.1.md` against the code as built. Status words: **done** (built and tested), **partly**, **not done**, **differs** (built, but not as written). Items marked ✔ were run again for this check.*

## 1. Summary

The engine, operations, validation, formats, CLI and MCP exist and agree with the documents in spirit. The documents describe a tidier architecture than the code has, and several named things are missing or differ. Nothing below needs a change of direction; it needs either the code or the text to move.

The first three gaps that mattered for the first benchmark (B1) — no manifest, fewer test kinds than the Specs list, Optimize only in the desktop app — were closed in R24 (marked **fixed** below).

## 2. Specs v0.1, section by section

| § | Specs says | State | Detail |
|---|---|---|---|
| 3 | Crates: core, formats, io, cli, mcp, py, app | **differs** | One `core` crate holds the formats (`mdl`, `mdlw`, `xmile`) and `loss`. Spreadsheet code is still in the app crate. MCP is part of the `cli` crate (`dyn mcp`). Python is a subprocess wrapper (`python/dynamist`), not PyO3. |
| 3 | Optimize and sensitivity in core | **fixed** (R24) | Both are in core; `dyn optimize` and an MCP `optimize` tool run the search. The window stays in the app. |
| 3 | Function registry owned by the core and passed explicitly | **not done** | Still a thread-local registry in `core/src/userfn.rs` ✔. |
| 4.1 | Names are semantic keys; numeric ids only in the diagram part | **partly** | `Model` is still one flat structure: positions and ids sit on variables. The meaning/layout split exists only as a *file* split (`io::write_split`, `read_split`), keyed by names. |
| 4.1 | Renames are an operation, logged, shown as a rename in diffs | **partly** | `rename_var` rewrites equations and checks; the semantic diff detects a rename by comparing the two models (one removed, one added, identical otherwise), not from the log. |
| 4.2 | `.sdm` v2 shape (`semantic`, `diagram`, `tests`, `passthrough`) | **not done** | `.sdm` is the old flat JSON. New fields since: `checks`, `defs`, `instances`, `functions`. No version field. |
| 4.3 | Module definitions and instances with typed ports | **done** | Ports declared (in/out, material/information, units). Definitions cannot carry their own tests. |
| 5.1 | Operation names: `add_stock`, `add_flow`, `add_aux`, `add_constant`, `add_lookup`, `remove`, `set_range`, `set_initial`, `add_module`, `add_port`, `move_to_module` | **differs** | One `add_var` with a `kind`; `remove_var`; a stock's start value is `set_equation`; no `set_range`; modules are `define_module` / `instantiate` (a module is a definition, not a group on the canvas). Full list: [API-Reference](API-Reference.md). |
| 5.1 | `auto_layout`, `tidy`, `problems`, `optimize`, `calibrate`, `png`, `xlsx` | **partly** | `validate` covers `problems`; `optimize` is now in `dyn` and MCP (R24); layout, png and xlsx are in the app only; no `calibrate`. New variables get a grid position, nothing smarter. |
| 5.2 | Transactions: atomic, committed or rejected | **done** ✔ | A rejected batch changes nothing and says why. |
| 5.3 | Log entry with `revision`, `reason`, `result`, `problems`, **rejections logged** | **fixed** (R24) ✔ | Entries have `seq`, `revision`, `result` (committed or rejected), `reason`, `problems`, `actor`, `at`, `ops`, `warnings`. Rejected batches are logged with their problems (CLI, MCP and the app). `dyn ops --reason`, and `reason` on the MCP `apply_ops`. Old log lines still read. |
| 6 | Unit algebra over declared base units; flow units × time = stock units | **partly** (R24) | Algebra is built; `units.base` is not declared (base units are read from the text). By default a rate may meet its amount, which keeps the fuel model clean; **`dyn validate --strict-units`** (and `strict_units` on the MCP tools) applies the Specs rule: a `Unit/Month` flow into a `Unit` stock with model time in Days is then reported. |
| 6 | Unknown or free-text units produce a warning | **partly** (R24) | Units that cannot be read (brackets, symbols) now give a warning. A variable with no units is still skipped without comment. |
| 6 | Model test kinds `nonneg`, `bounds`, `reference`, `conservation` | **fixed** (R24) | `add_check` takes `kind`: `bounds`, `nonneg`, `reference` (a tolerance band round a reference series; one point means flat) and `balance` (chosen stocks together change only by the boundary flows: a chain-level conservation test). Per-stock conservation stays automatic. All four are in the Checks sheet and survive native and XMILE files. |
| 6 | `nonneg` as a test | **fixed** (R24) | The `nonneg` check fails when a variable goes below zero. The stock flag `non_negative` is still a clamp (the engine holds it at zero) and still switches the per-stock conservation check off; the two are different things and the docs say so. |
| 6 | Generation gate for *agent actors* | **differs** | The gate is a session setting, on for every actor by default; `--no-gate` turns it off. |
| 7.2 | Mapping table filled | **partly** | [Vensim](Format-Mapping-Vensim.md), [XMILE](Format-Mapping-XMILE.md) and [Vensim and Stella side by side](Vensim-Stella-Reference.md) exist; name canonicalisation is not written down as one rule. |
| 7.3 | Loss report: `source`, `target`, `items[{construct, where, status}]`; statuses `exact`, `approximated`, `flattened`, `preserved_not_simulable`, `dropped` | **partly** (R24) | Reports have `source` and `target`; items have `kind` (`Dropped`, `Approximated`, `Flattened`, `PreservedNotSimulable`, `Preserved` for a note that comes back), `construct`, `detail`, and an optional `location`. Sets and submodels are reported as flattened; vendor-only functions as preserved-not-simulable. **The adapters do not fill `location` yet**, and there is no `exact` status (exact constructs are not listed). |
| 8 | CLI: convert, validate, test, run, diff, layout, export | **partly** | Also: info, list, get, deps, loops, sens, optimize, explain, manifest, ops, docs, mcp. No `layout`, `export`, `publish`. |
| 8 | MCP: one tool per group, `transaction`, `explain_problems` | **partly** (R24) | 20 tools; `apply_ops` is the transaction; `explain_problems` gives every issue with a hint on how to fix it; also `draft_manifest`, `optimize`, `sensitivity`, `describe_api`. |
| 8 | Python returns pandas DataFrames | **not done** | Returns dictionaries from JSON. |
| 9 | Manifest (`levers`, `outputs`, `scenarios`, `bindings`, `validity`, `tests`) | **partly** (R24) | A `manifest` field in the model, set with the `set_manifest` operation, checked by `validate` (code `manifest`: names the model lacks, ranges the wrong way round, scenarios outside a lever's range, tests that are not checks), drafted by `dyn manifest --draft`, and carried in native and XMILE files (reported as dropped in `.mdl`). `dyn run --set` warns outside a lever's declared range. **Nothing generates an app from it, nothing reads `bindings`, and the app has no screen for it.** |
| 10.1 | Round trip on SDXorg subset + fuel model | **partly** | The fuel model survives sdm → mdl → xmile → sdm with identical results (1,524 series) ✔. The XMILE reader was run on 67 SDXorg files: 40 match the expected outputs. `.mdl` was not run on SDXorg. |
| 10.3 | Manual check in Vensim or Stella | **not done** | Nobody has opened a written file in either. |
| 10.5 | Agent builds, validates, runs, exports to all three formats through MCP | **done** ✔ | Built a stock-management model, validated, ran, exported to `.mdl` and XMILE with empty loss reports. |
| 10.6 | App uses only the core API | **partly** | Engine moved; most screens still edit the model directly. *Run operations*, *History* and the conversion windows use the new layer. |
| 11 | Stage order (0 first) | **differs** | Core split was done before the mapping table, and **the Simlin spike (D5) was not done**: its licence, coverage and maintenance remain unchecked (open question 3). The mapping tables came later, from the format work. |

## 3. Vision v0.1, claim by claim

| Claim | State |
|---|---|
| Unit algebra, conservation and tests stored in every model | **partly**: algebra and conservation yes; tests are bounds checks only. |
| Native JSON plus full read and write of XMILE and `.mdl`, with loss reports | **partly**: both work and report; the XMILE writer is best-effort and flattens sets; neither was opened in the vendors' tools. |
| Agents edit only through validated, logged operations (MCP) | **done** for the MCP and `dyn ops`; the desktop app still edits directly. |
| Components with typed ports and *their own tests* | **partly**: typed ports yes; a definition cannot carry tests. |
| One model deployed across many sites through sets | **done** (sets with one parent). Several parents: not built. |
| Data bindings, forecasters as inputs, calibration with intervals | **not done**. *Compare with data* scores a model against four simple guesses; ETS from `augurs`, quantile paths and interval calibration do not exist. |
| A manifest from which an end-user app is generated | **not done**. |
| Fuel model reproduces the workbook (49 tanks) | **done** (the test matches optimal trucks, target-day fill and shortage for the 49 ready tanks). |
| Achievement 2 needs actuals the fuel model lacks | **correct**: the workbook holds none. |

Statements that are opinion or hypothesis, and marked so in the text or should be: "no tool combines validated logged transactions, enforced tests and units, typed composition, loss-reported round trips and an agent interface" (needs the prior-art check the Vision itself asks for); "SD expected to win on predicting the response to a policy change" (a hypothesis; B1 tests part of it); "hours, not weeks" (no measurement protocol is written down).

## 4. What this means for benchmark B1

| B1 needs | Available? |
|---|---|
| `manifest.json` per the Specs | **Yes (R24)**: the model carries a manifest (`set_manifest`, `dyn manifest --draft`). It can be exported with `dyn manifest FILE`. Nothing consumes `bindings` yet. |
| T1 chain-level conservation | **Yes (R24)**: a `balance` check over several stocks and the boundary flows. |
| T2 flat trajectories, T3 reference behaviour | **Yes (R24)**: a `reference` check (one point = flat). A *qualitative* pattern such as "amplification upstream" still needs a derived variable and a bound. |
| T4 bullwhip ratio close to 1 | Computable inside the model (running sums), then tested with a bound. |
| Units in cases, cases/week, currency/week | Yes; use `--strict-units` so a rate cannot meet an amount. |
| Optimize tunes P2 over 70 series | **Yes (R24)**: `dyn optimize` (random sampling, then Nelder-Mead), one model per call. Looping over 70 series is a script. |
| Per-element delays across an `Echelon` set | Not tested; delays take a per-item constant in principle. |
| Quantile-path input, external policy replay | **No** quantile type; replay is possible only as data lookups. |
| Python bindings for the DQN | A subprocess wrapper only. |
| Fixed seed Monte Carlo | Yes. |

## 5. What is still open

**Code:** the gate applies to every actor, not only agents (kept, stricter than the Specs); `.sdm` v2 shape and a true semantic/diagram split in the model type; a registry passed explicitly instead of a thread-local; the crate layout of §3; `dyn layout`, `dyn export`, `set_range`, `set_initial` names; pandas output; `location` filled by the adapters; a Vensim or Stella check by a person with a licence; the Simlin spike.

**Text:** Specs §3 and §11 (crate layout and stage order as built); §5.1 operation names; §6 (the gate, the base-unit declaration); §7.3 (statuses as built); Vision §4 and §10 (what is claimed as built); mark the Simlin spike as not done.


## B3: find what matters (20 November 2026)

| Item | Status | Where |
|---|---|---|
| Goals in the manifest | **done** (not shown in the app) | [Find-What-Matters](Find-What-Matters.md) |
| F1 `screen` (Morris, convergence) | **done**, agrees with SALib to 1e-9 on identical designs | `dyn screen` |
| F2 `importance` (Sobol, Loops That Matter through SMOOTH and DELAY1/3, eigenvalue elasticity, removable variables) | **done**; loop scores Euler only, other stateful functions unscored, no strongest-path search | `dyn importance`, `dyn dominance` |
| F3 `reduce` | **partly**: prune, fix, merge; no delay aggregation, no loop removal | `dyn reduce` |
| F4 `verify_reduction` | **done** | `dyn verify` |
| F5 `recommend_structures` | **partly**: five structures, roles named by the caller | `dyn recommend` |
| Benchmark | two cases scored (B2-1 7 of 7; B2-2 3 of 4 in each scenario), one run without a key (World3-03), two not run | [report](../benchmarks/b3/B3_report.md) |

Proposed specs changes: [SPECS-CHANGES.md](../benchmarks/b3/SPECS-CHANGES.md).
