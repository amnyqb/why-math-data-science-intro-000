//! Moving models between formats from the File menu: Vensim `.mdl` and XMILE in and out, with a
//! window that says exactly what each conversion dropped, approximated or carried over untouched.
#[cfg(not(feature = "gui"))]
use crate::rfd;

use egui::RichText;
use dynamist_core::io::{self, Format};
use dynamist_core::loss::{LossKind, LossReport};
use dynamist_core::model::Model;

use crate::app::App;

/// What the last conversion did, for the report window.
pub struct Conversion {
    pub title: String,
    pub summary: String,
    pub report: LossReport,
}

impl App {
    /// Read a model in a non-native format (Vensim or XMILE) and make it the current model.
    pub fn import_foreign(&mut self, p: std::path::PathBuf, text: &str, format: Format) {
        match io::read_str(text, format) {
            Ok((mut m, loss)) => {
                self.checkpoint();
                m.title = p.file_stem().map(|s| s.to_string_lossy().to_string()).filter(|s| !s.is_empty()).unwrap_or(m.title);
                self.path = None;
                self.saved_model_json = String::new();
                let name = match format {
                    Format::Mdl => "Vensim",
                    Format::Xmile => "XMILE",
                    Format::Sdm => "Dynamist",
                };
                self.status = format!("Imported {} variables from a {name} model", m.vars.len());
                self.finish_open(m);
                self.conversion = Some(Conversion {
                    title: format!("Import from {name}: {}", p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()),
                    summary: self.status.clone(),
                    report: loss,
                });
            }
            Err(e) => self.status = format!("Couldn't read the model: {e}"),
        }
    }

    /// Write the current model as Vensim `.mdl` or XMILE, then show what could not be carried over.
    pub fn export_foreign(&mut self, format: Format) {
        let (label, ext) = match format {
            Format::Mdl => ("Vensim model", "mdl"),
            Format::Xmile => ("XMILE model", "xmile"),
            Format::Sdm => return,
        };
        let name = format!("{}.{ext}", if self.model.title.is_empty() { "model" } else { &self.model.title });
        let Some(p) = rfd::FileDialog::new().add_filter(label, &[ext]).set_file_name(name).save_file() else { return };
        match io::write_str(&self.model, format) {
            Ok((text, loss)) => match std::fs::write(&p, text) {
                Ok(()) => {
                    self.status = format!("Saved {}", p.display());
                    self.conversion = Some(Conversion { title: format!("Export as {label}: {}", p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()), summary: self.status.clone(), report: loss });
                }
                Err(e) => self.status = format!("Couldn't write {}: {e}", p.display()),
            },
            Err(e) => self.status = format!("Couldn't convert the model: {e}"),
        }
    }

    /// The "Conversion report" window.
    pub fn conversion_window(&mut self, ctx: &egui::Context) {
        let Some(c) = &self.conversion else { return };
        let pal = self.pal;
        let mut open = true;
        egui::Window::new(&c.title).id(egui::Id::new("conversion-report")).open(&mut open).collapsible(false).default_width(620.0).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.label(RichText::new(&c.summary).size(15.0).strong());
            ui.add_space(6.0);
            let lost = c.report.items.iter().filter(|i| i.kind != LossKind::Preserved).count();
            if c.report.items.is_empty() {
                ui.label(RichText::new("Nothing was lost: every construct was carried over.").color(pal.ok));
            } else {
                ui.label(RichText::new(format!("{lost} thing(s) dropped or approximated, {} kept as a note", c.report.items.len() - lost)).color(if lost > 0 { pal.aux } else { pal.ok }));
                ui.add_space(4.0);
                egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                    for i in &c.report.items {
                        let (tag, col) = match i.kind {
                            LossKind::Dropped => ("dropped", pal.error),
                            LossKind::Approximated => ("approximated", pal.aux),
                            LossKind::Preserved => ("kept as a note", pal.muted),
                            LossKind::Flattened => ("flattened", pal.aux),
                            LossKind::PreservedNotSimulable => ("kept, cannot run", pal.error),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(tag).small().color(col));
                            ui.label(RichText::new(&i.construct).strong());
                            if let Some(l) = &i.location {
                                ui.label(RichText::new(format!("at {l}")).small().color(pal.muted));
                            }
                            ui.label(RichText::new(&i.detail).color(pal.text));
                        });
                    }
                });
            }
            ui.add_space(8.0);
            ui.label(RichText::new("The same report is printed by the command line: dyn convert. See the wiki pages Format mapping: Vensim and XMILE.").small().color(pal.muted));
        });
        if !open {
            self.conversion = None;
        }
    }
}

#[allow(dead_code)]
fn _uses(_: &Model) {}
