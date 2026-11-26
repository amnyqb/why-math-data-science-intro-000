# Layers

A large system is not one model. It is a **stack of layers**, each a model in its own right with its own question, horizon and state of completion. Layers talk to each other only through declared variables. Inspired by Kodama Yoichi's layered Japan energy model (1975): the approach, not the content.

Layers are metadata. They never change an equation, so a layered model gives the same results as the same model without layers, and a model without layers behaves exactly as before.

## Concepts

| Concept | Meaning |
|---|---|
| Layer | A named tier, outermost (slow, uncertain) to innermost (fast, detailed). Fields: name, order, question, horizon, structural limit, status, colour, description, owner, spheres. |
| Status | `planned`, `stub`, `built`, `tested`, `calibrated`. |
| Interface | A variable that crosses from one layer to another, written down as a contract: variable, from, to, units, kind (information or material), time treatment. |
| Stub | A layer standing in as formulas for its outgoing variables, so the layers inside it can run alone. |
| Horizon | How far ahead of the start of the run a layer's numbers are trusted. |

A variable's layer is its own setting, else its sheet's, else its submodel's. Material flows: a flow belongs to the layer you give it; if it fills or drains a stock in another layer, that is a material crossing named after the flow.

## Commands

```
dyn layers FILE                                  the stack: status, horizon, counts, interfaces, cross-layer loops
dyn layer add FILE --name N [--order K] [--question Q] [--horizon H] [--limit L] [--status S]
dyn layer set FILE NAME --status built | --horizon H | --question Q | --order K | --limit L
dyn layer assign FILE NAME --vars A,B | --sheet S | --module M
dyn layer check FILE                             exit 1 on errors
dyn layer freeze FILE                            write the current crossings as interface contracts
dyn layer run FILE NAME [--inputs CSV | --stub S | --from-full-run] [--csv OUT]
dyn layer decouple FILE NAME [--max-nrmse 0.05]  the decoupling test (exit 1 if the plumbing fails)
```

Every change goes through the operations language (`add_layer`, `set_layer`, `remove_layer`, `assign_layer`, `freeze_interfaces`, `add_stub`, `remove_stub`; `add_var` also takes `layer`), so it is validated, logged and undoable, and the MCP server offers the same operations plus `layers`, `layer_check` and `layer_decouple`.

## Checking

`dyn layer check` reports:

- `layer-unassigned` (error here; a warning in `dyn validate`): a variable is in no layer.
- `layer-crossing`: a variable passes between layers and no interface declares it. This only applies once interfaces exist: before the first `freeze` the crossings are listed as a warning (`layer-no-contracts`). After `freeze`, `dyn validate` and the ops gate refuse a batch that adds an undeclared crossing.
- `layer-units`, `layer-contract`: a contract disagrees with the variable, or names something that is gone.
- `layer-horizon`: a layer's horizon is longer than that of a built layer that feeds it. Stubs have no horizon; layers that feed each other (a cross-layer loop) get a warning instead, since the loop as a whole is only as far as its shortest member.
- `layer-cycle` (warning): layers feed each other. Cross-layer feedback is allowed once declared; it is listed, not forbidden.

## Running one layer alone

`dyn layer run` cuts a layer out and replaces every variable it reads from other layers with recorded series (from the full run, a CSV with a `time` column, or a stub's formulas). Nothing else changes.

## The decoupling test

`dyn layer decouple` gives two numbers, both as a fraction of each variable's range (worst variable):

- `plumbing_nrmse`: the layer run alone on inputs recorded from the full run, against its behaviour inside the full run. Near zero or the isolation is broken. `ok` is this one.
- `feedback_nrmse`: change each of the layer's own constants by 10%, run the full model, and run the layer alone with the inputs recorded from the *unchanged* run. The gap is how much the other layers react: feedback across the boundary. `decoupled` is false when it exceeds the limit.

A layer that feeds something that feeds it back is not decoupled, and that is information, not an error.

## Horizons

`horizon` is measured from the start of the run; `structural_limit` is the time beyond which only structure remains.

- `dyn run` adds `beyond_horizon` to its output: per layer, when it stops being trusted and its variables.
- `dyn chart` and the app's line charts shade the stretch beyond a layer's horizon and label it "ranges only", and beyond the structural limit "Illustrative structure, not a forecast".
- `dyn optimize` and `dyn recommend` refuse a goal measured beyond the horizon of the layer that owns it, unless `--allow-beyond-horizon` is given.

## File format

Additive. Old files open unchanged and a model without layers writes none of these keys.

```json
"layers": [{"id": 1, "name": "World", "order": 0, "question": "…", "horizon": 60, "structural_limit": 120, "status": "stub", "colour": "#8A9BB0", "spheres": ["market"]}],
"interfaces": [{"var": "Capital Climate", "from": 1, "to": 2, "units": "1", "kind": "information", "time": "as_is"}],
"stubs": [{"layer": 1, "name": "Cold capital", "series": {"Capital Climate": "0.6"}}]
```

`Var`, `Sheet` and `Module` gain an optional `layer`.

## The Architecture sheet

*New sheet → Architecture* draws the model as a stack, from the data in the model:

- **Purpose** on top: the problem, the kind of model (explain a behaviour, design a policy, forecast, learn), what is out of scope, and numbered **objectives**, each with a test in words and the variable that measures it (its KPI). `set_purpose`, `add_objective` and `remove_objective` write it; the *Edit purpose* button does the same from the sheet.
- **Layers** as lanes, outermost first, headed **Layer 1: World** (the number comes from the order; the title is the layer's name, and a layer called "Layer 2" shows just that). A lane is as tall as its content needs and folds to one line when you click its tab. The tab says which objectives the layer *serves* (those whose KPI is one of its variables).
- **Subsystems** inside a lane: each group of variables (`add_group`; the same groups as on the diagram) is a dashed frame, and the frames share the lane's width. Each frame holds a miniature of its part of the diagram, positions taken from the real diagram. Click a frame's tab to fold it to a tile. Variables in no group go in "Other" (or "Variables").
- **Contracts**: an arrow for every variable that crosses between lanes, labelled with its name, from the variable that gives to the variable that uses it.
- **What each variable is**, all derived, nothing to maintain: *exogenous* (depends on no stock, directly or through others: a dashed circle), *lever* (a slider), *constant* (a diamond), *lookup*, *switch* (a 0 or 1 constant used inside an IF), a *loop member* (a small R1 or B2 tag; the loops are listed under the sheet), and the *KPI* of an objective (a red target with its number).

A model with no layers is drawn as one lane. Clicking a variable selects it. Nothing on the sheet changes an equation.

## Not built yet (later phases)

Nested subsystems (a group inside a group), subsystems of one variable, collapse state kept in the file, `dyn diff --by-layer`, XMILE and MDL carrying layers, scenario worlds as stub switching in the app, module families, the block map, the concept map. Layer-owning of oplog entries.
