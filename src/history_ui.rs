//! Three windows about changes to a model:
//!  * **History** — what each undo step changed, in words (derived by comparing the saved states,
//!    so it covers everything done in this session), and the operation batches that were run.
//!  * **Compare with a saved model** — what differs in meaning from a file, and which results move.
//!  * **Run operations** — type or paste a batch of operations; it is validated before it is
//!    accepted, exactly as an agent's batch is (see the operations language reference).
#[cfg(not(feature = "gui"))]
use crate::rfd;

use egui::RichText;
use dynamist_core::diff::{self, Change, Difference, RunDiff};
use dynamist_core::io;
use dynamist_core::ops::{self, LogEntry, Op, Session};

use crate::app::App;

#[derive(Default)]
pub struct State {
    pub history_open: bool,
    /// (undo length, revision, differences of each step, oldest first)
    pub cache: Option<(usize, u64, Vec<Vec<Difference>>)>,
    pub compare: Option<CompareFile>,
    pub ops_open: bool,
    pub ops_text: String,
    pub ops_gate: bool,
    pub ops_message: Option<(bool, String)>,
    pub log: Vec<LogEntry>,
}

pub struct CompareFile {
    pub name: String,
    pub diffs: Vec<Difference>,
    pub runs: Result<RunDiff, String>,
}

fn change_mark(c: Change) -> (&'static str, bool) {
    match c {
        Change::Added => ("+", true),
        Change::Removed => ("−", false),
        Change::Changed => ("~", true),
    }
}

fn differences_ui(ui: &mut egui::Ui, pal: crate::theme::Palette, diffs: &[Difference]) {
    for d in diffs {
        let (mark, _) = change_mark(d.change);
        let col = match d.change {
            Change::Added => pal.ok,
            Change::Removed => pal.error,
            Change::Changed => pal.aux,
        };
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(mark).strong().color(col));
            ui.label(RichText::new(&d.what).small().color(pal.muted));
            ui.label(RichText::new(&d.name).strong());
            if !d.detail.is_empty() {
                ui.label(RichText::new(&d.detail).small().color(pal.text));
            }
        });
    }
}

impl App {
    /// File → Compare with a saved model…
    pub fn compare_with_file(&mut self) {
        let Some(p) = rfd::FileDialog::new().add_filter("Model", &["sdm", "flowmodel", "json", "mdl", "xmile", "stmx", "itmx", "xml"]).pick_file() else { return };
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let text = match std::fs::read_to_string(&p) {
            Ok(t) => t,
            Err(e) => {
                self.status = format!("Couldn't open {name}: {e}");
                return;
            }
        };
        let Some(format) = io::detect(&p.to_string_lossy(), &text) else {
            self.status = format!("{name}: cannot tell the format");
            return;
        };
        match io::read_str(&text, format) {
            Ok((other, _)) => self.history.compare = Some(self.compare_models(name, &other)),
            Err(e) => self.status = format!("Couldn't read {name}: {e}"),
        }
    }

    /// What changed going from `other` to the current model.
    pub fn compare_models(&self, name: String, other: &dynamist_core::model::Model) -> CompareFile {
        let diffs = diff::diff_models(other, &self.model);
        let runs = (|| {
            let a = dynamist_core::runner::run_with(other, &[])?;
            let b = dynamist_core::runner::run_with(&self.model, &[])?;
            Ok(diff::diff_runs(&a, &b, 1e-9))
        })();
        CompareFile { name, diffs, runs }
    }

    pub fn history_windows(&mut self, ctx: &egui::Context) {
        let pal = self.pal;
        // ------------------------------------------------------------ History
        if self.history.history_open {
            let key = (self.undo.len(), self.rev);
            if self.history.cache.as_ref().map(|c| (c.0, c.1)) != Some(key) {
                // each saved state against the next one (the last against the model as it is now)
                let start = self.undo.len().saturating_sub(60);
                let mut steps = Vec::new();
                for i in start..self.undo.len() {
                    let next = self.undo.get(i + 1).unwrap_or(&self.model);
                    steps.push(diff::diff_models(&self.undo[i], next));
                }
                self.history.cache = Some((key.0, key.1, steps));
            }
            let steps = self.history.cache.as_ref().map(|c| c.2.clone()).unwrap_or_default();
            let mut open = true;
            egui::Window::new("History").open(&mut open).default_width(640.0).default_height(520.0).show(ctx, |ui| {
                ui.label(RichText::new("What each step of this session changed, newest first. ⌘Z goes back one step. Moving things on the diagram is not a change of meaning and is not listed.").small().color(pal.muted));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    if steps.iter().all(|s| s.is_empty()) && self.history.log.is_empty() {
                        ui.label("Nothing has changed yet in this session.");
                    }
                    if !self.history.log.is_empty() {
                        ui.label(RichText::new("Operation batches").strong());
                        for e in self.history.log.iter().rev() {
                            ui.label(RichText::new(format!("#{} · {} · {} · {} operation(s){}", e.seq, e.actor, e.result, e.ops.len(), e.reason.as_ref().map(|r| format!(" · {r}")).unwrap_or_default())).small().color(if e.result == "rejected" { pal.error } else { pal.text }));
                        }
                        ui.add_space(6.0);
                    }
                    let n = steps.len();
                    for (k, s) in steps.iter().enumerate().rev().filter(|(_, s)| !s.is_empty()) {
                        egui::CollapsingHeader::new(RichText::new(format!("Step {} — {} change(s)", k + 1, s.len())).strong()).id_salt(("hist", k)).default_open(k + 1 == n).show(ui, |ui| {
                            differences_ui(ui, pal, s);
                        });
                    }
                });
            });
            if !open {
                self.history.history_open = false;
            }
        }
        // ------------------------------------------------------------ Compare with a file
        if self.history.compare.is_some() {
            let mut open = true;
            let c = self.history.compare.as_ref().unwrap();
            egui::Window::new(format!("Compared with {}", c.name)).id(egui::Id::new("compare-file")).open(&mut open).default_width(640.0).default_height(520.0).show(ctx, |ui| {
                ui.label(RichText::new("What changed from that file to the model as it is now.").small().color(pal.muted));
                ui.add_space(4.0);
                match &c.runs {
                    Ok(r) if r.is_same() => ui.label(RichText::new("The results are identical.").color(pal.ok)),
                    Ok(r) => {
                        let mut worst: Vec<_> = r.differing.iter().collect();
                        worst.sort_by(|a, b| b.max_rel.total_cmp(&a.max_rel));
                        let top = worst.iter().take(3).map(|d| format!("{} ({:.1}%)", d.name, d.max_rel * 100.0)).collect::<Vec<_>>().join(", ");
                        ui.label(RichText::new(format!("{} of {} series differ{}{}", r.differing.len(), r.compared, if r.differing.is_empty() { "" } else { "; most: " }, top)).color(pal.aux))
                    }
                    Err(e) => ui.label(RichText::new(format!("The results could not be compared: {e}")).color(pal.muted)),
                };
                ui.add_space(4.0);
                if c.diffs.is_empty() {
                    ui.label(RichText::new("No difference in meaning: the same variables, equations, units, sets and checks.").color(pal.ok));
                }
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| differences_ui(ui, pal, &c.diffs));
            });
            if !open {
                self.history.compare = None;
            }
        }
        // ------------------------------------------------------------ Run operations
        if self.history.ops_open {
            let mut open = true;
            let mut apply = false;
            egui::Window::new("Run operations").open(&mut open).default_width(640.0).default_height(520.0).show(ctx, |ui| {
                ui.label(RichText::new("A batch of operations (a JSON list) is applied to a copy, checked, and only then becomes the model: all or nothing. This is the same language the command line and AI agents use.").small().color(pal.muted));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::multiline(&mut self.history.ops_text).font(egui::TextStyle::Monospace).desired_rows(14).desired_width(f32::INFINITY).hint_text("[{\"op\": \"set_equation\", \"name\": \"Burn rate\", \"equation\": \"5\"}]"));
                ui.horizontal(|ui| {
                    apply = ui.button("Apply").clicked();
                    ui.checkbox(&mut self.history.ops_gate, "Refuse results that do not validate");
                    if ui.small_button("Copy the operations reference").on_hover_text("The full list of operations, as Markdown").clicked() {
                        ui.ctx().copy_text(dynamist_core::apidoc::markdown());
                        self.status = "Copied the operations reference".into();
                    }
                });
                if let Some((ok, msg)) = &self.history.ops_message {
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| ui.label(RichText::new(msg).color(if *ok { pal.ok } else { pal.error })));
                }
            });
            if apply {
                self.run_operations();
            }
            if !open {
                self.history.ops_open = false;
            }
        }
    }

    /// Apply the batch in the "Run operations" window.
    pub fn run_operations(&mut self) {
        let batch: Vec<Op> = match serde_json::from_str(&self.history.ops_text) {
            Ok(b) => b,
            Err(e) => {
                self.history.ops_message = Some((false, format!("That is not a list of operations: {e}")));
                return;
            }
        };
        let mut s = Session::new(self.model.clone());
        s.gate = self.history.ops_gate;
        match s.apply("user:app", &batch) {
            Ok(entry) => {
                let mut entry = entry.clone();
                entry.seq = self.history.log.last().map(|e| e.seq + 1).unwrap_or(1);
                self.checkpoint();
                self.model = s.model;
                self.model.ensure_sheets();
                self.changed();
                self.simulate();
                self.history.ops_message = Some((true, format!("Applied {} operation(s); {} warning(s) left.", entry.ops.len(), entry.warnings)));
                self.status = format!("Applied {} operation(s)", entry.ops.len());
                self.history.log.push(entry);
            }
            Err(r) => {
                if let Some(e) = s.log.last() {
                    let mut e = e.clone();
                    e.seq = self.history.log.last().map(|x| x.seq + 1).unwrap_or(1);
                    self.history.log.push(e);
                }
                let mut msg = match r.op_index {
                    Some(i) => format!("Operation {} was refused: {}", i + 1, r.reason),
                    None => r.reason.clone(),
                };
                for i in &r.issues {
                    msg.push_str(&format!("\n• {}{}", i.var.as_ref().map(|v| format!("{v}: ")).unwrap_or_default(), i.message));
                }
                msg.push_str("\nNothing was changed.");
                self.history.ops_message = Some((false, msg));
            }
        }
    }
}

#[allow(dead_code)]
fn _uses(_: &ops::LogEntry) {}
