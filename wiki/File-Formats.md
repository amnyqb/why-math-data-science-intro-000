# File formats

## `.sdm` — a model (File → Save)

Plain JSON, readable and diff-friendly. Top-level fields:

| Field | Contents |
|---|---|
| `title` | The model's name |
| `settings` | `initial_time`, `final_time`, `time_step`, `time_units`, `method` (`Euler` or `RungeKutta4`) |
| `vars` | Every stock, flow, variable, constant and lookup: `id`, `name`, `kind` (`Stock`, `Flow`, `Aux`, `Constant`, `Lookup`), `equation`, `units`, `comment`, `pos` [x, y], `table` [[x, y]…], `slider` [min, max], `non_negative`, `from`/`to` (stock ids for flows), `from_pt`/`to_pt` (cloud positions), `view` (the diagram sheet it's drawn on) |
| `links` | Arrows: `from`, `to` (variable ids), `bend`, `polarity` (`+`, `-`, empty), `delay_mark`, `weight`, `dashed`, `color`, `shape`, `head`, `via` (turning points `[x, y]`) |
| `notes` | Text notes on diagrams; a note may carry `fmt` (size, bold, italic, underline, mono, color, fill, align) |
| `controls` | Input devices on diagrams: `kind` (slider, knob, switch, number), `var` (the id of the constant), `pos`, optional `label`; the range is the constant's `slider`. In the split format they are in the layout file |
| `shapes`, `images` | Shapes (kind rect, rounded, ellipse, line, arrow, picture; `pos`, `size`, optional `fill`, `stroke`, `weight`, `image`) and the picture files they show (`id`, `name`, `data` base64 of a PNG or JPEG, `w`, `h`). In the split format they are in the layout file |
| `sheets` | Every sheet: `kind` (`Diagram`, `Table`, `Dashboard`, `CausalLoop`, `Optimize`, `Scenarios`) and its contents (dashboard `widgets` with chart `style`, causal loop `cld`, optimize `opt`, scenario comparison settings…), plus `caption` and `source` (written under the figure by `dyn render --with-caption`) |
| `groups`, `modules` | Groups of items; modules (each with its own sheet) |
| `scenarios` | `name`, `enabled`, `color`, `changes` [{`var`, `value`}] |
| `trend_style` | How the Trends graph looks |
| `sets` | Sets: `name`, `items`, `parent` (the set they belong to), `parent_of` (per item), `note`. Variables carry `set` ("one for each"), `by_item` (per-item `equation` / `table`) and `rollup` (`Total`/`Average`) |
| `next_id` | The next free id |

Missing fields take sensible defaults, so older files keep opening.

## `.flowmodule` — a module (Arrange → Export module…)

JSON with `"format": "dynamist-module"`: the module's `name`, `description`, its sheets, variables, internal links, notes, groups and nested modules, plus `inputs` (name, units, a default value and the arrows into the module) and `outputs`. Import with File → Import module…; a dialog connects each input.

## Spreadsheets (`.xlsx`, `.xls`, `.ods`, `.csv`)

The model template: see [Spreadsheet template](Spreadsheet-Template.md). Import, export and blank template are in the File menu. A CSV holds one sheet; its file name is used as the sheet name (e.g. `Parameters.csv`).

## XMILE and Vensim, and what a conversion keeps

The command line reads and writes the native `.sdm`, XMILE and Vensim `.mdl` (see [Model API](Model-API.md)). Every read and write returns a **loss report**: each construct that was left out, carried over in another form, or kept as an opaque block. An empty report means nothing was lost. How each construct maps is in [Format mapping: XMILE](Format-Mapping-XMILE.md) and [Format mapping: Vensim](Format-Mapping-Vensim.md).

`.sdm` can also be written as two files: a **meaning** file (variables, equations, sets, checks; no positions) and a **layout** file (where everything is drawn), so a version-control diff shows model changes and not coordinates that moved.

New in the `.sdm` JSON: `checks` (limits the model states about itself), `defs` and `instances` (reusable modules and their uses), and `functions` (your own functions).

## `.mdl` — classic text models

File → Import classic model reads equations, units, comments, lookups, run settings and, where present, diagram positions and arrows. Arrays (subscripts), macros and data variables are not supported.

## Pictures (`.png`)

Transparent PNG at 2×, 3× or 4× with its resolution recorded (192, 288 or 384 dpi), so Office places it at its intended size.

## Data (`.csv`, clipboard)

Results export as CSV (Time, then one column per variable). *Copy chart data* and *Copy for Excel* put tab-separated text on the clipboard, which pastes straight into spreadsheet cells.
