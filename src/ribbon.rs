//! The ribbon: tabs (Home, Insert, Arrange, Analyze, View) with labelled groups of buttons, in the
//! manner of Office. It only calls what the app can already do; the menus it replaced live on as
//! `file_menu`, `edit_menu`, `view_menu` and `help_menu` in `app.rs` and open from its dropdowns.

use egui::{Color32, CornerRadius, FontId, Response, RichText, Sense, Stroke, Ui, pos2, vec2};

use crate::app::{App, BottomTab, Tool};
use crate::arrange::Align;
use crate::cld::CldTool;
use crate::icons::{self, Ic};
use crate::model::{Method, SheetKind, TableMode};
use crate::theme::{self, Icon, Palette, ThemeChoice};

/// Height of the ribbon body (groups plus their titles).
pub const RIBBON_H: f32 = 92.0;
const BIG_H: f32 = 64.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RibbonTab {
    #[default]
    Home,
    Insert,
    Arrange,
    Analyze,
    View,
    /// Shown only while a note is selected.
    Format,
}

impl RibbonTab {
    /// The tabs that are always there; `Format` comes and goes with the selection.
    pub const ALL: [RibbonTab; 5] = [RibbonTab::Home, RibbonTab::Insert, RibbonTab::Arrange, RibbonTab::Analyze, RibbonTab::View];
    pub fn label(self) -> &'static str {
        match self {
            RibbonTab::Home => "Home",
            RibbonTab::Insert => "Insert",
            RibbonTab::Arrange => "Arrange",
            RibbonTab::Analyze => "Analyze",
            RibbonTab::View => "View",
            RibbonTab::Format => "Text",
        }
    }
}

// ---- drawing helpers -------------------------------------------------------------------------

fn fg(pal: &Palette, enabled: bool) -> Color32 {
    if enabled { pal.text } else { pal.muted.gamma_multiply(0.7) }
}

fn back(ui: &Ui, pal: &Palette, r: egui::Rect, resp: &Response, active: bool, enabled: bool) {
    if active {
        ui.painter().rect_filled(r, CornerRadius::same(6), pal.accent_soft);
        ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, pal.accent.gamma_multiply(0.5)), egui::StrokeKind::Inside);
    } else if resp.hovered() && enabled {
        ui.painter().rect_filled(r, CornerRadius::same(6), pal.surface);
        ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, pal.border), egui::StrokeKind::Inside);
    }
}

/// What a button draws as its picture.
#[derive(Clone, Copy)]
enum Pic {
    Ic(Ic),
    Tool(Icon, Option<Color32>),
    /// A little drawing of a shape kind.
    Shape(crate::model::ShapeKind),
    /// A little drawing of an input device.
    Control(crate::model::ControlKind),
}

fn paint_pic(ui: &Ui, pal: &Palette, rect: egui::Rect, pic: Pic, enabled: bool, active: bool) {
    match pic {
        Pic::Ic(ic) => icons::paint(ui.painter(), rect, ic, if active { pal.accent } else { fg(pal, enabled) }),
        Pic::Shape(k) => {
            use crate::model::ShapeKind as K;
            let col = if active { pal.accent } else { fg(pal, enabled) };
            let st = Stroke::new(1.8, col);
            let rr = rect.shrink(3.0);
            let p = ui.painter();
            match k {
                K::Rect => {
                    p.rect_stroke(rr, CornerRadius::ZERO, st, egui::StrokeKind::Middle);
                }
                K::Rounded => {
                    p.rect_stroke(rr, CornerRadius::same(6), st, egui::StrokeKind::Middle);
                }
                K::Ellipse => {
                    let pts: Vec<egui::Pos2> = (0..40).map(|i| { let t = i as f32 / 40.0 * std::f32::consts::TAU; pos2(rr.center().x + rr.width() / 2.0 * t.cos(), rr.center().y + rr.height() / 2.0 * 0.85 * t.sin()) }).collect();
                    p.add(egui::Shape::closed_line(pts, st));
                }
                K::Line => {
                    p.line_segment([rr.left_bottom(), rr.right_top()], st);
                }
                K::Arrow => {
                    let (a, b) = (rr.left_bottom(), rr.right_top());
                    p.line_segment([a, b - (b - a).normalized() * 4.0], st);
                    theme::arrow_head(p, b, (b - a).normalized(), 9.0, col);
                }
                K::Picture => {
                    p.rect_stroke(rr, CornerRadius::same(2), st, egui::StrokeKind::Middle);
                    p.circle_filled(rr.left_top() + vec2(rr.width() * 0.3, rr.height() * 0.32), 2.2, col);
                    p.line_segment([rr.left_bottom() + vec2(1.0, -1.0), rr.left_bottom() + vec2(rr.width() * 0.4, -rr.height() * 0.45)], st);
                    p.line_segment([rr.left_bottom() + vec2(rr.width() * 0.4, -rr.height() * 0.45), rr.right_bottom() + vec2(-1.0, -1.0)], st);
                }
            }
        }
        Pic::Control(k) => {
            use crate::model::ControlKind as K;
            let col = if active { pal.accent } else { fg(pal, enabled) };
            let st = Stroke::new(1.8, col);
            let rr = rect.shrink(2.0);
            let p = ui.painter();
            match k {
                K::Slider => {
                    let y = rr.center().y;
                    p.line_segment([pos2(rr.left(), y), pos2(rr.right(), y)], Stroke::new(3.0, col.gamma_multiply(0.45)));
                    p.line_segment([pos2(rr.left(), y), pos2(rr.center().x + 2.0, y)], Stroke::new(3.0, col));
                    p.circle_filled(pos2(rr.center().x + 2.0, y), 4.5, col);
                }
                K::Knob => {
                    let c = rr.center();
                    let arc: Vec<egui::Pos2> = (0..=24).map(|i| { let a = 135f32.to_radians() + 270f32.to_radians() * i as f32 / 24.0; pos2(c.x + 10.0 * a.cos(), c.y + 10.0 * a.sin()) }).collect();
                    p.add(egui::Shape::line(arc, Stroke::new(2.2, col.gamma_multiply(0.5))));
                    p.line_segment([c, pos2(c.x + 6.0, c.y - 6.0)], st);
                }
                K::Switch => {
                    let r = egui::Rect::from_center_size(rr.center(), vec2(22.0, 12.0));
                    p.rect_filled(r, CornerRadius::same(6), col.gamma_multiply(0.35));
                    p.circle_filled(pos2(r.right() - 6.0, r.center().y), 4.5, col);
                }
                K::Number => {
                    let r = egui::Rect::from_center_size(rr.center(), vec2(24.0, 14.0));
                    p.rect_stroke(r, CornerRadius::same(3), st, egui::StrokeKind::Middle);
                    p.line_segment([pos2(r.right() - 9.0, r.center().y), pos2(r.right() - 4.0, r.center().y)], Stroke::new(1.5, col));
                    p.line_segment([pos2(r.left() + 4.0, r.center().y), pos2(r.left() + 10.0, r.center().y)], Stroke::new(1.5, col));
                }
            }
        }
        Pic::Tool(icon, col) => theme::draw_icon(ui.painter(), icon, rect.expand(2.0), if enabled { col.unwrap_or(pal.text) } else { pal.muted.gamma_multiply(0.6) }),
    }
}

/// A large button: picture above, label below. `menu` adds a small drop arrow to the label.
fn big(ui: &mut Ui, pal: &Palette, pic: Pic, label: &str, enabled: bool, active: bool, menu: bool) -> Response {
    let font = FontId::proportional(12.0);
    let text = if menu { format!("{label} ▾") } else { label.to_string() };
    let g = ui.painter().layout_no_wrap(text, font, fg(pal, enabled));
    let w = (g.size().x + 14.0).max(52.0);
    let (r, resp) = ui.allocate_exact_size(vec2(w, BIG_H), if enabled { Sense::click() } else { Sense::hover() });
    back(ui, pal, r, &resp, active, enabled);
    paint_pic(ui, pal, egui::Rect::from_center_size(pos2(r.center().x, r.top() + 22.0), vec2(26.0, 26.0)), pic, enabled, active);
    let col = if active { pal.accent } else { fg(pal, enabled) };
    ui.painter().galley(pos2(r.center().x - g.size().x / 2.0, r.bottom() - g.size().y - 6.0), g, col);
    resp
}

/// A small button: picture and label side by side.
fn small(ui: &mut Ui, pal: &Palette, pic: Pic, label: &str, enabled: bool, active: bool) -> Response {
    let g = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(12.5), fg(pal, enabled));
    let w = 8.0 + 16.0 + 6.0 + g.size().x + 8.0;
    let (r, resp) = ui.allocate_exact_size(vec2(w.max(70.0), 21.0), if enabled { Sense::click() } else { Sense::hover() });
    back(ui, pal, r, &resp, active, enabled);
    paint_pic(ui, pal, egui::Rect::from_min_size(pos2(r.left() + 8.0, r.center().y - 8.0), vec2(16.0, 16.0)), pic, enabled, active);
    let col = if active { pal.accent } else { fg(pal, enabled) };
    ui.painter().galley(pos2(r.left() + 30.0, r.center().y - g.size().y / 2.0), g, col);
    resp
}

/// A group: its buttons in a row, its name underneath, a thin rule at the right.
fn group(ui: &mut Ui, pal: &Palette, title: &str, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
        ui.horizontal_top(|ui| {
            ui.set_min_height(BIG_H);
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            add(ui)
        });
        ui.add_space(3.0);
        ui.label(RichText::new(title).size(11.0).color(pal.muted));
    });
    let (r, _) = ui.allocate_exact_size(vec2(9.0, RIBBON_H - 14.0), Sense::hover());
    ui.painter().line_segment([pos2(r.center().x, r.top() + 2.0), pos2(r.center().x, r.bottom() - 2.0)], Stroke::new(1.0, pal.border));
}

/// Two or three small buttons stacked.
fn stack(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 1.0);
        add(ui)
    });
}

/// The drop-down list under a button.
fn dropdown(resp: &Response, min_w: f32, add: impl FnOnce(&mut Ui)) {
    egui::Popup::menu(resp).show(|ui| {
        ui.set_min_width(min_w);
        add(ui)
    });
}

impl App {
    /// The tab strip on the title row: File, the five tabs, Help.
    pub(crate) fn ribbon_tabs(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        ui.add_space(6.0);
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.menu_button(RichText::new("File").size(14.0), |ui| self.file_menu(ui));
        for t in RibbonTab::ALL {
            let sel = self.ribbon_tab == t && self.ribbon_open;
            let text = RichText::new(t.label()).size(14.0).color(if sel { pal.accent } else { pal.text });
            let r = ui.add(if sel { egui::Button::new(text).fill(pal.accent_soft) } else { egui::Button::new(text).frame(false) });
            if r.clicked() {
                self.ribbon_tab = t;
                self.ribbon_open = true;
            }
            if r.double_clicked() {
                self.ribbon_open = false;
            }
        }
        let (n_notes, n_shapes, n_ctl) = (self.selected_notes().len(), self.selected_shapes().len(), self.selected_controls().len());
        if n_notes + n_shapes + n_ctl > 0 {
            let sel = self.ribbon_tab == RibbonTab::Format && self.ribbon_open;
            let label = match (n_notes > 0, n_shapes > 0, n_ctl > 0) {
                (true, false, false) => "Text",
                (false, true, false) => "Shape",
                (false, false, true) => "Control",
                _ => "Format",
            };
            let text = RichText::new(label).size(14.0).color(if sel { pal.accent } else { pal.aux });
            let r = ui.add(if sel { egui::Button::new(text).fill(pal.accent_soft) } else { egui::Button::new(text).frame(false) }).on_hover_text("Format the selection");
            if r.clicked() {
                self.ribbon_tab = RibbonTab::Format;
                self.ribbon_open = true;
            }
        } else if self.ribbon_tab == RibbonTab::Format {
            self.ribbon_tab = RibbonTab::Home;
        }
        ui.menu_button(RichText::new("Help").size(14.0), |ui| self.help_menu(ui));
    }

    /// The notes in the selection.
    pub(crate) fn selected_notes(&self) -> Vec<u64> {
        self.selected_items().into_iter().filter_map(|s| if let crate::app::Sel::Note(id) = s { Some(id) } else { None }).collect()
    }

    /// The controls in the selection.
    pub(crate) fn selected_controls(&self) -> Vec<u64> {
        self.selected_items().into_iter().filter_map(|s| if let crate::app::Sel::Control(id) = s { Some(id) } else { None }).collect()
    }

    /// The shapes and pictures in the selection.
    pub(crate) fn selected_shapes(&self) -> Vec<u64> {
        self.selected_items().into_iter().filter_map(|s| if let crate::app::Sel::Shape(id) = s { Some(id) } else { None }).collect()
    }

    /// Change every selected shape in one undo step.
    fn edit_shapes(&mut self, f: impl Fn(&mut crate::model::Shape)) {
        let ids = self.selected_shapes();
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for s in self.model.shapes.iter_mut().filter(|s| ids.contains(&s.id)) {
            f(s);
        }
        self.changed();
    }

    /// Change the look of every selected note in one undo step.
    fn edit_fmt(&mut self, f: impl Fn(&mut crate::model::TextFmt)) {
        let ids = self.selected_notes();
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for n in self.model.notes.iter_mut().filter(|n| ids.contains(&n.id)) {
            f(&mut n.fmt);
        }
        self.changed();
    }

    /// The body under the title row.
    pub(crate) fn ribbon(&mut self, ui: &mut Ui) {
        let pal = self.pal;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
            match self.ribbon_tab {
                RibbonTab::Home => self.tab_home(ui, &pal),
                RibbonTab::Insert => self.tab_insert(ui, &pal),
                RibbonTab::Arrange => self.tab_arrange(ui, &pal),
                RibbonTab::Analyze => self.tab_analyze(ui, &pal),
                RibbonTab::View => self.tab_view(ui, &pal),
                RibbonTab::Format => self.tab_format(ui, &pal),
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                if icons::tool(ui, Ic::ChevronUp, "Hide the ribbon (double-click a tab to bring it back)", pal.muted, pal.accent).clicked() {
                    self.ribbon_open = false;
                }
            });
        });
    }

    fn tab_home(&mut self, ui: &mut Ui, pal: &Palette) {
        use crate::export::{ExportTo, ExportWhat};
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        let has_sel = diagram && !self.selected_items().is_empty();
        let ctx = ui.ctx().clone();

        group(ui, pal, "Clipboard", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Paste), "Paste", self.clip.is_some(), false, false).on_hover_text("Paste (⌘V)").clicked() {
                self.paste();
            }
            stack(ui, |ui| {
                if small(ui, pal, Pic::Ic(Ic::Close), "Cut", has_sel, false).on_hover_text("Cut (⌘X)").clicked() {
                    self.copy(true);
                }
                if small(ui, pal, Pic::Ic(Ic::Copy), "Copy", has_sel, false).on_hover_text("Copy (⌘C)").clicked() && self.copy(false) {
                    self.request_export(ExportWhat::Selection, ExportTo::Clipboard);
                }
                if small(ui, pal, Pic::Ic(Ic::Duplicate), "Duplicate", has_sel, false).on_hover_text("Duplicate (⌘D)").clicked() {
                    self.duplicate();
                }
            });
        });
        group(ui, pal, "Edit", |ui| {
            stack(ui, |ui| {
                if small(ui, pal, Pic::Ic(Ic::Undo), "Undo", !self.undo.is_empty(), false).on_hover_text("Undo (⌘Z)").clicked() {
                    self.undo();
                }
                if small(ui, pal, Pic::Ic(Ic::Redo), "Redo", !self.redo.is_empty(), false).on_hover_text("Redo (⇧⌘Z)").clicked() {
                    self.redo();
                }
                if small(ui, pal, Pic::Ic(Ic::Grid), "Select all", diagram, false).on_hover_text("Select everything on this sheet (⌘A)").clicked() {
                    self.select_all();
                }
            });
            if big(ui, pal, Pic::Ic(Ic::Trash), "Delete", self.sel.is_some(), false, false).on_hover_text("Delete the selection (⌫)").clicked() {
                self.delete_selection();
            }
            let more = big(ui, pal, Pic::Ic(Ic::Settings), "More", true, false, true).on_hover_text("History, functions, modules, checks, comparisons…");
            dropdown(&more, 270.0, |ui| self.edit_menu(ui));
        });
        group(ui, pal, "Run", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Play), "Run", true, false, false).on_hover_text("Simulate and watch it unfold over time (⌘R)").clicked() {
                self.run_and_play(&ctx);
            }
            stack(ui, |ui| {
                if small(ui, pal, Pic::Ic(Ic::Pin), "Keep run", self.run.is_some(), false).on_hover_text("Keep this run to compare against later runs").clicked() {
                    self.keep_run();
                }
                if small(ui, pal, Pic::Ic(Ic::Loop), "Live", true, self.live).on_hover_text("Re-simulate instantly as you edit").clicked() {
                    self.live = !self.live;
                }
                if small(ui, pal, Pic::Ic(Ic::Replay), "Replay", self.animate, false).on_hover_text("Watch the run unfold again").clicked() {
                    self.run_and_play(&ctx);
                }
            });
        });
        group(ui, pal, "Simulation", |ui| self.sim_settings_ui(ui, pal));
        group(ui, pal, "Sheets", |ui| {
            let b = big(ui, pal, Pic::Ic(Ic::Plus), "New sheet", true, false, true).on_hover_text("Add a diagram, table, dashboard, causal loop diagram or optimisation sheet");
            dropdown(&b, 230.0, |ui| {
                for (ic, name, kind, mode) in [
                    (Ic::Diagram, "Stock and flow diagram", SheetKind::Diagram, TableMode::Variables),
                    (Ic::Loop, "Causal loop diagram", SheetKind::CausalLoop, TableMode::Variables),
                    (Ic::Table, "Table of variables", SheetKind::Table, TableMode::Variables),
                    (Ic::Table, "Table of results", SheetKind::Table, TableMode::Results),
                    (Ic::Dashboard, "Dashboard", SheetKind::Dashboard, TableMode::Variables),
                    (Ic::Target, "Optimise", SheetKind::Optimize, TableMode::Variables),
                    (Ic::Group, "Architecture: purpose, layers, subsystems", SheetKind::Architecture, TableMode::Variables),
                ] {
                    if icons::menu_item(ui, ic, name, "", true) {
                        self.add_sheet_pub(kind, mode);
                    }
                }
            });
        });
    }

    /// Method, time units, step and the two times, as two rows of small fields.
    fn sim_settings_ui(&mut self, ui: &mut Ui, pal: &Palette) {
        let s = &mut self.model.settings;
        let before = (s.initial_time, s.final_time, s.time_step, s.method);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Time").color(pal.muted));
                ui.add(egui::DragValue::new(&mut s.initial_time).speed(1.0)).on_hover_text("Initial time");
                ui.label(RichText::new("to").color(pal.muted));
                ui.add(egui::DragValue::new(&mut s.final_time).speed(1.0)).on_hover_text("Final time");
                ui.add(egui::TextEdit::singleline(&mut s.time_units).desired_width(52.0)).on_hover_text("Time units");
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Step").color(pal.muted));
                ui.add(egui::DragValue::new(&mut s.time_step).speed(0.01).range(0.0001..=1000.0)).on_hover_text("Time step (dt)");
                egui::ComboBox::from_id_salt("method-ribbon")
                    .width(60.0)
                    .selected_text(match s.method {
                        Method::Euler => "Euler",
                        Method::RungeKutta2 => "RK2",
                        Method::RungeKutta3 => "RK3",
                        Method::RungeKutta4 => "RK4",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut s.method, Method::Euler, "Euler (1st order)");
                        ui.selectable_value(&mut s.method, Method::RungeKutta2, "Runge-Kutta 2 (Heun)");
                        ui.selectable_value(&mut s.method, Method::RungeKutta3, "Runge-Kutta 3");
                        ui.selectable_value(&mut s.method, Method::RungeKutta4, "Runge-Kutta 4");
                    });
            });
        });
        if before != (s.initial_time, s.final_time, s.time_step, s.method) {
            self.changed();
        }
    }

    fn tab_insert(&mut self, ui: &mut Ui, pal: &Palette) {
        let kind = self.active_sheet_kind();
        match kind {
            SheetKind::Diagram => {
                group(ui, pal, "Model", |ui| {
                    for t in [Tool::Select, Tool::Stock, Tool::Flow, Tool::Aux, Tool::Constant, Tool::Lookup, Tool::Link] {
                        let (name, help, key) = t.help();
                        let col = match t {
                            Tool::Stock => Some(pal.stock),
                            Tool::Flow => Some(pal.flow),
                            Tool::Aux => Some(pal.aux),
                            Tool::Constant => Some(pal.constant),
                            Tool::Lookup => Some(pal.lookup),
                            _ => None,
                        };
                        let short = match t {
                            Tool::Select => "Select",
                            Tool::Aux => "Variable",
                            _ => name,
                        };
                        let r = big(ui, pal, Pic::Tool(t.icon(), col), short, true, self.tool == t, false)
                            .on_hover_ui(|ui| {
                                ui.label(RichText::new(format!("{name}  ({})", key.name())).strong());
                                ui.label(help);
                            });
                        if r.clicked() {
                            self.tool = t;
                        }
                    }
                });
                group(ui, pal, "Text", |ui| {
                    let (name, help, key) = Tool::Note.help();
                    let r = big(ui, pal, Pic::Tool(Tool::Note.icon(), None), "Note", true, self.tool == Tool::Note, false)
                        .on_hover_ui(|ui| {
                            ui.label(RichText::new(format!("{name}  ({})", key.name())).strong());
                            ui.label(help);
                        });
                    if r.clicked() {
                        self.tool = Tool::Note;
                    }
                });
                group(ui, pal, "Shapes", |ui| {
                    for k in crate::model::ShapeKind::DRAWN {
                        let on = self.tool == Tool::Shape && self.shape_kind == k;
                        if big(ui, pal, Pic::Shape(k), k.label(), true, on, false).on_hover_text("Drag on the diagram to draw it; click for a standard size").clicked() {
                            self.shape_kind = k;
                            self.tool = Tool::Shape;
                        }
                    }
                });
                group(ui, pal, "Picture", |ui| {
                    if big(ui, pal, Pic::Shape(crate::model::ShapeKind::Picture), "Picture", true, false, false).on_hover_text("Put a PNG or JPEG picture on the diagram").clicked() {
                        self.add_picture();
                    }
                });
                group(ui, pal, "Controls", |ui| {
                    for k in crate::model::ControlKind::ALL {
                        let on = self.tool == Tool::Control && self.control_kind == k;
                        if big(ui, pal, Pic::Control(k), k.label(), true, on, false).on_hover_text("Click on the diagram to put it down; it is tied to the selected constant").clicked() {
                            self.control_kind = k;
                            self.tool = Tool::Control;
                        }
                    }
                });
                group(ui, pal, "Modules", |ui| {
                    let n = self.selected_items().len();
                    if big(ui, pal, Pic::Ic(Ic::Module), "From selection", n >= 1, false, false).on_hover_text("Make a module from the selected items (⇧⌘M)").clicked() {
                        self.make_module();
                    }
                    if big(ui, pal, Pic::Ic(Ic::Import), "Import module", true, false, false).on_hover_text("Bring in a module saved in a file").clicked() {
                        self.import_module();
                    }
                });
                group(ui, pal, "Arrows", |ui| {
                    if big(ui, pal, Pic::Ic(Ic::Trace), "Add inputs", true, false, false).on_hover_text("Add arrows for every equation input").clicked() {
                        self.checkpoint();
                        let n = crate::canvas::sync_links(&mut self.model);
                        self.status = format!("Added {n} arrow(s)");
                        self.changed();
                    }
                });
            }
            SheetKind::CausalLoop => {
                group(ui, pal, "Causal loop diagram", |ui| {
                    for (t, icon, name, tip) in [
                        (CldTool::Select, Icon::Select, "Select", "Click to select, drag to move. Drag empty space to pan."),
                        (CldTool::Variable, Icon::Note, "Variable", "Click to add a word or phrase."),
                        (CldTool::Link, Icon::Link, "Causal arrow", "Drag from a cause to its effect. New arrows start as +."),
                        (CldTool::Loop, Icon::Loop, "Loop marker", "Click inside a loop to label it R (reinforcing) or B (balancing)."),
                    ] {
                        let col = if t == CldTool::Link { Some(pal.link) } else if t == CldTool::Loop { Some(pal.aux) } else { None };
                        if big(ui, pal, Pic::Tool(icon, col), name, true, self.cld.tool == t, false).on_hover_text(tip).clicked() {
                            self.cld.tool = t;
                        }
                    }
                });
            }
            _ => {
                ui.add_space(8.0);
                ui.label(RichText::new("Nothing to insert on this kind of sheet. Open a diagram sheet to add stocks, flows and variables.").color(pal.muted));
            }
        }
    }

    fn tab_arrange(&mut self, ui: &mut Ui, pal: &Palette) {
        let n = self.selected_items().len();
        let diagram = self.active_sheet_kind() == SheetKind::Diagram;
        group(ui, pal, "Align", |ui| {
            // three rows of three: left/centre/right, top/middle/bottom, spread
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                ui.add_space(2.0);
                for row in Align::ALL.chunks(3) {
                    ui.horizontal(|ui| {
                        for op in row {
                            let need = if matches!(op, Align::DistributeH | Align::DistributeV) { 3 } else { 2 };
                            let on = n >= need;
                            let (r, resp) = ui.allocate_exact_size(vec2(28.0, 18.0), if on { Sense::click() } else { Sense::hover() });
                            back(ui, pal, r, &resp, false, on);
                            theme::draw_align(ui.painter(), egui::Rect::from_center_size(r.center(), vec2(14.0, 14.0)), *op, if on { pal.text } else { pal.muted.gamma_multiply(0.6) }, pal.accent);
                            if resp.on_hover_text(format!("{}  {}", op.label(), op.shortcut())).clicked() && on {
                                self.align(*op);
                            }
                        }
                    });
                }
            });
        });
        group(ui, pal, "Group", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Group), "Group", n >= 2, false, false).on_hover_text("Group the selection (⌘G)").clicked() {
                self.group_selection();
            }
            if big(ui, pal, Pic::Ic(Ic::Ungroup), "Ungroup", n >= 1, false, false).on_hover_text("Ungroup (⇧⌘G)").clicked() {
                self.ungroup();
            }
        });
        group(ui, pal, "Module", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Module), "Make module", n >= 1, false, false).on_hover_text("Make a module from the selection (⇧⌘M)").clicked() {
                self.make_module();
            }
            stack(ui, |ui| {
                let m = if let Some(crate::app::Sel::Module(m)) = self.sel { Some(m) } else { None };
                if small(ui, pal, Pic::Ic(Ic::Ungroup), "Dissolve", m.is_some(), false).on_hover_text("Turn the module back into its parts").clicked() {
                    if let Some(m) = m {
                        self.dissolve_module(m);
                    }
                }
                if small(ui, pal, Pic::Ic(Ic::Export), "Export…", n >= 1, false).clicked() {
                    self.export_module();
                }
                if small(ui, pal, Pic::Ic(Ic::Import), "Import…", diagram, false).clicked() {
                    self.import_module();
                }
            });
        });
        group(ui, pal, "Layout", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Align), "Tidy up", diagram, false, false).on_hover_text("Inputs on top, stocks and flows in the middle, calculations below in columns").clicked() {
                self.tidy_diagram();
            }
            if big(ui, pal, Pic::Ic(Ic::Fit), "Fit", true, false, false).on_hover_text("Zoom to fit (0)").clicked() {
                self.fit_pending = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Grid), "Snap", true, self.snap, false).on_hover_text("Snap to the grid").clicked() {
                self.snap = !self.snap;
            }
        });
    }

    fn tab_analyze(&mut self, ui: &mut Ui, pal: &Palette) {
        group(ui, pal, "Results", |ui| {
            for (tab, ic, name, tip) in [
                (BottomTab::Trends, Ic::Graph, "Trends", "Charts of the run"),
                (BottomTab::Table, Ic::Table, "Table", "Values over time"),
                (BottomTab::Causes, Ic::Trace, "Causes & uses", "What drives the selected variable and what it affects"),
                (BottomTab::Problems, Ic::Warning, "Problems", "Equation errors and suggestions"),
                (BottomTab::Runs, Ic::Scenarios, "Runs", "Kept runs and scenarios"),
            ] {
                let on = self.show_bottom && self.bottom_tab == tab;
                if big(ui, pal, Pic::Ic(ic), name, true, on, false).on_hover_text(tip).clicked() {
                    self.bottom_tab = tab;
                    self.show_bottom = true;
                }
            }
        });
        group(ui, pal, "Explore", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Target), "Sensitivity", true, false, false).on_hover_text("Vary the key levers at random and show a band of outcomes").clicked() {
                self.sens_open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Graph), "Compare data", true, false, false).on_hover_text("Compare the model with observed data").clicked() {
                self.compare_open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Target), "Optimise", true, false, false).on_hover_text("Search for the best settings of chosen levers").clicked() {
                self.add_sheet_pub(SheetKind::Optimize, TableMode::Variables);
            }
        });
        group(ui, pal, "Check", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Check), "Model checks", true, false, false).on_hover_text("Conservation and sanity checks").clicked() {
                self.checks_open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Check), "Model health", true, false, false).on_hover_text("What the model doctor found, with fixes you can apply").clicked() {
                self.health.open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Module), "Layers", true, false, false).on_hover_text("The stack of layers: questions, horizons, interfaces, tests").clicked() {
                self.layers.open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Import), "Compare model", true, false, false).on_hover_text("Compare with a saved model").clicked() {
                self.compare_with_file();
            }
            if big(ui, pal, Pic::Ic(Ic::History), "History", true, false, false).on_hover_text("Every change made to this model").clicked() {
                self.history.history_open = true;
            }
        });
        group(ui, pal, "Functions", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Help), "Reference", true, false, false).on_hover_text("Function reference (F1)").clicked() {
                let t = if let Some(crate::app::Sel::Var(v)) = self.sel { Some(v) } else { None };
                self.open_functions(t);
            }
            if big(ui, pal, Pic::Ic(Ic::Sliders), "My functions", true, false, false).on_hover_text("Functions you defined").clicked() {
                self.functions_open = true;
            }
            if big(ui, pal, Pic::Ic(Ic::Module), "Modules", true, false, false).on_hover_text("Reusable modules").clicked() {
                self.modules_open = true;
            }
        });
    }

    fn tab_view(&mut self, ui: &mut Ui, pal: &Palette) {
        group(ui, pal, "Zoom", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Plus), "Zoom in", true, false, false).clicked() {
                self.zoom = (self.zoom * 1.25).clamp(0.2, 3.0);
            }
            if big(ui, pal, Pic::Ic(Ic::Minus), "Zoom out", true, false, false).clicked() {
                self.zoom = (self.zoom / 1.25).clamp(0.2, 3.0);
            }
            if big(ui, pal, Pic::Ic(Ic::Fit), "Fit", true, false, false).on_hover_text("Zoom to fit (0)").clicked() {
                self.fit_pending = true;
            }
        });
        group(ui, pal, "Show", |ui| {
            stack(ui, |ui| {
                if small(ui, pal, Pic::Ic(Ic::PanelBottom), "Results panel", true, self.show_bottom).on_hover_text("⌘J").clicked() {
                    self.show_bottom = !self.show_bottom;
                }
                if small(ui, pal, Pic::Ic(Ic::PanelRight), "Side panel", true, self.show_right).on_hover_text("⌘B").clicked() {
                    self.show_right = !self.show_right;
                }
                if small(ui, pal, Pic::Ic(Ic::Table), "Outline", true, self.show_outline).on_hover_text("The model as a tree").clicked() {
                    self.show_outline = !self.show_outline;
                }
            });
            stack(ui, |ui| {
                if small(ui, pal, Pic::Ic(Ic::Dashboard), "Mini-map", true, self.show_minimap).clicked() {
                    self.show_minimap = !self.show_minimap;
                }
                if small(ui, pal, Pic::Ic(Ic::Help), "Quick guide", true, self.show_help).clicked() {
                    self.show_help = !self.show_help;
                }
                if small(ui, pal, Pic::Ic(Ic::Replay), "Play runs", true, self.animate).on_hover_text("Play runs over time").clicked() {
                    self.animate = !self.animate;
                }
            });
        });
        group(ui, pal, "Appearance", |ui| {
            let ctx = ui.ctx().clone();
            for (c, ic, tip) in [
                (ThemeChoice::Light, Ic::Sun, "Always light"),
                (ThemeChoice::Dark, Ic::Moon, "Always dark"),
                (ThemeChoice::System, Ic::Settings, "Light or dark, as the system is set"),
            ] {
                if big(ui, pal, Pic::Ic(ic), c.label(), true, self.theme_choice == c, false).on_hover_text(tip).clicked() {
                    self.set_theme_choice(&ctx, c);
                }
            }
        });
        group(ui, pal, "Options", |ui| {
            let b = big(ui, pal, Pic::Ic(Ic::Sliders), "Look & arrows", true, false, true).on_hover_text("Diagram look, module style, arrow shape and tip, scenarios in charts");
            dropdown(&b, 280.0, |ui| self.view_menu(ui));
        });
    }

    fn tab_format(&mut self, ui: &mut Ui, pal: &Palette) {
        if !self.selected_notes().is_empty() {
            self.format_text(ui, pal);
        }
        if !self.selected_shapes().is_empty() {
            self.format_shape(ui, pal);
        }
        if !self.selected_controls().is_empty() {
            self.format_control(ui, pal);
        }
    }

    fn format_control(&mut self, ui: &mut Ui, pal: &Palette) {
        use crate::model::{ControlKind, Kind};
        let ids = self.selected_controls();
        let Some(cur) = self.model.controls.iter().find(|c| ids.contains(&c.id)).cloned() else { return };
        let consts: Vec<(u64, String)> = self.model.vars.iter().filter(|v| v.kind == Kind::Constant && v.set.is_none()).map(|v| (v.id, v.name.clone())).collect();
        let st = self.control_state(&cur);
        group(ui, pal, "Variable", |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Sets").color(pal.muted));
                    let shown = consts.iter().find(|(i, _)| *i == cur.var).map(|(_, n)| n.clone()).unwrap_or_else(|| "pick a constant".into());
                    egui::ComboBox::from_id_salt("control-var").width(130.0).selected_text(shown).show_ui(ui, |ui| {
                        for (id, name) in &consts {
                            if ui.selectable_label(cur.var == *id, name).clicked() {
                                let id = *id;
                                self.checkpoint();
                                if let Some(v) = self.model.var_mut(id) {
                                    if v.slider.is_none() {
                                        let x = v.equation.trim().parse::<f64>().unwrap_or(0.0);
                                        v.slider = Some([0.0, (x.abs() * 2.0).max(1.0)]);
                                    }
                                }
                                for c in self.model.controls.iter_mut().filter(|c| ids.contains(&c.id)) {
                                    c.var = id;
                                }
                                self.changed();
                            }
                        }
                    });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Label").color(pal.muted));
                    let mut text = cur.label.clone();
                    if ui.add(egui::TextEdit::singleline(&mut text).desired_width(130.0).hint_text("the variable's name")).changed() {
                        self.checkpoint();
                        for c in self.model.controls.iter_mut().filter(|c| ids.contains(&c.id)) {
                            c.label = text.clone();
                        }
                        self.changed();
                    }
                });
            });
        });
        if let Some(st) = st {
            group(ui, pal, "Range", |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let (mut lo, mut hi) = (st.lo, st.hi);
                        ui.label(RichText::new("From").color(pal.muted));
                        let a = ui.add(egui::DragValue::new(&mut lo).speed(st.step() * 5.0)).on_hover_text("Lowest value").changed();
                        ui.label(RichText::new("to").color(pal.muted));
                        let b = ui.add(egui::DragValue::new(&mut hi).speed(st.step() * 5.0)).on_hover_text("Highest value").changed();
                        if a || b {
                            self.checkpoint();
                            if let Some(v) = self.model.var_mut(cur.var) {
                                v.slider = Some([lo.min(hi), hi.max(lo)]);
                            }
                            self.changed();
                        }
                    });
                    ui.horizontal(|ui| {
                        let mut x = st.value;
                        ui.label(RichText::new("Value").color(pal.muted));
                        if ui.add(egui::DragValue::new(&mut x).speed(st.step() * 5.0)).changed() {
                            self.checkpoint();
                            self.set_control_value(cur.var, x);
                        }
                    });
                });
            });
        }
        group(ui, pal, "Look", |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                for row in ControlKind::ALL.chunks(2) {
                    ui.horizontal(|ui| {
                        for k in row {
                            if small(ui, pal, Pic::Control(*k), k.label(), true, cur.kind == *k).clicked() {
                                self.checkpoint();
                                for c in self.model.controls.iter_mut().filter(|c| ids.contains(&c.id)) {
                                    c.kind = *k;
                                }
                                self.changed();
                            }
                        }
                    });
                }
            });
        });
        group(ui, pal, "Control", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Duplicate), "Duplicate", true, false, false).clicked() {
                self.duplicate();
            }
            if big(ui, pal, Pic::Ic(Ic::Trash), "Delete", true, false, false).clicked() {
                self.delete_selection();
            }
        });
    }

    fn format_shape(&mut self, ui: &mut Ui, pal: &Palette) {
        use crate::model::ShapeKind;
        let ids = self.selected_shapes();
        let Some(cur) = self.model.shapes.iter().find(|s| ids.contains(&s.id)).cloned() else { return };
        let picture = cur.kind == ShapeKind::Picture;
        let line = cur.kind.is_line();
        if !picture {
            group(ui, pal, "Colour", |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                    ui.add_space(4.0);
                    if !line {
                        ui.horizontal(|ui| {
                            let mut c = cur.fill.unwrap_or([255, 255, 255]);
                            if ui.color_edit_button_srgb(&mut c).on_hover_text("Fill colour").changed() {
                                self.edit_shapes(|s| s.fill = Some(c));
                            }
                            if ui.add(egui::Button::new("Fill").selected(cur.fill.is_some()).frame(cur.fill.is_some())).on_hover_text("Fill the inside, or leave it empty").clicked() {
                                let on = cur.fill.is_some();
                                self.edit_shapes(|s| s.fill = if on { None } else { Some(c) });
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        let d = self.pal.text.gamma_multiply(0.65);
                        let mut c = cur.stroke.unwrap_or([d.r(), d.g(), d.b()]);
                        if ui.color_edit_button_srgb(&mut c).on_hover_text("Line colour").changed() {
                            self.edit_shapes(|s| s.stroke = Some(c));
                        }
                        ui.label(RichText::new("Line").color(pal.muted));
                    });
                });
            });
            group(ui, pal, "Line", |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Width").color(pal.muted));
                        let mut w = cur.line_weight();
                        if ui.add(egui::DragValue::new(&mut w).speed(0.1).range(0.5..=12.0).fixed_decimals(1)).on_hover_text("Line width in points").changed() {
                            self.edit_shapes(|s| s.weight = w);
                        }
                    });
                    if line {
                        let is_arrow = cur.kind == ShapeKind::Arrow;
                        if ui.add(egui::Button::new("Arrow head").selected(is_arrow)).clicked() {
                            self.edit_shapes(|s| s.kind = if is_arrow { ShapeKind::Line } else { ShapeKind::Arrow });
                        }
                    }
                });
            });
        }
        if !line {
            group(ui, pal, "Size", |ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                    ui.add_space(4.0);
                    let (mut w, mut h) = (cur.size[0], cur.size[1]);
                    let ratio = h / w.max(1.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Width").color(pal.muted));
                        if ui.add(egui::DragValue::new(&mut w).speed(1.0).range(4.0..=4000.0).fixed_decimals(0)).changed() {
                            self.edit_shapes(|s| {
                                s.size[0] = w;
                                if picture {
                                    s.size[1] = w * ratio;
                                }
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Height").color(pal.muted));
                        if ui.add(egui::DragValue::new(&mut h).speed(1.0).range(4.0..=4000.0).fixed_decimals(0)).changed() {
                            self.edit_shapes(|s| {
                                s.size[1] = h;
                                if picture {
                                    s.size[0] = h / ratio.max(0.001);
                                }
                            });
                        }
                    });
                });
            });
        }
        group(ui, pal, "Shape", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Duplicate), "Duplicate", true, false, false).clicked() {
                self.duplicate();
            }
            if big(ui, pal, Pic::Ic(Ic::Trash), "Delete", true, false, false).clicked() {
                self.delete_selection();
            }
        });
    }

    fn format_text(&mut self, ui: &mut Ui, pal: &Palette) {
        use crate::model::{TextAlign, TextFmt};
        let ids = self.selected_notes();
        let cur = self.model.notes.iter().find(|n| ids.contains(&n.id)).map(|n| n.fmt.clone()).unwrap_or_default();
        let toggle_btn = |ui: &mut Ui, label: RichText, on: bool, tip: &str| ui.add(egui::Button::new(label).selected(on).min_size(vec2(28.0, 24.0))).on_hover_text(tip).clicked();
        group(ui, pal, "Font", |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Size").color(pal.muted));
                    let mut s = cur.points();
                    if ui.add(egui::DragValue::new(&mut s).speed(0.25).range(6.0..=96.0).fixed_decimals(1)).on_hover_text("Size in points").changed() {
                        self.edit_fmt(|f| f.size = s);
                    }
                    if ui.small_button("A+").on_hover_text("Larger").clicked() {
                        let s = (cur.points() + 1.0).min(96.0);
                        self.edit_fmt(|f| f.size = s);
                    }
                    if ui.small_button("A−").on_hover_text("Smaller").clicked() {
                        let s = (cur.points() - 1.0).max(6.0);
                        self.edit_fmt(|f| f.size = s);
                    }
                });
                ui.horizontal(|ui| {
                    if toggle_btn(ui, RichText::new("B").strong(), cur.bold, "Bold") {
                        self.edit_fmt(|f| f.bold = !cur.bold);
                    }
                    if toggle_btn(ui, RichText::new("I").italics(), cur.italic, "Italic") {
                        self.edit_fmt(|f| f.italic = !cur.italic);
                    }
                    if toggle_btn(ui, RichText::new("U").underline(), cur.underline, "Underline") {
                        self.edit_fmt(|f| f.underline = !cur.underline);
                    }
                    if toggle_btn(ui, RichText::new("</>").monospace(), cur.mono, "Fixed-width letters") {
                        self.edit_fmt(|f| f.mono = !cur.mono);
                    }
                });
            });
        });
        group(ui, pal, "Colour", |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 8.0);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let mut c = cur.color.unwrap_or([pal.muted.r(), pal.muted.g(), pal.muted.b()]);
                    if ui.color_edit_button_srgb(&mut c).on_hover_text("Text colour").changed() {
                        self.edit_fmt(|f| f.color = Some(c));
                    }
                    ui.label(RichText::new("Text").color(pal.muted));
                });
                ui.horizontal(|ui| {
                    let mut c = cur.fill.unwrap_or(if pal.dark { [pal.surface.r(), pal.surface.g(), pal.surface.b()] } else { [255, 251, 235] });
                    if ui.color_edit_button_srgb(&mut c).on_hover_text("Background colour").changed() {
                        self.edit_fmt(|f| f.fill = Some(c));
                    }
                    ui.label(RichText::new("Background").color(pal.muted));
                });
            });
        });
        group(ui, pal, "Paragraph", |ui| {
            for (al, tip) in [(TextAlign::Left, "Align left"), (TextAlign::Center, "Centre"), (TextAlign::Right, "Align right")] {
                let (r, resp) = ui.allocate_exact_size(vec2(32.0, 28.0), Sense::click());
                back(ui, pal, r, &resp, cur.align == al, true);
                let col = if cur.align == al { pal.accent } else { pal.text };
                let (x0, x1) = (r.left() + 8.0, r.right() - 8.0);
                for (i, frac) in [1.0f32, 0.6, 0.85, 0.5].into_iter().enumerate() {
                    let w = (x1 - x0) * frac;
                    let (a, b) = match al {
                        TextAlign::Left => (x0, x0 + w),
                        TextAlign::Center => (r.center().x - w / 2.0, r.center().x + w / 2.0),
                        TextAlign::Right => (x1 - w, x1),
                    };
                    let y = r.top() + 6.0 + i as f32 * 4.5;
                    ui.painter().line_segment([pos2(a, y), pos2(b, y)], Stroke::new(1.6, col));
                }
                if resp.on_hover_text(tip).clicked() {
                    self.edit_fmt(|f| f.align = al);
                }
            }
        });
        group(ui, pal, "Note", |ui| {
            if big(ui, pal, Pic::Ic(Ic::Replay), "Plain", !cur.is_plain(), false, false).on_hover_text("Back to the plain note").clicked() {
                self.edit_fmt(|f| *f = TextFmt::default());
            }
            if big(ui, pal, Pic::Ic(Ic::Duplicate), "Duplicate", true, false, false).clicked() {
                self.duplicate();
            }
            if big(ui, pal, Pic::Ic(Ic::Trash), "Delete", true, false, false).clicked() {
                self.delete_selection();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::Headless;
    use dynamist_core::model::Model;

    #[test]
    fn every_tab_draws_on_a_diagram_sheet() {
        let mut h = Headless::new(Model::example(), None);
        for t in RibbonTab::ALL {
            let png = h.window_png(Some(t.label()), (1300.0, 400.0), 1.0).unwrap();
            assert!(png.len() > 2000, "{} drew nothing", t.label());
        }
        assert!(h.window_png(Some("Nonsense"), (800.0, 300.0), 1.0).is_err());
    }

    #[test]
    fn a_formatted_note_reaches_the_svg() {
        use dynamist_core::model::{TextAlign, TextFmt};
        let mut m = Model::example();
        m.notes[0].fmt = TextFmt { italic: true, underline: true, bold: true, size: 20.0, align: TextAlign::Center, color: Some([200, 0, 0]), ..Default::default() };
        let mut h = Headless::new(m, None);
        let fig = h.sheet(None).unwrap();
        let svg = fig.svg(&Default::default()).unwrap().text;
        assert!(svg.contains("font-style=\"italic\""), "italic is missing");
        assert!(svg.contains("text-decoration=\"underline\""), "underline is missing");
        // a plain note adds none of it, and its text is smaller
        let mut h = Headless::new(Model::example(), None);
        let plain = h.sheet(None).unwrap().svg(&Default::default()).unwrap().text;
        assert!(!plain.contains("text-decoration"));
        let biggest = |s: &str| s.split("font-size=\"").skip(1).filter_map(|r| r.split('"').next()?.parse::<f32>().ok()).fold(0.0f32, f32::max);
        assert!(biggest(&svg) > biggest(&plain) * 1.3, "size is missing: {} against {}", biggest(&svg), biggest(&plain));
    }
}
