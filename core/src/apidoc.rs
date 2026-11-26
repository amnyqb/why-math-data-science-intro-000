//! The reference for the operations language, written once here and used three ways: printed by
//! `dyn docs` (Markdown for the wiki, or JSON for tools), offered to an AI agent by the MCP tool
//! `describe_api`, and tested. A test parses every example as a real operation and checks that
//! every operation in `ops.rs` has an entry, so this cannot fall behind the code.

use serde_json::{Value, json};

pub struct Field {
    pub name: &'static str,
    pub ty: &'static str,
    pub required: bool,
    pub about: &'static str,
}

pub struct OpDoc {
    pub op: &'static str,
    pub group: &'static str,
    pub summary: &'static str,
    pub fields: &'static [Field],
    pub example: &'static str,
    /// Why a well-formed operation can still be refused.
    pub refused_when: &'static str,
}

const fn f(name: &'static str, ty: &'static str, required: bool, about: &'static str) -> Field {
    Field { name, ty, required, about }
}

pub const OPS: &[OpDoc] = &[
    OpDoc {
        op: "add_var", group: "Variables",
        summary: "Add a stock, flow, auxiliary, constant or lookup. Arrows are made from the equation.",
        fields: &[
            f("name", "string", true, "Unique name (case, spaces and underscores are not significant)."),
            f("kind", "string", true, "stock, flow, aux, constant or lookup."),
            f("equation", "string", false, "Formula; for a stock, its start value; for a constant, its value."),
            f("units", "string", false, "Text such as MB, Truck/Day, Barrels per Truck (read as algebra)."),
            f("doc", "string", false, "What it means."),
            f("from", "string", false, "Flows only: the stock it drains (leave out for outside the model)."),
            f("to", "string", false, "Flows only: the stock it fills."),
            f("non_negative", "bool", false, "Stock is held at zero instead of going negative."),
            f("layer", "string", false, "The layer it belongs to (default: the layer of its sheet)."),
        ],
        example: r#"{"op":"add_var","name":"Burn","kind":"flow","equation":"Burn rate","units":"MB/Day","from":"Tank"}"#,
        refused_when: "the name exists; the kind is unknown; from/to names no variable; from/to on a non-flow.",
    },
    OpDoc {
        op: "remove_var", group: "Variables",
        summary: "Remove a variable and its arrows; flows it touched become open-ended.",
        fields: &[f("name", "string", true, "Variable to remove.")],
        example: r#"{"op":"remove_var","name":"Burn"}"#,
        refused_when: "no such variable. The batch is also refused if equations still use it (validation: equation).",
    },
    OpDoc {
        op: "rename_var", group: "Variables",
        summary: "Rename a variable and follow it through every equation, item value and check.",
        fields: &[f("name", "string", true, "Current name."), f("to", "string", true, "New name.")],
        example: r#"{"op":"rename_var","name":"Burn rate","to":"Daily burn"}"#,
        refused_when: "no such variable; the new name is taken.",
    },
    OpDoc {
        op: "set_equation", group: "Variables",
        summary: "Replace a variable's formula (a stock's start value, a constant's value). Adds any missing arrows.",
        fields: &[f("name", "string", true, "Variable."), f("equation", "string", true, "New formula.")],
        example: r#"{"op":"set_equation","name":"Burn rate","equation":"5"}"#,
        refused_when: "no such variable; or the result does not validate (a formula that does not read, a broken check).",
    },
    OpDoc {
        op: "set_units", group: "Variables",
        summary: "Set a variable's units text.",
        fields: &[f("name", "string", true, "Variable."), f("units", "string", true, "Units text; empty clears it.")],
        example: r#"{"op":"set_units","name":"Tank","units":"MB"}"#,
        refused_when: "no such variable. Unit mismatches are warnings, not refusals.",
    },
    OpDoc {
        op: "set_doc", group: "Variables",
        summary: "Set a variable's description.",
        fields: &[f("name", "string", true, "Variable."), f("doc", "string", true, "Description.")],
        example: r#"{"op":"set_doc","name":"Tank","doc":"Fuel on hand at the plant"}"#,
        refused_when: "no such variable.",
    },
    OpDoc {
        op: "set_non_negative", group: "Variables",
        summary: "Hold a stock at zero instead of letting it go negative (the conservation check then skips it).",
        fields: &[f("name", "string", true, "Stock."), f("on", "bool", true, "On or off.")],
        example: r#"{"op":"set_non_negative","name":"Tank","on":true}"#,
        refused_when: "no such variable.",
    },
    OpDoc {
        op: "set_flow", group: "Variables",
        summary: "Change which stock a flow drains and fills.",
        fields: &[
            f("name", "string", true, "Flow."),
            f("from", "string|null", true, "Stock drained, or null for outside."),
            f("to", "string|null", true, "Stock filled, or null for outside."),
        ],
        example: r#"{"op":"set_flow","name":"Burn","from":"Tank","to":null}"#,
        refused_when: "the variable is not a flow; a stock name is unknown.",
    },
    OpDoc {
        op: "set_table", group: "Variables",
        summary: "Give a lookup its curve.",
        fields: &[f("name", "string", true, "Lookup."), f("points", "list of [x, y]", true, "At least two points, x strictly increasing.")],
        example: r#"{"op":"set_table","name":"Effect of price","points":[[0,1.2],[1,1],[2,0.6]]}"#,
        refused_when: "the variable is not a lookup; fewer than two points; x not increasing.",
    },
    OpDoc {
        op: "connect", group: "Arrows",
        summary: "Draw an arrow from one variable to another (arrows are also made from equations automatically).",
        fields: &[
            f("from", "string", true, "Cause."),
            f("to", "string", true, "Effect."),
            f("polarity", "string", false, "+, - or empty."),
        ],
        example: r#"{"op":"connect","from":"Price","to":"Demand","polarity":"-"}"#,
        refused_when: "either name is unknown. An arrow the equation does not use is a warning.",
    },
    OpDoc {
        op: "disconnect", group: "Arrows",
        summary: "Remove an arrow.",
        fields: &[f("from", "string", true, "Cause."), f("to", "string", true, "Effect.")],
        example: r#"{"op":"disconnect","from":"Price","to":"Demand"}"#,
        refused_when: "either name is unknown.",
    },
    OpDoc {
        op: "set_setting", group: "Settings",
        summary: "Change a simulation setting or the title.",
        fields: &[
            f("key", "string", true, "initial_time, final_time, time_step, time_units, method (euler, rk2, rk3, rk4), title, units_scale_values (true/false), figure_model (true/false: the units rules report as information), teaching_draft (true/false: the extreme condition experiments report as information), unit_choice (SYMBOL=meaning, e.g. MB=thousand barrels), start_date (ISO 8601), time_convention (abstract or calendar), year_days_abstract, weekend (sat-sun or fri-sat), holidays (ISO dates, comma separated), seed, starts_in_equilibrium (true/false)."),
            f("value", "string", true, "The new value (numbers as text)."),
        ],
        example: r#"{"op":"set_setting","key":"final_time","value":"31"}"#,
        refused_when: "unknown key; a number that is not a number; a time step of zero or less; unknown method.",
    },
    OpDoc {
        op: "add_set", group: "Sets",
        summary: "Add a set of similar things (tanks, regions) that variables can be written once for.",
        fields: &[
            f("name", "string", true, "Set name."),
            f("items", "list of string", true, "Item names."),
            f("parent", "string", false, "A set these items belong to (tank → region)."),
            f("parent_of", "list of string", false, "With a parent: for each item, the parent item it belongs to."),
        ],
        example: r#"{"op":"add_set","name":"Tank","items":["A","B","C"],"parent":"Region","parent_of":["North","North","South"]}"#,
        refused_when: "the set exists; no items; the parent is not a set or parent_of does not name one parent item per item.",
    },
    OpDoc {
        op: "set_per", group: "Sets",
        summary: "Make a variable exist once for every item of a set (or back to a single one with null).",
        fields: &[f("var", "string", true, "Variable."), f("set", "string|null", true, "Set name, or null.")],
        example: r#"{"op":"set_per","var":"Capacity","set":"Tank"}"#,
        refused_when: "no such variable or set.",
    },
    OpDoc {
        op: "set_item_value", group: "Sets",
        summary: "Give one item of a per-set variable its own formula or value.",
        fields: &[f("var", "string", true, "Per-set variable."), f("item", "string", true, "Item."), f("equation", "string", true, "Its formula.")],
        example: r#"{"op":"set_item_value","var":"Capacity","item":"A","equation":"1200"}"#,
        refused_when: "the variable is not per a set.",
    },
    OpDoc {
        op: "set_lever", group: "Sets",
        summary: "Mark a constant as a key lever (shown with controls, varied by sensitivity runs).",
        fields: &[f("var", "string", true, "Constant."), f("on", "bool", true, "On or off.")],
        example: r#"{"op":"set_lever","var":"Burn rate","on":true}"#,
        refused_when: "the variable is not a constant.",
    },
    OpDoc {
        op: "add_check", group: "Checks and functions",
        summary: "State something the results must satisfy. Five kinds: bounds (the default: between min and max), conservation (each named stock, or every stock, changes only by its flows), nonneg (never below zero: a test, unlike a stock held at zero), reference (within a tolerance of a reference series; one point means a flat line) and balance (the chosen stocks together change only by the boundary flows). A broken check is a validation error and refuses the batch under the gate.",
        fields: &[
            f("name", "string", true, "What the check says."),
            f("kind", "string", false, "bounds (default), nonneg, reference, balance or conservation."),
            f("var", "string", false, "Variable to watch (bounds, nonneg, reference)."),
            f("min", "number", false, "Bounds: lower limit."),
            f("max", "number", false, "Bounds: upper limit."),
            f("when", "string", false, "always (every step, default) or end."),
            f("tolerance", "number", false, "Reference and balance: how far is still fine (absolute, or relative to the size when `relative` is true; a balance defaults to 1e-6 for Euler, 2% for the Runge-Kutta methods)."),
            f("relative", "bool", false, "The tolerance is a fraction of the reference value (or of the size of the flows)."),
            f("reference", "list of [time, value]", false, "Reference: the series to stay near, linear between points, held beyond the ends."),
            f("stocks", "list of string", false, "Balance: the stocks taken together."),
            f("inflows", "list of string", false, "Balance: flows that enter from outside."),
            f("outflows", "list of string", false, "Balance: flows that leave to outside."),
            f("at", "number", false, "Bounds: test at this time only."),
            f("source", "string", false, "A benchmark: where the number comes from (a citation). A check with a source is reported as a benchmark."),
        ],
        example: r#"{"op":"add_check","name":"Never empty","var":"Tank","min":0,"when":"always"}"#,
        refused_when: "a bounds check with neither min nor max; an unknown variable, stock or flow; a reference check with no points or a negative tolerance; an unknown kind or when; and the batch is refused if the model, as it runs, breaks the check.",
    },
    OpDoc {
        op: "remove_check", group: "Checks and functions",
        summary: "Remove a check by name.",
        fields: &[f("name", "string", true, "Check name.")],
        example: r#"{"op":"remove_check","name":"Never empty"}"#,
        refused_when: "no such check.",
    },
    OpDoc {
        op: "add_function", group: "Checks and functions",
        summary: "Define your own function in the equation language; it is used like a built-in.",
        fields: &[
            f("name", "string", true, "Function name."),
            f("params", "list of string", true, "Parameter names."),
            f("body", "string", true, "Formula using the parameter names."),
            f("doc", "string", false, "What it is for."),
        ],
        example: r#"{"op":"add_function","name":"Effect of price","params":["p"],"body":"1 - 0.3 * LN(p)"}"#,
        refused_when: "the name exists; the body does not read.",
    },
    OpDoc {
        op: "set_manifest", group: "Manifest",
        summary: "Set what the model offers someone who is not its author: levers with ranges, outputs to show, named scenarios, input bindings, where it is trusted, and which checks vouch for it. Replaces the whole manifest. `dyn manifest FILE --draft` (or the MCP tool `draft_manifest`) drafts one from the levers, stocks and checks.",
        fields: &[f("manifest", "object", true, "Fields: title, description, levers [{name, units, min, max, default, explanation, group}], outputs [{variable, chart (line, area, bar, table), units, reference_lines}], scenarios [{name, values: {lever: number}}], bindings [{input, source, column, kind (data or forecast), note}], validity [{statement, variable, min, max, horizon}], tests [check names], goals [{name, output, measure (final, peak, lowest, total, average, mean:FROM-TO, at:TIME), direction (min, max, target:VALUE), policies [scenario names], protected_ranking, weight (optional, positive: lets recommend give one ranked list), transform (none or log: read the goal through the natural logarithm, for goals that grow exponentially)}].")],
        example: r#"{"op":"set_manifest","manifest":{"title":"Tank","levers":[{"name":"Burn rate","units":"MB/Day","min":2,"max":6,"default":4}],"outputs":[{"variable":"Tank","chart":"line"}],"scenarios":[{"name":"Fast burn","values":{"Burn rate":6}}],"tests":["Never empty"]}}"#,
        refused_when: "a lever that is not a constant or has min not below max or a default outside its range; an output, binding or validity variable the model lacks; a scenario outside a lever's range or naming a non-lever; a binding with no source; a test that is not one of the checks; a goal that reads a variable the model lacks, has an unknown measure or direction, ranks a policy that is not a scenario, or protects a ranking of fewer than two policies. These are validation errors (code manifest), so the batch is refused under the gate.",
    },
    OpDoc {
        op: "add_layer", group: "Layers",
        summary: "Add a tier to the stack of layers (outermost first by `order`). A layer is a model in its own right: its own question, horizon and state of completion. Layers are metadata only; they never change an equation.",
        fields: &[
            f("name", "string", true, "Unique layer name."),
            f("order", "integer", false, "0 = outermost; larger is further in. Default: next after the last."),
            f("question", "string", false, "The one question this layer exists to answer."),
            f("horizon", "number", false, "How far ahead of the start of the run its numbers are trusted (in the model's time unit)."),
            f("structural_limit", "number", false, "Beyond this, structure only: illustrative, not a forecast."),
            f("status", "string", false, "planned (default), stub, built, tested or calibrated."),
            f("colour", "string", false, "Hex colour for views."),
            f("description", "string", false, "Notes."),
            f("spheres", "list of string", false, "Where this layer's exogenous inputs come from."),
        ],
        example: r#"{"op":"add_layer","name":"World","order":0,"question":"What climate does the firm face?","horizon":60,"status":"stub"}"#,
        refused_when: "the name exists; the status is unknown.",
    },
    OpDoc {
        op: "set_layer", group: "Layers",
        summary: "Change one field of a layer.",
        fields: &[
            f("name", "string", true, "The layer."),
            f("key", "string", true, "name, order, question, horizon, structural_limit, status, colour, description, owner or spheres (comma separated)."),
            f("value", "string", true, "The new value; empty clears a horizon or limit."),
        ],
        example: r#"{"op":"set_layer","name":"Firm","key":"horizon","value":"36"}"#,
        refused_when: "no such layer or field; a value that is not a number where one is needed; the new name is taken.",
    },
    OpDoc {
        op: "remove_layer", group: "Layers",
        summary: "Remove a layer. What it held becomes unassigned; its interfaces and stubs are removed.",
        fields: &[f("name", "string", true, "The layer.")],
        example: r#"{"op":"remove_layer","name":"World"}"#,
        refused_when: "no such layer.",
    },
    OpDoc {
        op: "assign_layer", group: "Layers",
        summary: "Put variables, a whole sheet or a submodel in a layer. Items on a sheet inherit the sheet's layer unless they say otherwise.",
        fields: &[
            f("layer", "string", true, "The layer."),
            f("vars", "list of string", false, "Variables to assign."),
            f("sheet", "string", false, "A sheet to assign."),
            f("module", "string", false, "A submodel to assign."),
        ],
        example: r#"{"op":"assign_layer","layer":"Firm","vars":["Cash","Hiring"]}"#,
        refused_when: "no such layer, variable, sheet or submodel; nothing named to assign.",
    },
    OpDoc {
        op: "set_slider", group: "Variables",
        summary: "Give a constant a slider range, for the controls and the lever panel.",
        fields: &[f("var", "string", true, "The constant."), f("min", "number", true, "Lower end."), f("max", "number", true, "Upper end, above the lower.")],
        example: r#"{"op":"set_slider","var":"Price","min":1,"max":4}"#,
        refused_when: "the variable is not a constant, or the range is empty.",
    },
    OpDoc {
        op: "set_purpose", group: "Purpose",
        summary: "Write why the model exists: the problem in one sentence, the kind of model, and what is out of scope. Objectives are added separately.",
        fields: &[
            f("problem", "string", true, "The problem, as a question or sentence."),
            f("kind", "string", false, "explain a behaviour, design a policy, forecast or learn."),
            f("out_of_scope", "string", false, "What is left out on purpose."),
        ],
        example: r#"{"op":"set_purpose","problem":"Can a young firm stay alive while it builds capacity?","kind":"design a policy","out_of_scope":"competitors, regulation"}"#,
        refused_when: "never; it replaces the earlier text.",
    },
    OpDoc {
        op: "add_objective", group: "Purpose",
        summary: "Add an objective the model must meet, with its test in words and the variable that measures it (the KPI, marked on the architecture view).",
        fields: &[
            f("title", "string", true, "Short name, unique."),
            f("kind", "string", false, "grow, decline, reach a target, settle at equilibrium, damp oscillation, stay within a band or grow to a plateau."),
            f("test", "string", false, "The measurable test, in words."),
            f("var", "string", false, "The variable that measures it."),
        ],
        example: r#"{"op":"add_objective","title":"Stay alive","kind":"stay within a band","test":"Cash stays above zero in every month","var":"Cash"}"#,
        refused_when: "the title is empty or taken; the variable does not exist.",
    },
    OpDoc {
        op: "remove_objective", group: "Purpose",
        summary: "Remove an objective by its title.",
        fields: &[f("title", "string", true, "The objective.")],
        example: r#"{"op":"remove_objective","title":"Stay alive"}"#,
        refused_when: "no such objective.",
    },
    OpDoc {
        op: "add_group", group: "Layers",
        summary: "Group variables into a named subsystem. Subsystems are the dashed frames inside a layer on the architecture view; a variable is in one group only, and moving it to a new group takes it out of the old one.",
        fields: &[f("name", "string", true, "Unique group name."), f("vars", "list of string", true, "The variables.")],
        example: r#"{"op":"add_group","name":"Cash account","vars":["Cash","Funding","Revenue","Spend"]}"#,
        refused_when: "the name exists or is empty; a variable does not exist.",
    },
    OpDoc {
        op: "add_to_group", group: "Layers",
        summary: "Put variables in a named group, making the group first if it does not exist (the model doctor keeps its conversion constants in a group called Conversions). A variable is in one group only.",
        fields: &[f("name", "string", true, "Group name."), f("vars", "list of string", true, "The variables.")],
        example: r#"{"op":"add_to_group","name":"Conversions","vars":["MB per barrel"]}"#,
        refused_when: "the name is empty; a variable does not exist.",
    },
    OpDoc {
        op: "set_cld_polarity", group: "Layers",
        summary: "Change the sign of an arrow between two variables on a causal loop sheet (or of a diagram arrow when no sheet is named).",
        fields: &[f("from", "string", true, "The variable the arrow leaves."), f("to", "string", true, "The variable it enters."), f("polarity", "string", true, "+, - or empty for unknown."), f("sheet", "string", false, "The sheet name.")],
        example: r#"{"op":"set_cld_polarity","from":"Price","to":"Demand","polarity":"-"}"#,
        refused_when: "no such arrow or sheet; a sign that is not + or -.",
    },
    OpDoc {
        op: "set_calibration", group: "Checks and functions",
        summary: "Record the result of a calibration: the fitted parameters with their plausible ranges, how each data series was matched (RMSE and Theil's shares) and the correlations of the estimates. The model doctor reads it (rules C1 to C5).",
        fields: &[f("calibration", "object", true, "params (name, value, min, max), fits (var, units, data, rmse, shares), correlations (a, b, r).")],
        example: r#"{"op":"set_calibration","calibration":{"params":[{"name":"Contact Rate","value":2.5,"min":1.0,"max":4.0}],"fits":[],"correlations":[]}}"#,
        refused_when: "never; the doctor reports what is wrong with it.",
    },
    OpDoc {
        op: "freeze_interfaces", group: "Layers",
        summary: "Write every variable that crosses between layers down as an interface contract (variable, from, to, units, kind, time treatment). After this, a crossing that is not declared is a validation error. Contracts already written keep their time treatment.",
        fields: &[],
        example: r#"{"op":"freeze_interfaces"}"#,
        refused_when: "the model has no layers.",
    },
    OpDoc {
        op: "add_stub", group: "Layers",
        summary: "A layer standing in as given formulas for its outgoing variables, so the layers inside it can run alone. Scenario worlds are stubs of the outermost layer.",
        fields: &[
            f("layer", "string", true, "The layer the stub stands in for."),
            f("name", "string", true, "Unique stub name."),
            f("series", "object", true, "variable → formula (it may use TIME)."),
        ],
        example: r#"{"op":"add_stub","layer":"World","name":"Cold capital","series":{"Capital Climate":"0.6"}}"#,
        refused_when: "no such layer; the stub name exists.",
    },
    OpDoc {
        op: "remove_stub", group: "Layers",
        summary: "Remove a stub.",
        fields: &[f("name", "string", true, "The stub.")],
        example: r#"{"op":"remove_stub","name":"Cold capital"}"#,
        refused_when: "no such stub.",
    },
    OpDoc {
        op: "remove_function", group: "Checks and functions",
        summary: "Remove one of your functions.",
        fields: &[f("name", "string", true, "Function name.")],
        example: r#"{"op":"remove_function","name":"Effect of price"}"#,
        refused_when: "no such function; or equations still call it.",
    },
    OpDoc {
        op: "define_module", group: "Modules",
        summary: "Define a reusable module: its variables (local names) and its typed ports.",
        fields: &[
            f("name", "string", true, "Module name."),
            f("description", "string", false, "What it is for."),
            f("vars", "list of variable", true, "Each: name, kind, equation, units, doc, from, to, non_negative (as add_var; flow ends are local names)."),
            f("ports", "list of port", false, "Each: name, direction (in/out), kind (material/information), units, var (the variable inside it stands for)."),
        ],
        example: r#"{"op":"define_module","name":"Tank","vars":[{"name":"Level","kind":"stock","equation":"Start","units":"MB"},{"name":"Start","kind":"constant","equation":"100","units":"MB"},{"name":"Draw","kind":"flow","equation":"Rate","units":"MB/Day","from":"Level"},{"name":"Rate","kind":"aux","equation":"1","units":"MB/Day"}],"ports":[{"name":"Rate in","direction":"in","kind":"information","units":"MB/Day","var":"Rate"},{"name":"Level out","direction":"out","kind":"material","units":"MB","var":"Level"}]}"#,
        refused_when: "the module exists; and at validation: a port naming no variable, a material port on a non-stock/flow, an information port on a stock/flow, an in port on a stock.",
    },
    OpDoc {
        op: "remove_module", group: "Modules",
        summary: "Remove a module definition.",
        fields: &[f("name", "string", true, "Module name.")],
        example: r#"{"op":"remove_module","name":"Tank"}"#,
        refused_when: "no such module; it still has instances.",
    },
    OpDoc {
        op: "instantiate", group: "Modules",
        summary: "Use a module: give it a name, its own parameter values and its in-port formulas. Its variables become `Name.Variable`; read them as \"Name.Variable\" in equations.",
        fields: &[
            f("def", "string", true, "Module to use."),
            f("name", "string", true, "Instance name."),
            f("params", "object", false, "Constant (local name) → value or formula."),
            f("inputs", "object", false, "In port → formula in the rest of the model."),
        ],
        example: r#"{"op":"instantiate","def":"Tank","name":"A","params":{"Start":"50"},"inputs":{"Rate in":"2"}}"#,
        refused_when: "no such module; the instance exists; and at validation: a parameter that is not a constant, an input that is not an in port, an in port left unconnected without a default.",
    },
    OpDoc {
        op: "set_instance_param", group: "Modules",
        summary: "Change one parameter of an instance.",
        fields: &[f("instance", "string", true, "Instance."), f("param", "string", true, "Constant of the module."), f("value", "string", true, "Value or formula.")],
        example: r#"{"op":"set_instance_param","instance":"A","param":"Start","value":"80"}"#,
        refused_when: "no such instance; the parameter is not a constant of the module.",
    },
    OpDoc {
        op: "set_instance_input", group: "Modules",
        summary: "Connect (or reconnect) an in port of an instance to a formula.",
        fields: &[f("instance", "string", true, "Instance."), f("port", "string", true, "In port."), f("formula", "string", true, "Formula in the rest of the model.")],
        example: r#"{"op":"set_instance_input","instance":"A","port":"Rate in","formula":"3"}"#,
        refused_when: "no such instance; the port is not an in port.",
    },
    OpDoc {
        op: "remove_instance", group: "Modules",
        summary: "Remove an instance.",
        fields: &[f("name", "string", true, "Instance.")],
        example: r#"{"op":"remove_instance","name":"A"}"#,
        refused_when: "no such instance; or equations still read its variables.",
    },
];

/// More examples for operations whose forms differ a lot: (operation, example).
pub const EXTRA_EXAMPLES: &[(&str, &str)] = &[
    ("add_check", r#"{"op":"add_check","name":"Stock never negative","kind":"nonneg","var":"Inventory"}"#),
    ("add_check", r#"{"op":"add_check","name":"Equilibrium stays flat","kind":"reference","var":"Inventory","reference":[[0,100]],"tolerance":1e-9}"#),
    ("add_check", r#"{"op":"add_check","name":"Chain conserves beer","kind":"balance","stocks":["Retailer stock","Wholesaler stock"],"inflows":["Factory output"],"outflows":["Customer sales"]}"#),
];

pub const VALIDATION_CODES: &[(&str, &str, &str)] = &[
    ("equation", "error", "An equation does not read, names something unknown, or a module instance is wired wrongly."),
    ("run", "error", "The model compiled but could not run."),
    ("check", "error", "A limit the model states about itself is broken in this run."),
    ("conservation", "error", "A stock changed by something other than its flows."),
    ("layer-crossing", "error", "A variable passes from one layer to another and no interface declares it (only once interfaces have been frozen)."),
    ("layer-units", "error", "A variable's units differ from its interface contract."),
    ("layer-contract", "error", "An interface names a variable or a layer that no longer exists."),
    ("layer-horizon", "error", "A layer's horizon is longer than that of a built layer that feeds it."),
    ("layer-unassigned", "warning", "A variable is in no layer although the model has layers (an error for `dyn layer check`)."),
    ("layer-cycle", "warning", "Layers feed each other (a cross-layer loop)."),
    ("arrow-unused", "warning", "An arrow whose source the equation does not use."),
    ("arrow-missing", "warning", "An equation uses a variable with no arrow to it."),
    ("stock-no-flow", "warning", "A stock nothing fills or drains."),
    ("units", "warning", "Units do not fit as algebra, or a module input carries other units than its port."),
];

pub const QUERIES: &[(&str, &str)] = &[
    ("info", "Counts, sets, modules, instances, checks, settings."),
    ("list [kind]", "Variables with equation, units, flow ends, set, lever, curve."),
    ("get NAME", "One variable in full."),
    ("deps NAME", "What a variable depends on (causes) and what depends on it (uses). A stock's causes are its flows; a flow's uses include the stocks it moves."),
    ("loops", "Feedback loops: reinforcing or balancing, with the variables around each."),
    ("structure_notes", "Stocks without flows; variables nothing uses."),
];

fn type_check() {
    // keeps `Value` and `json!` in use for callers that build documents from this table
    let _: Value = json!(null);
}

/// The whole reference as JSON (for tools).
pub fn json() -> Value {
    type_check();
    json!({
        "operations": OPS.iter().map(|o| json!({
            "op": o.op, "group": o.group, "summary": o.summary, "refused_when": o.refused_when,
            "fields": o.fields.iter().map(|x| json!({"name": x.name, "type": x.ty, "required": x.required, "about": x.about})).collect::<Vec<_>>(),
            "example": serde_json::from_str::<Value>(o.example).unwrap_or(Value::Null),
        })).collect::<Vec<_>>(),
        "validation_codes": VALIDATION_CODES.iter().map(|(c, s, d)| json!({"code": c, "severity": s, "meaning": d})).collect::<Vec<_>>(),
        "queries": QUERIES.iter().map(|(q, d)| json!({"query": q, "returns": d})).collect::<Vec<_>>(),
        "batch": {
            "rule": "A batch is applied to a copy, validated, and committed only if it passes (errors only: warnings never block). All or nothing.",
            "refusal": {"op_index": "index of the failing operation, or null when validation refused the result", "reason": "text", "issues": "validation errors found"},
        },
    })
}

/// The whole reference as Markdown (for the wiki).
pub fn markdown() -> String {
    let mut s = String::new();
    s.push_str("# Operations language: reference\n\n");
    s.push_str("*Generated by `dyn docs` from the code: do not edit by hand. A test parses every example below and checks that every operation has an entry.*\n\n");
    s.push_str("A model is changed by a **batch**: a JSON list of operations. The batch is applied to a copy, the copy is validated, and only a batch that passes becomes the model. All or nothing. Only validation *errors* refuse a batch; warnings never do. A refusal says which operation failed (`op_index`) or, when the operations applied but the result does not validate, lists the errors (`issues`). **Rejected batches are logged too**, with the problems, and every batch can carry a `reason`.\n\n");
    s.push_str("```json\n[\n  {\"op\": \"add_var\", \"name\": \"Tank\", \"kind\": \"stock\", \"equation\": \"100\", \"units\": \"MB\"},\n  {\"op\": \"add_var\", \"name\": \"Burn\", \"kind\": \"flow\", \"equation\": \"4\", \"units\": \"MB/Day\", \"from\": \"Tank\"}\n]\n```\n\n");
    s.push_str("Names are matched ignoring case, spaces and underscores. Kinds, directions and port kinds are accepted in lowercase.\n\n");
    let mut group = "";
    for o in OPS {
        if o.group != group {
            s.push_str(&format!("## {}\n\n", o.group));
            group = o.group;
        }
        s.push_str(&format!("### `{}`\n\n{}\n\n| Field | Type | | Meaning |\n|---|---|---|---|\n", o.op, o.summary));
        for x in o.fields {
            s.push_str(&format!("| `{}` | {} | {} | {} |\n", x.name, x.ty, if x.required { "required" } else { "" }, x.about));
        }
        s.push_str(&format!("\n```json\n{}\n```\n\n", o.example));
        for (_, extra) in EXTRA_EXAMPLES.iter().filter(|(op, _)| *op == o.op) {
            s.push_str(&format!("```json\n{extra}\n```\n\n"));
        }
        s.push_str(&format!("*Refused when:* {}\n\n", o.refused_when));
    }
    s.push_str("## Queries (read-only)\n\n| Query | Returns |\n|---|---|\n");
    for (q, d) in QUERIES {
        s.push_str(&format!("| `{q}` | {d} |\n"));
    }
    s.push_str("\n## Validation codes\n\n| Code | Severity | Meaning |\n|---|---|---|\n");
    for (c, sev, d) in VALIDATION_CODES {
        s.push_str(&format!("| `{c}` | {sev} | {d} |\n"));
    }
    s.push_str("\n## Related\n\n- [Model API](Model-API.md): the crate, `dyn`, the MCP server, Python.\n- [Equation language](Equation-Language.md) and the [function reference](Function-Reference.md): what goes in `equation`.\n- [Format mapping: Vensim](Format-Mapping-Vensim.md), [Vensim and Stella side by side](Vensim-Stella-Reference.md).\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::Op;

    #[test]
    fn every_example_is_a_real_operation_with_the_documented_name() {
        for o in OPS {
            let op: Op = serde_json::from_str(o.example).unwrap_or_else(|e| panic!("{}: {e}", o.op));
            let tag = serde_json::to_value(&op).unwrap()["op"].as_str().unwrap().to_string();
            assert_eq!(tag, o.op, "example of {} parses as {tag}", o.op);
        }
    }

    #[test]
    fn the_extra_examples_are_real_operations_too() {
        for (op, ex) in EXTRA_EXAMPLES {
            let parsed: Op = serde_json::from_str(ex).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(serde_json::to_value(&parsed).unwrap()["op"].as_str().unwrap(), *op);
        }
    }

    #[test]
    fn every_operation_in_the_code_is_documented() {
        let src = include_str!("ops.rs");
        let start = src.find("pub enum Op {").unwrap();
        let body = &src[start..];
        let end = body.find("\n}\n").unwrap();
        let mut names: Vec<String> = Vec::new();
        for line in body[..end].lines().skip(1) {
            // variants sit at exactly four spaces of indent and start with a capital letter
            if line.starts_with("    ") && !line.starts_with("     ") {
                let t = line.trim();
                if t.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
                    let name: String = t.chars().take_while(|c| c.is_alphanumeric()).collect();
                    let mut snake = String::new();
                    for (i, c) in name.chars().enumerate() {
                        if c.is_ascii_uppercase() && i > 0 {
                            snake.push('_');
                        }
                        snake.push(c.to_ascii_lowercase());
                    }
                    names.push(snake);
                }
            }
        }
        names.sort();
        let mut documented: Vec<String> = OPS.iter().map(|o| o.op.to_string()).collect();
        documented.sort();
        assert_eq!(names, documented, "the enum and the reference must list the same operations");
    }

    #[test]
    fn the_reference_renders_in_both_forms() {
        let md = markdown();
        assert!(md.contains("### `instantiate`") && md.contains("## Validation codes"));
        assert_eq!(json()["operations"].as_array().unwrap().len(), OPS.len());
    }
}
