# Architecture

> From R22 the engine is a separate crate, `core/` (`dynamist-core`): model, expr, sim, sets, units, checks, validate, ops, diff, io, runner, modules, cld, sensitivity, mdl. The app (`src/`) and the command line (`cli/`) use it. Files named `*_ui.rs` hold the windows for engine features. See [Model API](Model-API.md). (for developers)

Rust 2024, [eframe/egui 0.37](https://github.com/emilk/egui) for the window, `egui_plot` for charts. Build with `cargo build`; tests with `cargo test`.

## Source map (`app/src/`)

| File | Responsibility |
|---|---|
| `main.rs` | Start-up, command-line options (`--screenshot`, `--sheet N`, `--write-docs`, `--import-sheet`, demo options) |
| `app.rs` | The `App` state, top bar and menus, tool strip, status bar, keyboard shortcuts, undo/redo, simulate + scenarios, run playback |
| `model.rs` | The saved data: variables, links, notes, settings, sheets, groups, modules, scenarios, chart styles, the example model |
| `expr.rs` | Equation language: tokenizer, parser, built-in function list, renaming, lookup interpolation |
| `sim.rs` | Compiles a model (dependency order, stateful delay/smooth nodes) and simulates it (Euler / RK4) |
| `canvas.rs` | The stock-and-flow diagram: drawing, mouse editing, modules on the canvas, overlays (time slider, zoom, mini-map, arrange bar) |
| `arrange.rs` | Multi-selection, copy/paste, align/distribute, snapping, groups, modules (make, dissolve, ports) |
| `inspector.rs` | Right panel: variable editor, autocomplete, argument hints, lookup editor, selection/group/module panels |
| `results.rs` | Bottom panel: trends, table, causes & uses, problems, runs & scenarios |
| `charts.rs` | The one chart renderer (`chart_styled`): every chart type, styles, legends, captions |
| `chartstyle.rs` | The *Customize chart* window |
| `sheets.rs` | Sheet tabs, table sheets, dashboards, sheet exports |
| `cld.rs` | Causal loop sheets: drawing, editing, loop finding, building from the model |
| `optimize.rs` | Optimization sheet and the background search (sampling + Nelder–Mead) |
| `scenarios.rs` | Scenarios sheet: table, comparisons, key numbers |
| `export.rs` | Pictures: off-screen drawing, CPU rasterizer with vector text, PNG with dpi, clipboard (macOS pasteboard / arboard), chart references, copy-data |
| `spreadsheet.rs` | Spreadsheet template: read (calamine), model ⇄ workbook, write (rust_xlsxwriter), generated docs |
| `modfile.rs` | `.flowmodule` export/import and the input-mapping dialog |
| `docs.rs` | Function documentation and the Function reference window |
| `palette.rs` | Command palette, shortcut and About windows |
| `icons.rs` | Vector icon set, icon buttons, menu items |
| `theme.rs` | Colours, visuals, diagram shapes (valves, clouds, arcs), sparklines |
| `sets.rs` | Sets: before a run, expands per-set variables into plain copies (`Stock[item]`), resolves `TOTAL`/`AVERAGE`/`[item]`, and lists the roll-ups (one number per set variable) |
| `mdl.rs` | Import of classic `.mdl` text models |

## Principles

- **Every edit re-simulates** (when *Live* is on); models are small, a run takes well under a millisecond.
- **Undo** = a snapshot of the whole model before each change (`App::checkpoint`).
- **One chart renderer** for every chart; it must fill exactly the height it is given (egui panels shrink to their contents).
- **Exports redraw** the sheet off-screen in the light theme and rasterize on the CPU, so pictures are independent of the window and its size.

## Tests

`cargo test` runs the parser, simulator (delays, smoothing, the example), `.mdl` import, optimizer, copy/paste/modules/groups, module files, scenarios, chart exports, rasterizer, autocomplete, function docs, and spreadsheet round trips. `cargo test picture_of_autocomplete -- --ignored` renders the suggestion list to `/tmp/fc_autocomplete.png`.

## Regenerating generated docs

`app/target/debug/dynamist --write-docs app/wiki` rewrites `Function-Reference.md` and `templates/`.
