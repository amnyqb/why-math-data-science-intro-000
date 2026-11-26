# User guide

Each section matches a part of the app. Shortcuts are listed in [Keyboard shortcuts](Keyboard-Shortcuts.md).

## The window

- **Title row and ribbon** — the title row has **File** (open, save, import, export, pictures) and **Help** menus, the six tabs, quick actions (undo, redo, save, inbox, command palette, light/dark, panel toggles), the model's title, **Live**, the pin (keep this run) and **Run**. Under it the **ribbon** shows the buttons of the chosen tab in labelled groups: **Home** (clipboard, edit, run, simulation settings, new sheet), **Insert** (stock, flow, variable, constant, lookup, arrow, note, modules; on a causal loop sheet its own tools), **Arrange** (align, group, module, tidy up, fit, snap), **Analyze** (results panels, sensitivity, compare with data, optimise, checks, history, functions) and **View** (zoom, panels, appearance, look and arrows). Insert has **Controls** (slider, knob, switch, number input: click to put one down, tied to the selected constant, then operate it on the diagram; a **Control** tab sets the constant, label and range) and **Shapes** (rectangle, rounded box, ellipse, line, arrow: drag to draw) and **Picture** (PNG or JPEG, kept in the model file). While a shape is selected a **Shape** tab appears (colours, line width, size). While a note is selected a **Text** tab appears (size, bold, italic, underline, fixed-width, colours, alignment). Double-click a tab or press the ˄ at the right to hide the ribbon; click a tab to bring it back. ⌘K still finds every command.
- **Sheet tabs** — every sheet of the model; **+** adds a sheet; right-click a tab to rename, duplicate, move or delete it. On the right: **Copy as picture**, **Save PNG**, picture quality (2×, 3×, 4×).
- **Status bar** — whether the run succeeded and how long it took, messages, counts, zoom.

## Diagram sheets (stock-and-flow)

- **Tools** (left strip): select, stock, flow, variable, constant, lookup, arrow, note, delete.
- **Flows**: drag from a stock to another stock or to empty space (a cloud = outside the model). Drag a cloud onto a stock to connect it.
- **Arrows**: drag from a cause to the variable that uses it. They are curved; select one and drag its round handle to bend it. Right-click for polarity (+/−), delay mark, dashed, straighten. The right panel sets weight and colour.
- **Selecting**: click; shift-click to add; drag a box on empty space; ⌘A for all. With two or more selected, a bar appears with **align** (left, centre, right, top, middle, bottom), **distribute**, **group**, **make module**, **export module**, copy, copy picture, delete.
- **Groups** (⌘G) keep items together under a labelled frame. Click once selects the group; click again selects one member.
- **Modules** (⇧⌘M) wrap a selection into a submodel shown as one box with its **inputs** (left, green) and **outputs** (right, orange). Double-click opens its own sheet, where inputs appear as dashed boxes on the left. Modules can be dissolved, exported to a `.flowmodule` file and imported into other models (File → Import module…), where a dialog connects each input to an existing variable or a new constant.
- **Turning points**: an arrow can pass through several turning points and is drawn as one smooth path through them (no curve handles). Double-click an arrow to add a point there; drag a point to move it; drag the small circle between two points to pull out a new one; right-click a point to remove it (or *Remove all turning points*). The arrow's panel has *Turning points → + Add one*.
- **Diagram look**: *View → Diagram look* — *Cards* (coloured cards) or *Classic* (plain labels with a small marker and thin dark arrows, like a textbook diagram; the default for big imported models).
- **Arrows**: *View → Arrows* sets how every arrow is drawn — curved, straight or right-angle (like a circuit diagram) — and the tip (filled, open or dot). Select one arrow to give it its own route and tip. In a crowded diagram arrows stay quiet until you click a variable.
- **Tidy up**: *Arrange → Tidy up this diagram* puts inputs on top, stocks and flows in the middle and the calculations below in columns by distance from the inputs (⌘Z undoes it).
- **Moving around**: scroll to pan, pinch or ⌘-scroll to zoom, space-drag to pan, **0** to fit; the mini-map appears when part of the sheet is off-screen.
- **Time slider** (bottom left of the diagram): show the values at any moment, or press play.
- **Copy & paste**: ⌘C copies the items *and* a transparent picture (for Office); ⌘V pastes onto any diagram sheet (names are made unique and equations follow), or onto a causal loop sheet as words.

## Editing a variable (right panel)

- **Name** (renaming updates every equation that uses it), **type** (variable / constant / lookup), **equation** with:
  - suggestions while typing (↑↓, Tab or Enter to insert, Esc to close);
  - a hint showing the function you're inside and the argument you're typing;
  - **ƒ Functions…** / **F1** — the full [function reference](Function-Reference.md) with an Insert button;
  - inputs from arrows as clickable chips; a warning for inputs without arrows;
  - a live check: "✓ Looks good · now 4.73 People" or the problem.
- **Constants**: a slider and its range. **Stocks**: what changes them (inflows − outflows), "never below zero". **Flows**: which stocks they drain and fill. **Lookups**: drag points on the curve, double-click to add, right-click to remove, or edit numbers.
- **Behaviour over time**: a small graph of this variable, with scenarios and kept runs.

## Levels: modules

**Fold and unfold:** the chevron at the right of a module card's header folds it to a short card (name, purpose, size) or unfolds it to show its inputs and outputs; *double-click* opens the diagram inside. *View → Fold all / Unfold all* does every module. So there are three depths: folded card → inputs and outputs → the inside.

**Key levers:** tick *Lever* under a constant's Details to mark it as one of the few controls that drive the model. When any are marked, the side panel shows only them at the top (*Key levers*), with every other constant folded under *All other constants*. On a sheet of modules the levers are drawn as **circles outside the boxes** (name above, value inside), with an arrow to each module that uses them; click a circle to adjust it in the side panel. On other sheets they sit in a **Key levers card on the diagram**, so you can play with the model from the overview; fold it with its chevron. A lever that is per tank gets, with nothing picked, an **all-items slider** (× 0.5 to × 1.5) that nudges every tank at once, and once you pick a tank in the *Showing* bar, a slider for that tank's own value. In a spreadsheet, a *Lever* column (yes) on Parameters does it.

A big model is organised in levels. The **Model** sheet can be an overview of a few **module** boxes, each showing its inputs (left, green) and outputs (right, orange with the current value) and the few arrows between modules. **Double-click** a module, or use **Modules ▾** next to the tabs, to open the diagram inside it; **◂ <sheet name>** at the left of the tabs goes up one level. Modules can hold modules, to any depth (module of modules of modules): select modules and make a module of them, or give a spreadsheet's *Module* column a path like `Plans / Optimal plan`. The tabs show the path you are in (**Model › Plans ›**) — click a name to go up to that level; ports are traced through every level, so an outer module's inputs and outputs are what really cross its edge. A module box lists its **inputs on the left** (green rings) and its **outputs on the right** (orange dots, with the current value), has a one-line purpose, and a role by colour: green *inputs*, purple *mechanism*, orange *results*. **View → Modules → Compact** hides the ports and labels each arrow between modules with what passes along it; *Detailed* (the default) shows them. To make one by hand, select variables and use *Arrange → Make a module from selection* (⇧⌘M). A spreadsheet's *Module* column builds them on import.

## Sets of similar things

The **Showing** bar above the diagram has a dropdown per set: pick one tank (or region) and numbers, graphs and sliders are for it; *All* shows totals.

A variable can be *one for each* item of a set (tanks, regions, products): select it, *Details → One for each*. It is drawn as a stack of cards with a badge (*Tank ×57*), shows the total (Σ) or average, and has a *Differs by …* list for item-specific values. See [Sets](Sets.md).

## Showing and hiding panels

Two layout icons sit in the top bar (after the light/dark icon): **results panel (below)** and **side panel (right)**. Each is highlighted while its panel is shown; click to hide or show. The panels also have a chevron at their corner (⌄ results, › side) and, when hidden, a slim strip with a chevron to bring them back. Shortcuts: ⌘J and ⌘B.

## Side panel (right)

Hide it with **Hide ⟩** (top right of the panel), **⌘B** or *View → Side panel*. A slim strip with **⟨** stays at the right edge to bring it back. It covers the details of the selection (diagram and table sheets) and the loop list (causal loop sheets).

## Outline, sensitivity, units

- **View → Outline** shows the model as a tree by module (folders open and close). Click a name to go to it, tick the box to plot it; levers are in italics.
- **Edit → Compare with data…** asks whether the model beats a simple guess. Choose a result and a data series, and a time to hold the data out after; the model is scored on the held-out points against "nothing changes", a straight-line trend, a smoothed trend (exponential smoothing, damped) and (if you give a season length) the same time one season earlier. To fit levers to data first, add an aux for the squared error and minimise its average on an Optimize sheet.
- **Edit → Model checks…** lets the model say what "behaves sensibly" means: a variable stays between two limits, at all times or at the end of the run. A broken limit is listed in the Problems tab. Every stock is also checked automatically to change by exactly what flowed in and out (stocks marked *never negative* or averaged are left out).
- **Edit → Sensitivity runs…** varies the key levers at random (each ± a percentage; all numeric constants if none are marked), runs the model many times in the background and draws a band for one variable: the middle, the 25–75% and the 5–95% of runs, against the run as it is set now. A per-tank lever moves all tanks together. The draws use a fixed seed, so a run can be repeated.
- **Units** are checked as algebra. A unit like `Truck/Day`, `Barrels per Truck` or `1/Day` is read into base units with exponents; sums and comparisons need equal dimensions, products and quotients combine them, and a variable's declared units must be those of its equation. A flow's units per time must be its stock's. The *Problems* tab lists what does not fit. A rate may meet its amount (MB and MB/Day); the scale of a unit (MB, barrel) is not tracked. Unknown units are never reported.
- **Layout** (panels, dark mode) is remembered between sessions.

## Other formats, history and operations (File and Edit menus)

- **File → Import Vensim or Stella model…** reads Vensim `.mdl` and XMILE (`.xmile`, `.stmx`); **File → Export as Vensim model…** and **Export as XMILE model…** write them. After each, a **conversion report** lists what was dropped, approximated or kept as a note. XMILE cannot hold sets with parents, so they come back flattened. Nothing was tried in Vensim or Stella themselves.
- **Edit → Reusable modules…** defines a module once, with named ports (in or out, material or information, with units), and uses it as many times as you like: each instance has its own parameter values and its own inputs. Instance variables are called `Instance.Variable`; each output port has a **plot** button, and any equation can read one as `"Instance.Variable"`. Instances have no box on the diagram.
- **Edit → History…** lists what each undo step of this session changed, in words (a rename is one change; moving things on the diagram is not a change of meaning). It also lists operation batches run from the window below.
- **File → Compare with a saved model…** shows what differs in meaning between a file (any format) and the model as it is now, and how many result series move.
- **Edit → Run operations…** takes a batch of operations (a JSON list). It is applied to a copy, checked, and only then becomes the model; a refused batch changes nothing and says why. *Copy the operations reference* puts the full language on the clipboard (also in [Operations language reference](API-Reference.md)).
- In the **Function reference** (F1) each function shows what it is called in Vensim and in Stella, and where the same name means something different. See [Vensim and Stella side by side](Vensim-Stella-Reference.md).

## Modules you can reuse, and scripting

A module can be defined once with declared **ports** and used several times as **instances**, each with its own parameter values and inputs (Edit → Reusable modules…, the command line and the operations language). The engine, validation, operations, the `dyn` command line, the MCP server for AI agents and the Python wrapper are described in [Model API](Model-API.md).

## Results panel (bottom of diagram sheets)

Hide it with **⌄ Hide** (top right of the panel), **⌘J** or *View → Results panel*; the diagram gets the room. A slim bar stays at the bottom: **⌃ Show results** or click a tab name (Trends, Table, Causes & uses, Problems, Runs & scenarios) to bring it back at that tab. Problems shows its count in amber even while hidden.

- **Trends**: chosen variables over time; chart type (line, area, stacked, step, bars, phase, donut); *Split* for one chart per variable; **Replay**; customize (sliders icon); copy picture.
- **Table**: values over time. **Causes & uses**: what drives the selected variable and what it affects, as trees. **Problems**: equation errors and suggestions (unused arrows, missing arrows, stocks with no flows). **Runs & scenarios**: kept runs and scenario toggles.

## Charts

Every chart can be **right-clicked**: *Copy chart as picture*, *Save chart as PNG…*, *Copy chart data (for Excel)*, and — for the Trends graph and dashboard graphs — *Customize chart…*: type, variables, title, subtitle, axis titles and ranges, legend position, grid, line width, point markers, colours, and whether scenarios and kept runs are drawn. Legends say what each line is: `Infected (current model)`, `Scenario: Masks`, `Kept run: …`; a caption under the chart explains solid and dashed lines.

## Table sheets

- **Equations**: every variable as a row; edit equations, units and notes in the cells; filter by text.
- **Results over time**: values at every N steps; pick columns; **Copy for Excel**; export CSV.

## Dashboards

Cards on a four-column grid: **number** cards (value, change since start, sparkline), **graphs** (any chart type, customizable), **sliders** for constants, **text**. Turn on *Edit layout* to add, configure (⚙), resize, reorder or remove cards.

## Scenarios sheet

A table of named what-ifs: each scenario changes some values (empty cell = baseline; a cell can hold a formula). All scenarios re-run with every change. **Compare** shows chosen variables over time (one colour per scenario), a bar chart of a summary measure (final, peak, lowest, total, average) and a key-numbers table with % change against the baseline. *Show scenarios in every chart* adds them to all other charts.

## Causal loop sheets

Words joined by curved +/− arrows, delay marks and **R/B loop markers**. Tools: select, word (T), arrow, loop marker. The side panel lists **every feedback loop found automatically**, labelled reinforcing or balancing; click one to highlight it, then *Add a marker*. **Build from model** turns the stock-and-flow model into a causal loop diagram.

## Optimize sheet

Four steps: what to make small or large (peak, final, total… of a variable); which constants may change and within what range; limits that must hold; effort. **Optimize** searches in the background and shows the best settings, the limits check, a before/after graph and the search progress. *Use these values in the model* applies them.

## Pictures for Office

Any sheet (⇧⌘C), selection (⌘C on a diagram), chart (right-click) or dashboard card can be copied or saved as a **transparent, high-resolution PNG** (2× to 4×, with the resolution recorded so Word places it at the right size).

## Spreadsheets

- File → **Import model from spreadsheet…** — builds a model from the [template](Spreadsheet-Template.md) and shows a report of anything skipped.
- File → **Export model to spreadsheet…** — the current model in the same template (a good way to review a model in Excel, or to hand it to someone).
- File → **Save blank spreadsheet template…**

## Help

Help → **Documentation (wiki)** opens this folder; **Function reference** (F1); **Command palette** (⌘K: type any command, sheet or variable name); **Keyboard shortcuts** (?).
