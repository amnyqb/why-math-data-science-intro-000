//! The "Model health" panel: what the model doctor (`dynamist_core::doctor`) found, grouped by family, each
//! finding with its fixes. A fix is applied through the operations language, so it is logged and undone like any other edit.
//!
//! Stages C1 to C3 run again after every edit (once the model has been still for 300 ms); C4 and C5 run on a worker
//! thread when asked for. The gate badge in the status bar says where the model stands: Draft, Review or Publish.

use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use dynamist_core::doctor::{self, Finding, Fix, Gate, Options, Report, Severity, Stage};
use dynamist_core::ops::Op;
use egui::{Color32, CornerRadius, Margin, RichText, Stroke};

use crate::app::{App, Sel, is_headless};
use crate::expr;

/// How long the model must stand still before the quick stages run again.
const DEBOUNCE: Duration = Duration::from_millis(300);
/// Above this the quick stages move to a worker thread, so a big model never stalls a frame.
const SYNC_LIMIT_MS: f64 = 80.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Deep {
    C4,
    C5,
}

impl Deep {
    fn label(self) -> &'static str {
        match self {
            Deep::C4 => "C4 checks",
            Deep::C5 => "experiments (C5)",
        }
    }
    fn options(self) -> Options {
        match self {
            Deep::C4 => Options { stage: Some(Stage::C4), ..Default::default() },
            Deep::C5 => Options { experiments: true, stage: Some(Stage::C5), ..Default::default() },
        }
    }
}

struct Job {
    rx: Receiver<Report>,
    rev: u64,
    what: Option<Deep>,
    started: Instant,
}

#[derive(Default)]
pub struct State {
    pub open: bool,
    /// The last quick report (stages C1 to C3) and the revision of the model it describes.
    pub report: Option<Report>,
    pub report_rev: Option<u64>,
    /// Slower reports (C4, C5), each with the revision it was made for.
    pub deep: Vec<(Deep, u64, Report)>,
    last_ms: f64,
    seen_rev: u64,
    since: Option<Instant>,
    fast_job: Option<Job>,
    deep_job: Option<Job>,
    /// Hide the "info" findings.
    pub hide_info: bool,
}

/// The families of the rule catalogue, in the order the panel lists them.
pub const FAMILIES: [(char, &str); 10] = [
    ('S', "Definition"),
    ('G', "Graph"),
    ('U', "Units"),
    ('F', "Formulation"),
    ('N', "Numerical"),
    ('I', "Initialisation"),
    ('X', "Experiments"),
    ('C', "Calibration"),
    ('R', "Reproducibility"),
    ('P', "Purpose"),
];

pub fn family_name(c: char) -> &'static str {
    FAMILIES.iter().find(|(k, _)| *k == c).map(|(_, n)| *n).unwrap_or("Other")
}

/// The family letter of a finding (the first letter of its rule id; the code when it has no id).
pub fn family_of(f: &Finding) -> char {
    let src = if f.rule.is_empty() { &f.code } else { &f.rule };
    src.chars().next().map(|c| c.to_ascii_uppercase()).filter(|c| FAMILIES.iter().any(|(k, _)| k == c)).unwrap_or('?')
}

pub fn gate_name(g: Gate) -> &'static str {
    match g {
        Gate::Draft => "Draft",
        Gate::Review => "Review",
        Gate::Publish => "Publish",
    }
}

pub fn gate_meaning(g: Gate) -> &'static str {
    match g {
        Gate::Draft => "Draft: the model has errors. Fix them before it is reviewed.",
        Gate::Review => "Review: no errors, but warnings remain. Ready for a reviewer, not yet for publication.",
        Gate::Publish => "Publish: no errors and no warnings.",
    }
}

/// The gate of a list of findings.
pub fn gate_of(findings: &[Finding]) -> Gate {
    if findings.iter().any(|f| f.severity == Severity::Error) {
        Gate::Draft
    } else if findings.iter().any(|f| f.severity == Severity::Warning) {
        Gate::Review
    } else {
        Gate::Publish
    }
}

fn clip(s: &str, n: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= n {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(n).collect::<String>().trim_end())
    }
}

/// One line of plain words for what an operation does to the model.
pub fn describe_op(op: &Op) -> String {
    match op {
        Op::AddVar { name, kind, equation, units, from, to, .. } => {
            let mut s = format!("add {kind} {name}");
            if !equation.trim().is_empty() {
                s.push_str(&format!(" = {}", clip(equation, 80)));
            }
            if !units.trim().is_empty() {
                s.push_str(&format!(" (units {})", units.trim()));
            }
            match (from, to) {
                (Some(a), Some(b)) => s.push_str(&format!(", from {a} to {b}")),
                (Some(a), None) => s.push_str(&format!(", draining {a}")),
                (None, Some(b)) => s.push_str(&format!(", filling {b}")),
                _ => {}
            }
            s
        }
        Op::RemoveVar { name } => format!("remove {name}"),
        Op::SetEquation { name, equation } => format!("set equation of {name} to {}", clip(equation, 100)),
        Op::SetUnits { name, units } => format!("set units of {name} to {}", if units.trim().is_empty() { "none" } else { units.trim() }),
        Op::SetDoc { name, doc } => format!("set the description of {name} to \u{201c}{}\u{201d}", clip(doc, 80)),
        Op::SetSetting { key, value } => format!("set the model setting {key} to {}", clip(value, 60)),
        Op::SetNonNegative { name, on } => if *on { format!("keep {name} from going negative") } else { format!("allow {name} to go negative") },
        Op::AddToGroup { name, vars } => format!("add {} to the group {name}", clip(&vars.join(", "), 80)),
        Op::Connect { from, to, .. } => format!("draw an arrow from {from} to {to}"),
        Op::SetTable { name, points } => format!("set the curve of {name} ({} points)", points.len()),
        Op::SetManifest { manifest } => {
            let levers = serde_json::to_value(manifest).ok().and_then(|v| v.get("levers").and_then(|l| l.as_array()).map(|a| a.len())).unwrap_or(0);
            format!("write the model manifest ({levers} levers)")
        }
        Op::AddCheck { name, var, min, max, .. } => {
            let range = match (min, max) {
                (Some(a), Some(b)) => format!(" between {a} and {b}"),
                (Some(a), None) => format!(" at least {a}"),
                (None, Some(b)) => format!(" at most {b}"),
                _ => String::new(),
            };
            if var.trim().is_empty() { format!("add the check \u{201c}{name}\u{201d}") } else { format!("add the check \u{201c}{name}\u{201d}: {var}{range}") }
        }
        Op::SetPurpose { problem, .. } => format!("set the purpose of the model: {}", clip(problem, 80)),
        other => {
            let v = serde_json::to_value(other).unwrap_or_default();
            let tag = v.get("op").and_then(|t| t.as_str()).unwrap_or("change").replace('_', " ");
            let target = ["name", "var", "layer"].iter().find_map(|k| v.get(*k).and_then(|x| x.as_str())).unwrap_or("");
            if target.is_empty() { format!("operation: {tag}") } else { format!("operation: {tag} {target}") }
        }
    }
}

/// What to tell about a finding that has no message: the doctor's hint for its code.
fn hint_text(code: &str) -> String {
    use dynamist_core::validate;
    let r = validate::Report { issues: vec![validate::Issue { severity: validate::Severity::Warning, code: code.into(), var: None, var_id: None, message: String::new() }], ran: false };
    validate::explain(&r).into_iter().next().map(|e| e.hint).filter(|h| !h.starts_with("No advice")).unwrap_or_default()
}

fn same(a: &Finding, b: &Finding) -> bool {
    a.code == b.code && a.var == b.var && a.message == b.message
}

impl State {
    /// Everything the panel shows: the quick findings, then those only the slower stages found, in the order of the
    /// catalogue families and, within a family, errors first.
    pub fn findings(&self) -> Vec<Finding> {
        let mut out: Vec<Finding> = self.report.as_ref().map(|r| r.findings.clone()).unwrap_or_default();
        for (_, _, r) in &self.deep {
            for f in &r.findings {
                if !out.iter().any(|g| same(g, f)) {
                    out.push(f.clone());
                }
            }
        }
        let order = |f: &Finding| FAMILIES.iter().position(|(k, _)| *k == family_of(f)).unwrap_or(99);
        out.sort_by_key(|f| (order(f), std::cmp::Reverse(f.severity)));
        out
    }

    /// The gate over the shown findings.
    pub fn gate(&self) -> Option<Gate> {
        self.report.as_ref().map(|_| gate_of(&self.findings()))
    }

    pub fn running(&self) -> Option<Deep> {
        self.deep_job.as_ref().and_then(|j| j.what)
    }
}

fn badge(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.14))
        .stroke(Stroke::new(1.0, color.gamma_multiply(0.55)))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::symmetric(7, 1))
        .show(ui, |ui| ui.label(RichText::new(text).small().strong().color(color)))
        .response
}

impl App {
    pub fn gate_color(&self, g: Gate) -> Color32 {
        match g {
            Gate::Draft => self.pal.error,
            Gate::Review => self.pal.aux,
            Gate::Publish => self.pal.ok,
        }
    }

    fn severity_color(&self, s: Severity) -> Color32 {
        match s {
            Severity::Error => self.pal.error,
            Severity::Warning => self.pal.aux,
            Severity::Info => self.pal.muted,
        }
    }

    /// Run the quick stages (C1 to C3) now, on this thread.
    pub fn health_refresh(&mut self) {
        let t = Instant::now();
        let report = doctor::check(&self.model, &Options { stage: Some(Stage::C3), ..Default::default() });
        self.health.last_ms = t.elapsed().as_secs_f64() * 1000.0;
        self.health.report = Some(report);
        self.health.report_rev = Some(self.rev);
        self.health.fast_job = None;
    }

    /// Start stage C4 or C5 on a worker thread.
    pub fn health_run_deep(&mut self, what: Deep) {
        if self.health.deep_job.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let model = self.model.clone();
        let opts = what.options();
        std::thread::spawn(move || {
            let _ = tx.send(doctor::check(&model, &opts));
        });
        self.health.deep_job = Some(Job { rx, rev: self.rev, what: Some(what), started: Instant::now() });
    }

    fn health_poll(&mut self) {
        if let Some(j) = &self.health.fast_job {
            if let Ok(r) = j.rx.try_recv() {
                let rev = j.rev;
                self.health.last_ms = j.started.elapsed().as_secs_f64() * 1000.0;
                self.health.report = Some(r);
                self.health.report_rev = Some(rev);
                self.health.fast_job = None;
            }
        }
        if let Some(j) = &self.health.deep_job {
            if let Ok(r) = j.rx.try_recv() {
                if let Some(what) = j.what {
                    let rev = j.rev;
                    self.health.deep.retain(|(w, _, _)| *w != what);
                    self.health.deep.push((what, rev, r));
                }
                self.health.deep_job = None;
            }
        }
    }

    /// Called every frame: collect finished jobs and rerun the quick stages once the model has been still for 300 ms.
    pub fn health_tick(&mut self, ctx: &egui::Context) {
        if is_headless() {
            return;
        }
        self.health_poll();
        if self.health.fast_job.is_some() || self.health.deep_job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        if self.health.report_rev == Some(self.rev) || self.health.fast_job.is_some() {
            return;
        }
        if self.health.seen_rev != self.rev {
            self.health.seen_rev = self.rev;
            self.health.since = Some(Instant::now());
        }
        let first = self.health.report.is_none();
        let waited = self.health.since.map(|t| t.elapsed() >= DEBOUNCE).unwrap_or(true);
        if !first && !waited {
            ctx.request_repaint_after(Duration::from_millis(100));
            return;
        }
        if self.health.last_ms <= SYNC_LIMIT_MS {
            self.health_refresh();
        } else {
            let (tx, rx) = channel();
            let model = self.model.clone();
            std::thread::spawn(move || {
                let _ = tx.send(doctor::check(&model, &Options { stage: Some(Stage::C3), ..Default::default() }));
            });
            self.health.fast_job = Some(Job { rx, rev: self.rev, what: None, started: Instant::now() });
        }
    }

    /// Apply a fix through the normal edit path: logged, one undo step, then the quick stages again at once.
    pub fn health_apply(&mut self, fix: &Fix) {
        let mut s = dynamist_core::ops::Session::new(self.model.clone());
        s.gate = false;
        match s.apply("user:app", &fix.ops) {
            Ok(entry) => {
                let mut e = entry.clone();
                e.seq = self.history.log.last().map(|x| x.seq + 1).unwrap_or(1);
                self.checkpoint();
                self.model = s.model;
                self.changed();
                self.history.log.push(e);
                self.status = format!("Applied: {}", fix.title);
                self.health_refresh();
            }
            Err(r) => self.status = format!("Could not apply the fix: {}", r.reason),
        }
    }

    pub fn health_window(&mut self, ctx: &egui::Context) {
        if !self.health.open {
            return;
        }
        let mut open = true;
        egui::Window::new("Model health").open(&mut open).default_width(700.0).default_height(580.0).show(ctx, |ui| self.health_panel(ui));
        if !open {
            self.health.open = false;
        }
    }

    /// The gate badge for the status bar; clicking it opens the panel.
    pub fn gate_badge(&mut self, ui: &mut egui::Ui) {
        let Some(g) = self.health.gate() else { return };
        let col = self.gate_color(g);
        let findings = self.health.findings();
        let (e, w) = (findings.iter().filter(|f| f.severity == Severity::Error).count(), findings.iter().filter(|f| f.severity == Severity::Warning).count());
        let tip = format!("{}\n{e} errors, {w} warnings. Click to open Model health.", gate_meaning(g));
        let r = badge(ui, gate_name(g), col).interact(egui::Sense::click()).on_hover_text(tip);
        if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            self.health.open = true;
        }
    }

    fn summary_line(r: &Report) -> String {
        let s = &r.summary;
        format!(
            "{} stocks, {} flows, {} loops. Units given for {:.0}% of the variables, described for {:.0}%.",
            s.stocks, s.flows, s.loops, s.units_coverage_pct, s.documentation_coverage_pct
        )
    }

    /// The panel itself (drawn in a window, and by the tests and the screenshot).
    pub fn health_panel(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let findings = self.health.findings();
        let mut select: Option<String> = None;
        let mut apply: Option<Fix> = None;
        let mut run: Option<Deep> = None;
        let running = self.health.running();
        let mut hide_info = self.health.hide_info;

        ui.horizontal(|ui| {
            match self.health.gate() {
                Some(g) => {
                    let col = self.gate_color(g);
                    badge(ui, gate_name(g), col).on_hover_text(gate_meaning(g));
                    let (e, w, i) = (
                        findings.iter().filter(|f| f.severity == Severity::Error).count(),
                        findings.iter().filter(|f| f.severity == Severity::Warning).count(),
                        findings.iter().filter(|f| f.severity == Severity::Info).count(),
                    );
                    ui.label(RichText::new(format!("{e} errors, {w} warnings, {i} notes")).color(pal.text));
                }
                None => {
                    ui.label(RichText::new("Not checked yet").color(pal.muted));
                }
            }
        });
        if let Some(r) = &self.health.report {
            ui.label(RichText::new(Self::summary_line(r)).small().color(pal.muted));
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let busy = running.is_some();
            if ui.add_enabled(!busy, egui::Button::new("Run C4 checks")).on_hover_text("The base run, the eigenvalues and the time step halving. Takes a few seconds on a big model.").clicked() {
                run = Some(Deep::C4);
            }
            if ui.add_enabled(!busy, egui::Button::new("Run experiments (C5)")).on_hover_text("Extreme values, sources set to zero and arrow signs, each a run of the model. Slow.").clicked() {
                run = Some(Deep::C5);
            }
            if let Some(w) = running {
                ui.spinner();
                ui.label(RichText::new(format!("Running {}...", w.label())).color(pal.muted));
            }
            ui.checkbox(&mut hide_info, "Hide notes");
        });
        let stale: Vec<&str> = self.health.deep.iter().filter(|(_, rev, _)| *rev != self.rev).map(|(w, _, _)| w.label()).collect();
        if !stale.is_empty() {
            ui.label(RichText::new(format!("The {} results are from before the last edit. Run them again to refresh.", stale.join(" and "))).small().color(pal.aux));
        }
        ui.separator();

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let shown: Vec<&Finding> = findings.iter().filter(|f| !(hide_info && f.severity == Severity::Info)).collect();
            if self.health.report.is_some() && shown.is_empty() {
                ui.label(RichText::new("Nothing found by the stages that have run.").color(pal.ok));
            }
            let mut last_family = '\0';
            for (idx, f) in shown.iter().enumerate() {
                let fam = family_of(f);
                if fam != last_family {
                    last_family = fam;
                    let n = shown.iter().filter(|g| family_of(g) == fam).count();
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("{}  ({n})", if fam == '?' { "Other" } else { family_name(fam) })).strong().color(pal.accent));
                }
                egui::Frame::new().fill(pal.surface).corner_radius(CornerRadius::same(8)).inner_margin(Margin::same(10)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        let (word, col) = match f.severity {
                            Severity::Error => ("error", self.severity_color(f.severity)),
                            Severity::Warning => ("warning", self.severity_color(f.severity)),
                            Severity::Info => ("info", self.severity_color(f.severity)),
                        };
                        // a finding no run has confirmed is a possibility, not an error
                        badge(ui, if f.confirmed { word } else { "possible" }, col);
                        if !f.confirmed {
                            badge(ui, "can: not confirmed by a run", pal.aux).on_hover_text("Found by the interval analysis: the problem can happen for some values in range. Run the C4 checks or the experiments to see if it does.");
                        }
                        if let Some(v) = &f.var {
                            if ui.link(RichText::new(v).strong()).on_hover_text("Select it on the diagram").clicked() {
                                select = Some(v.clone());
                            }
                        }
                        let rule = if f.rule.is_empty() { f.code.clone() } else { f.rule.clone() };
                        let cite = if f.source.trim().is_empty() { format!("{rule}") } else { format!("{rule}: {}", f.source.trim()) };
                        ui.label(RichText::new(cite).small().color(pal.muted));
                    });
                    let msg = if f.message.trim().is_empty() { hint_text(&f.code) } else { f.message.clone() };
                    ui.label(RichText::new(msg).color(pal.text));
                    if f.fixes.is_empty() && !f.message.trim().is_empty() {
                        let hint = hint_text(&f.code);
                        if !hint.is_empty() {
                            ui.label(RichText::new(hint).small().color(pal.muted));
                        }
                    }
                    for (k, fix) in f.fixes.iter().enumerate() {
                        ui.add_space(3.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.add(egui::Button::new(RichText::new(&fix.title).color(pal.accent)).fill(pal.accent_soft)).on_hover_text("Apply this fix. One undo step takes it back.").clicked() {
                                apply = Some(fix.clone());
                            }
                            if fix.keeps_numbers {
                                badge(ui, "keeps the numbers", pal.ok);
                            }
                            let _ = (idx, k);
                        });
                        for op in &fix.ops {
                            ui.label(RichText::new(format!("   \u{2192}  {}", describe_op(op))).small().color(pal.muted));
                        }
                    }
                });
                ui.add_space(4.0);
            }
        });

        self.health.hide_info = hide_info;
        if let Some(n) = select {
            self.sel = self.model.by_name(&expr::normalize(&n)).map(|v| Sel::Var(v.id));
        }
        if let Some(fix) = apply {
            self.health_apply(&fix);
        }
        if let Some(w) = run {
            self.health_run_deep(w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dynamist_core::mdl;
    use egui::epaint::ClippedShape;
    use egui::{Context, Pos2, RawInput, Rect, Shape, vec2};

    fn trap() -> dynamist_core::model::Model {
        let mut m = mdl::import(include_str!("../core/tests/data/units_trap.mdl")).unwrap();
        m.settings.unit_choices.insert("MB".into(), "thousand barrels".into());
        m.settings.units_scale_values = true;
        m.ensure_sheets();
        m
    }

    fn texts(shapes: &[ClippedShape], out: &mut Vec<String>) {
        fn walk(s: &Shape, out: &mut Vec<String>) {
            match s {
                Shape::Text(t) => out.push(t.galley.text().to_string()),
                Shape::Vec(v) => v.iter().for_each(|x| walk(x, out)),
                _ => {}
            }
        }
        for c in shapes {
            walk(&c.shape, out);
        }
    }

    /// Draw the panel of an app for the trap model: (the shapes, the area, the app).
    fn draw(dark: bool) -> (Vec<ClippedShape>, Rect, App, Context) {
        crate::app::HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
        let ctx = Context::default();
        ctx.set_pixels_per_point(2.0);
        let mut app = App::new(&ctx, None);
        app.finish_open(trap());
        if dark {
            app.pal = crate::theme::Palette::dark();
        }
        crate::theme::apply(&ctx, &app.pal);
        app.health_refresh();
        let area = Rect::from_min_size(Pos2::ZERO, vec2(760.0, 800.0));
        let mut shapes = Vec::new();
        for i in 0..4 {
            let input = RawInput { screen_rect: Some(area), time: Some(i as f64 * 0.05), focused: true, ..Default::default() };
            let pal = app.pal;
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().frame(egui::Frame::new().fill(pal.panel).inner_margin(Margin::same(14))).show(ui, |ui| app.health_panel(ui));
            });
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        (shapes, area, app, ctx)
    }

    #[test]
    fn the_panel_shows_the_units_trap() {
        let (shapes, _, mut app, _ctx) = draw(false);
        let mut t = Vec::new();
        texts(&shapes, &mut t);
        let all = t.join("\n");
        assert!(all.contains("Draft"), "gate badge: {all}");
        assert!(all.contains("Units"), "family heading");
        assert!(all.contains("Delivery"), "the variable");
        assert!(all.contains("barrel/Day"), "the finding message");
        assert!(all.contains("MB per barrel") && all.contains("0.001"), "the fix preview: {all}");
        assert!(all.contains("Run C4 checks") && all.contains("Run experiments (C5)"));
        assert!(all.contains("error") && all.contains("info"), "severity badges");
        // the first fix of the scale mismatch applies through the undoable path
        let f = app.health.findings().into_iter().find(|f| f.code == "unit-scale-mismatch" && f.var.as_deref() == Some("Delivery")).unwrap();
        let before = app.model.vars.len();
        app.health_apply(&f.fixes[0]);
        assert!(app.model.vars.iter().any(|v| v.name == "MB per barrel"));
        assert!(!app.health.findings().iter().any(|g| g.code == "unit-scale-mismatch" && g.var.as_deref() == Some("Delivery")), "the finding is gone after the fix");
        app.undo();
        assert_eq!(app.model.vars.len(), before, "undo takes the fix back");
    }

    #[test]
    fn ops_read_as_sentences() {
        let ops: Vec<Op> = serde_json::from_str(
            r#"[{"op":"add_var","name":"MB per barrel","kind":"constant","equation":"0.001","units":"MB/barrel"},
                {"op":"set_equation","name":"Tank net flow","equation":"Delivery * MB per barrel"},
                {"op":"set_units","name":"Use","units":"MB/Day"},
                {"op":"remove_var","name":"X"},{"op":"connect","from":"A","to":"B"},
                {"op":"set_non_negative","name":"Tank","on":true},{"op":"rename_var","name":"A","to":"B"}]"#,
        )
        .unwrap();
        let s: Vec<String> = ops.iter().map(describe_op).collect();
        assert_eq!(s[0], "add constant MB per barrel = 0.001 (units MB/barrel)");
        assert_eq!(s[1], "set equation of Tank net flow to Delivery * MB per barrel");
        assert_eq!(s[2], "set units of Use to MB/Day");
        assert_eq!(s[3], "remove X");
        assert_eq!(s[4], "draw an arrow from A to B");
        assert_eq!(s[5], "keep Tank from going negative");
        assert_eq!(s[6], "operation: rename var A");
    }

    #[test]
    fn the_gate_follows_the_findings() {
        let mk = |s| Finding::new("x", s, None, "m");
        assert_eq!(gate_of(&[mk(Severity::Error), mk(Severity::Warning)]), Gate::Draft);
        assert_eq!(gate_of(&[mk(Severity::Warning), mk(Severity::Info)]), Gate::Review);
        assert_eq!(gate_of(&[mk(Severity::Info)]), Gate::Publish);
    }

    #[test]
    fn deep_checks_run_on_a_thread_and_merge() {
        let (_, _, mut app, _ctx) = draw(false);
        app.health_run_deep(Deep::C4);
        assert!(app.health.running().is_some());
        let t = Instant::now();
        while app.health.running().is_some() && t.elapsed() < Duration::from_secs(60) {
            std::thread::sleep(Duration::from_millis(20));
            app.health_poll();
        }
        assert!(app.health.running().is_none(), "the job finished");
        assert_eq!(app.health.deep.len(), 1);
    }

    /// Writes the screenshots of the wiki page when `DYN_WRITE_IMG` is set (the pictures differ a little between platforms).
    #[test]
    fn health_panel_screenshots() {
        let Ok(dir) = std::env::var("DYN_WRITE_IMG") else { return };
        for (dark, name) in [(false, "health-panel-units-trap.png"), (true, "health-panel-units-trap-dark.png")] {
            let (shapes, area, app, _ctx) = draw(dark);
            let bg = ClippedShape { clip_rect: area, shape: Shape::rect_filled(area, 0.0, app.pal.panel), transform_after_tessellation: Default::default() };
            let mut all = vec![bg];
            all.extend(shapes);
            let img = crate::export::rasterize(&all, area, 2.0);
            let png = crate::export::png_bytes(&img, 2.0).unwrap();
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(std::path::Path::new(&dir).join(name), png).unwrap();
        }
    }

    /// One frame of the whole window of `app`, as shapes over the canvas colour.
    fn window_shapes(ctx: &Context, app: &mut App, area: Rect, frames: usize) -> Vec<ClippedShape> {
        let mut shapes = Vec::new();
        for i in 0..frames {
            let input = RawInput { screen_rect: Some(area), time: Some(i as f64 * 0.05), focused: true, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| app.frame_ui(ui));
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        let bg = ClippedShape { clip_rect: area, shape: Shape::rect_filled(area, 0.0, app.pal.canvas), transform_after_tessellation: Default::default() };
        let mut all = vec![bg];
        all.extend(shapes);
        all
    }

    /// The part of a 2x picture inside `crop` (in points), as PNG bytes.
    fn crop_png(png: &[u8], crop: Rect) -> Vec<u8> {
        let img = image::load_from_memory(png).unwrap();
        let (x, y, w, h) = ((crop.min.x * 2.0) as u32, (crop.min.y * 2.0) as u32, (crop.width() * 2.0) as u32, (crop.height() * 2.0) as u32);
        let cropped = img.crop_imm(x, y, w.min(img.width() - x), h.min(img.height() - y));
        let mut out = Vec::new();
        cropped.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    /// The three pictures of the book handover (2x, light theme), written when `DYN_WRITE_SHOTS` names a folder:
    /// the Model health panel on the units trap, the status bar with the gate badge, the page of a function.
    #[test]
    fn handover_shots() {
        let Ok(dir) = std::env::var("DYN_WRITE_SHOTS") else { return };
        std::fs::create_dir_all(&dir).unwrap();
        let out = std::path::Path::new(&dir);
        // 1. the health panel (a finding and its fix preview)
        let (shapes, area, app, _ctx) = draw(false);
        let bg = ClippedShape { clip_rect: area, shape: Shape::rect_filled(area, 0.0, app.pal.panel), transform_after_tessellation: Default::default() };
        let mut all = vec![bg];
        all.extend(shapes);
        let png = crate::export::png_bytes(&crate::export::rasterize(&all, area, 2.0), 2.0).unwrap();
        std::fs::write(out.join("shot_health_panel.png"), png).unwrap();
        // the whole window, for the status bar and the function page
        crate::app::HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
        let ctx = Context::default();
        ctx.set_pixels_per_point(2.0);
        let mut app = App::new(&ctx, None);
        app.animate = false;
        app.finish_open(trap());
        crate::theme::apply(&ctx, &app.pal);
        app.health_refresh();
        let area = Rect::from_min_size(Pos2::ZERO, vec2(1280.0, 800.0));
        let shapes = window_shapes(&ctx, &mut app, area, 4);
        let full = crate::export::png_bytes(&crate::export::rasterize(&shapes, area, 2.0), 2.0).unwrap();
        std::fs::write(out.join("full_window.png"), &full).unwrap();
        // 2. the status bar with the gate badge
        std::fs::write(out.join("shot_gate_badge.png"), crop_png(&full, Rect::from_min_max(Pos2::new(760.0, area.max.y - 40.0), area.max))).unwrap();
        // 3. a function page
        app.open_functions(None);
        app.functions.as_mut().unwrap().selected = "LEVELISED";
        let shapes = window_shapes(&ctx, &mut app, area, 40); // long enough for the window to fade in
        let full = crate::export::png_bytes(&crate::export::rasterize(&shapes, area, 2.0), 2.0).unwrap();
        std::fs::write(out.join("full_function.png"), &full).unwrap();
        std::fs::write(out.join("shot_function_help.png"), crop_png(&full, Rect::from_min_max(Pos2::new(240.0, 10.0), Pos2::new(1280.0, 665.0)))).unwrap();
    }
}
