//! The command palette (⌘K), the keyboard shortcut sheet and the About window.

use egui::{Align2, CornerRadius, Key, RichText, Stroke, vec2};

use crate::app::{App, Tool};
use crate::arrange::Align;
use crate::model::{SheetKind, TableMode};

#[derive(Clone, Copy)]
enum Cmd {
    ToggleResults,
    ToggleSide,
    Run,
    Save,
    Open,
    New,
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    SelectAll,
    Delete,
    Group,
    Ungroup,
    Module,
    Align(Align),
    Fit,
    Dark,
    SystemTheme,
    Snap,
    Minimap,
    Shortcuts,
    ExportCsv,
    Tool(Tool),
    Sheet(u64),
    Var(u64),
    NewSheet(SheetKind, TableMode),
    Picture(crate::export::ExportWhat, crate::export::ExportTo),
    Functions,
    Inbox,
}

impl App {
    fn commands(&self) -> Vec<(String, &'static str, Cmd)> {
        let mut c: Vec<(String, &'static str, Cmd)> = vec![
            ("Run the simulation".into(), "⌘R", Cmd::Run),
            ("Save".into(), "⌘S", Cmd::Save),
            ("Open…".into(), "⌘O", Cmd::Open),
            ("New model".into(), "", Cmd::New),
            ("Undo".into(), "⌘Z", Cmd::Undo),
            ("Redo".into(), "⇧⌘Z", Cmd::Redo),
            ("Copy selection".into(), "⌘C", Cmd::Copy),
            ("Paste".into(), "⌘V", Cmd::Paste),
            ("Duplicate selection".into(), "⌘D", Cmd::Duplicate),
            (
                "Select everything on this sheet".into(),
                "⌘A",
                Cmd::SelectAll,
            ),
            ("Delete selection".into(), "⌫", Cmd::Delete),
            ("Group selection".into(), "⌘G", Cmd::Group),
            ("Ungroup".into(), "⇧⌘G", Cmd::Ungroup),
            ("Make a module from selection".into(), "⇧⌘M", Cmd::Module),
            ("Show or hide the results panel".into(), "⌘J", Cmd::ToggleResults),
            ("Show or hide the side panel".into(), "⌘B", Cmd::ToggleSide),
            ("Zoom to fit".into(), "0", Cmd::Fit),
            ("Toggle dark mode".into(), "", Cmd::Dark),
            ("Follow the system's light or dark mode".into(), "", Cmd::SystemTheme),
            ("Toggle snap to grid".into(), "", Cmd::Snap),
            ("Toggle mini-map".into(), "", Cmd::Minimap),
            ("Keyboard shortcuts".into(), "?", Cmd::Shortcuts),
            ("Function reference — every function explained".into(), "F1", Cmd::Functions),
            ("Inbox — messages between conversations".into(), "", Cmd::Inbox),
            ("Export results to CSV…".into(), "", Cmd::ExportCsv),
            (
                "Copy sheet as picture (for Word, PowerPoint, Excel)".into(),
                "⇧⌘C",
                Cmd::Picture(
                    crate::export::ExportWhat::Sheet,
                    crate::export::ExportTo::Clipboard,
                ),
            ),
            (
                "Copy selection as picture".into(),
                "⌘C",
                Cmd::Picture(
                    crate::export::ExportWhat::Selection,
                    crate::export::ExportTo::Clipboard,
                ),
            ),
            (
                "Save sheet as PNG…".into(),
                "",
                Cmd::Picture(
                    crate::export::ExportWhat::Sheet,
                    crate::export::ExportTo::File,
                ),
            ),
            (
                "Copy graph as picture".into(),
                "",
                Cmd::Picture(
                    crate::export::ExportWhat::Trends,
                    crate::export::ExportTo::Clipboard,
                ),
            ),
            (
                "New diagram sheet".into(),
                "",
                Cmd::NewSheet(SheetKind::Diagram, TableMode::Variables),
            ),
            (
                "New equations table".into(),
                "",
                Cmd::NewSheet(SheetKind::Table, TableMode::Variables),
            ),
            (
                "New results table".into(),
                "",
                Cmd::NewSheet(SheetKind::Table, TableMode::Results),
            ),
            (
                "New dashboard".into(),
                "",
                Cmd::NewSheet(SheetKind::Dashboard, TableMode::Variables),
            ),
            (
                "New causal loop diagram".into(),
                "",
                Cmd::NewSheet(SheetKind::CausalLoop, TableMode::Variables),
            ),
            (
                "New optimization".into(),
                "",
                Cmd::NewSheet(SheetKind::Optimize, TableMode::Variables),
            ),
        ];
        for op in Align::ALL {
            c.push((op.label().into(), op.shortcut(), Cmd::Align(op)));
        }
        for t in crate::app::TOOLS {
            c.push((format!("Tool: {}", t.help().0), "", Cmd::Tool(t)));
        }
        for s in &self.model.sheets {
            c.push((
                format!("Go to sheet: {}", s.name),
                s.kind.icon(),
                Cmd::Sheet(s.id),
            ));
        }
        for v in &self.model.vars {
            c.push((format!("Find: {}", v.name), v.kind.label(), Cmd::Var(v.id)));
        }
        c
    }

    fn run_cmd(&mut self, ctx: &egui::Context, c: Cmd) {
        match c {
            Cmd::Run => self.run_and_play(ctx),
            Cmd::Save => self.save(false),
            Cmd::Open => self.open(),
            Cmd::New => self.new_model(),
            Cmd::Undo => self.undo(),
            Cmd::Redo => self.redo(),
            Cmd::Copy => {
                self.copy(false);
            }
            Cmd::Paste => self.paste(),
            Cmd::Duplicate => self.duplicate(),
            Cmd::SelectAll => self.select_all(),
            Cmd::Delete => self.delete_selection(),
            Cmd::Group => self.group_selection(),
            Cmd::Ungroup => self.ungroup(),
            Cmd::Module => self.make_module(),
            Cmd::Align(op) => self.align(op),
            Cmd::Fit => self.fit_pending = true,
            Cmd::ToggleResults => self.show_bottom = !self.show_bottom,
            Cmd::ToggleSide => self.show_right = !self.show_right,
            Cmd::Dark => {
                let d = !self.pal.dark;
                self.set_dark(ctx, d);
            }
            Cmd::SystemTheme => self.set_theme_choice(ctx, crate::theme::ThemeChoice::System),
            Cmd::Snap => self.snap = !self.snap,
            Cmd::Minimap => self.show_minimap = !self.show_minimap,
            Cmd::Shortcuts => self.show_shortcuts = true,
            Cmd::ExportCsv => self.export_csv(),
            Cmd::Tool(t) => {
                self.sheet = self.diagram_view();
                self.tool = t;
            }
            Cmd::Sheet(id) => {
                self.sheet = id;
                self.fit_pending = true;
            }
            Cmd::Var(id) => self.reveal_var(id),
            Cmd::NewSheet(k, m) => self.add_sheet_pub(k, m),
            Cmd::Picture(w, t) => self.request_export(w, t),
            Cmd::Inbox => self.open_inbox(crate::inbox::Tab::Open),
            Cmd::Functions => {
                let t = if let Some(crate::app::Sel::Var(v)) = self.sel { Some(v) } else { None };
                self.open_functions(t);
            }
        }
    }

    pub fn command_palette(&mut self, ctx: &egui::Context) {
        let Some((mut query, mut idx)) = self.palette.clone() else {
            return;
        };
        let pal = self.pal;
        let q = query.to_lowercase();
        let all = self.commands();
        // every word typed must appear somewhere in the entry
        let hits: Vec<(String, &'static str, Cmd)> = all
            .into_iter()
            .filter(|(l, _, _)| q.split_whitespace().all(|w| l.to_lowercase().contains(w)))
            .take(60)
            .collect();
        let (up, down, enter, esc) = ctx.input(|i| {
            (
                i.key_pressed(Key::ArrowUp),
                i.key_pressed(Key::ArrowDown),
                i.key_pressed(Key::Enter),
                i.key_pressed(Key::Escape),
            )
        });
        if down {
            idx = (idx + 1).min(hits.len().saturating_sub(1));
        }
        if up {
            idx = idx.saturating_sub(1);
        }
        idx = idx.min(hits.len().saturating_sub(1));
        let mut run: Option<Cmd> = None;
        let mut close = esc;
        // dim everything behind
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::unique("palette-dim"))
            .fixed_pos(screen.min)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let r = ui.allocate_rect(screen, egui::Sense::click());
                ui.painter().rect_filled(
                    screen,
                    CornerRadius::ZERO,
                    egui::Color32::from_black_alpha(70),
                );
                if r.clicked() {
                    close = true;
                }
            });
        egui::Area::new(egui::Id::unique("palette"))
            .anchor(Align2::CENTER_TOP, vec2(0.0, 110.0))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(pal.panel)
                    .stroke(Stroke::new(1.0, pal.border))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(egui::Margin::same(12))
                    .shadow(egui::Shadow {
                        offset: [0, 10],
                        blur: 40,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_width(560.0);
                        let r = ui.add(
                            egui::TextEdit::singleline(&mut query)
                                .hint_text("Type a command, a sheet or a variable…")
                                .desired_width(f32::INFINITY)
                                .font(egui::FontId::proportional(17.0))
                                .frame(egui::Frame::NONE),
                        );
                        r.request_focus();
                        if r.changed() {
                            idx = 0;
                        }
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .max_height(380.0)
                            .show(ui, |ui| {
                                for (i, (label, hint, c)) in hits.iter().enumerate() {
                                    let on = i == idx;
                                    let resp = egui::Frame::new()
                                        .fill(if on {
                                            pal.accent_soft
                                        } else {
                                            egui::Color32::TRANSPARENT
                                        })
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(10, 6))
                                        .show(ui, |ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(label).color(if on {
                                                    pal.accent
                                                } else {
                                                    pal.text
                                                }));
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            RichText::new(*hint)
                                                                .small()
                                                                .color(pal.muted),
                                                        );
                                                    },
                                                );
                                            });
                                        });
                                    if on && (down || up) {
                                        resp.response.scroll_to_me(None);
                                    }
                                    if resp.response.interact(egui::Sense::click()).clicked() {
                                        run = Some(*c);
                                    }
                                }
                                if hits.is_empty() {
                                    ui.label(RichText::new("Nothing matches").color(pal.muted));
                                }
                            });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("↑↓ to choose · Enter to run · Esc to close")
                                .small()
                                .color(pal.muted),
                        );
                    });
            });
        if enter {
            run = hits.get(idx).map(|h| h.2);
        }
        if let Some(c) = run {
            self.palette = None;
            self.run_cmd(ctx, c);
        } else if close {
            self.palette = None;
        } else {
            self.palette = Some((query, idx));
        }
    }

    pub fn help_windows(&mut self, ctx: &egui::Context) {
        let pal = self.pal;
        let mut open = self.show_shortcuts;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.columns(2, |cols| {
                    let groups: [(&str, &[(&str, &str)]); 4] = [
                        (
                            "Editing",
                            &[
                                ("⌘Z / ⇧⌘Z", "Undo / redo"),
                                (
                                    "⌘X ⌘C ⌘V",
                                    "Cut, copy, paste — ⌘C also copies a picture for Office",
                                ),
                                ("⇧⌘C", "Copy the whole sheet as a transparent picture"),
                                ("⌘D", "Duplicate"),
                                ("⌘A", "Select everything on the sheet"),
                                ("⌫", "Delete selection"),
                                ("Shift-click", "Add to / remove from selection"),
                                ("Drag on empty space", "Select with a box"),
                            ],
                        ),
                        (
                            "Arranging",
                            &[
                                ("⌘G / ⇧⌘G", "Group / ungroup"),
                                ("⇧⌘M", "Make a module from selection"),
                                ("⌥⌘ ← → ↑ ↓", "Align left, right, top, bottom"),
                                ("Arrow keys", "Nudge (hold ⇧ for bigger steps)"),
                                ("Double-click a module", "Open it"),
                            ],
                        ),
                        (
                            "Tools",
                            &[
                                ("V", "Select"),
                                ("S", "Stock"),
                                ("F", "Flow"),
                                ("A", "Variable"),
                                ("C", "Constant"),
                                ("G", "Lookup table"),
                                ("K", "Arrow"),
                                ("N", "Note"),
                                ("Esc", "Back to select / clear selection"),
                            ],
                        ),
                        (
                            "Everything else",
                            &[
                                ("⌘K", "Command palette"),
                                ("⌘J", "Show or hide the results panel"),
                                ("⌘B", "Show or hide the side panel"),
                                ("⌘R", "Run and play over time"),
                                ("⌘S / ⌘O", "Save / open"),
                                ("0", "Zoom to fit"),
                                ("Space-drag", "Pan the diagram"),
                                ("Pinch / ⌘-scroll", "Zoom"),
                                ("?", "This list"),
                            ],
                        ),
                    ];
                    for (i, (title, rows)) in groups.iter().enumerate() {
                        let ui = &mut cols[i / 2];
                        ui.label(RichText::new(*title).strong().color(pal.accent));
                        egui::Grid::new(("keys", i))
                            .num_columns(2)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                for (k, what) in rows.iter() {
                                    ui.label(RichText::new(*k).monospace().color(pal.text));
                                    ui.label(RichText::new(*what).color(pal.muted));
                                    ui.end_row();
                                }
                            });
                        ui.add_space(10.0);
                    }
                });
            });
        self.show_shortcuts = open;
        let mut about = self.show_about;
        egui::Window::new("About Dynamist").open(&mut about).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0)).show(ctx, |ui| {
            ui.set_width(360.0);
            ui.label(RichText::new("Dynamist").size(22.0).strong());
            ui.label(RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION"))).color(pal.muted));
            ui.add_space(6.0);
            ui.label("System dynamics modelling: stock-and-flow diagrams, causal loops, simulation, dashboards and optimization.");
            ui.add_space(6.0);
            ui.label(RichText::new("Written in Rust, drawn with egui. Runs natively on macOS, Windows and Linux.").color(pal.muted));
        });
        self.show_about = about;
    }
}
