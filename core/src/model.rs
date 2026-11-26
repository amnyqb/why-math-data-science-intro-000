//! The model: variables placed on the diagram, the links between them and the run settings.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    #[serde(alias = "stock", alias = "level")]
    Stock,
    #[serde(alias = "flow", alias = "rate")]
    Flow,
    #[serde(alias = "aux", alias = "auxiliary")]
    Aux,
    #[serde(alias = "constant", alias = "parameter")]
    Constant,
    #[serde(alias = "lookup")]
    Lookup,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Stock => "Stock",
            Kind::Flow => "Flow",
            Kind::Aux => "Variable",
            Kind::Constant => "Constant",
            Kind::Lookup => "Lookup",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Var {
    pub id: u64,
    pub name: String,
    pub kind: Kind,
    /// Formula. For a stock this is its initial value; for a lookup it is the
    /// input (leave empty to use the lookup as a function: `name(x)`).
    pub equation: String,
    #[serde(default)]
    pub units: String,
    #[serde(default)]
    pub comment: String,
    pub pos: [f32; 2],
    /// Graphical function points, kept sorted by x.
    #[serde(default)]
    pub table: Vec<[f64; 2]>,
    /// Slider range for constants.
    #[serde(default)]
    pub slider: Option<[f64; 2]>,
    /// Stocks only: never let the value drop below zero.
    #[serde(default)]
    pub non_negative: bool,
    /// Flows only: stock ids at each end (None = cloud / outside the model).
    #[serde(default)]
    pub from: Option<u64>,
    #[serde(default)]
    pub to: Option<u64>,
    /// Flows only: cloud positions when an end isn't attached to a stock.
    #[serde(default)]
    pub from_pt: [f32; 2],
    #[serde(default)]
    pub to_pt: [f32; 2],
    /// The diagram sheet this sits on (0 = the first diagram).
    #[serde(default)]
    pub view: u64,
    /// "Per ..." — the set of similar things this variable exists once for (None = just one).
    #[serde(default)]
    pub set: Option<String>,
    /// Items whose own formula or curve differs from the one above (set variables only).
    #[serde(default)]
    pub by_item: Vec<ItemValue>,
    /// How the items are combined into the single number shown on the diagram and in charts.
    #[serde(default)]
    pub rollup: Rollup,
    /// A key lever: one of the few variables that drive the model. Levers are what the side
    /// panel shows with its controls; every other constant stays folded away.
    #[serde(default)]
    pub lever: bool,
    /// A lever that is per a set can be nudged for every item at once: all its values are
    /// multiplied by this (1 = as entered).
    #[serde(default = "one_f64")]
    pub lever_scale: f64,
    /// The layer this belongs to (None = the layer of its sheet or submodel; see `layers`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<u64>,
}

fn one_f64() -> f64 {
    1.0
}

/// A formula (or curve) that one item of a set uses instead of the variable's usual one.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ItemValue {
    pub item: String,
    #[serde(default)]
    pub equation: String,
    #[serde(default)]
    pub table: Vec<[f64; 2]>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Rollup {
    #[default]
    Total,
    Average,
}

/// A set of similar things — tanks, regions, products — that share the same structure, so a
/// variable can be written once for all of them (what other tools call an array or subscript).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ItemSet {
    pub id: u64,
    pub name: String,
    pub items: Vec<String>,
    /// The set these items belong to (for example a tank belongs to a region).
    #[serde(default)]
    pub parent: Option<String>,
    /// For each item, the name of the parent item it belongs to.
    #[serde(default)]
    pub parent_of: Vec<String>,
    #[serde(default)]
    pub note: String,
}

impl ItemSet {
    pub fn parent_item(&self, i: usize) -> Option<&str> {
        self.parent_of.get(i).map(|s| s.as_str()).filter(|s| !s.is_empty())
    }
}

impl Var {
    pub fn new(id: u64, name: &str, kind: Kind, pos: [f32; 2]) -> Self {
        Var {
            id,
            name: name.into(),
            kind,
            equation: String::new(),
            units: String::new(),
            comment: String::new(),
            pos,
            table: Vec::new(),
            slider: None,
            non_negative: false,
            from: None,
            to: None,
            from_pt: [pos[0] - 80.0, pos[1]],
            to_pt: [pos[0] + 80.0, pos[1]],
            view: 0,
            set: None,
            by_item: Vec::new(),
            rollup: Rollup::Total,
            lever: false,
            lever_scale: 1.0,
            layer: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Link {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    /// Sideways bend of the curve, in diagram units.
    #[serde(default)]
    pub bend: f32,
    /// "+", "-" or "" for no polarity mark.
    #[serde(default)]
    pub polarity: String,
    /// Draw the two short bars that mark a delay on the arrow.
    #[serde(default)]
    pub delay_mark: bool,
    /// Line weight: 0 thin, 1 normal, 2 bold.
    #[serde(default = "one")]
    pub weight: u8,
    #[serde(default)]
    pub dashed: bool,
    /// Custom colour, or None for the theme's arrow colour.
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    /// None = the diagram's usual shape / head (View → Arrows).
    #[serde(default)]
    pub shape: Option<ArrowShape>,
    #[serde(default)]
    pub head: Option<ArrowHead>,
    /// Turning points the arrow passes through, in order from cause to effect. With any, the arrow
    /// is a smooth path through them instead of a single arc.
    #[serde(default)]
    pub via: Vec<[f32; 2]>,
}

fn one() -> u8 {
    1
}

/// How an arrow is routed between its two ends.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowShape {
    #[default]
    Curved,
    Straight,
    /// Right angles, like a circuit diagram.
    Elbow,
}

impl ArrowShape {
    pub const ALL: [ArrowShape; 3] = [ArrowShape::Curved, ArrowShape::Straight, ArrowShape::Elbow];
    pub fn label(self) -> &'static str {
        match self {
            ArrowShape::Curved => "Curved",
            ArrowShape::Straight => "Straight",
            ArrowShape::Elbow => "Right-angle",
        }
    }
}

/// How variables and arrows look overall.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramStyle {
    /// Variables as cards, coloured arrows.
    #[default]
    Cards,
    /// Variables as plain labels with a small marker, thin dark arrows: the textbook look.
    Classic,
}

/// How a module is drawn on the sheet that holds it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStyle {
    /// Every input and output listed.
    #[default]
    Detailed,
    /// A short card: name, what it is for, its role; arrows say what passes between modules.
    Compact,
}

/// The tip of an arrow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArrowHead {
    #[default]
    Filled,
    Open,
    Dot,
}

impl ArrowHead {
    pub const ALL: [ArrowHead; 3] = [ArrowHead::Filled, ArrowHead::Open, ArrowHead::Dot];
    pub fn label(self) -> &'static str {
        match self {
            ArrowHead::Filled => "Filled",
            ArrowHead::Open => "Open",
            ArrowHead::Dot => "Dot",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetKind {
    /// A stock-and-flow view of part of the model.
    Diagram,
    /// Spreadsheet of every variable, or of results over time.
    Table,
    /// Cards, graphs and sliders arranged freely.
    Dashboard,
    /// A qualitative causal loop diagram: words, signed arrows and loop markers.
    CausalLoop,
    /// Pick levers, a goal and limits, then search for the best settings.
    Optimize,
    /// Named what-if scenarios, run side by side and compared.
    Scenarios,
    /// The structure of the model as cards, one per set of similar things (like an ER diagram).
    Entities,
    /// The purpose, then the layers as lanes, each holding its subsystems; see `architecture`.
    Architecture,
}

impl SheetKind {
    pub fn label(self) -> &'static str {
        match self {
            SheetKind::Diagram => "Diagram",
            SheetKind::Table => "Table",
            SheetKind::Dashboard => "Dashboard",
            SheetKind::CausalLoop => "Causal loop diagram",
            SheetKind::Optimize => "Optimization",
            SheetKind::Scenarios => "Scenarios",
            SheetKind::Entities => "Entities",
            SheetKind::Architecture => "Architecture",
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            SheetKind::Diagram => "◇",
            SheetKind::Table => "▦",
            SheetKind::Dashboard => "▣",
            SheetKind::CausalLoop => "⟲",
            SheetKind::Optimize => "◎",
            SheetKind::Scenarios => "≋",
            SheetKind::Entities => "▤",
            SheetKind::Architecture => "▦",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableMode {
    Variables,
    Results,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WidgetKind {
    Graph,
    Number,
    Slider,
    Text,
}

impl WidgetKind {
    pub fn label(self) -> &'static str {
        match self {
            WidgetKind::Graph => "Graph",
            WidgetKind::Number => "Number card",
            WidgetKind::Slider => "Sliders",
            WidgetKind::Text => "Text",
        }
    }
}

/// How a graph draws its variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ChartKind {
    #[default]
    Line,
    Area,
    Stacked,
    Step,
    Bars,
    Phase,
    Donut,
}

impl ChartKind {
    pub const ALL: [ChartKind; 7] = [
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Stacked,
        ChartKind::Step,
        ChartKind::Bars,
        ChartKind::Phase,
        ChartKind::Donut,
    ];
    pub fn label(self) -> &'static str {
        match self {
            ChartKind::Line => "Line",
            ChartKind::Area => "Area",
            ChartKind::Stacked => "Stacked area",
            ChartKind::Step => "Step",
            ChartKind::Bars => "Bars (at the current time)",
            ChartKind::Phase => "Phase plot (first two against each other)",
            ChartKind::Donut => "Donut (shares at the current time)",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            ChartKind::Line => "Line",
            ChartKind::Area => "Area",
            ChartKind::Stacked => "Stacked",
            ChartKind::Step => "Step",
            ChartKind::Bars => "Bars",
            ChartKind::Phase => "Phase",
            ChartKind::Donut => "Donut",
        }
    }
}

/// Where a chart's legend sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LegendPos {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
    Hidden,
}

impl LegendPos {
    pub const ALL: [LegendPos; 5] = [LegendPos::TopRight, LegendPos::TopLeft, LegendPos::BottomRight, LegendPos::BottomLeft, LegendPos::Hidden];
    pub fn label(self) -> &'static str {
        match self {
            LegendPos::TopRight => "Top right",
            LegendPos::TopLeft => "Top left",
            LegendPos::BottomRight => "Bottom right",
            LegendPos::BottomLeft => "Bottom left",
            LegendPos::Hidden => "Hidden",
        }
    }
}

/// How a chart looks. Empty labels and unset ranges mean "choose automatically".
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct ChartStyle {
    pub title: String,
    pub subtitle: String,
    pub x_label: String,
    pub y_label: String,
    pub x_min: Option<f64>,
    pub x_max: Option<f64>,
    pub y_min: Option<f64>,
    pub y_max: Option<f64>,
    pub legend: LegendPos,
    pub grid: bool,
    pub line_width: f32,
    pub markers: bool,
    pub time_marker: bool,
    /// Draw scenarios and kept runs next to the current model.
    pub compare: bool,
    /// Custom colours per variable.
    pub colors: Vec<(String, [u8; 3])>,
}

impl Default for ChartStyle {
    fn default() -> Self {
        ChartStyle {
            title: String::new(),
            subtitle: String::new(),
            x_label: String::new(),
            y_label: String::new(),
            x_min: None,
            x_max: None,
            y_min: None,
            y_max: None,
            legend: LegendPos::TopRight,
            grid: true,
            line_width: 1.0,
            markers: false,
            time_marker: true,
            compare: true,
            colors: Vec::new(),
        }
    }
}

impl ChartStyle {
    pub fn color_for(&self, var: &str) -> Option<[u8; 3]> {
        self.colors.iter().find(|(n, _)| n == var).map(|x| x.1)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Widget {
    pub id: u64,
    pub kind: WidgetKind,
    pub title: String,
    /// Variables shown (graph lines, the number, or the sliders).
    #[serde(default)]
    pub vars: Vec<String>,
    /// Text widgets: the text.
    #[serde(default)]
    pub text: String,
    /// Graph cards: how to draw.
    #[serde(default)]
    pub chart: ChartKind,
    /// Graph cards: titles, axes, legend and colours.
    #[serde(default)]
    pub style: ChartStyle,
    /// Width in grid columns (1–4).
    pub span: u8,
    /// Height in pixels.
    pub height: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sheet {
    pub id: u64,
    pub name: String,
    pub kind: SheetKind,
    #[serde(default = "table_vars")]
    pub table_mode: TableMode,
    /// Results tables: the columns (empty = every variable).
    #[serde(default)]
    pub columns: Vec<String>,
    #[serde(default)]
    pub widgets: Vec<Widget>,
    #[serde(default)]
    pub cld: CldData,
    #[serde(default)]
    pub opt: OptSpec,
    /// Set on a module's own sheet.
    #[serde(default)]
    pub module: Option<u64>,
    /// The layer the sheet belongs to (items on it inherit it unless they say otherwise).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<u64>,
    /// Scenario sheets: the variables being compared and the summary measure.
    #[serde(default)]
    pub compare_vars: Vec<String>,
    #[serde(default)]
    pub compare_measure: Option<Measure>,
    /// Figure caption, written under the picture by `dyn render --with-caption`.
    #[serde(default)]
    pub caption: String,
    /// Where the data or idea comes from ("Source: …" under the caption).
    #[serde(default)]
    pub source: String,
}

/// One value a scenario changes: a variable's equation (usually just a number).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Change {
    pub var: String,
    pub value: String,
}

/// A named what-if: the model with some values changed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scenario {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub color: Option<[u8; 3]>,
    /// Shown in charts and comparisons.
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub changes: Vec<Change>,
    #[serde(default)]
    pub notes: String,
}

impl Scenario {
    pub fn value_of(&self, var: &str) -> Option<&str> {
        let k = crate::expr::normalize(var);
        self.changes.iter().find(|c| crate::expr::normalize(&c.var) == k).map(|c| c.value.as_str())
    }
}

// ---------------------------------------------------------------- causal loop diagrams

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CldNode {
    pub id: u64,
    pub text: String,
    pub pos: [f32; 2],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CldLink {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub bend: f32,
    /// "+", "-" or "" (unknown)
    pub polarity: String,
    #[serde(default)]
    pub delay: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CldLoop {
    pub id: u64,
    pub pos: [f32; 2],
    /// true = reinforcing (R), false = balancing (B)
    pub reinforcing: bool,
    #[serde(default)]
    pub label: String,
    /// The little circular arrow turns clockwise.
    #[serde(default = "yes")]
    pub clockwise: bool,
}

fn yes() -> bool {
    true
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CldData {
    pub nodes: Vec<CldNode>,
    pub links: Vec<CldLink>,
    pub loops: Vec<CldLoop>,
}

// ---------------------------------------------------------------- optimization

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Measure {
    Final,
    Peak,
    Lowest,
    Total,
    Average,
    /// Constraints only: the limit must hold at every moment of the run.
    Always,
}

impl Measure {
    pub fn label(self) -> &'static str {
        match self {
            Measure::Final => "final value",
            Measure::Peak => "peak",
            Measure::Lowest => "lowest value",
            Measure::Total => "total over the run",
            Measure::Average => "average",
            Measure::Always => "at every moment",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lever {
    pub name: String,
    pub on: bool,
    pub min: f64,
    pub max: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Limit {
    pub var: String,
    pub measure: Measure,
    /// true = must stay at or below `value`, false = at or above.
    pub at_most: bool,
    pub value: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptSpec {
    pub maximize: bool,
    pub goal: String,
    pub measure: Measure,
    pub levers: Vec<Lever>,
    pub limits: Vec<Limit>,
    /// How hard to search: 1 quick … 3 thorough.
    pub effort: u8,
}

impl Default for OptSpec {
    fn default() -> Self {
        OptSpec {
            maximize: false,
            goal: String::new(),
            measure: Measure::Peak,
            levers: Vec::new(),
            limits: Vec::new(),
            effort: 2,
        }
    }
}

fn table_vars() -> TableMode {
    TableMode::Variables
}

impl Sheet {
    pub fn new(id: u64, name: &str, kind: SheetKind) -> Self {
        Sheet {
            id,
            name: name.into(),
            kind,
            table_mode: TableMode::Variables,
            columns: Vec::new(),
            widgets: Vec::new(),
            cld: CldData::default(),
            opt: OptSpec::default(),
            module: None,
            compare_vars: Vec::new(),
            compare_measure: None,
            caption: String::new(),
            source: String::new(),
            layer: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub text: String,
    pub pos: [f32; 2],
    #[serde(default)]
    pub view: u64,
    /// How the text looks; absent in a file means the plain note.
    #[serde(default, skip_serializing_if = "TextFmt::is_plain")]
    pub fmt: TextFmt,
}

/// Where the lines of a text sit inside its box.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

/// The look of a piece of text on a diagram. All-default is the plain note (12.5 points, the muted colour).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TextFmt {
    /// Points; 0 means the usual size.
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// Fixed-width letters (for equations and code).
    pub mono: bool,
    /// Text colour; None = the theme's note colour.
    pub color: Option<[u8; 3]>,
    /// Background of the note; None = the theme's note colour.
    pub fill: Option<[u8; 3]>,
    pub align: TextAlign,
}

impl TextFmt {
    pub const DEFAULT_SIZE: f32 = 12.5;
    pub fn is_plain(&self) -> bool {
        *self == TextFmt::default()
    }
    /// The size to draw at.
    pub fn points(&self) -> f32 {
        if self.size > 0.0 { self.size } else { Self::DEFAULT_SIZE }
    }
}

/// What a drawn shape is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShapeKind {
    Rect,
    Rounded,
    Ellipse,
    Line,
    Arrow,
    Picture,
}

impl ShapeKind {
    pub const DRAWN: [ShapeKind; 5] = [ShapeKind::Rect, ShapeKind::Rounded, ShapeKind::Ellipse, ShapeKind::Line, ShapeKind::Arrow];
    pub fn label(self) -> &'static str {
        match self {
            ShapeKind::Rect => "Rectangle",
            ShapeKind::Rounded => "Rounded box",
            ShapeKind::Ellipse => "Ellipse",
            ShapeKind::Line => "Line",
            ShapeKind::Arrow => "Arrow",
            ShapeKind::Picture => "Picture",
        }
    }
    /// A line or an arrow: `size` is the vector from the start to the end, and may be negative.
    pub fn is_line(self) -> bool {
        matches!(self, ShapeKind::Line | ShapeKind::Arrow)
    }
}

/// A piece of drawing on a diagram sheet. It does not take part in the model's meaning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    pub id: u64,
    pub view: u64,
    pub kind: ShapeKind,
    /// Top-left corner; for a line or arrow, the start.
    pub pos: [f32; 2],
    /// Width and height; for a line or arrow, the vector to the end.
    pub size: [f32; 2],
    /// Inside colour; None = no fill (a picture ignores it).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<[u8; 3]>,
    /// Outline colour; None = the theme's line colour.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<[u8; 3]>,
    /// Outline width in points; 0 = the usual 1.5.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub weight: f32,
    /// For a picture: the `ImageAsset` it shows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<u64>,
}

fn is_zero(v: &f32) -> bool {
    *v == 0.0
}

impl Shape {
    pub const DEFAULT_WEIGHT: f32 = 1.5;
    pub fn new(id: u64, view: u64, kind: ShapeKind, pos: [f32; 2], size: [f32; 2]) -> Shape {
        Shape { id, view, kind, pos, size, fill: None, stroke: None, weight: 0.0, image: None }
    }
    pub fn line_weight(&self) -> f32 {
        if self.weight > 0.0 { self.weight } else { Self::DEFAULT_WEIGHT }
    }
    /// The box that holds it: (min, max).
    pub fn bounds(&self) -> ([f32; 2], [f32; 2]) {
        let (a, b) = (self.pos, [self.pos[0] + self.size[0], self.pos[1] + self.size[1]]);
        ([a[0].min(b[0]), a[1].min(b[1])], [a[0].max(b[0]), a[1].max(b[1])])
    }
}

/// A picture file kept inside the model file, so the model travels alone.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageAsset {
    pub id: u64,
    /// The file's name when it was added.
    pub name: String,
    /// The file as it was (PNG or JPEG), in base64.
    pub data: String,
    pub w: u32,
    pub h: u32,
}

/// What a control on the diagram looks like and how it is operated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ControlKind {
    /// A track: click or drag along it.
    Slider,
    /// A dial: drag up or down.
    Knob,
    /// On or off: the top or the bottom of the range.
    Switch,
    /// A number with steppers: drag sideways or press − and +.
    Number,
}

impl ControlKind {
    pub const ALL: [ControlKind; 4] = [ControlKind::Slider, ControlKind::Knob, ControlKind::Switch, ControlKind::Number];
    pub fn label(self) -> &'static str {
        match self {
            ControlKind::Slider => "Slider",
            ControlKind::Knob => "Knob",
            ControlKind::Switch => "Switch",
            ControlKind::Number => "Number",
        }
    }
    /// Width and height on the diagram.
    pub fn size(self) -> [f32; 2] {
        match self {
            ControlKind::Slider => [220.0, 54.0],
            ControlKind::Knob => [100.0, 116.0],
            ControlKind::Switch => [170.0, 38.0],
            ControlKind::Number => [170.0, 56.0],
        }
    }
}

/// An input device on a diagram: operating it sets the number in a constant, and the model re-runs.
/// It carries no meaning of its own, so it lives in the layout file of the split format.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Control {
    pub id: u64,
    pub view: u64,
    pub kind: ControlKind,
    /// The constant it sets (0 = none yet).
    pub var: u64,
    /// Top-left corner.
    pub pos: [f32; 2],
    /// What it says; empty means the variable's name.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
}

impl Control {
    pub fn new(id: u64, view: u64, kind: ControlKind, var: u64, pos: [f32; 2]) -> Control {
        Control { id, view, kind, var, pos, label: String::new() }
    }
}

/// Items that move and select together, drawn inside a labelled frame.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    /// Variable, note and module ids.
    pub members: Vec<u64>,
    pub view: u64,
    #[serde(default)]
    pub color: Option<[u8; 3]>,
}

/// A submodel: its parts live on their own diagram sheet (`inner`) and it shows up as a
/// single box on its parent sheet (`view`), with its inputs and outputs as ports.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Module {
    pub id: u64,
    pub name: String,
    pub view: u64,
    pub inner: u64,
    pub pos: [f32; 2],
    #[serde(default)]
    pub description: String,
    /// Drawn as a short card (name and purpose) instead of listing its inputs and outputs.
    #[serde(default)]
    pub collapsed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layer: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Method {
    Euler,
    /// Heun's method: second order.
    RungeKutta2,
    /// Kutta's third-order method.
    RungeKutta3,
    RungeKutta4,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub initial_time: f64,
    pub final_time: f64,
    pub time_step: f64,
    pub time_units: String,
    pub method: Method,
    /// A declared unit carries value scale: a constant "5 ~ %/Day" is 0.05 per Day. New models: true. A file that does
    /// not say (every model written before this setting, and every imported Vensim or Stella model): false, so numbers
    /// do not change.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub units_scale_values: bool,
    /// A figure model: drawn for a picture, with constants and small variables folded into the equations on purpose. The
    /// units rules then report what they find as information, not as errors.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub figure_model: bool,
    /// A teaching draft: a model built to show a weakness on purpose (an unguarded stock, an exogenous input). The extreme
    /// condition experiments (X rules) report what they find as information, and the sanity sheet says it is not for delivery.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub teaching_draft: bool,
    /// For a unit symbol with several meanings (MB, ton, gallon…): symbol → the meaning chosen (see `unit_table`).
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub unit_choices: std::collections::BTreeMap<String, String>,
    /// The calendar date of the first time (ISO 8601, optional offset such as +03:00).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    /// `Abstract` (Year = `year_days_abstract` days, Month = Year / 12) or `Calendar` (steps follow the real calendar).
    #[serde(default, skip_serializing_if = "is_abstract")]
    pub time_convention: TimeConvention,
    /// Length of a Year in days under the abstract convention.
    #[serde(default = "default_year_days", skip_serializing_if = "is_default_year_days")]
    pub year_days_abstract: f64,
    /// "sat-sun" (default) or "fri-sat".
    #[serde(default = "default_weekend", skip_serializing_if = "is_default_weekend")]
    pub weekend: String,
    /// ISO dates that are not workdays.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holidays: Vec<String>,
    /// Seed of the random streams (a call without its own seed derives a stream from this, the variable and the call).
    #[serde(default = "default_seed", skip_serializing_if = "is_default_seed")]
    pub seed: u64,
    /// The modeller has recorded the seed on purpose (even if it is the default 1).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub seed_recorded: bool,
    /// The modeller states that the model starts in equilibrium (the doctor then checks it).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub starts_in_equilibrium: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeConvention {
    #[default]
    Abstract,
    Calendar,
}

fn is_abstract(c: &TimeConvention) -> bool {
    *c == TimeConvention::Abstract
}
fn is_default_year_days(d: &f64) -> bool {
    *d == 365.25
}
fn is_default_seed(s: &u64) -> bool {
    *s == 1
}
fn default_year_days() -> f64 {
    365.25
}
fn default_weekend() -> String {
    "sat-sun".into()
}
fn is_default_weekend(s: &String) -> bool {
    s == "sat-sun"
}
fn default_seed() -> u64 {
    1
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            initial_time: 0.0,
            final_time: 100.0,
            time_step: 0.25,
            time_units: "Month".into(),
            method: Method::Euler,
            units_scale_values: true,
            figure_model: false,
            teaching_draft: false,
            unit_choices: Default::default(),
            start: None,
            time_convention: TimeConvention::Abstract,
            year_days_abstract: 365.25,
            weekend: "sat-sun".into(),
            holidays: Vec::new(),
            seed: 1,
            seed_recorded: false,
            starts_in_equilibrium: false,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Model {
    pub title: String,
    pub vars: Vec<Var>,
    pub links: Vec<Link>,
    #[serde(default)]
    pub notes: Vec<Note>,
    /// Rectangles, ellipses, lines, arrows and pictures drawn on diagram sheets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shapes: Vec<Shape>,
    /// The picture files the shapes of kind `picture` show.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<ImageAsset>,
    /// Sliders, knobs, switches and number inputs on diagram sheets, each tied to a constant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controls: Vec<Control>,
    pub settings: Settings,
    pub next_id: u64,
    #[serde(default)]
    pub sheets: Vec<Sheet>,
    #[serde(default)]
    pub groups: Vec<Group>,
    #[serde(default)]
    pub modules: Vec<Module>,
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
    /// How the Trends graph in the results panel looks.
    #[serde(default)]
    pub trend_style: ChartStyle,
    /// Sets of similar things (tanks, regions…) that variables can be "per".
    #[serde(default)]
    pub sets: Vec<ItemSet>,
    /// How arrows are drawn unless an arrow says otherwise.
    #[serde(default)]
    pub arrow_shape: ArrowShape,
    #[serde(default)]
    pub arrow_head: ArrowHead,
    #[serde(default)]
    pub diagram_style: DiagramStyle,
    #[serde(default)]
    pub module_style: ModuleStyle,
    /// Functions defined by the person, written in the equation language (Edit → My functions).
    #[serde(default)]
    pub functions: Vec<crate::userfn::UserFn>,
    /// Limits the results must respect (Edit → Checks, or a *Checks* sheet in a spreadsheet).
    #[serde(default)]
    pub checks: Vec<crate::checks::Check>,
    /// Reusable pieces of model with named, typed ports (see `modules`).
    #[serde(default)]
    pub defs: Vec<crate::modules::ModuleDef>,
    /// Uses of those pieces, each with its own parameters and inputs.
    #[serde(default)]
    pub instances: Vec<crate::modules::Instance>,
    /// What the model offers a user who is not its author (see `manifest`).
    #[serde(default)]
    pub manifest: crate::manifest::Manifest,
    /// How `dyn render` draws figures from this model (font, colours, line widths…); see `figstyle`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub figure_style: Option<crate::figstyle::FigStyle>,
    /// The stack of layers, outermost first by `order` (empty = one implicit layer; see `layers`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<crate::layers::Layer>,
    /// The declared contracts between layers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<crate::layers::Interface>,
    /// Layers standing in as given formulas.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stubs: Vec<crate::layers::Stub>,
    /// Why the model exists and what counts as success (see `architecture`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<Purpose>,
    /// The result of a calibration, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calibration: Option<Calibration>,
}

/// What a calibration left behind: the parameters that were fitted, how well each data series was matched, and how the
/// fitted parameters move together. The doctor's rules C1 to C5 read it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Calibration {
    #[serde(default)]
    pub params: Vec<CalParam>,
    #[serde(default)]
    pub fits: Vec<CalFit>,
    /// Correlation of the estimates of two parameters (names, r).
    #[serde(default)]
    pub correlations: Vec<(String, String, f64)>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CalParam {
    pub name: String,
    pub value: f64,
    /// The plausible range the modeller gave it.
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CalFit {
    /// The model variable the data series is compared with.
    pub var: String,
    /// The units the data are in.
    #[serde(default)]
    pub units: String,
    /// (time, value) in model time.
    #[serde(default)]
    pub data: Vec<[f64; 2]>,
    #[serde(default)]
    pub rmse: Option<f64>,
    /// Theil's shares of the error: bias, variance, covariance (None when not reported).
    #[serde(default)]
    pub shares: Option<[f64; 3]>,
}

/// The brief of a model: the problem, what kind of model this is, and the objectives it must meet.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Purpose {
    /// The problem in one sentence.
    #[serde(default)]
    pub problem: String,
    /// explain a behaviour, design a policy, forecast, or learn.
    #[serde(default)]
    pub kind: String,
    /// What is left out on purpose.
    #[serde(default)]
    pub out_of_scope: String,
    #[serde(default)]
    pub objectives: Vec<Objective>,
}

/// One thing the model must achieve, with a test anyone can check.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Objective {
    pub title: String,
    /// grow, decline, reach a target, settle at equilibrium, damp oscillation, stay within a band, grow to a plateau.
    #[serde(default)]
    pub kind: String,
    /// The measurable test, in words.
    #[serde(default)]
    pub test: String,
    /// The variable that measures it (the KPI).
    #[serde(default)]
    pub var: String,
}

impl Model {
    /// Make sure there's at least one diagram sheet and every item sits on an existing one.
    pub fn ensure_sheets(&mut self) {
        if !self.sheets.iter().any(|s| s.kind == SheetKind::Diagram) {
            let id = self.new_id();
            self.sheets
                .insert(0, Sheet::new(id, "Model", SheetKind::Diagram));
        }
        let first = self.first_diagram();
        let diagrams: Vec<u64> = self
            .sheets
            .iter()
            .filter(|s| s.kind == SheetKind::Diagram)
            .map(|s| s.id)
            .collect();
        for v in self.vars.iter_mut() {
            if !diagrams.contains(&v.view) {
                v.view = first;
            }
        }
        for n in self.notes.iter_mut() {
            if !diagrams.contains(&n.view) {
                n.view = first;
            }
        }
        for g in self.groups.iter_mut() {
            if !diagrams.contains(&g.view) {
                g.view = first;
            }
        }
        // modules whose own sheet is gone are dropped; ones whose parent is gone move to the first diagram
        self.modules.retain(|m| diagrams.contains(&m.inner));
        for m in self.modules.iter_mut() {
            if !diagrams.contains(&m.view) {
                m.view = first;
            }
        }
        let modules: Vec<u64> = self.modules.iter().map(|m| m.id).collect();
        for s in self.sheets.iter_mut() {
            if s.module.is_some_and(|m| !modules.contains(&m)) {
                s.module = None;
            }
        }
        let alive: Vec<u64> = self
            .vars
            .iter()
            .map(|v| v.id)
            .chain(self.notes.iter().map(|n| n.id))
            .chain(modules.iter().copied())
            .collect();
        for g in self.groups.iter_mut() {
            g.members.retain(|m| alive.contains(m));
        }
        self.groups.retain(|g| g.members.len() > 1);
    }
    pub fn module(&self, id: u64) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }
    pub fn first_diagram(&self) -> u64 {
        self.sheets
            .iter()
            .find(|s| s.kind == SheetKind::Diagram)
            .map(|s| s.id)
            .unwrap_or(0)
    }
    pub fn sheet_mut(&mut self, id: u64) -> Option<&mut Sheet> {
        self.sheets.iter_mut().find(|s| s.id == id)
    }
    pub fn new_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
    pub fn var(&self, id: u64) -> Option<&Var> {
        self.vars.iter().find(|v| v.id == id)
    }
    pub fn var_mut(&mut self, id: u64) -> Option<&mut Var> {
        self.vars.iter_mut().find(|v| v.id == id)
    }
    pub fn by_name(&self, norm: &str) -> Option<&Var> {
        self.vars
            .iter()
            .find(|v| crate::expr::normalize(&v.name) == norm)
            // `Name[item]` (one item of a set variable) finds the variable it is an item of
            .or_else(|| {
                let i = norm.find('[').filter(|_| norm.ends_with(']'))?;
                let base = norm[..i].trim();
                self.vars.iter().find(|v| v.set.is_some() && crate::expr::normalize(&v.name) == base)
            })
    }
    /// The items of the set a variable is per (empty for a single-valued variable).
    pub fn items_of(&self, v: &Var) -> Vec<String> {
        v.set.as_ref().and_then(|s| self.sets.iter().find(|x| &x.name == s)).map(|s| s.items.clone()).unwrap_or_default()
    }
    pub fn unique_name(&self, base: &str) -> String {
        for i in 1.. {
            let n = format!("{base} {i}");
            if self.by_name(&crate::expr::normalize(&n)).is_none() {
                return n;
            }
        }
        unreachable!()
    }
    /// Remove a variable together with its links and any flow ends that pointed at it.
    pub fn remove_var(&mut self, id: u64) {
        let Some(v) = self.var(id).cloned() else {
            return;
        };
        self.vars.retain(|x| x.id != id);
        self.links.retain(|l| l.from != id && l.to != id);
        for g in self.groups.iter_mut() {
            g.members.retain(|m| *m != id);
        }
        self.groups.retain(|g| g.members.len() > 1);
        for f in self.vars.iter_mut() {
            if f.from == Some(id) {
                f.from = None;
                f.from_pt = [v.pos[0] + 60.0, v.pos[1]];
            }
            if f.to == Some(id) {
                f.to = None;
                f.to_pt = [v.pos[0] - 60.0, v.pos[1]];
            }
        }
    }
    /// Set a variable to a new value or formula, the way a scenario does. For a variable that is
    /// per a set, `Name` changes every item and `Name[item]` just one. False if nothing matched.
    pub fn apply_change(&mut self, var: &str, value: &str) -> bool {
        let key = crate::expr::normalize(var);
        if let Some(v) = self.vars.iter_mut().find(|v| crate::expr::normalize(&v.name) == key) {
            v.equation = value.to_string();
            v.by_item.clear();
            return true;
        }
        if let Some(a) = var.rfind('[') {
            if var.ends_with(']') {
                let (base, item) = (&var[..a], var[a + 1..var.len() - 1].trim());
                let bk = crate::expr::normalize(base);
                let ik = crate::expr::normalize(item);
                if let Some(v) = self.vars.iter_mut().find(|v| crate::expr::normalize(&v.name) == bk && v.set.is_some()) {
                    match v.by_item.iter_mut().find(|o| crate::expr::normalize(&o.item) == ik) {
                        Some(o) => o.equation = value.to_string(),
                        None => v.by_item.push(ItemValue { item: item.to_string(), equation: value.to_string(), table: Vec::new() }),
                    }
                    return true;
                }
            }
        }
        false
    }
    pub fn add_link(&mut self, from: u64, to: u64) -> bool {
        if from == to || self.links.iter().any(|l| l.from == from && l.to == to) {
            return false;
        }
        let id = self.new_id();
        self.links.push(Link {
            id,
            from,
            to,
            bend: 0.0,
            polarity: String::new(),
            delay_mark: false,
            weight: 1,
            dashed: false,
            color: None,
            shape: None,
            head: None,
            via: Vec::new(),
        });
        true
    }

    /// The starter model: a spreading-illness model that shows stocks, flows,
    /// constants with sliders, a lookup table and feedback.
    pub fn example() -> Model {
        let mut m = Model {
            title: "Spread of an Illness".into(),
            settings: Settings {
                initial_time: 0.0,
                final_time: 60.0,
                time_step: 0.125,
                time_units: "Day".into(),
                method: Method::Euler, ..Default::default()
            },
            ..Default::default()
        };
        let add =
            |m: &mut Model, name: &str, kind: Kind, pos: [f32; 2], eq: &str, units: &str| -> u64 {
                let id = m.new_id();
                let mut v = Var::new(id, name, kind, pos);
                v.equation = eq.into();
                v.units = units.into();
                m.vars.push(v);
                id
            };
        let s = add(
            &mut m,
            "Susceptible",
            Kind::Stock,
            [140.0, 260.0],
            "9990",
            "People",
        );
        let i = add(
            &mut m,
            "Infected",
            Kind::Stock,
            [480.0, 260.0],
            "10",
            "People",
        );
        let r = add(
            &mut m,
            "Recovered",
            Kind::Stock,
            [820.0, 260.0],
            "0",
            "People",
        );
        let inf = add(
            &mut m,
            "Infection Rate",
            Kind::Flow,
            [310.0, 260.0],
            "Contact Rate * Infectivity * Susceptible * Fraction Infected * Effect of Awareness(Fraction Infected)",
            "People/Day",
        );
        let rec = add(
            &mut m,
            "Recovery Rate",
            Kind::Flow,
            [650.0, 260.0],
            "Infected / Illness Duration",
            "People/Day",
        );
        let cr = add(
            &mut m,
            "Contact Rate",
            Kind::Constant,
            [200.0, 110.0],
            "6",
            "1/Day",
        );
        let iv = add(
            &mut m,
            "Infectivity",
            Kind::Constant,
            [400.0, 110.0],
            "0.25",
            "Dmnl",
        );
        let dur = add(
            &mut m,
            "Illness Duration",
            Kind::Constant,
            [760.0, 120.0],
            "5",
            "Day",
        );
        let tot = add(
            &mut m,
            "Total Population",
            Kind::Aux,
            [480.0, 440.0],
            "Susceptible + Infected + Recovered",
            "People",
        );
        let fi = add(
            &mut m,
            "Fraction Infected",
            Kind::Aux,
            [300.0, 420.0],
            "Infected / Total Population",
            "Dmnl",
        );
        let aw = add(
            &mut m,
            "Effect of Awareness",
            Kind::Lookup,
            [150.0, 420.0],
            "",
            "Dmnl",
        );
        for v in m.vars.iter_mut() {
            match v.id {
                x if x == inf => {
                    v.from = Some(s);
                    v.to = Some(i);
                }
                x if x == rec => {
                    v.from = Some(i);
                    v.to = Some(r);
                }
                x if x == cr => v.slider = Some([0.0, 15.0]),
                x if x == iv => v.slider = Some([0.0, 1.0]),
                x if x == dur => v.slider = Some([1.0, 20.0]),
                x if x == aw => {
                    v.table = vec![
                        [0.0, 1.0],
                        [0.05, 0.92],
                        [0.1, 0.7],
                        [0.15, 0.5],
                        [0.2, 0.38],
                        [0.3, 0.3],
                        [0.5, 0.25],
                    ];
                    v.comment =
                        "People cut their contacts as more of the population is visibly ill."
                            .into();
                }
                _ => {}
            }
        }
        for (a, b, bend, pol) in [
            (cr, inf, -22.0, "+"),
            (iv, inf, 22.0, "+"),
            (s, inf, -28.0, "+"),
            (fi, inf, 26.0, "+"),
            (aw, inf, 34.0, ""),
            (i, rec, -26.0, "+"),
            (dur, rec, 18.0, "-"),
            (s, tot, 40.0, ""),
            (i, tot, -18.0, ""),
            (r, tot, -40.0, ""),
            (i, fi, -30.0, "+"),
            (tot, fi, 22.0, "-"),
        ] {
            m.add_link(a, b);
            let l = m.links.last_mut().unwrap();
            l.bend = bend;
            l.polarity = pol.into();
        }
        let gid = m.new_id();
        m.groups.push(Group {
            id: gid,
            name: "Policy levers".into(),
            members: vec![cr, iv],
            view: 0,
            color: Some([22, 163, 74]),
        });
        let nid = m.new_id();
        m.notes.push(Note {
            id: nid,
            text: "Try the sliders: lower the Contact Rate and watch the peak flatten.".into(),
            pos: [640.0, 440.0],
            view: 0,
            fmt: TextFmt::default(),
        });

        // sheets: the diagram, a spreadsheet of the model and a dashboard
        let d = m.new_id();
        m.sheets.push(Sheet::new(d, "Model", SheetKind::Diagram));
        let t = m.new_id();
        m.sheets.push(Sheet::new(t, "Equations", SheetKind::Table));
        let r = m.new_id();
        let mut rs = Sheet::new(r, "Results", SheetKind::Table);
        rs.table_mode = TableMode::Results;
        rs.columns = vec![
            "Susceptible".into(),
            "Infected".into(),
            "Recovered".into(),
            "Infection Rate".into(),
            "Recovery Rate".into(),
        ];
        m.sheets.push(rs);
        let db = m.new_id();
        let mut dash = Sheet::new(db, "Dashboard", SheetKind::Dashboard);
        let w =
            |m: &mut Model, kind: WidgetKind, title: &str, vars: &[&str], span: u8, height: f32| {
                let id = m.new_id();
                Widget {
                    id,
                    kind,
                    title: title.into(),
                    vars: vars.iter().map(|s| s.to_string()).collect(),
                    text: String::new(),
                    chart: ChartKind::Line,
                    style: ChartStyle::default(),
                    span,
                    height,
                }
            };
        dash.widgets = vec![
            w(
                &mut m,
                WidgetKind::Number,
                "Currently ill",
                &["Infected"],
                1,
                130.0,
            ),
            w(
                &mut m,
                WidgetKind::Number,
                "Recovered",
                &["Recovered"],
                1,
                130.0,
            ),
            w(
                &mut m,
                WidgetKind::Number,
                "Still susceptible",
                &["Susceptible"],
                1,
                130.0,
            ),
            w(
                &mut m,
                WidgetKind::Number,
                "New infections per day",
                &["Infection Rate"],
                1,
                130.0,
            ),
            w(
                &mut m,
                WidgetKind::Graph,
                "The outbreak",
                &["Susceptible", "Infected", "Recovered"],
                3,
                320.0,
            ),
            w(
                &mut m,
                WidgetKind::Slider,
                "Policy levers",
                &["Contact Rate", "Infectivity", "Illness Duration"],
                1,
                320.0,
            ),
            w(
                &mut m,
                WidgetKind::Graph,
                "Infections and recoveries per day",
                &["Infection Rate", "Recovery Rate"],
                2,
                280.0,
            ),
            w(
                &mut m,
                WidgetKind::Graph,
                "Population now",
                &["Susceptible", "Infected", "Recovered"],
                1,
                280.0,
            ),
            w(
                &mut m,
                WidgetKind::Graph,
                "Phase: infected vs susceptible",
                &["Susceptible", "Infected"],
                1,
                280.0,
            ),
            w(&mut m, WidgetKind::Text, "About", &[], 4, 120.0),
        ];
        dash.widgets[4].chart = ChartKind::Stacked;
        dash.widgets[6].chart = ChartKind::Area;
        dash.widgets[7].chart = ChartKind::Donut;
        dash.widgets[8].chart = ChartKind::Phase;
        dash.widgets[9].text = "A simple outbreak model. Move the policy levers and every card, graph and table updates at once.\n\nLowering contacts or infectivity flattens and delays the peak.".into();
        m.sheets.push(dash);

        // a causal loop diagram drawn from the model, and a ready-made optimization
        let c = m.new_id();
        let mut cld = Sheet::new(c, "Causal loops", SheetKind::CausalLoop);
        cld.cld = crate::cld::from_model(&mut m);
        m.sheets.push(cld);
        let o = m.new_id();
        let mut opt = Sheet::new(o, "Optimize", SheetKind::Optimize);
        opt.opt = OptSpec {
            maximize: false,
            goal: "Infected".into(),
            measure: Measure::Peak,
            levers: vec![
                Lever {
                    name: "Contact Rate".into(),
                    on: true,
                    min: 2.0,
                    max: 10.0,
                },
                Lever {
                    name: "Infectivity".into(),
                    on: true,
                    min: 0.1,
                    max: 0.4,
                },
                Lever {
                    name: "Illness Duration".into(),
                    on: false,
                    min: 1.0,
                    max: 20.0,
                },
            ],
            limits: vec![Limit {
                var: "Recovered".into(),
                measure: Measure::Final,
                at_most: false,
                value: 5000.0,
            }],
            effort: 2,
        };
        m.sheets.push(opt);

        // three what-ifs, compared on their own sheet and in every chart
        let sc = |m: &mut Model, name: &str, changes: &[(&str, &str)]| {
            let id = m.new_id();
            m.scenarios.push(Scenario { id, name: name.into(), color: None, enabled: true, changes: changes.iter().map(|(v, x)| Change { var: v.to_string(), value: x.to_string() }).collect(), notes: String::new() });
        };
        sc(&mut m, "Distancing", &[("Contact Rate", "3")]);
        sc(&mut m, "Masks", &[("Infectivity", "0.15")]);
        sc(&mut m, "Both", &[("Contact Rate", "3"), ("Infectivity", "0.15")]);
        let sid = m.new_id();
        let mut ss = Sheet::new(sid, "Scenarios", SheetKind::Scenarios);
        ss.columns = vec!["Contact Rate".into(), "Infectivity".into()];
        ss.compare_vars = vec!["Infected".into(), "Recovered".into()];
        ss.compare_measure = Some(Measure::Peak);
        let at = m.sheets.iter().position(|s| s.kind == SheetKind::Dashboard).map(|i| i + 1).unwrap_or(m.sheets.len());
        m.sheets.insert(at, ss);
        m.ensure_sheets();
        m
    }
}

#[cfg(test)]
mod textfmt_tests {
    use super::*;

    #[test]
    fn a_plain_note_saves_without_a_format_and_old_files_still_load() {
        let m = Model::example();
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("\"fmt\""), "a plain note must not write a format");
        let back: Model = serde_json::from_str(&json).unwrap();
        assert!(back.notes[0].fmt.is_plain());
    }

    #[test]
    fn a_format_round_trips_and_partial_ones_fill_in() {
        let mut m = Model::example();
        m.notes[0].fmt = TextFmt { bold: true, size: 18.0, color: Some([1, 2, 3]), align: TextAlign::Right, ..Default::default() };
        let back: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.notes[0].fmt, m.notes[0].fmt);
        let f: TextFmt = serde_json::from_str(r#"{"italic": true}"#).unwrap();
        assert!(f.italic && !f.bold && f.points() == TextFmt::DEFAULT_SIZE);
        assert!(serde_json::from_str::<TextFmt>(r#"{"bolt": true}"#).is_err());
    }
}

#[cfg(test)]
mod shape_tests {
    use super::*;

    #[test]
    fn a_model_without_shapes_writes_none_and_shapes_survive_a_round_trip() {
        let m = Model::example();
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("\"shapes\"") && !json.contains("\"images\""));
        let mut m = m;
        let mut s = Shape::new(900, 0, ShapeKind::Arrow, [1.0, 2.0], [-30.0, 40.0]);
        s.stroke = Some([1, 2, 3]);
        s.weight = 3.0;
        m.shapes.push(s.clone());
        m.images.push(ImageAsset { id: 901, name: "a.png".into(), data: crate::b64::encode(b"not really a picture"), w: 2, h: 3 });
        let back: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.shapes, vec![s]);
        assert_eq!(back.images, m.images);
    }

    #[test]
    fn bounds_of_a_line_drawn_backwards_are_ordered() {
        let s = Shape::new(1, 0, ShapeKind::Line, [100.0, 50.0], [-40.0, 20.0]);
        assert_eq!(s.bounds(), ([60.0, 50.0], [100.0, 70.0]));
        assert!(s.kind.is_line());
    }

    #[test]
    fn the_split_format_keeps_shapes_out_of_the_meaning_file_and_puts_them_back() {
        let mut m = Model::example();
        m.shapes.push(Shape::new(900, 0, ShapeKind::Rect, [0.0, 0.0], [10.0, 10.0]));
        let (meaning, layout) = crate::io::write_split(&m).unwrap();
        assert!(!meaning.contains("\"shapes\""));
        let back = crate::io::read_split(&meaning, Some(&layout)).unwrap();
        assert_eq!(back.shapes, m.shapes);
        let alone = crate::io::read_split(&meaning, None).unwrap();
        assert!(alone.shapes.is_empty());
    }
}

#[cfg(test)]
mod control_tests {
    use super::*;

    #[test]
    fn controls_are_absent_when_unused_round_trip_and_live_in_the_layout_file() {
        let mut m = Model::example();
        assert!(!serde_json::to_string(&m).unwrap().contains("\"controls\""));
        let var = m.vars.iter().find(|v| v.kind == Kind::Constant).unwrap().id;
        let mut c = Control::new(950, 0, ControlKind::Knob, var, [5.0, 6.0]);
        c.label = "Strength".into();
        m.controls.push(c);
        let back: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.controls, m.controls);
        let (meaning, layout) = crate::io::write_split(&m).unwrap();
        assert!(!meaning.contains("\"controls\""));
        assert_eq!(crate::io::read_split(&meaning, Some(&layout)).unwrap().controls, m.controls);
        assert_eq!(ControlKind::Slider.size(), [220.0, 54.0]);
    }
}
