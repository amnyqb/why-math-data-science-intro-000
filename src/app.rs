//! Application state, the top bar, the tool strip and the status bar.
#[cfg(not(feature = "gui"))]
use crate::rfd;

use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use egui::{Align, Color32, CornerRadius, Key, Layout, RichText, Sense, Stroke, Vec2, vec2};

use crate::model::{Kind, Model, SheetKind};
use crate::sim::{self, Problem, Run};
use crate::theme::{self, Icon, Palette, ThemeChoice};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Stock,
    Flow,
    Aux,
    Constant,
    Lookup,
    Link,
    Note,
    /// Drag out a shape of `App::shape_kind`.
    Shape,
    /// Click to put down a control of `App::control_kind`.
    Control,
}

impl Tool {
    pub fn icon(self) -> Icon {
        match self {
            Tool::Select => Icon::Select,
            Tool::Stock => Icon::Stock,
            Tool::Flow => Icon::Flow,
            Tool::Aux => Icon::Aux,
            Tool::Constant => Icon::Constant,
            Tool::Lookup => Icon::Lookup,
            Tool::Link => Icon::Link,
            Tool::Note => Icon::Note,
            Tool::Shape => Icon::Stock,
            Tool::Control => Icon::Constant,
        }
    }
    pub fn help(self) -> (&'static str, &'static str, Key) {
        match self {
            Tool::Select => (
                "Select & move",
                "Click to select, drag to move. Drag empty space to pan.",
                Key::V,
            ),
            Tool::Stock => (
                "Stock",
                "Click to add an accumulation (a level that fills and drains).",
                Key::S,
            ),
            Tool::Flow => (
                "Flow",
                "Drag from one stock to another (or to empty space) to add a rate.",
                Key::F,
            ),
            Tool::Aux => ("Variable", "Click to add a calculated variable.", Key::A),
            Tool::Constant => (
                "Constant",
                "Click to add a fixed number you can tune with a slider.",
                Key::C,
            ),
            Tool::Lookup => (
                "Lookup table",
                "Click to add a graphical function drawn by hand.",
                Key::G,
            ),
            Tool::Link => (
                "Arrow",
                "Drag from a cause to an effect to connect them.",
                Key::K,
            ),
            Tool::Note => ("Note", "Click to add a text note to the diagram.", Key::N),
            Tool::Shape => ("Shape", "Drag to draw the chosen shape; click for a standard size.", Key::R),
            Tool::Control => ("Control", "Click to put a slider, knob, switch or number input on the diagram, tied to a constant.", Key::U),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sel {
    Var(u64),
    Link(u64),
    Note(u64),
    Module(u64),
    Group(u64),
    /// A drawn shape or picture.
    Shape(u64),
    /// A slider, knob, switch or number input.
    Control(u64),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BottomTab {
    Trends,
    Table,
    Causes,
    Problems,
    Runs,
}

/// A run drawn for comparison; scenarios carry their own colour.
pub struct CompareRun {
    pub run: Run,
    pub color: Option<Color32>,
}

pub struct SavedRun {
    pub run: Run,
    pub visible: bool,
}

pub struct App {
    pub model: Model,
    pub path: Option<PathBuf>,
    pub saved_model_json: String,
    pub undo: Vec<Model>,
    pub redo: Vec<Model>,
    pub tool: Tool,
    pub sel: Option<Sel>,
    pub pal: Palette,
    /// Light, dark, or following the system; `pal` is what that resolves to right now.
    pub theme_choice: ThemeChoice,
    pub pan: Vec2,
    pub zoom: f32,
    pub fit_pending: bool,
    pub canvas: crate::canvas::CanvasState,
    pub live: bool,
    pub run: Option<Run>,
    pub saved_runs: Vec<SavedRun>,
    pub problems: Vec<Problem>,
    pub run_error: Option<String>,
    pub sim_ms: f64,
    pub rev: u64,
    pub sim_rev: u64,
    pub scrub: Option<usize>,
    pub bottom_tab: BottomTab,
    pub plotted: Vec<String>,
    pub separate_charts: bool,
    pub status: String,
    pub focus_name: bool,
    pub eq_cursor: Option<usize>,
    pub screenshot: Option<(PathBuf, Arc<AtomicBool>)>,
    pub frame: u64,
    pub show_bottom: bool,
    /// The panel on the right (details of the selection, causal loop list).
    pub show_right: bool,
    /// The sensitivity-runs window and its state.
    pub sens_open: bool,
    pub sens: crate::sens::SensState,
    /// The outline tree on the left.
    pub show_outline: bool,
    /// Which ribbon tab is showing, and whether the ribbon body is open.
    pub ribbon_tab: crate::ribbon::RibbonTab,
    pub ribbon_open: bool,
    /// What the Shape tool draws.
    pub shape_kind: crate::model::ShapeKind,
    /// What the Control tool puts down.
    pub control_kind: crate::model::ControlKind,
    /// Decoded pictures and their textures.
    pub pictures: crate::pictures::PictureCache,
    /// What was last written to the preferences file.
    last_prefs: Option<Prefs>,
    /// Is the "My functions" window open?
    pub functions_open: bool,
    /// Is the "Model checks" window open?
    pub checks_open: bool,
    /// The Layers window and the tags on the diagram.
    pub layers: crate::layers_ui::State,
    /// The Model health panel and the doctor's last reports.
    pub health: crate::health_ui::State,
    pub arch: crate::architecture::State,
    pub layer_cache: std::cell::RefCell<Option<(u64, std::rc::Rc<crate::layers_ui::LayerView>)>>,
    /// Is the "Reusable modules" window open?
    pub modules_open: bool,
    pub history: crate::history_ui::State,
    /// The "Compare with data" window and its choices.
    pub compare_open: bool,
    pub compare: crate::forecast::State,
    /// The "Function from Python" window and its form.
    pub py_open: bool,
    pub py_form: crate::pyfn::PyForm,
    /// Is the key-levers card on the diagram unfolded?
    pub levers_open: bool,
    /// What the model is shown for: set name → the item (or group) picked in the Showing bar.
    pub focus: crate::sets::Focus,
    pub show_help: bool,
    /// The sheet being shown.
    pub sheet: u64,
    /// A sheet tab being renamed, with the text typed so far.
    pub renaming: Option<(u64, String)>,
    pub dash_edit: bool,
    /// Playing a run in: when it started (seconds) and how far along it is (0–1).
    pub reveal_start: Option<f64>,
    pub reveal_p: Option<f32>,
    pub reveal_secs: f32,
    pub animate: bool,
    pub cld: crate::cld::CldState,
    pub opt: crate::optimize::OptState,
    /// Everything selected on the diagram besides (or including) `sel`.
    pub multi: Vec<Sel>,
    pub clip: Option<crate::arrange::Clip>,
    pub paste_count: u32,
    pub snap: bool,
    pub show_minimap: bool,
    pub palette: Option<(String, usize)>,
    pub show_shortcuts: bool,
    pub show_about: bool,
    pub import_dialog: Option<crate::modfile::ImportDialog>,
    /// The function reference window.
    pub functions: Option<crate::docs::Browser>,
    /// Text the function reference wants inserted into a variable's equation.
    pub pending_insert: Option<(u64, String)>,
    /// The equation editor's suggestion list, and where it was last drawn.
    pub ac: Option<crate::inspector::Ac>,
    pub ac_rect: Option<egui::Rect>,
    pub ac_dismissed: Option<(u64, String)>,
    pub trend_chart: crate::model::ChartKind,
    /// Pixels per point for exported pictures (3 ≈ 288 dpi).
    pub export_scale: f32,
    pub pending_export: Option<crate::export::ExportJob>,
    /// The chart an ExportWhat::Chart export redraws.
    pub export_chart: Option<crate::export::ChartRef>,
    /// Results of each enabled scenario (by scenario id).
    pub scenario_runs: Vec<(u64, Result<Run, String>)>,
    /// Draw enabled scenarios in every chart, not only on the Scenarios sheet.
    pub scenarios_everywhere: bool,
    /// The chart being customized, and whether an undo step was taken for it.
    pub chart_editor: Option<(crate::chartstyle::ChartTarget, bool)>,
    /// Notes from the last spreadsheet import, shown in a window.
    pub import_notes: Option<Vec<String>>,
    /// What the last import or export from another format dropped or approximated.
    pub conversion: Option<crate::convert_ui::Conversion>,
    /// Suggestions for the model as of this revision, so the Problems tab does not recompute them every frame.
    pub warn_cache: std::cell::RefCell<Option<(u64, Vec<(u64, String)>)>>,
    /// The Inbox window, and a copy kept only for the top-bar badge.
    pub inbox: Option<crate::inbox::Inbox>,
    pub inbox_badge: crate::inbox::Inbox,
    /// True while a sheet is being drawn for export (hides editing controls).
    pub exporting: bool,
    /// While exporting a selection: the ids to draw.
    pub export_only: Option<Vec<u64>>,
    /// Saves without asking where (used by the --export-png start option).
    pub export_path: Option<PathBuf>,
    pub cli_export: Option<(crate::export::ExportWhat, PathBuf)>,
    /// Colours (and so on) a figure export should use instead of the theme's.
    pub export_style: Option<dynamist_core::figstyle::FigStyle>,
    pub cli_frame: u32,
}

/// Set by the headless renderer: no preferences file is read or written, nothing depends on the machine.
pub static HEADLESS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn is_headless() -> bool {
    HEADLESS.load(std::sync::atomic::Ordering::Relaxed)
}

impl App {
    pub fn new(ctx: &egui::Context, screenshot: Option<PathBuf>) -> Self {
        let pal = Palette::light();
        theme::apply(ctx, &pal);
        let model = Model::example();
        let json = serde_json::to_string(&model).unwrap_or_default();
        let mut app = App {
            model,
            path: None,
            saved_model_json: json,
            undo: Vec::new(),
            redo: Vec::new(),
            tool: Tool::Select,
            sel: None,
            pal,
            theme_choice: ThemeChoice::Light,
            pan: Vec2::ZERO,
            zoom: 1.0,
            fit_pending: true,
            canvas: Default::default(),
            live: true,
            run: None,
            saved_runs: Vec::new(),
            problems: Vec::new(),
            run_error: None,
            sim_ms: 0.0,
            rev: 1,
            sim_rev: 0,
            scrub: None,
            bottom_tab: BottomTab::Trends,
            plotted: vec!["Susceptible".into(), "Infected".into(), "Recovered".into()],
            separate_charts: false,
            status: "Ready".into(),
            focus_name: false,
            eq_cursor: None,
            screenshot: screenshot.map(|p| (p, Arc::new(AtomicBool::new(false)))),
            frame: 0,
            show_bottom: true,
            show_right: true,
            sens_open: false,
            sens: Default::default(),
            show_outline: false,
            ribbon_tab: crate::ribbon::RibbonTab::Home,
            ribbon_open: true,
            shape_kind: crate::model::ShapeKind::Rect,
            control_kind: crate::model::ControlKind::Slider,
            pictures: Default::default(),
            last_prefs: None,
            functions_open: false,
            checks_open: false,
            layers: Default::default(),
            health: Default::default(),
            arch: Default::default(),
            layer_cache: std::cell::RefCell::new(None),
            modules_open: false,
            history: crate::history_ui::State { ops_gate: true, ..Default::default() },
            compare_open: false,
            compare: Default::default(),
            py_open: false,
            py_form: Default::default(),
            levers_open: true,
            focus: Default::default(),
            show_help: false,
            sheet: 0,
            renaming: None,
            dash_edit: false,
            reveal_start: None,
            reveal_p: None,
            reveal_secs: 3.0,
            animate: true,
            cld: Default::default(),
            opt: Default::default(),
            multi: Vec::new(),
            clip: None,
            paste_count: 0,
            snap: true,
            show_minimap: true,
            palette: None,
            show_shortcuts: false,
            show_about: false,
            import_dialog: None,
            functions: None,
            pending_insert: None,
            ac: None,
            ac_rect: None,
            ac_dismissed: None,
            trend_chart: Default::default(),
            export_scale: 3.0,
            pending_export: None,
            export_chart: None,
            scenario_runs: Vec::new(),
            scenarios_everywhere: true,
            chart_editor: None,
            import_notes: None,
            conversion: None,
            warn_cache: std::cell::RefCell::new(None),
            inbox: None,
            inbox_badge: Default::default(),
            exporting: false,
            export_only: None,
            export_path: None,
            cli_export: None,
            export_style: None,
            cli_frame: 0,
        };
        app.sheet = app.model.first_diagram();
        app.simulate();
        let i = app
            .model
            .vars
            .iter()
            .find(|v| v.name == "Infected")
            .map(|v| v.id);
        app.sel = i.map(Sel::Var);
        // how the person left the window last time (not for screenshots and tests)
        if app.screenshot.is_none() && !cfg!(test) && !is_headless() {
            if let Some(p) = Prefs::load() {
                app.show_bottom = p.show_bottom;
                app.show_right = p.show_right;
                app.levers_open = p.levers_open;
                let choice = p.theme.unwrap_or(if p.dark { ThemeChoice::Dark } else { ThemeChoice::Light });
                app.set_theme_choice(ctx, choice);
                app.last_prefs = Some(p);
            }
        }
        app
    }

    /// The layout choices worth remembering between sessions.
    fn prefs(&self) -> Prefs {
        Prefs { show_bottom: self.show_bottom, show_right: self.show_right, levers_open: self.levers_open, dark: self.pal.dark, theme: Some(self.theme_choice) }
    }

    /// Save them when they change.
    fn remember_prefs(&mut self) {
        if self.cli_export.is_some() || cfg!(test) || is_headless() {
            return;
        }
        let now = self.prefs();
        if self.last_prefs.as_ref() != Some(&now) {
            now.save();
            self.last_prefs = Some(now);
        }
    }

    /// Record the model before a change, so it can be undone.
    pub fn checkpoint(&mut self) {
        self.undo.push(self.model.clone());
        if self.undo.len() > 200 {
            self.undo.remove(0);
        }
        self.redo.clear();
    }
    pub fn changed(&mut self) {
        self.rev += 1;
    }
    pub fn undo(&mut self) {
        if let Some(m) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.model, m));
            self.after_history();
        }
    }
    pub fn redo(&mut self) {
        if let Some(m) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.model, m));
            self.after_history();
        }
    }
    fn after_history(&mut self) {
        if !self.model.sheets.iter().any(|s| s.id == self.sheet) {
            self.sheet = self.model.first_diagram();
        }
        if let Some(Sel::Var(id)) = self.sel {
            if self.model.var(id).is_none() {
                self.sel = None;
            }
        }
        self.changed();
    }
    pub fn is_dirty(&self) -> bool {
        serde_json::to_string(&self.model).unwrap_or_default() != self.saved_model_json
    }

    pub fn simulate(&mut self) {
        let start = std::time::Instant::now();
        self.sim_rev = self.rev;
        // forget picks that no longer exist (after an import, or when an item is renamed)
        let sets = self.model.sets.clone();
        self.focus.retain(|k, v| sets.iter().any(|s| &s.name == k && s.items.contains(v)));
        match sim::compile(&self.model) {
            Ok(c) => {
                self.problems.clear();
                match sim::simulate(&c, &self.model, "Current") {
                    Ok(mut r) => {
                        crate::sets::apply_focus(&mut r, &self.model, &self.focus);
                        self.run = Some(r);
                        self.run_error = None;
                    }
                    Err(e) => self.run_error = Some(e),
                }
            }
            Err(p) => {
                self.problems = p;
                self.run_error = Some(format!(
                    "{} problem(s) stop the simulation",
                    self.problems.len()
                ));
            }
        }
        self.run_scenarios();
        self.sim_ms = start.elapsed().as_secs_f64() * 1000.0;
        if let Some(r) = &self.run {
            if self.scrub.is_some_and(|s| s >= r.times.len()) {
                self.scrub = None;
            }
        }
    }

    /// Run every enabled scenario: the model with its changes applied.
    pub fn run_scenarios(&mut self) {
        let mut out = Vec::new();
        for sc in self.model.scenarios.iter().filter(|s| s.enabled) {
            let mut m = self.model.clone();
            let mut missing = Vec::new();
            for c in &sc.changes {
                if c.value.trim().is_empty() {
                    continue;
                }
                if !m.apply_change(&c.var, c.value.trim()) {
                    missing.push(c.var.clone());
                }
            }
            let res = if !missing.is_empty() {
                Err(format!("No variable called {}", missing.join(", ")))
            } else {
                match sim::compile(&m) {
                    Ok(c) => sim::simulate(&c, &m, &sc.name).map(|mut r| {
                        crate::sets::apply_focus(&mut r, &m, &self.focus);
                        r
                    }),
                    Err(p) => Err(p.first().map(|p| format!("{}: {}", p.var.clone().unwrap_or_default(), p.message)).unwrap_or_default()),
                }
            };
            out.push((sc.id, res));
        }
        self.scenario_runs = out;
    }

    /// The colour a scenario is drawn in.
    pub fn scenario_color(&self, id: u64) -> Color32 {
        let i = self.model.scenarios.iter().position(|s| s.id == id).unwrap_or(0);
        match self.model.scenarios.get(i).and_then(|s| s.color) {
            Some([r, g, b]) => Color32::from_rgb(r, g, b),
            None => {
                // a palette of its own, so scenarios don't look like variables
                const C: [(u8, u8, u8); 6] = [(219, 39, 119), (202, 138, 4), (8, 145, 178), (101, 163, 13), (147, 51, 234), (220, 38, 38)];
                let (r, g, b) = C[i % C.len()];
                if self.pal.dark { Color32::from_rgb(r.saturating_add(40), g.saturating_add(40), b.saturating_add(40)) } else { Color32::from_rgb(r, g, b) }
            }
        }
    }

    /// Runs to draw next to the current one: kept runs, then the enabled scenarios.
    pub fn compare_runs(&self) -> Vec<CompareRun> {
        let mut v: Vec<CompareRun> = self.saved_runs.iter().filter(|s| s.visible).map(|s| CompareRun { run: s.run.clone(), color: None }).collect();
        if self.scenarios_everywhere {
            for (id, r) in &self.scenario_runs {
                if let Ok(run) = r {
                    v.push(CompareRun { run: run.clone(), color: Some(self.scenario_color(*id)) });
                }
            }
        }
        v
    }

    /// How a comparison run is drawn: scenarios in their own colour (solid when a chart shows
    /// one variable, dashed alongside several); kept runs faint and dashed in the variable's colour.
    pub fn compare_style(&self, cr: &CompareRun, base: Color32, single: bool) -> (Color32, egui_plot::LineStyle, f32) {
        match cr.color {
            Some(t) if single => (t, egui_plot::LineStyle::Solid, 2.0),
            Some(t) => (t.gamma_multiply(0.85), egui_plot::LineStyle::dashed_dense(), 1.6),
            None => (base.gamma_multiply(0.45), egui_plot::LineStyle::dashed_loose(), 1.5),
        }
    }

    /// Simulate, then play the result in over a few seconds.
    pub fn run_and_play(&mut self, ctx: &egui::Context) {
        self.simulate();
        if self.animate && self.run.is_some() {
            self.reveal_start = Some(ctx.input(|i| i.time));
            self.reveal_p = Some(0.0);
            self.scrub = Some(0);
        }
        self.status = "Simulated".into();
    }

    /// How many points of an `n`-point series to draw right now.
    pub fn reveal_len(&self, n: usize) -> usize {
        match self.reveal_p {
            Some(p) => ((p * n.saturating_sub(1) as f32).floor() as usize + 1).clamp(1, n.max(1)),
            None => n,
        }
    }

    /// A series as plot points, cut short while a run is playing in.
    pub fn plot_pts(&self, run: &Run, name: &str) -> Option<Vec<[f64; 2]>> {
        let v = run.get(name)?;
        let n = self.reveal_len(v.len());
        Some(
            run.times
                .iter()
                .zip(v)
                .take(n)
                .map(|(t, y)| [*t, *y])
                .collect(),
        )
    }

    /// While playing, fix the axes to the whole run so they don't jump as lines grow.
    pub fn hold_axes<'a>(
        &self,
        mut plot: egui_plot::Plot<'a>,
        run: &Run,
        names: &[String],
    ) -> egui_plot::Plot<'a> {
        if self.reveal_p.is_none() || run.times.is_empty() {
            return plot;
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut runs: Vec<&Run> = vec![run];
        runs.extend(self.saved_runs.iter().filter(|s| s.visible).map(|s| &s.run));
        for r in runs {
            for n in names {
                for v in r.get(n).into_iter().flatten().filter(|v| v.is_finite()) {
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                }
            }
        }
        plot = plot
            .include_x(run.times[0])
            .include_x(*run.times.last().unwrap());
        if lo.is_finite() {
            plot = plot.include_y(lo).include_y(hi);
        }
        plot
    }

    /// A dot at the moving tip of a line while the run plays.
    pub fn draw_head(&self, pu: &mut egui_plot::PlotUi, pts: &[[f64; 2]], color: Color32) {
        if self.reveal_p.is_some() {
            if let Some(last) = pts.last() {
                pu.points(
                    egui_plot::Points::new("", vec![*last])
                        .radius(4.5)
                        .color(color),
                );
            }
        }
    }

    fn tick_reveal(&mut self, ctx: &egui::Context) {
        let Some(start) = self.reveal_start else {
            return;
        };
        let p = ((ctx.input(|i| i.time) - start) as f32 / self.reveal_secs.max(0.2)).max(0.0);
        let n = self.run.as_ref().map(|r| r.times.len()).unwrap_or(0);
        if p >= 1.0 || n == 0 {
            self.reveal_start = None;
            self.reveal_p = None;
            self.scrub = None;
        } else {
            self.reveal_p = Some(p);
            self.scrub = Some(self.reveal_len(n) - 1);
            ctx.request_repaint();
        }
    }

    /// Index into the current run's time axis that the diagram shows.
    pub fn time_index(&self) -> Option<usize> {
        let r = self.run.as_ref()?;
        Some(
            self.scrub
                .unwrap_or(r.times.len().saturating_sub(1))
                .min(r.times.len().saturating_sub(1)),
        )
    }

    pub fn value_now(&self, name: &str) -> Option<f64> {
        let i = self.time_index()?;
        self.run.as_ref()?.get(name).and_then(|s| s.get(i).copied())
    }

    // ---------- files ----------

    pub fn new_model(&mut self) {
        self.checkpoint();
        self.model = Model {
            title: "Untitled model".into(),
            ..Default::default()
        };
        self.model.settings.final_time = 100.0;
        self.model.ensure_sheets();
        self.sheet = self.model.first_diagram();
        self.path = None;
        self.sel = None;
        self.run = None;
        self.plotted.clear();
        self.saved_runs.clear();
        self.saved_model_json = serde_json::to_string(&self.model).unwrap_or_default();
        self.changed();
        self.fit_pending = true;
        self.status = "New model".into();
    }

    pub fn open(&mut self) {
        let Some(p) = rfd::FileDialog::new()
            .add_filter("Dynamist model", &["sdm", "flowmodel", "json"])
            .add_filter("Vensim model", &["mdl"])
            .add_filter("XMILE model (Stella)", &["xmile", "stmx", "itmx", "xml"])
            .pick_file()
        else {
            return;
        };
        self.open_path(p);
    }

    pub fn open_path(&mut self, p: PathBuf) {
        let text = match dynamist_core::io::read_text(&p) {
            Ok(t) => t,
            Err(e) => {
                self.status = format!("Couldn't open: {e}");
                return;
            }
        };
        match dynamist_core::io::detect(&p.to_string_lossy(), &text) {
            Some(f @ (dynamist_core::io::Format::Mdl | dynamist_core::io::Format::Xmile)) => {
                self.import_foreign(p, &text, f);
            }
            _ => match serde_json::from_str::<Model>(&text) {
                Ok(mut m) => {
                    self.checkpoint();
                    self.path = Some(p.clone());
                    self.saved_model_json = serde_json::to_string(&m).unwrap_or_default();
                    self.status = format!("Opened {}", p.display());
                    m.ensure_sheets();
                    self.finish_open(m);
                }
                Err(e) => self.status = format!("Couldn't read the model: {e}"),
            },
        }
    }

    /// Make an opened or imported model the current one.
    pub fn finish_open(&mut self, mut m: Model) {
        m.ensure_sheets();
        self.sheet = m.first_diagram();
        self.model = m;
        // a figure keeps the stocks and clouds where the model has them
        if !crate::canvas::raw_pipes() {
            self.straighten_flows(true);
        }
        self.sel = None;
        self.saved_runs.clear();
        self.plotted = self.model.vars.iter().filter(|v| v.kind == Kind::Stock).take(4).map(|v| v.name.clone()).collect();
        self.fit_pending = true;
        self.changed();
        self.simulate();
    }

    pub fn save(&mut self, save_as: bool) {
        let path = match (&self.path, save_as) {
            (Some(p), false) => p.clone(),
            _ => {
                let name = format!(
                    "{}.sdm",
                    if self.model.title.is_empty() {
                        "model"
                    } else {
                        &self.model.title
                    }
                );
                let Some(p) = rfd::FileDialog::new()
                    .add_filter("Dynamist model", &["sdm"])
                    .set_file_name(name)
                    .save_file()
                else {
                    return;
                };
                p
            }
        };
        match serde_json::to_string_pretty(&self.model)
            .map_err(|e| e.to_string())
            .and_then(|s| std::fs::write(&path, s).map_err(|e| e.to_string()))
        {
            Ok(()) => {
                self.saved_model_json = serde_json::to_string(&self.model).unwrap_or_default();
                self.status = format!("Saved {}", path.display());
                self.path = Some(path);
            }
            Err(e) => self.status = format!("Couldn't save: {e}"),
        }
    }

    pub fn export_csv(&mut self) {
        let Some(run) = &self.run else { return };
        let Some(p) = rfd::FileDialog::new()
            .add_filter("CSV", &["csv"])
            .set_file_name("results.csv")
            .save_file()
        else {
            return;
        };
        let vars: Vec<&crate::model::Var> = self
            .model
            .vars
            .iter()
            .filter(|v| run.get(&v.name).is_some())
            .collect();
        let mut s = String::from("Time");
        for v in &vars {
            s.push_str(&format!(",\"{}\"", v.name));
        }
        s.push('\n');
        for (i, t) in run.times.iter().enumerate() {
            s.push_str(&t.to_string());
            for v in &vars {
                s.push_str(&format!(",{}", run.get(&v.name).unwrap()[i]));
            }
            s.push('\n');
        }
        self.status = match std::fs::write(&p, s) {
            Ok(()) => format!("Exported results to {}", p.display()),
            Err(e) => format!("Couldn't export: {e}"),
        };
    }

    pub fn keep_run(&mut self) {
        if let Some(r) = &self.run {
            let mut r = r.clone();
            r.name = format!("Run {}", self.saved_runs.len() + 1);
            self.saved_runs.push(SavedRun {
                run: r,
                visible: true,
            });
            self.status = "Kept this run for comparison".into();
        }
    }

    pub fn delete_selection(&mut self) {
        let items = self.selected_items();
        let link = if let Some(Sel::Link(id)) = self.sel {
            Some(id)
        } else {
            None
        };
        if items.is_empty() && link.is_none() {
            return;
        }
        self.checkpoint();
        if let Some(id) = link {
            self.model.links.retain(|l| l.id != id);
        }
        let n = items.len();
        for s in items {
            match s {
                Sel::Var(id) => self.model.remove_var(id),
                Sel::Note(id) => self.model.notes.retain(|x| x.id != id),
                Sel::Shape(id) => self.model.shapes.retain(|x| x.id != id),
                Sel::Control(id) => self.model.controls.retain(|x| x.id != id),
                Sel::Module(id) => self.remove_module(id),
                _ => {}
            }
        }
        self.model.ensure_sheets();
        self.select_one(None);
        if n > 1 {
            self.status = format!("Deleted {n} items (⌘Z brings them back)");
        }
        self.changed();
    }

    /// Delete a module together with everything inside it.
    pub fn remove_module(&mut self, mid: u64) {
        let Some(m) = self.model.module(mid).cloned() else {
            return;
        };
        let inner: Vec<u64> = self
            .model
            .modules
            .iter()
            .filter(|x| x.view == m.inner)
            .map(|x| x.id)
            .collect();
        for x in inner {
            self.remove_module(x);
        }
        let vars: Vec<u64> = self
            .model
            .vars
            .iter()
            .filter(|v| v.view == m.inner)
            .map(|v| v.id)
            .collect();
        for v in vars {
            self.model.remove_var(v);
        }
        self.model.notes.retain(|n| n.view != m.inner);
        self.model.sheets.retain(|s| s.id != m.inner);
        self.model.modules.retain(|x| x.id != mid);
        if self.sheet == m.inner {
            self.sheet = m.view;
        }
    }

    /// Switch to light or dark for good (this stops following the system).
    pub fn set_dark(&mut self, ctx: &egui::Context, dark: bool) {
        self.set_theme_choice(ctx, if dark { ThemeChoice::Dark } else { ThemeChoice::Light });
    }

    /// Light, dark, or follow the system; applies it at once.
    pub fn set_theme_choice(&mut self, ctx: &egui::Context, choice: ThemeChoice) {
        self.theme_choice = choice;
        self.follow_theme(ctx);
    }

    /// Bring the palette in line with the choice (and, for System, with what the system says now).
    fn follow_theme(&mut self, ctx: &egui::Context) {
        let dark = self.theme_choice.is_dark(ctx.system_theme());
        if dark != self.pal.dark || ctx.theme() != if dark { egui::Theme::Dark } else { egui::Theme::Light } {
            self.pal = if dark { Palette::dark() } else { Palette::light() };
            theme::apply(ctx, &self.pal);
        }
    }

    // ---------- keyboard ----------

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.egui_wants_keyboard_input();
        let (cmd, shift) = ctx.input(|i| (i.modifiers.command, i.modifiers.shift));
        let pressed = |k: Key| ctx.input(|i| i.key_pressed(k));
        if cmd && pressed(Key::Z) {
            if shift { self.redo() } else { self.undo() }
        }
        if cmd && pressed(Key::Y) {
            self.redo();
        }
        if cmd && pressed(Key::S) {
            self.save(shift);
        }
        if cmd && pressed(Key::O) {
            self.open();
        }
        if cmd && pressed(Key::R) {
            self.run_and_play(ctx);
        }
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        if !typing && self.palette.is_none() {
            let (copy, cut, paste, alt) = ctx.input(|i| {
                (
                    i.events.iter().any(|e| matches!(e, egui::Event::Copy)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Cut)),
                    i.events.iter().any(|e| matches!(e, egui::Event::Paste(_))),
                    i.modifiers.alt,
                )
            });
            if copy {
                use crate::export::{ExportTo, ExportWhat};
                // ⌘C also puts a picture on the clipboard, so it pastes straight into Word or PowerPoint
                if shift {
                    self.request_export(ExportWhat::Sheet, ExportTo::Clipboard);
                } else if diagram {
                    if self.copy(false) {
                        self.request_export(ExportWhat::Selection, ExportTo::Clipboard);
                    } else {
                        self.request_export(ExportWhat::Sheet, ExportTo::Clipboard);
                    }
                } else {
                    self.request_export(ExportWhat::Sheet, ExportTo::Clipboard);
                }
            }
            if diagram && cut {
                self.copy(true);
            }
            if paste {
                self.paste();
            }
            if cmd && pressed(Key::K) {
                self.palette = Some((String::new(), 0));
            }
            if cmd && pressed(Key::J) {
                self.show_bottom = !self.show_bottom;
            }
            if cmd && pressed(Key::B) {
                self.show_right = !self.show_right;
            }
            if diagram && cmd {
                if pressed(Key::A) {
                    self.select_all();
                }
                if pressed(Key::D) {
                    self.duplicate();
                }
                if pressed(Key::G) {
                    if shift {
                        self.ungroup()
                    } else {
                        self.group_selection()
                    }
                }
                if shift && pressed(Key::M) {
                    self.make_module();
                }
                if alt {
                    for (k, op) in [
                        (Key::ArrowLeft, crate::arrange::Align::Left),
                        (Key::ArrowRight, crate::arrange::Align::Right),
                        (Key::ArrowUp, crate::arrange::Align::Top),
                        (Key::ArrowDown, crate::arrange::Align::Bottom),
                    ] {
                        if pressed(k) {
                            self.align(op);
                        }
                    }
                }
            }
            if diagram && !cmd {
                let step = if self.snap { crate::arrange::GRID } else { 1.0 }
                    * if shift { 4.0 } else { 1.0 };
                for (k, d) in [
                    (Key::ArrowLeft, vec2(-step, 0.0)),
                    (Key::ArrowRight, vec2(step, 0.0)),
                    (Key::ArrowUp, vec2(0.0, -step)),
                    (Key::ArrowDown, vec2(0.0, step)),
                ] {
                    if pressed(k) {
                        self.nudge(d);
                    }
                }
            }
            if pressed(Key::F1) && self.functions.is_none() {
                let t = if let Some(Sel::Var(v)) = self.sel { Some(v) } else { None };
                self.open_functions(t);
            }
            if !cmd && pressed(Key::Questionmark) {
                self.show_shortcuts = !self.show_shortcuts;
            }
        }
        if typing || cmd {
            return;
        }
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            match self.active_sheet_kind() {
                SheetKind::Diagram => self.delete_selection(),
                SheetKind::CausalLoop => self.cld_delete(),
                _ => {}
            }
        }
        if pressed(Key::Escape) {
            if self.tool == Tool::Select {
                self.select_one(None);
            }
            self.tool = Tool::Select;
            self.canvas.drag = None;
        }
        for t in TOOLS {
            if pressed(t.help().2) {
                self.tool = t;
            }
        }
        if pressed(Key::Num0) {
            self.fit_pending = true;
        }
    }

    // ---------- top bar ----------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal_centered(|ui| {
            // a simple brand mark: three nested shapes
            let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
            let p = ui.painter();
            p.rect_filled(r, CornerRadius::same(7), pal.accent);
            let c = r.center();
            p.rect_stroke(
                egui::Rect::from_center_size(c + vec2(-3.5, 0.0), vec2(9.0, 7.0)),
                CornerRadius::same(2),
                Stroke::new(1.6, Color32::WHITE),
                egui::StrokeKind::Middle,
            );
            p.line_segment(
                [c + vec2(1.0, 0.0), c + vec2(8.0, 0.0)],
                Stroke::new(1.6, Color32::WHITE),
            );
            p.circle_filled(c + vec2(8.0, 0.0), 2.0, Color32::WHITE);
            ui.label(
                RichText::new("Dynamist")
                    .strong()
                    .size(15.0)
                    .color(pal.text),
            );
            ui.add_space(6.0);

            use crate::icons::Ic;
            self.ribbon_tabs(ui);

            // quick actions
            ui.add_space(6.0);
            let (fg, ac) = (pal.text.gamma_multiply(0.8), pal.accent);
            if crate::icons::tool(ui, Ic::Undo, "Undo (⌘Z)", fg, ac).clicked() {
                self.undo();
            }
            if crate::icons::tool(ui, Ic::Redo, "Redo (⇧⌘Z)", fg, ac).clicked() {
                self.redo();
            }
            if crate::icons::tool(ui, Ic::Save, "Save (⌘S)", fg, ac).clicked() {
                self.save(false);
            }
            let waiting = self.inbox_waiting();
            let r = crate::icons::tool(ui, Ic::Mail, &format!("Inbox: {waiting} open message(s) between you, the builder and other conversations"), fg, ac);
            if waiting > 0 {
                let c = r.rect.right_top() + vec2(-4.0, 6.0);
                ui.painter().circle_filled(c, 7.0, pal.aux);
                ui.painter().text(c, egui::Align2::CENTER_CENTER, waiting.min(9).to_string(), egui::FontId::proportional(10.0), Color32::WHITE);
            }
            if r.clicked() {
                self.open_inbox(crate::inbox::Tab::Open);
            }
            if crate::icons::tool(ui, Ic::Search, "Command palette (⌘K)", fg, ac).clicked() {
                self.palette = Some((String::new(), 0));
            }
            if crate::icons::tool(ui, if pal.dark { Ic::Sun } else { Ic::Moon }, if self.theme_choice == ThemeChoice::System { "Light / dark (now following the system; View menu for more)" } else { "Light / dark (View menu: follow the system)" }, fg, ac).clicked() {
                let ctx = ui.ctx().clone();
                self.set_dark(&ctx, !pal.dark);
            }
            // where things show: the results panel below and the side panel on the right
            if crate::icons::button(ui, Ic::PanelBottom, None, self.show_bottom, fg, ac).on_hover_text("Show or hide the results panel (⌘J)").clicked() {
                self.show_bottom = !self.show_bottom;
            }
            if crate::icons::button(ui, Ic::PanelRight, None, self.show_right, fg, ac).on_hover_text("Show or hide the side panel (⌘B)").clicked() {
                self.show_right = !self.show_right;
            }
            ui.add_space(10.0);
            // the title gives way when the bar gets crowded, so it never runs into the time controls
            let title_w = (ui.available_width() - 330.0).clamp(70.0, 220.0);
            let r = ui.add(
                egui::TextEdit::singleline(&mut self.model.title)
                    .desired_width(title_w)
                    .frame(egui::Frame::NONE)
                    .font(egui::TextStyle::Body)
                    .text_color(pal.muted),
            );
            if r.gained_focus() {
                self.checkpoint();
            }
            if self.is_dirty() {
                ui.label(RichText::new("●").color(pal.aux).small())
                    .on_hover_text("Unsaved changes");
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                // Run: a filled button with a play icon
                let (r, resp) = ui.allocate_exact_size(vec2(88.0, 32.0), Sense::click());
                ui.painter().rect_filled(r, CornerRadius::same(8), if resp.hovered() { pal.accent.gamma_multiply(0.88) } else { pal.accent });
                crate::icons::paint(ui.painter(), egui::Rect::from_min_size(egui::pos2(r.left() + 14.0, r.center().y - 8.0), vec2(16.0, 16.0)), crate::icons::Ic::Play, Color32::WHITE);
                ui.painter().text(egui::pos2(r.left() + 38.0, r.center().y), egui::Align2::LEFT_CENTER, "Run", egui::FontId::proportional(14.5), Color32::WHITE);
                if resp.on_hover_text("Simulate and watch it unfold over time (⌘R)").clicked() {
                    let ctx = ui.ctx().clone();
                    self.run_and_play(&ctx);
                }
                if crate::icons::tool(ui, crate::icons::Ic::Pin, "Keep this run to compare against later runs", pal.text, pal.accent).clicked() {
                    self.keep_run();
                }
                toggle(ui, &mut self.live, "Live", pal).on_hover_text("Re-simulate instantly as you edit");
            });
        });
    }

    fn tool_strip(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            for (i, t) in TOOLS.into_iter().enumerate() {
                if i == 1 || i == 6 {
                    ui.add_space(4.0);
                    let (r, _) = ui.allocate_exact_size(vec2(28.0, 1.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::ZERO, pal.border);
                    ui.add_space(4.0);
                }
                let (name, help, key) = t.help();
                let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                let active = self.tool == t;
                let color = match t {
                    Tool::Stock => pal.stock,
                    Tool::Flow => pal.flow,
                    Tool::Aux => pal.aux,
                    Tool::Constant => pal.constant,
                    Tool::Lookup => pal.lookup,
                    _ => pal.text,
                };
                if active {
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(9), pal.accent_soft);
                    ui.painter().rect_stroke(
                        r,
                        CornerRadius::same(9),
                        Stroke::new(1.0, pal.accent.gamma_multiply(0.5)),
                        egui::StrokeKind::Inside,
                    );
                } else if resp.hovered() {
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(9), pal.surface);
                }
                theme::draw_icon(
                    ui.painter(),
                    t.icon(),
                    r.shrink(7.0),
                    if active || resp.hovered() {
                        color
                    } else {
                        color.gamma_multiply(0.8)
                    },
                );
                let resp = resp.on_hover_ui(|ui| {
                    ui.label(RichText::new(format!("{name}  ({})", key.name())).strong());
                    ui.label(help);
                });
                if resp.clicked() {
                    self.tool = t;
                }
                ui.add_space(2.0);
            }
            ui.add_space(10.0);
            let (r, resp) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
            if resp.hovered() && self.sel.is_some() {
                ui.painter()
                    .rect_filled(r, CornerRadius::same(9), pal.error.gamma_multiply(0.12));
            }
            theme::draw_icon(
                ui.painter(),
                Icon::Delete,
                r.shrink(8.0),
                if self.sel.is_some() {
                    pal.error
                } else {
                    pal.muted.gamma_multiply(0.5)
                },
            );
            if resp.on_hover_text("Delete the selection (⌫)").clicked() {
                self.delete_selection();
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal_centered(|ui| {
            let (txt, col) = match (&self.run_error, &self.run) {
                (Some(e), _) => (format!("⚠ {e}"), pal.error),
                (None, Some(r)) => (
                    format!(
                        "✓ Simulated {} steps in {:.1} ms",
                        r.times.len().saturating_sub(1),
                        self.sim_ms
                    ),
                    pal.ok,
                ),
                _ => ("Not simulated yet".to_string(), pal.muted),
            };
            let r = ui.label(RichText::new(txt).small().color(col));
            if self.run_error.is_some() && r.on_hover_text("Open the Problems tab").clicked() {
                self.bottom_tab = BottomTab::Problems;
            }
            ui.separator();
            ui.label(RichText::new(&self.status).small().color(pal.muted));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                self.gate_badge(ui);
                ui.separator();
                ui.label(
                    RichText::new(format!("{:.0}%", self.zoom * 100.0))
                        .small()
                        .color(pal.muted),
                );
                ui.separator();
                let counts = |k: Kind| self.model.vars.iter().filter(|v| v.kind == k).count();
                ui.label(
                    RichText::new(format!(
                        "{} stocks · {} flows · {} variables · {} arrows",
                        counts(Kind::Stock),
                        counts(Kind::Flow),
                        counts(Kind::Aux) + counts(Kind::Constant) + counts(Kind::Lookup),
                        self.model.links.len()
                    ))
                    .small()
                    .color(pal.muted),
                );
            });
        });
    }

    fn screenshot_tick(&mut self, ctx: &egui::Context) {
        let Some((path, done)) = &self.screenshot else {
            return;
        };
        self.frame += 1;
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
        ctx.request_repaint();
        if done.load(Ordering::SeqCst) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.frame == 120 {
            let path = path.clone();
            let done = done.clone();
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let mut buf = Vec::with_capacity(w * h * 4);
                for p in &img.pixels {
                    buf.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
                }
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                if let Err(e) = image::save_buffer(
                    &path,
                    &buf,
                    w as u32,
                    h as u32,
                    image::ExtendedColorType::Rgba8,
                ) {
                    eprintln!("could not save screenshot: {e}");
                }
                done.store(true, Ordering::SeqCst);
            });
        }
        if self.frame > 600 {
            // never hang a check: give up after a while
            done.store(true, Ordering::SeqCst);
        }
    }
}

pub const TOOLS: [Tool; 8] = [
    Tool::Select,
    Tool::Stock,
    Tool::Flow,
    Tool::Aux,
    Tool::Constant,
    Tool::Lookup,
    Tool::Link,
    Tool::Note,
];

/// A small pill-shaped on/off switch.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool, label: &str, pal: Palette) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(
        vec2(
            34.0 + 8.0
                + ui.fonts_mut(|f| f.glyph_width(&egui::FontId::proportional(14.0), 'W'))
                    * label.len() as f32
                    * 0.62,
            22.0,
        ),
        Sense::click(),
    );
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let track = egui::Rect::from_min_size(rect.right_top() - vec2(34.0, -2.0), vec2(34.0, 18.0));
    let col = if *on { pal.accent } else { pal.border };
    ui.painter().rect_filled(track, CornerRadius::same(9), col);
    let x = egui::lerp(track.left() + 9.0..=track.right() - 9.0, t);
    ui.painter()
        .circle_filled(egui::pos2(x, track.center().y), 7.0, Color32::WHITE);
    ui.painter().text(
        egui::pos2(rect.left(), rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(14.0),
        pal.text,
    );
    resp
}

#[cfg(feature = "gui")]
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frame_ui(ui);
    }
}

impl App {
    /// One frame of the whole window (the headless build calls this directly).
    pub fn frame_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.follow_theme(&ctx);
        self.tick_reveal(&ctx);
        self.shortcuts(&ctx);
        let pal = self.pal;
        let bar = |fill: Color32| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .stroke(Stroke::new(1.0, pal.border))
        };

        egui::Panel::top("top")
            .exact_size(48.0)
            .frame(bar(pal.panel))
            .show(ui, |ui| self.top_bar(ui));
        if self.ribbon_open {
            egui::Panel::top("ribbon")
                .exact_size(crate::ribbon::RIBBON_H)
                .frame(
                    egui::Frame::new()
                        .fill(pal.surface)
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.ribbon(ui));
        }
        egui::Panel::bottom("status")
            .exact_size(26.0)
            .frame(bar(pal.panel))
            .show(ui, |ui| self.status_bar(ui));
        egui::Panel::top("sheets")
            .exact_size(38.0)
            .frame(
                egui::Frame::new()
                    .fill(pal.surface)
                    .inner_margin(egui::Margin {
                        left: 12,
                        right: 12,
                        top: 4,
                        bottom: 0,
                    })
                    .stroke(Stroke::new(1.0, pal.border)),
            )
            .show(ui, |ui| self.sheet_tabs(ui));
        let kind = self.active_sheet_kind();
        if kind == SheetKind::Diagram {
            egui::Panel::left("tools")
                .exact_size(56.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.tool_strip(ui));
        }
        if kind == SheetKind::CausalLoop {
            egui::Panel::left("cld-tools")
                .exact_size(56.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.cld_tools(ui));
            if self.show_right {
            egui::Panel::right("cld-side")
                .resizable(true)
                .default_size(330.0)
                .size_range(260.0..=560.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::same(14))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| {
                    self.hide_right_button(ui);
                    self.cld_side(ui)
                });
            }
        }
        if matches!(kind, SheetKind::Diagram | SheetKind::Table | SheetKind::CausalLoop | SheetKind::Entities | SheetKind::Architecture) && !self.show_right {
            // hidden: a slim strip keeps a way back
            egui::Panel::right("right-collapsed")
                .resizable(false)
                .exact_size(34.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::symmetric(3, 8))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        if crate::icons::tool(ui, crate::icons::Ic::ChevronLeft, "Show the side panel (⌘B)", pal.text, pal.accent).clicked() {
                            self.show_right = true;
                        }
                    });
                });
        }
        if self.show_outline && matches!(kind, SheetKind::Diagram | SheetKind::Table | SheetKind::Entities) {
            egui::Panel::left("outline")
                .resizable(true)
                .default_size(270.0)
                .size_range(180.0..=460.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::same(12))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.outline(ui));
        }
        if matches!(kind, SheetKind::Diagram | SheetKind::Table | SheetKind::Entities) && self.show_right {
            egui::Panel::right("inspector")
                .resizable(true)
                .default_size(360.0)
                .size_range(280.0..=620.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::same(14))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| {
                    self.hide_right_button(ui);
                    self.inspector(ui)
                });
        }
        if self.show_bottom && kind == SheetKind::Diagram {
            egui::Panel::bottom("results")
                .resizable(true)
                .default_size(340.0)
                .size_range(140.0..=700.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::symmetric(14, 10))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.results(ui));
        } else if kind == SheetKind::Diagram {
            // hidden: a slim bar at the bottom brings it back, straight to the tab you want
            egui::Panel::bottom("results-collapsed")
                .resizable(false)
                .exact_size(30.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::symmetric(14, 3))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        if crate::icons::button(ui, crate::icons::Ic::ChevronUp, Some("Show results"), false, pal.text, pal.accent).on_hover_text("Show the results panel (⌘J)").clicked() {
                            self.show_bottom = true;
                        }
                        let n_problems = self.problems.len() + self.warnings().len() + self.check_failures().len() + self.layer_errors().len();
                        for (tab, label) in [
                            (BottomTab::Trends, "Trends".to_string()),
                            (BottomTab::Table, "Table".to_string()),
                            (BottomTab::Causes, "Causes & uses".to_string()),
                            (BottomTab::Problems, if n_problems > 0 { format!("Problems ({n_problems})") } else { "Problems".to_string() }),
                            (BottomTab::Runs, "Runs & scenarios".to_string()),
                        ] {
                            let warn = tab == BottomTab::Problems && n_problems > 0;
                            let text = egui::RichText::new(label).small().color(if warn { pal.aux } else { pal.muted });
                            if ui.add(egui::Button::new(text).frame(false)).clicked() {
                                self.bottom_tab = tab;
                                self.show_bottom = true;
                            }
                        }
                    });
                });
        }
        if !self.model.sets.is_empty() && matches!(kind, SheetKind::Diagram | SheetKind::Table | SheetKind::Entities) {
            egui::Panel::top("focus-bar")
                .resizable(false)
                .exact_size(36.0)
                .frame(
                    egui::Frame::new()
                        .fill(pal.panel)
                        .inner_margin(egui::Margin::symmetric(14, 2))
                        .stroke(Stroke::new(1.0, pal.border)),
                )
                .show(ui, |ui| self.focus_bar(ui));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| match kind {
            SheetKind::Diagram => self.canvas_ui(ui),
            SheetKind::Table => self.table_sheet(ui),
            SheetKind::Dashboard => self.dashboard(ui),
            SheetKind::CausalLoop => self.cld_canvas(ui),
            SheetKind::Optimize => self.optimize_sheet(ui),
            SheetKind::Scenarios => self.scenarios_sheet(ui),
            SheetKind::Entities => self.entities_sheet(ui),
            SheetKind::Architecture => self.architecture_sheet(ui),
        });
        self.poll_optimizer(&ctx);
        if let Some(job) = self.pending_export.take() {
            self.run_export(&ctx, job);
        }
        if let Some((what, path)) = self.cli_export.clone() {
            self.cli_frame += 1;
            ctx.request_repaint();
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
            if self.cli_frame == 10 {
                if path.as_os_str() == "clipboard" {
                    self.request_export(what, crate::export::ExportTo::Clipboard);
                } else {
                    self.export_path = Some(path);
                    self.request_export(what, crate::export::ExportTo::File);
                }
            }
            if self.cli_frame >= 14 {
                eprintln!("{}", self.status);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.command_palette(&ctx);
        self.help_windows(&ctx);
        self.import_window(&ctx);
        self.conversion_window(&ctx);
        self.function_window(&ctx);
        self.functions_window(&ctx);
        self.checks_window(&ctx);
        self.layers_window(&ctx);
        self.health_tick(&ctx);
        self.health_window(&ctx);
        self.modules_window(&ctx);
        self.history_windows(&ctx);
        self.compare_window(&ctx);
        self.python_window(&ctx);
        self.sensitivity_window(&ctx);
        self.remember_prefs();
        self.chart_editor_window(&ctx);
        self.import_report_window(&ctx);
        self.inbox_window(&ctx);
        match self.sel {
            None => self.multi.clear(),
            Some(Sel::Group(_)) => {}
            Some(s) => {
                if !self.multi.is_empty() && !self.multi.contains(&s) {
                    self.multi.clear();
                }
            }
        }

        if self.live && self.sim_rev != self.rev {
            self.simulate();
        }
        self.screenshot_tick(&ctx);
    }
}

impl App {
    /// A small "Hide" at the top right of the side panel.
    fn hide_right_button(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            let pal = self.pal;
            if crate::icons::tool(ui, crate::icons::Ic::ChevronRight, "Hide this panel (⌘B) — the diagram gets the room", pal.text, pal.accent).clicked() {
                self.show_right = false;
            }
        });
    }
}

/// Layout preferences, kept in `~/.dynamist-prefs.json`.
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Prefs {
    pub show_bottom: bool,
    pub show_right: bool,
    pub levers_open: bool,
    /// Whether the app was dark (kept so older versions still read the file).
    pub dark: bool,
    /// Light, dark or system; missing in files from before the choice existed.
    #[serde(default)]
    pub theme: Option<ThemeChoice>,
}

impl Prefs {
    fn path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".dynamist-prefs.json"))
    }
    fn load() -> Option<Prefs> {
        let old = std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".flowcanvas-prefs.json"));
        let text = std::fs::read_to_string(Self::path()?).ok().or_else(|| std::fs::read_to_string(old?).ok())?;
        serde_json::from_str(&text).ok()
    }
    fn save(&self) {
        if let (Some(p), Ok(t)) = (Self::path(), serde_json::to_string_pretty(self)) {
            let _ = std::fs::write(p, t);
        }
    }
}

impl App {
    // The bodies of the former menu bar, kept as functions so the ribbon can open the same lists.
    pub(crate) fn file_menu(&mut self, ui: &mut egui::Ui) {
        use crate::export::{ExportTo, ExportWhat};
        use crate::icons::{Ic, menu_item};
        let pal = self.pal;
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        let has = !self.selected_items().is_empty();
        let has_sel = diagram && has;
        let _ = (&pal, diagram, has, has_sel);
            ui.set_min_width(290.0);
            if menu_item(ui, Ic::New, "New model", "", true) {
                self.new_model();
            }
            if menu_item(ui, Ic::Open, "Open…", "⌘O", true) {
                self.open();
            }
            if menu_item(ui, Ic::Table, "Import model from spreadsheet…", "", true) {
                self.import_spreadsheet();
            }
            if menu_item(ui, Ic::Import, "Import Vensim or Stella model…", "", true) {
                if let Some(p) = rfd::FileDialog::new().add_filter("Vensim or XMILE model", &["mdl", "xmile", "stmx", "itmx", "xml"]).pick_file() {
                    self.open_path(p);
                }
            }
            ui.separator();
            if menu_item(ui, Ic::Save, "Save", "⌘S", true) {
                self.save(false);
            }
            if menu_item(ui, Ic::Save, "Save as…", "⇧⌘S", true) {
                self.save(true);
            }
            ui.separator();
            if menu_item(ui, Ic::Import, "Import module…", "", diagram) {
                self.import_module();
            }
            let module_sel = matches!(self.sel, Some(Sel::Module(_))) || has_sel;
            if menu_item(ui, Ic::Export, "Export module…", "", module_sel) {
                self.export_module();
            }
            ui.separator();
            if menu_item(ui, Ic::Image, "Copy sheet as picture", "⇧⌘C", true) {
                self.request_export(ExportWhat::Sheet, ExportTo::Clipboard);
            }
            if menu_item(ui, Ic::Image, "Copy selection as picture", "⌘C", has_sel) {
                self.request_export(ExportWhat::Selection, ExportTo::Clipboard);
            }
            if menu_item(ui, Ic::Download, "Save sheet as PNG…", "", true) {
                self.request_export(ExportWhat::Sheet, ExportTo::File);
            }
            if menu_item(ui, Ic::Download, "Save selection as PNG…", "", has_sel) {
                self.request_export(ExportWhat::Selection, ExportTo::File);
            }
            ui.menu_button(format!("      Picture quality: {:.0}×", self.export_scale), |ui| {
                for (k, label) in [(2.0, "2×  (192 dpi, screens)"), (3.0, "3×  (288 dpi, documents)"), (4.0, "4×  (384 dpi, print)")] {
                    if ui.selectable_label(self.export_scale == k, label).clicked() {
                        self.export_scale = k;
                        ui.close();
                    }
                }
            });
            ui.separator();
            if menu_item(ui, Ic::Table, "Export model to spreadsheet…", "", true) {
                self.export_spreadsheet();
            }
            if menu_item(ui, Ic::Download, "Save blank spreadsheet template…", "", true) {
                self.save_template();
            }
            if menu_item(ui, Ic::Export, "Export as Vensim model (.mdl)…", "", true) {
                self.export_foreign(dynamist_core::io::Format::Mdl);
            }
            if menu_item(ui, Ic::Export, "Export as XMILE model…", "", true) {
                self.export_foreign(dynamist_core::io::Format::Xmile);
            }
            if menu_item(ui, Ic::Csv, "Export results to CSV…", "", self.run.is_some()) {
                self.export_csv();
            }
            if menu_item(ui, Ic::Replay, "Load the example model", "", true) {
                self.checkpoint();
                self.model = Model::example();
                self.sheet = self.model.first_diagram();
                self.plotted = vec!["Susceptible".into(), "Infected".into(), "Recovered".into()];
                self.fit_pending = true;
                self.changed();
            }
    }
    pub(crate) fn edit_menu(&mut self, ui: &mut egui::Ui) {
        use crate::export::{ExportTo, ExportWhat};
        use crate::icons::{Ic, menu_item};
        let pal = self.pal;
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        let has = !self.selected_items().is_empty();
        let has_sel = diagram && has;
        let _ = (&pal, diagram, has, has_sel);
            ui.set_min_width(270.0);
            if menu_item(ui, Ic::Undo, "Undo", "⌘Z", !self.undo.is_empty()) {
                self.undo();
            }
            if menu_item(ui, Ic::Redo, "Redo", "⇧⌘Z", !self.redo.is_empty()) {
                self.redo();
            }
            ui.separator();
            if menu_item(ui, Ic::Help, "My functions…", "", true) {
                self.functions_open = true;
            }
            if menu_item(ui, Ic::Replay, "History…", "", true) {
                self.history.history_open = true;
            }
            if menu_item(ui, Ic::Import, "Compare with a saved model…", "", true) {
                self.compare_with_file();
            }
            if menu_item(ui, Ic::Help, "Run operations…", "", true) {
                self.history.ops_open = true;
            }
            ui.separator();
            if menu_item(ui, Ic::Module, "Reusable modules…", "", true) {
                self.modules_open = true;
            }
            if menu_item(ui, Ic::Check, "Model checks…", "", true) {
                self.checks_open = true;
            }
            if menu_item(ui, Ic::Check, "Model health…", "", true) {
                self.health.open = true;
            }
            if menu_item(ui, Ic::Module, "Layers…", "", true) {
                self.layers.open = true;
            }
            if menu_item(ui, Ic::Graph, "Compare with data…", "", true) {
                self.compare_open = true;
            }
            if menu_item(ui, Ic::Graph, "Function from Python…", "", true) {
                self.py_open = true;
            }
            if menu_item(ui, Ic::Target, "Sensitivity runs…", "", true) {
                self.sens_open = true;
            }
            ui.separator();
            if menu_item(ui, Ic::Close, "Cut", "⌘X", has_sel) {
                self.copy(true);
            }
            if menu_item(ui, Ic::Copy, "Copy", "⌘C", has_sel) {
                if self.copy(false) {
                    self.request_export(ExportWhat::Selection, ExportTo::Clipboard);
                }
            }
            if menu_item(ui, Ic::Paste, "Paste", "⌘V", self.clip.is_some()) {
                self.paste();
            }
            if menu_item(ui, Ic::Duplicate, "Duplicate", "⌘D", has_sel) {
                self.duplicate();
            }
            if menu_item(ui, Ic::Grid, "Select all", "⌘A", diagram) {
                self.select_all();
            }
            ui.separator();
            if menu_item(ui, Ic::Trash, "Delete selection", "⌫", self.sel.is_some()) {
                self.delete_selection();
            }
            if menu_item(ui, Ic::Trace, "Add arrows for every equation input", "", true) {
                self.checkpoint();
                let n = crate::canvas::sync_links(&mut self.model);
                self.status = format!("Added {n} arrow(s)");
                self.changed();
            }
    }
    pub(crate) fn view_menu(&mut self, ui: &mut egui::Ui) {
        use crate::icons::{Ic, menu_item};
        let pal = self.pal;
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        let has = !self.selected_items().is_empty();
        let has_sel = diagram && has;
        let _ = (&pal, diagram, has, has_sel);
            ui.set_min_width(250.0);
            if menu_item(ui, Ic::Fit, "Zoom to fit", "0", true) {
                self.fit_pending = true;
            }
            ui.separator();
            ui.label(egui::RichText::new("Appearance").small().color(pal.muted));
            ui.horizontal(|ui| {
                for (c, tip) in [
                    (ThemeChoice::Light, "Always light"),
                    (ThemeChoice::Dark, "Always dark"),
                    (ThemeChoice::System, "Light or dark, as the system is set"),
                ] {
                    if ui.selectable_label(self.theme_choice == c, c.label()).on_hover_text(tip).clicked() {
                        let ctx = ui.ctx().clone();
                        self.set_theme_choice(&ctx, c);
                    }
                }
            });
            ui.separator();
            ui.checkbox(&mut self.show_bottom, "Results panel  (⌘J)");
            ui.checkbox(&mut self.show_right, "Side panel  (⌘B)");
            ui.checkbox(&mut self.show_outline, "Outline (the model as a tree)");
            ui.checkbox(&mut self.animate, "Play runs over time");
            ui.add_enabled(self.animate, egui::Slider::new(&mut self.reveal_secs, 0.5..=12.0).suffix(" s").text("play length"));
            ui.checkbox(&mut self.show_help, "Quick guide");
            ui.checkbox(&mut self.show_minimap, "Mini-map");
            ui.checkbox(&mut self.scenarios_everywhere, "Scenarios in every chart");
            ui.separator();
            ui.label(egui::RichText::new("Diagram look").small().color(pal.muted));
            ui.horizontal(|ui| {
                for (st, label) in [(crate::model::DiagramStyle::Cards, "Cards"), (crate::model::DiagramStyle::Classic, "Classic")] {
                    if ui.selectable_label(self.model.diagram_style == st, label).on_hover_text(if st == crate::model::DiagramStyle::Classic { "Plain labels, thin dark arrows" } else { "Variables as cards, coloured arrows" }).clicked() {
                        self.checkpoint();
                        self.model.diagram_style = st;
                        self.changed();
                    }
                }
            });
            ui.label(egui::RichText::new("Modules").small().color(pal.muted));
            ui.horizontal(|ui| {
                for (st, label) in [(crate::model::ModuleStyle::Compact, "Compact"), (crate::model::ModuleStyle::Detailed, "Detailed")] {
                    if ui.selectable_label(self.model.module_style == st, label).on_hover_text(if st == crate::model::ModuleStyle::Compact { "Name, purpose and role; arrows say what passes between modules" } else { "Every input and output listed" }).clicked() {
                        self.checkpoint();
                        self.model.module_style = st;
                        self.changed();
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui.small_button("Fold all").on_hover_text("Every module as a short card").clicked() {
                    self.checkpoint();
                    self.model.modules.iter_mut().for_each(|m| m.collapsed = true);
                    self.changed();
                }
                if ui.small_button("Unfold all").on_hover_text("Every module with its inputs and outputs").clicked() {
                    self.checkpoint();
                    self.model.modules.iter_mut().for_each(|m| m.collapsed = false);
                    self.model.module_style = crate::model::ModuleStyle::Detailed;
                    self.changed();
                }
            });
            ui.label(egui::RichText::new("Arrows").small().color(pal.muted));
            ui.horizontal(|ui| {
                for sh in crate::model::ArrowShape::ALL {
                    if ui.selectable_label(self.model.arrow_shape == sh, sh.label()).clicked() {
                        self.checkpoint();
                        self.model.arrow_shape = sh;
                        self.changed();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Tip").small().color(pal.muted));
                for h in crate::model::ArrowHead::ALL {
                    if ui.selectable_label(self.model.arrow_head == h, h.label()).clicked() {
                        self.checkpoint();
                        self.model.arrow_head = h;
                        self.changed();
                    }
                }
            });
            if ui.button("Use this style for every arrow").clicked() {
                self.checkpoint();
                for l in self.model.links.iter_mut() {
                    l.shape = None;
                    l.head = None;
                }
                self.changed();
                ui.close();
            }
    }
    pub(crate) fn help_menu(&mut self, ui: &mut egui::Ui) {
        use crate::icons::{Ic, menu_item};
        let pal = self.pal;
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        let has = !self.selected_items().is_empty();
        let has_sel = diagram && has;
        let _ = (&pal, diagram, has, has_sel);
            ui.set_min_width(250.0);
            if menu_item(ui, Ic::Search, "Command palette", "⌘K", true) {
                self.palette = Some((String::new(), 0));
            }
            if menu_item(ui, Ic::Mail, "Inbox — messages between conversations", "", true) {
                self.open_inbox(crate::inbox::Tab::Open);
            }
            if menu_item(ui, Ic::Open, "Documentation (wiki)", "", true) {
                self.open_wiki();
            }
            if menu_item(ui, Ic::Help, "Function reference", "F1", true) {
                let t = if let Some(Sel::Var(v)) = self.sel { Some(v) } else { None };
                self.open_functions(t);
            }
            if menu_item(ui, Ic::Keyboard, "Keyboard shortcuts", "?", true) {
                self.show_shortcuts = true;
            }
            if menu_item(ui, Ic::Help, "About Dynamist", "", true) {
                self.show_about = true;
            }
    }
}
