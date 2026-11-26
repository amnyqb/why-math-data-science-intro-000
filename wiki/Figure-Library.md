# Figure library: pictures from the command line

Book figures (stock and flow diagrams, causal loop diagrams, charts) are made **without the app and without a display**: `dyn` runs the same engine and the same drawing code as the window and writes an SVG (or a PNG). A request from the book team started it: [`book/FIGURE-LIBRARY-REQUEST.md`](../book/FIGURE-LIBRARY-REQUEST.md). The one-page version for an assistant is [`book/FIGURE-LIBRARY.md`](../book/FIGURE-LIBRARY.md).

## How it works, and why this way

The drawing rules (pipes, valves, clouds, curved arrows, signs, delay bars, R/B markers, cards, charts) live in `src/canvas.rs`, `src/cld.rs` and `src/charts.rs`, written against egui painters, and the app's PNG export (`src/export.rs`) rasterises the shapes egui painted. Two ways to get the same picture without a window were considered:

| | Run egui's `Context` with no window (chosen) | Move the geometry into `core/` with its own SVG backend |
|---|---|---|
| Same picture as the app | Yes by construction: the shapes are the ones the PNG export rasterises | Only as long as the second copy is kept in step with 3 000 lines that change often |
| Cost | The egui crates (pure Rust) in the headless build | A rewrite and a second place to fix every drawing bug |
| Display, GUI crates | None: `egui::Context` needs no backend; `eframe`, `winit`, `rfd` and `arboard` are not linked | None |

So the crate `dynamist` (the app) is now a library plus a binary. A Cargo feature `gui` (on by default) holds `eframe`, the file dialogs and the clipboard; `default-features = false` is the **headless build**. `src/headless.rs` creates the `App`, feeds an `egui::Context` a fake `RawInput` for a few frames, takes the painted shapes with `App::render_shapes` (the function the app's own PNG export uses), and `src/svg.rs` turns them into SVG. egui's pure geometry crates are therefore used; no display is. The `dyn` binary links the headless build: `cargo tree -p dynamist-cli` has no `eframe`, `winit`, `rfd` or `arboard`.

What stays the same as the app: card sizes, text measuring (egui's own fonts: Ubuntu Light and Hack), pipe and cloud geometry, arrow routing, chart axes. Anti-aliasing is the viewer's. Two choices of the library: the app is drawn as on a retina screen (2 pixels per point, which sets how text is spaced), and the picture is cropped to what is drawn plus 10 points, as the app's PNG export does.

## Build

```bash
cargo build --release -p dynamist-cli      # target/release/dyn ; Linux x86_64, no display, no GUI libraries
cargo test  --release -p dynamist-cli --test figures
```

`cargo build --release -p dynamist` (default features) still builds the window. Dependencies are pinned in `Cargo.lock`; nothing needs the network at run time. For heavy builds use the Contabo server (`CONTABO-SERVER.md`).

## Commands

All print JSON, like the other `dyn` commands. `--sheet` takes the sheet's name (case and spacing do not matter); without it, the first diagram (or, for `layout` and `set`, the first causal loop sheet where that is what the command needs).

```
dyn render FILE [--sheet NAME] --out fig.svg [--png [--scale 3]] [--with-caption] [--style FILE]
dyn chart  FILE --vars A,B --out chart.svg [--from T0] [--to T1] [--kind line|area|step|stacked|bars]
              [--title T] [--y-label L] [--scenarios A,B|all] [--set Name=Value]… [--png] [--scale N] [--style FILE]
dyn layout FILE [--sheet NAME] [--mode tidy|loops] [--loops R1,B2] [--out OUT]
dyn cld    FILE --from-model [--loops R1,B2] --out cld.sdm [--sheet-name NAME] [--layout keep|loops]
dyn cld    FILE --list
dyn set    FILE [--sheet NAME] [--out OUT] --pos VAR X Y | --polarity A B + | --delay A B [off]
              | --loop-name B1 "Inventory control" | --caption TEXT | --source TEXT      (any number, in one call)
dyn style  FILE --apply book.style.json | --show [--out OUT]
```

- **render** draws one stock and flow sheet or causal loop sheet. `--out x.svg` writes the SVG; with `--png` also `x.png` (`--scale` 0.5 to 8, default 3); `--out x.png` writes only the PNG. The PNG is the app's own rasteriser (transparent background); font, `text_scale` and `line_scale` of a style apply to the SVG only, so for a styled PNG convert the SVG: `rsvg-convert -z 4 fig.svg -o fig.png`. `--with-caption` writes the sheet's `caption` under the figure and its `source` below that as "Source: …"; set them with `dyn set --caption … --source …` (they are saved with the sheet).
- **chart** draws variables over time as the app's charts do. `--from`/`--to` show a time window (the vertical axis fits the window); `--scenarios` adds the model's scenarios (names, or `all`); `--set` changes a value for this picture only.
- **layout** writes positions back to the file, which must be a `.sdm` (for a `.mdl` or XMILE file, `dyn convert` first or give `--out NAME.sdm`). `tidy` on a stock and flow sheet is the app's *Tidy up* (inputs on top, stocks and flows in the middle, calculations below). `loops` (and `tidy` on a causal loop sheet) draws each loop as a circle with its words evenly spaced, clockwise in the direction of the arrows, the marker in the middle; `--loops` picks some; a word in two loops stays where the first loop put it; words in no loop go in a row underneath.
- **cld** `--list` names the model's feedback loops the way the app's side panel does: `R1`, `R2`… reinforcing and `B1`… balancing, in the order the loop finder gives them (shortest first), each with the words around it. `--from-model` builds a causal loop sheet ("Causal loops", or `--sheet-name`) from the model keeping only the named loops (all if `--loops` is left out), puts a marker on each, and writes the **whole model with the new sheet** to `--out`. Arrows without a sign in the model count as `+`; set signs on the model's arrows first (`dyn set FILE --polarity A B -`) so the loops are named correctly. `--layout loops` lays the new sheet out at once.
- **set** edits one figure. `--pos` moves a variable (a flow's clouds go with it) or, with `--sheet` naming a causal loop sheet, a word. `--polarity` and `--delay` change an arrow A → B. `--loop-name B1 "Inventory control"` makes the marker read `B1 Inventory control`; the same id works again later.
- **style** stores a style file in the model (see below) and applies the parts the model carries; `--show` prints what is stored.

Everything is deterministic: the same file and style give the same bytes (the tests render twice and compare). Nothing is read from the machine: no preferences file, no clock, no installed font.

## The style file

One JSON file; every key is optional and unknown keys are refused (a typo is an error, not silence). `dyn style FILE --apply book.style.json` keeps it in the model (`figure_style`), so later `render` and `chart` calls need no flags; `--style FILE` on a render adds to or overrides the stored one for that call. A sample is [`book/examples/book.style.json`](../book/examples/book.style.json).

| Key | Meaning |
|---|---|
| `font` | SVG font family of ordinary text, e.g. `"Georgia"`. The text is real SVG `<text>`; every letter has its own position from egui's layout, so a viewer with a different font keeps the same spacing. The boxes are measured with egui's font, so choose one of similar width. |
| `mono_font` | The same for monospace text (values on cards). |
| `text_scale` | Multiplies every font size (the boxes keep theirs). Default 1. |
| `line_scale` | Multiplies every line width: arrows, outlines, pipes, chart lines. Default 1. |
| `width_mm` | Width of the figure in millimetres; the height follows. Default: points. |
| `background` | `"#rrggbb"` behind the figure; default transparent. |
| `colors` | Palette colours by name, each `"#rrggbb"`: `text muted accent accent_soft stock stock_fill flow aux constant lookup link error ok panel surface border grid`. In a causal loop diagram `ok` and `error` are the + and − signs, `aux` the R markers and `accent` the B markers. |
| `arrow_shape` | `curved`, `straight` or `elbow`: how arrows are routed (set on the model). |
| `arrow_head` | `filled`, `open` or `dot` (set on the model). |
| `arrow_weight` | 0 thin, 1 normal, 2 bold: set on every arrow of the model. |
| `diagram_style` | `cards` (the app's look) or `classic` (plain labels, thin dark arrows). |
| `caption_size` | Font size of the caption (default 13); the source line is 2 smaller. |

## Text and what the SVG contains

Text is written as `<text x="x1 x2 …" y=… font-family=… font-size=… xml:space="preserve">`; shapes as `rect`, `circle`, `ellipse`, `line`, `polyline`, `polygon` and `path` (Bézier shapes keep their curves; arrows are the app's own point lists). There are no images and no outlined glyphs. A shape the converter cannot draw (an image or gradient mesh) is counted in the `skipped` field of the command's JSON and left out, never guessed; the sheets and charts of the book draw none.

## Examples and tests

[`book/examples/`](../book/examples) holds chapters 10 to 15: `chNN/model.sdm` (the saved layout, caption and style), `diagram.svg` (the stock and flow sheet), `chart.svg`, and `loops.svg` where the model has feedback loops (11 and 13). `book/examples/build.sh` rebuilds all of them from the `.mdl` files with `dyn` only; it is a worked recipe for the commands above. The test `cli/tests/figures.rs` rebuilds them in a temporary folder and compares with the saved files (byte for byte on Linux x86_64; elsewhere numbers within 0.06), and checks real text, the style, captions, PNG, determinism and the editing commands. `UPDATE_EXPECTED=1` rewrites the saved files.

## Python

```python
import dynamist as dyn
m = dyn.Model("model.sdm")
m.style("book.style.json")
print(m.loop_ids())                                   # [{'id': 'B1', 'reinforcing': False, 'variables': [...]}]
m.cld("cld.sdm", loops=["B1"], layout="loops")
c = dyn.Model("cld.sdm")
c.set(loop_name={"B1": "Inventory control"}, caption="Figure 4.2  …", sheet="Causal loops")
c.render("fig.svg", sheet="Causal loops", with_caption=True)
m.chart(["Stock"], "chart.svg", start=0, end=40)
```

## Not built yet

Lint (unlabelled polarity, unnamed loop, verb-named variable, overlapping labels), a hierarchical causal loop diagram by module level, direct PDF output, hiding the values on the cards, and drawing a module or entities sheet (render says so). See [BACKLOG](BACKLOG.md), B60 to B64.
