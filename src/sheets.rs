//! Sheets: the tab strip, spreadsheet-style table sheets and dashboard sheets.

use egui::{
    Align, Align2, Color32, CornerRadius, FontId, Layout, Rect, RichText, Sense, Stroke,
    StrokeKind, UiBuilder, pos2, vec2,
};

use crate::app::{App, Sel};
use crate::expr;
use crate::model::{Kind, Sheet, SheetKind, TableMode, Widget, WidgetKind};
use crate::theme;

impl App {
    pub fn active_sheet_kind(&self) -> SheetKind {
        self.model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .map(|s| s.kind)
            .unwrap_or(SheetKind::Diagram)
    }

    pub fn add_sheet_pub(&mut self, kind: SheetKind, mode: TableMode) {
        self.add_sheet(kind, mode);
    }

    fn add_sheet(&mut self, kind: SheetKind, mode: TableMode) {
        self.checkpoint();
        let id = self.model.new_id();
        let n = self.model.sheets.iter().filter(|s| s.kind == kind).count() + 1;
        let name = match (kind, mode) {
            (SheetKind::Diagram, _) => format!("View {n}"),
            (SheetKind::Table, TableMode::Variables) => format!("Equations {n}"),
            (SheetKind::Table, TableMode::Results) => format!("Results {n}"),
            (SheetKind::Dashboard, _) => format!("Dashboard {n}"),
            (SheetKind::CausalLoop, _) => format!("Causal loops {n}"),
            (SheetKind::Optimize, _) => format!("Optimize {n}"),
            (SheetKind::Scenarios, _) => format!("Scenarios {n}"),
            (SheetKind::Entities, _) => format!("Entities {n}"),
            (SheetKind::Architecture, _) => format!("Architecture {n}"),
        };
        let mut s = Sheet::new(id, &name, kind);
        s.table_mode = mode;
        if kind == SheetKind::Table && mode == TableMode::Results {
            s.columns = self
                .model
                .vars
                .iter()
                .filter(|v| v.kind == Kind::Stock)
                .map(|v| v.name.clone())
                .collect();
        }
        if kind == SheetKind::Dashboard {
            let stocks: Vec<String> = self
                .model
                .vars
                .iter()
                .filter(|v| v.kind == Kind::Stock)
                .map(|v| v.name.clone())
                .collect();
            let consts: Vec<String> = self
                .model
                .vars
                .iter()
                .filter(|v| v.kind == Kind::Constant)
                .map(|v| v.name.clone())
                .collect();
            for st in stocks.iter().take(4) {
                let wid = self.model.new_id();
                s.widgets.push(Widget {
                    id: wid,
                    kind: WidgetKind::Number,
                    title: st.clone(),
                    vars: vec![st.clone()],
                    text: String::new(),
                    chart: Default::default(), style: Default::default(),
                    span: 1,
                    height: 130.0,
                });
            }
            let wid = self.model.new_id();
            s.widgets.push(Widget {
                id: wid,
                kind: WidgetKind::Graph,
                title: "Stocks over time".into(),
                vars: stocks,
                text: String::new(),
                chart: Default::default(), style: Default::default(),
                span: 3,
                height: 300.0,
            });
            let wid = self.model.new_id();
            s.widgets.push(Widget {
                id: wid,
                kind: WidgetKind::Slider,
                title: "Inputs".into(),
                vars: consts,
                text: String::new(),
                chart: Default::default(), style: Default::default(),
                span: 1,
                height: 300.0,
            });
        }
        self.model.sheets.push(s);
        self.sheet = id;
        if kind == SheetKind::Diagram {
            self.fit_pending = true;
        }
    }

    fn delete_sheet(&mut self, id: u64) {
        let Some(s) = self.model.sheets.iter().find(|s| s.id == id).cloned() else {
            return;
        };
        let diagrams = self
            .model
            .sheets
            .iter()
            .filter(|x| x.kind == SheetKind::Diagram)
            .count();
        if s.kind == SheetKind::Diagram && diagrams <= 1 {
            self.status = "A model needs at least one diagram sheet".into();
            return;
        }
        self.checkpoint();
        self.model.sheets.retain(|x| x.id != id);
        // anything drawn on a removed diagram moves to the first remaining one
        let moved = self.model.vars.iter().filter(|v| v.view == id).count();
        self.model.ensure_sheets();
        if moved > 0 {
            self.status = format!(
                "Sheet removed; its {moved} item(s) moved to '{}'",
                self.model
                    .sheets
                    .iter()
                    .find(|x| x.id == self.model.first_diagram())
                    .map(|x| x.name.as_str())
                    .unwrap_or("")
            );
        }
        if self.sheet == id {
            self.sheet = self.model.first_diagram();
            self.fit_pending = true;
        }
    }

    /// The strip of sheet tabs, Excel-style, under the top bar.
    pub fn sheet_tabs(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let sheets = self.model.sheets.clone();
            let mut action: Option<(u64, &str)> = None;
            // levels: inside a module, the way up — Model › Plans › Optimal plan
            let mut ancestors: Vec<(u64, String)> = Vec::new();
            let mut cur = self.sheet;
            for _ in 0..32 {
                let Some(m) = self.model.modules.iter().find(|m| m.inner == cur) else { break };
                if let Some(up) = self.model.sheets.iter().find(|x| x.id == m.view) {
                    ancestors.push((up.id, up.name.clone()));
                }
                cur = m.view;
            }
            if !ancestors.is_empty() {
                ancestors.reverse();
                for (id, name) in ancestors {
                    if ui.add(egui::Button::new(RichText::new(name).color(pal.accent)).frame(false)).on_hover_text("Go up to this level").clicked() {
                        self.sheet = id;
                        self.fit_pending = true;
                    }
                    ui.label(RichText::new("›").color(pal.muted));
                }
            }
            for (i, s) in sheets.iter().enumerate() {
                let active = s.id == self.sheet;
                // module sheets are a level below: reached from their box or the Modules menu, shown here only while open
                if s.module.is_some() && !active {
                    continue;
                }
                if let Some((rid, buf)) = &mut self.renaming {
                    if *rid == s.id {
                        let r = ui.add(egui::TextEdit::singleline(buf).desired_width(120.0));
                        r.request_focus();
                        if r.lost_focus() {
                            let name = buf.trim().to_string();
                            let rid = *rid;
                            self.renaming = None;
                            if !name.is_empty() {
                                self.checkpoint();
                                if let Some(x) = self.model.sheet_mut(rid) {
                                    x.name = name;
                                }
                            }
                        }
                        continue;
                    }
                }
                let col = match s.kind {
                    SheetKind::Diagram => pal.stock,
                    SheetKind::Table => pal.constant,
                    SheetKind::Dashboard => pal.lookup,
                    SheetKind::CausalLoop => pal.aux,
                    SheetKind::Optimize => pal.flow,
                    SheetKind::Scenarios => pal.series(4),
                    SheetKind::Entities => pal.series(2),
                    SheetKind::Architecture => pal.series(5),
                };
                // a tab: coloured icon for the sheet type, then its name
                let ic = if s.module.is_some() {
                    crate::icons::Ic::Module
                } else {
                    match s.kind {
                        SheetKind::Diagram => crate::icons::Ic::Diagram,
                        SheetKind::Table => crate::icons::Ic::Table,
                        SheetKind::Dashboard => crate::icons::Ic::Dashboard,
                        SheetKind::CausalLoop => crate::icons::Ic::Loop,
                        SheetKind::Optimize => crate::icons::Ic::Target,
                        SheetKind::Scenarios => crate::icons::Ic::Scenarios,
                        SheetKind::Entities => crate::icons::Ic::Group,
                        SheetKind::Architecture => crate::icons::Ic::Group,
                    }
                };
                let font = egui::FontId::proportional(14.0);
                let g = ui.painter().layout_no_wrap(s.name.clone(), font, if active { pal.text } else { pal.muted });
                let (tr, r) = ui.allocate_exact_size(vec2(g.size().x + 44.0, 32.0), Sense::click());
                if active {
                    ui.painter().rect_filled(tr, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, pal.canvas);
                    ui.painter().rect_stroke(tr, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, Stroke::new(1.0, pal.border), StrokeKind::Inside);
                } else if r.hovered() {
                    ui.painter().rect_filled(tr, CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 }, pal.panel.gamma_multiply(0.7));
                }
                crate::icons::paint(ui.painter(), egui::Rect::from_min_size(pos2(tr.left() + 12.0, tr.center().y - 8.0), vec2(16.0, 16.0)), ic, if active { col } else { col.gamma_multiply(0.75) });
                ui.painter().galley(pos2(tr.left() + 34.0, tr.center().y - g.size().y / 2.0), g, pal.text);
                let r = r.on_hover_text(format!("{} sheet · double-click to rename · right-click for more", if s.module.is_some() { "Module" } else { s.kind.label() }));
                if active {
                    let rr = r.rect;
                    ui.painter().line_segment([pos2(rr.left() + 6.0, rr.bottom() - 1.0), pos2(rr.right() - 6.0, rr.bottom() - 1.0)], Stroke::new(2.5, col));
                }
                if r.clicked() && !active {
                    self.sheet = s.id;
                    if s.kind == SheetKind::Diagram {
                        self.fit_pending = true;
                    }
                }
                if r.double_clicked() {
                    self.renaming = Some((s.id, s.name.clone()));
                }
                r.context_menu(|ui| {
                    if ui.button("Rename").clicked() {
                        self.renaming = Some((s.id, s.name.clone()));
                        ui.close();
                    }
                    if s.kind != SheetKind::Diagram && ui.button("Duplicate").clicked() {
                        action = Some((s.id, "dup"));
                        ui.close();
                    }
                    if i > 0 && ui.button("Move left").clicked() {
                        action = Some((s.id, "left"));
                        ui.close();
                    }
                    if i + 1 < sheets.len() && ui.button("Move right").clicked() {
                        action = Some((s.id, "right"));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(RichText::new("Delete sheet").color(pal.error)).clicked() {
                        action = Some((s.id, "del"));
                        ui.close();
                    }
                });
            }
            ui.add_space(4.0);
            if !self.model.modules.is_empty() {
                ui.menu_button(RichText::new("Modules ▾").color(pal.accent), |ui| {
                    ui.set_min_width(220.0);
                    ui.label(RichText::new("Open a module — a level below its sheet").small().color(pal.muted));
                    for m in self.model.modules.clone() {
                        // indent by how many levels down it sits
                        let mut depth = 0;
                        let mut at = m.view;
                        while let Some(p) = self.model.modules.iter().find(|x| x.inner == at) {
                            depth += 1;
                            at = p.view;
                            if depth > 8 {
                                break;
                            }
                        }
                        let label = format!("{}{}", "    ".repeat(depth), m.name);
                        if ui.selectable_label(self.sheet == m.inner, label).clicked() {
                            self.sheet = m.inner;
                            self.fit_pending = true;
                            ui.close();
                        }
                    }
                })
                .response
                .on_hover_text("Modules are levels below the overview");
            }
            ui.menu_button(RichText::new("+").strong().color(pal.accent), |ui| {
                ui.label(RichText::new("New sheet").small().strong().color(pal.muted));
                ui.set_min_width(330.0);
                use crate::icons::{Ic, menu_item};
                for (ic, label, kind, mode) in [
                    (Ic::Diagram, "Diagram — another view of the model", SheetKind::Diagram, TableMode::Variables),
                    (Ic::Table, "Equations table — every variable as a row", SheetKind::Table, TableMode::Variables),
                    (Ic::Table, "Results table — values over time", SheetKind::Table, TableMode::Results),
                    (Ic::Dashboard, "Dashboard — cards, graphs and sliders", SheetKind::Dashboard, TableMode::Variables),
                    (Ic::Loop, "Causal loop diagram — loops and signs", SheetKind::CausalLoop, TableMode::Variables),
                    (Ic::Target, "Optimize — find the best settings", SheetKind::Optimize, TableMode::Variables),
                    (Ic::Scenarios, "Scenarios — run what-ifs side by side", SheetKind::Scenarios, TableMode::Variables),
                ] {
                    if menu_item(ui, ic, label, "", true) {
                        self.add_sheet(kind, mode);
                    }
                }
                ui.separator();
                if menu_item(ui, Ic::Import, "Import a module…", "", true) {
                    self.import_module();
                }
            })
            .response
            .on_hover_text("Add a sheet");
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                egui::ComboBox::from_id_salt("export-scale")
                    .width(54.0)
                    .selected_text(format!("{:.0}×", self.export_scale))
                    .show_ui(ui, |ui| {
                        for (k, label) in [(2.0, "2×  (192 dpi)"), (3.0, "3×  (288 dpi)"), (4.0, "4×  (384 dpi, print)")] {
                            ui.selectable_value(&mut self.export_scale, k, label);
                        }
                    })
                    .response
                    .on_hover_text("Picture quality for copies and saved PNGs");
                if crate::icons::button(ui, crate::icons::Ic::Download, Some("Save PNG"), false, pal.text, pal.accent).on_hover_text("Save this sheet as a transparent PNG picture").clicked() {
                    self.request_export(crate::export::ExportWhat::Sheet, crate::export::ExportTo::File);
                }
                if crate::icons::button(ui, crate::icons::Ic::Image, Some("Copy as picture"), false, pal.accent, pal.accent).on_hover_text("Copy this sheet as a transparent picture — paste into Word, PowerPoint or Excel (⇧⌘C)").clicked() {
                    self.request_export(crate::export::ExportWhat::Sheet, crate::export::ExportTo::Clipboard);
                }
            });
            if let Some((id, what)) = action {
                let i = self.model.sheets.iter().position(|s| s.id == id).unwrap();
                match what {
                    "del" => self.delete_sheet(id),
                    "left" => {
                        self.checkpoint();
                        self.model.sheets.swap(i, i - 1);
                    }
                    "right" => {
                        self.checkpoint();
                        self.model.sheets.swap(i, i + 1);
                    }
                    _ => {
                        self.checkpoint();
                        let mut copy = self.model.sheets[i].clone();
                        copy.id = self.model.new_id();
                        copy.name = format!("{} copy", copy.name);
                        for w in copy.widgets.iter_mut() {
                            w.id = self.model.new_id();
                        }
                        self.sheet = copy.id;
                        self.model.sheets.insert(i + 1, copy);
                    }
                }
            }
        });
    }

    // ------------------------------------------------------------------ tables

    pub fn table_sheet(&mut self, ui: &mut egui::Ui) {
        let Some(sheet) = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .cloned()
        else {
            return;
        };
        let pal = self.pal;
        egui::Frame::new()
            .fill(pal.canvas)
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&sheet.name).size(18.0).strong());
                    ui.add_space(10.0);
                    let mut mode = sheet.table_mode;
                    ui.selectable_value(&mut mode, TableMode::Variables, "Equations");
                    ui.selectable_value(&mut mode, TableMode::Results, "Results over time");
                    if mode != sheet.table_mode {
                        if let Some(s) = self.model.sheet_mut(sheet.id) {
                            s.table_mode = mode;
                        }
                    }
                });
                ui.add_space(6.0);
                match sheet.table_mode {
                    TableMode::Variables => self.variables_table(ui, &sheet),
                    TableMode::Results => self.results_table(ui, &sheet),
                }
            });
    }

    fn variables_table(&mut self, ui: &mut egui::Ui, sheet: &Sheet) {
        let pal = self.pal;
        let filter_id = ui.make_persistent_id(("filter", sheet.id));
        let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id)).unwrap_or_default();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut filter).hint_text("🔍 Filter by name, equation or units").desired_width(280.0));
            ui.label(RichText::new("Edit equations, units and notes right in the cells. Click a row number to open it on the right.").small().color(pal.muted));
        });
        ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
        ui.add_space(6.0);
        let f = filter.to_lowercase();
        let rows: Vec<u64> = self
            .model
            .vars
            .iter()
            .filter(|v| {
                f.is_empty()
                    || v.name.to_lowercase().contains(&f)
                    || v.equation.to_lowercase().contains(&f)
                    || v.units.to_lowercase().contains(&f)
            })
            .map(|v| v.id)
            .collect();
        let cols: [(&str, f32); 9] = [
            ("#", 36.0),
            ("Name", 170.0),
            ("Type", 86.0),
            ("Equation / initial value", 0.0),
            ("Units", 100.0),
            ("Now", 90.0),
            ("Min", 80.0),
            ("Max", 80.0),
            ("Notes", 200.0),
        ];
        let fixed: f32 = cols.iter().map(|c| c.1).sum();
        let flex = (ui.available_width() - fixed - 2.0).max(220.0);
        let widths: Vec<f32> = cols
            .iter()
            .map(|c| if c.1 == 0.0 { flex } else { c.1 })
            .collect();
        let row_h = 28.0;
        let total_w: f32 = widths.iter().sum();

        let sheet_frame = egui::Frame::new()
            .fill(pal.panel)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(CornerRadius::same(8));
        sheet_frame.show(ui, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // header
                    let (hr, _) = ui.allocate_exact_size(vec2(total_w, row_h), Sense::hover());
                    ui.painter()
                        .rect_filled(hr, CornerRadius::ZERO, pal.surface);
                    let mut x = hr.left();
                    for (i, (name, _)) in cols.iter().enumerate() {
                        ui.painter().text(
                            pos2(x + 8.0, hr.center().y),
                            Align2::LEFT_CENTER,
                            *name,
                            FontId::proportional(12.0),
                            pal.muted,
                        );
                        x += widths[i];
                        ui.painter().line_segment(
                            [pos2(x, hr.top()), pos2(x, hr.bottom())],
                            Stroke::new(1.0, pal.border),
                        );
                    }
                    ui.painter().line_segment(
                        [hr.left_bottom(), hr.right_bottom()],
                        Stroke::new(1.0, pal.border),
                    );
                    let ti = self.time_index();
                    for (ri, id) in rows.iter().enumerate() {
                        let Some(orig) = self.model.var(*id).cloned() else {
                            continue;
                        };
                        let mut v = orig.clone();
                        let (rr, _) = ui.allocate_exact_size(vec2(total_w, row_h), Sense::hover());
                        let selected = self.sel == Some(Sel::Var(*id));
                        if selected {
                            ui.painter()
                                .rect_filled(rr, CornerRadius::ZERO, pal.accent_soft);
                        } else if ri % 2 == 1 {
                            ui.painter().rect_filled(
                                rr,
                                CornerRadius::ZERO,
                                pal.surface.gamma_multiply(0.5),
                            );
                        }
                        let mut x = rr.left();
                        let cell = |i: usize, x: f32| {
                            Rect::from_min_size(pos2(x, rr.top()), vec2(widths[i], row_h))
                                .shrink2(vec2(4.0, 2.0))
                        };
                        let has_problem = self
                            .problems
                            .iter()
                            .any(|p| p.var.as_deref() == Some(orig.name.as_str()));
                        for i in 0..cols.len() {
                            let c = cell(i, x);
                            match i {
                                0 => {
                                    let r = ui.put(
                                        c,
                                        egui::Button::new(
                                            RichText::new(format!("{}", ri + 1))
                                                .small()
                                                .color(pal.muted),
                                        )
                                        .frame(false),
                                    );
                                    if r.on_hover_text("Open in the inspector").clicked() {
                                        self.sel = Some(Sel::Var(*id));
                                    }
                                }
                                1 => {
                                    let r =
                                        ui.put(
                                            c,
                                            egui::Button::new(RichText::new(&v.name).color(
                                                if has_problem { pal.error } else { pal.text },
                                            ))
                                            .frame(false)
                                            .truncate(),
                                        );
                                    if r.clicked() {
                                        self.sel = Some(Sel::Var(*id));
                                    }
                                }
                                2 => {
                                    let col = pal.kind(v.kind);
                                    ui.painter().rect_filled(
                                        Rect::from_center_size(
                                            c.left_center() + vec2(36.0, 0.0),
                                            vec2(72.0, 18.0),
                                        ),
                                        CornerRadius::same(9),
                                        col.gamma_multiply(0.13),
                                    );
                                    ui.painter().text(
                                        c.left_center() + vec2(36.0, 0.0),
                                        Align2::CENTER_CENTER,
                                        v.kind.label(),
                                        FontId::proportional(11.0),
                                        col,
                                    );
                                }
                                3 => {
                                    let lookup_fn =
                                        v.kind == Kind::Lookup && v.equation.trim().is_empty();
                                    let r = ui.put(
                                        c,
                                        egui::TextEdit::singleline(&mut v.equation)
                                            .font(egui::TextStyle::Monospace)
                                            .frame(egui::Frame::NONE)
                                            .hint_text(if lookup_fn {
                                                format!("lookup table · {} points", v.table.len())
                                            } else {
                                                String::new()
                                            }),
                                    );
                                    if r.gained_focus() {
                                        self.checkpoint();
                                    }
                                }
                                4 => {
                                    let r = ui.put(
                                        c,
                                        egui::TextEdit::singleline(&mut v.units)
                                            .frame(egui::Frame::NONE)
                                            .hint_text("—"),
                                    );
                                    if r.gained_focus() {
                                        self.checkpoint();
                                    }
                                }
                                5..=7 => {
                                    let series = self.run.as_ref().and_then(|r| r.get(&v.name));
                                    let val = series.and_then(|s| match i {
                                        5 => ti.and_then(|t| s.get(t).copied()),
                                        6 => s.iter().copied().reduce(f64::min),
                                        _ => s.iter().copied().reduce(f64::max),
                                    });
                                    ui.painter().text(
                                        c.right_center() - vec2(4.0, 0.0),
                                        Align2::RIGHT_CENTER,
                                        val.map(theme::fmt_num).unwrap_or_default(),
                                        FontId::monospace(11.5),
                                        if i == 5 { pal.kind(v.kind) } else { pal.muted },
                                    );
                                }
                                _ => {
                                    let r = ui.put(
                                        c,
                                        egui::TextEdit::singleline(&mut v.comment)
                                            .frame(egui::Frame::NONE)
                                            .hint_text("—"),
                                    );
                                    if r.gained_focus() {
                                        self.checkpoint();
                                    }
                                }
                            }
                            x += widths[i];
                            ui.painter().line_segment(
                                [pos2(x, rr.top()), pos2(x, rr.bottom())],
                                Stroke::new(1.0, pal.border.gamma_multiply(0.7)),
                            );
                        }
                        ui.painter().line_segment(
                            [rr.left_bottom(), rr.right_bottom()],
                            Stroke::new(1.0, pal.border.gamma_multiply(0.7)),
                        );
                        if v.equation != orig.equation
                            || v.units != orig.units
                            || v.comment != orig.comment
                        {
                            if let Some(slot) = self.model.var_mut(*id) {
                                *slot = v;
                            }
                            self.changed();
                        }
                    }
                    if rows.is_empty() {
                        ui.add_space(10.0);
                        ui.label(RichText::new("No variables match.").color(pal.muted));
                    }
                });
        });
    }

    fn results_table(&mut self, ui: &mut egui::Ui, sheet: &Sheet) {
        let pal = self.pal;
        let Some(run) = self.run.clone() else {
            ui.label(
                RichText::new("No results yet — fix the problems listed on the diagram sheet.")
                    .color(pal.muted),
            );
            return;
        };
        let mut cols: Vec<String> = sheet
            .columns
            .iter()
            .filter(|c| run.get(c).is_some())
            .cloned()
            .collect();
        if cols.is_empty() {
            cols = self
                .model
                .vars
                .iter()
                .filter(|v| run.get(&v.name).is_some())
                .map(|v| v.name.clone())
                .collect();
        }
        let every_id = ui.make_persistent_id(("every", sheet.id));
        let mut every: usize = ui
            .data_mut(|d| d.get_temp(every_id))
            .unwrap_or_else(|| ((1.0 / self.model.settings.time_step).round() as usize).max(1));
        let rows: Vec<usize> = (0..run.times.len()).step_by(every.max(1)).collect();
        ui.horizontal(|ui| {
            ui.menu_button("Columns…", |ui| {
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        for v in self
                            .model
                            .vars
                            .iter()
                            .filter(|v| run.get(&v.name).is_some())
                        {
                            let mut on = sheet.columns.contains(&v.name);
                            if ui
                                .checkbox(&mut on, RichText::new(&v.name).color(pal.kind(v.kind)))
                                .changed()
                            {
                                let name = v.name.clone();
                                if let Some(s) =
                                    self.model.sheets.iter_mut().find(|s| s.id == sheet.id)
                                {
                                    if on {
                                        s.columns.push(name)
                                    } else {
                                        s.columns.retain(|c| *c != name)
                                    }
                                }
                            }
                        }
                    });
            });
            ui.label(RichText::new("Show every").color(pal.muted));
            ui.add(
                egui::DragValue::new(&mut every)
                    .range(1..=10_000)
                    .suffix(" step(s)"),
            );
            ui.label(
                RichText::new(format!("= {} rows", rows.len()))
                    .small()
                    .color(pal.muted),
            );
            if ui
                .button("📋 Copy for Excel")
                .on_hover_text("Copy the table; paste it straight into a spreadsheet")
                .clicked()
            {
                let mut s = format!("Time ({})", run.time_units);
                for c in &cols {
                    s.push('\t');
                    s.push_str(c);
                }
                s.push('\n');
                for &i in &rows {
                    s.push_str(&run.times[i].to_string());
                    for c in &cols {
                        s.push('\t');
                        s.push_str(&run.get(c).map(|v| v[i].to_string()).unwrap_or_default());
                    }
                    s.push('\n');
                }
                ui.ctx().copy_text(s);
                self.status = format!("Copied {} rows — paste into any spreadsheet", rows.len());
            }
            if ui.button("Export CSV…").clicked() {
                self.export_csv();
            }
        });
        ui.data_mut(|d| d.insert_temp(every_id, every));
        ui.add_space(6.0);
        let col_w = 120.0;
        let time_w = 90.0;
        let row_h = 24.0;
        let total_w = time_w + col_w * cols.len() as f32;
        let cur = self.time_index();
        egui::Frame::new()
            .fill(pal.panel)
            .stroke(Stroke::new(1.0, pal.border))
            .corner_radius(CornerRadius::same(8))
            .show(ui, |ui| {
                egui::ScrollArea::horizontal()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let (hr, _) =
                            ui.allocate_exact_size(vec2(total_w, row_h + 14.0), Sense::hover());
                        ui.painter()
                            .rect_filled(hr, CornerRadius::ZERO, pal.surface);
                        ui.painter().text(
                            pos2(hr.left() + 8.0, hr.center().y),
                            Align2::LEFT_CENTER,
                            format!("Time ({})", run.time_units),
                            FontId::proportional(12.0),
                            pal.muted,
                        );
                        for (j, c) in cols.iter().enumerate() {
                            let x = hr.left() + time_w + col_w * j as f32;
                            let kind = self
                                .model
                                .by_name(&expr::normalize(c))
                                .map(|v| (v.kind, v.units.clone()));
                            let colr = kind.as_ref().map(|k| pal.kind(k.0)).unwrap_or(pal.text);
                            let g = ui.painter().layout(
                                c.clone(),
                                FontId::proportional(12.0),
                                pal.text,
                                col_w - 12.0,
                            );
                            ui.painter().galley(
                                pos2(x + col_w - 6.0 - g.size().x, hr.top() + 4.0),
                                g,
                                pal.text,
                            );
                            ui.painter().text(
                                pos2(x + col_w - 6.0, hr.bottom() - 4.0),
                                Align2::RIGHT_BOTTOM,
                                kind.map(|k| k.1).unwrap_or_default(),
                                FontId::proportional(10.0),
                                colr,
                            );
                            ui.painter().line_segment(
                                [pos2(x, hr.top()), pos2(x, hr.bottom())],
                                Stroke::new(1.0, pal.border),
                            );
                        }
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show_rows(ui, row_h, rows.len(), |ui, range| {
                                for ri in range {
                                    let i = rows[ri];
                                    let (rr, resp) = ui
                                        .allocate_exact_size(vec2(total_w, row_h), Sense::click());
                                    if Some(i) == cur {
                                        ui.painter().rect_filled(
                                            rr,
                                            CornerRadius::ZERO,
                                            pal.accent_soft,
                                        );
                                    } else if resp.hovered() {
                                        ui.painter().rect_filled(
                                            rr,
                                            CornerRadius::ZERO,
                                            pal.surface,
                                        );
                                    } else if ri % 2 == 1 {
                                        ui.painter().rect_filled(
                                            rr,
                                            CornerRadius::ZERO,
                                            pal.surface.gamma_multiply(0.5),
                                        );
                                    }
                                    if resp.clicked() {
                                        self.scrub = Some(i);
                                    }
                                    ui.painter().text(
                                        pos2(rr.left() + 8.0, rr.center().y),
                                        Align2::LEFT_CENTER,
                                        theme::fmt_num(run.times[i]),
                                        FontId::monospace(11.5),
                                        pal.muted,
                                    );
                                    for (j, c) in cols.iter().enumerate() {
                                        let x = rr.left() + time_w + col_w * (j + 1) as f32;
                                        let v = run.get(c).and_then(|s| s.get(i)).copied();
                                        ui.painter().text(
                                            pos2(x - 8.0, rr.center().y),
                                            Align2::RIGHT_CENTER,
                                            v.map(theme::fmt_num).unwrap_or_default(),
                                            FontId::monospace(11.5),
                                            pal.text,
                                        );
                                        ui.painter().line_segment(
                                            [
                                                pos2(x - col_w, rr.top()),
                                                pos2(x - col_w, rr.bottom()),
                                            ],
                                            Stroke::new(1.0, pal.border.gamma_multiply(0.6)),
                                        );
                                    }
                                    ui.painter().line_segment(
                                        [rr.left_bottom(), rr.right_bottom()],
                                        Stroke::new(1.0, pal.border.gamma_multiply(0.6)),
                                    );
                                }
                            });
                    });
            });
    }

    // ------------------------------------------------------------------ dashboards

    pub fn dashboard(&mut self, ui: &mut egui::Ui) {
        let Some(sheet) = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .cloned()
        else {
            return;
        };
        let pal = self.pal;
        egui::Frame::new().fill(pal.canvas).inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.horizontal(|ui| {
                ui.label(RichText::new(&sheet.name).size(20.0).strong());
                if let Some(r) = &self.run {
                    let t = self.time_index().map(|i| r.times[i]).unwrap_or(0.0);
                    ui.label(RichText::new(format!("at time {} {}", theme::fmt_num(t), r.time_units)).color(pal.muted));
                }
                if self.exporting {
                    return;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    crate::app::toggle(ui, &mut self.dash_edit, "Edit layout", pal);
                    ui.menu_button(RichText::new("+ Add card").color(pal.accent), |ui| {
                        for k in [WidgetKind::Graph, WidgetKind::Number, WidgetKind::Slider, WidgetKind::Text] {
                            if ui.button(k.label()).clicked() {
                                self.checkpoint();
                                let id = self.model.new_id();
                                let vars = match k {
                                    WidgetKind::Slider => self.model.vars.iter().filter(|v| v.kind == Kind::Constant).map(|v| v.name.clone()).collect(),
                                    WidgetKind::Text => vec![],
                                    _ => self.model.vars.iter().filter(|v| v.kind == Kind::Stock).take(if k == WidgetKind::Number { 1 } else { 3 }).map(|v| v.name.clone()).collect(),
                                };
                                let (span, height) = match k {
                                    WidgetKind::Number => (1, 130.0),
                                    WidgetKind::Graph => (2, 280.0),
                                    _ => (1, 240.0),
                                };
                                let text = if k == WidgetKind::Text { "Write your notes here.".to_string() } else { String::new() };
                                if let Some(s) = self.model.sheet_mut(sheet.id) {
                                    s.widgets.push(Widget { id, kind: k, title: k.label().into(), vars, text, chart: Default::default(), style: Default::default(), span, height });
                                }
                                self.dash_edit = true;
                                ui.close();
                            }
                        }
                    });
                });
            });
            ui.add_space(10.0);
            if sheet.widgets.is_empty() {
                ui.label(RichText::new("This dashboard is empty — use “+ Add card” to place graphs, numbers and sliders.").color(pal.muted));
                return;
            }
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let gap = 14.0;
                let width = ui.available_width();
                let col_w = (width - gap * 3.0) / 4.0;
                let top = ui.cursor().min;
                // flow the cards into rows of four columns
                let mut placed: Vec<(Widget, Rect)> = Vec::new();
                let (mut col, mut y, mut row_h) = (0u8, top.y, 0.0f32);
                for w in &sheet.widgets {
                    let span = w.span.clamp(1, 4);
                    if col + span > 4 {
                        col = 0;
                        y += row_h + gap;
                        row_h = 0.0;
                    }
                    let x = top.x + col as f32 * (col_w + gap);
                    let r = Rect::from_min_size(pos2(x, y), vec2(col_w * span as f32 + gap * (span - 1) as f32, w.height));
                    placed.push((w.clone(), r));
                    col += span;
                    row_h = row_h.max(w.height);
                }
                let bottom = y + row_h;
                ui.allocate_rect(Rect::from_min_max(top, pos2(top.x + width, bottom + 10.0)), Sense::hover());
                let mut edits: Vec<(u64, Option<Widget>, i32)> = Vec::new();
                for (w, r) in placed {
                    ui.painter().rect_filled(r.translate(vec2(0.0, 2.0)), CornerRadius::same(12), Color32::from_black_alpha(if pal.dark { 60 } else { 10 }));
                    ui.painter().rect_filled(r, CornerRadius::same(12), pal.panel);
                    ui.painter().rect_stroke(r, CornerRadius::same(12), Stroke::new(1.0, if self.dash_edit { pal.accent.gamma_multiply(0.4) } else { pal.border }), StrokeKind::Inside);
                    let mut child = ui.new_child(UiBuilder::new().max_rect(r.shrink(14.0)).id_salt(("card", w.id)).layout(Layout::top_down(Align::Min)));
                    if let Some(e) = self.card(&mut child, &w) {
                        edits.push(e);
                    }
                }
                if !edits.is_empty() {
                    self.checkpoint();
                    if let Some(s) = self.model.sheet_mut(sheet.id) {
                        for (id, new, mv) in edits {
                            let Some(i) = s.widgets.iter().position(|w| w.id == id) else { continue };
                            match new {
                                None => {
                                    s.widgets.remove(i);
                                }
                                Some(n) => {
                                    s.widgets[i] = n;
                                    let j = i as i32 + mv;
                                    if mv != 0 && j >= 0 && (j as usize) < s.widgets.len() {
                                        s.widgets.swap(i, j as usize);
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
    }

    /// Draws one dashboard card; returns an edit (widget id, new widget or None to delete, move by).
    fn card(&mut self, ui: &mut egui::Ui, w: &Widget) -> Option<(u64, Option<Widget>, i32)> {
        let pal = self.pal;
        let mut edit = None;
        ui.horizontal(|ui| {
            ui.label(RichText::new(&w.title).strong().color(pal.text));
            if !self.dash_edit && !self.exporting {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if w.kind == WidgetKind::Graph && crate::icons::tool(ui, crate::icons::Ic::Sliders, "Customize this chart: type, titles, axes, legend, colours", pal.muted, pal.accent).clicked() {
                        self.customize_chart(crate::chartstyle::ChartTarget::Card(w.id));
                    }
                    if crate::icons::tool(ui, crate::icons::Ic::Image, "Copy this card as a picture for Word or PowerPoint", pal.muted, pal.accent).clicked() {
                        self.request_export(
                            crate::export::ExportWhat::Card(w.id),
                            crate::export::ExportTo::Clipboard,
                        );
                    }
                });
            }
            if self.dash_edit {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.menu_button("⚙", |ui| {
                        ui.set_min_width(240.0);
                        let mut n = w.clone();
                        ui.label(RichText::new("Title").small().color(pal.muted));
                        ui.text_edit_singleline(&mut n.title);
                        ui.label(RichText::new("Width").small().color(pal.muted));
                        ui.horizontal(|ui| {
                            for s in 1..=4u8 {
                                ui.selectable_value(&mut n.span, s, format!("{s}/4"));
                            }
                        });
                        ui.add(egui::Slider::new(&mut n.height, 100.0..=600.0).text("height"));
                        if n.kind == WidgetKind::Graph {
                            ui.label(RichText::new("Chart").small().color(pal.muted));
                            egui::ComboBox::from_id_salt(("chart", w.id))
                                .selected_text(n.chart.short())
                                .show_ui(ui, |ui| {
                                    for k in crate::model::ChartKind::ALL {
                                        ui.selectable_value(&mut n.chart, k, k.label());
                                    }
                                });
                        }
                        if n.kind != WidgetKind::Text {
                            ui.label(
                                RichText::new(if n.kind == WidgetKind::Number {
                                    "Variable"
                                } else {
                                    "Variables"
                                })
                                .small()
                                .color(pal.muted),
                            );
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for v in self.model.vars.iter().filter(|v| {
                                        !(v.kind == Kind::Lookup && v.equation.trim().is_empty())
                                    }) {
                                        if n.kind == WidgetKind::Slider && v.kind != Kind::Constant
                                        {
                                            continue;
                                        }
                                        let mut on = n.vars.contains(&v.name);
                                        if ui
                                            .checkbox(
                                                &mut on,
                                                RichText::new(&v.name).color(pal.kind(v.kind)),
                                            )
                                            .changed()
                                        {
                                            if n.kind == WidgetKind::Number {
                                                n.vars =
                                                    if on { vec![v.name.clone()] } else { vec![] };
                                            } else if on {
                                                n.vars.push(v.name.clone());
                                            } else {
                                                n.vars.retain(|x| *x != v.name);
                                            }
                                        }
                                    }
                                });
                        }
                        let mut mv = 0;
                        ui.horizontal(|ui| {
                            if ui.button("← Earlier").clicked() {
                                mv = -1;
                            }
                            if ui.button("Later →").clicked() {
                                mv = 1;
                            }
                        });
                        let changed =
                            serde_json::to_string(&n).ok() != serde_json::to_string(w).ok();
                        if changed || mv != 0 {
                            edit = Some((w.id, Some(n), mv));
                        }
                        ui.separator();
                        if ui
                            .button(RichText::new("Remove card").color(pal.error))
                            .clicked()
                        {
                            edit = Some((w.id, None, 0));
                            ui.close();
                        }
                    });
                });
            }
        });
        ui.add_space(4.0);
        let run = self.run.clone();
        let ti = self.time_index();
        match w.kind {
            WidgetKind::Number => {
                let name = w.vars.first().cloned().unwrap_or_default();
                let unit = self
                    .model
                    .by_name(&expr::normalize(&name))
                    .map(|v| v.units.clone())
                    .unwrap_or_default();
                let series = run.as_ref().and_then(|r| r.get(&name).cloned());
                match (series, ti) {
                    (Some(s), Some(i)) => {
                        let now = s[i];
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(theme::fmt_num(now))
                                    .size(30.0)
                                    .strong()
                                    .color(pal.text),
                            );
                            ui.label(RichText::new(unit).color(pal.muted));
                        });
                        let start = s[0];
                        let delta = now - start;
                        let pct = if start.abs() > 1e-12 {
                            format!(" ({:+.0}%)", delta / start.abs() * 100.0)
                        } else {
                            String::new()
                        };
                        let col = if delta >= 0.0 { pal.ok } else { pal.error };
                        ui.label(
                            RichText::new(format!(
                                "{} {} since start{pct}",
                                if delta >= 0.0 { "▲" } else { "▼" },
                                theme::fmt_num(delta.abs())
                            ))
                            .small()
                            .color(col),
                        );
                        let (r, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), ui.available_height().max(10.0)),
                            Sense::hover(),
                        );
                        let upto = self.reveal_len(s.len());
                        theme::sparkline_upto(
                            ui.painter(),
                            r,
                            &s,
                            pal.series(0),
                            Some(i),
                            true,
                            upto,
                        );
                    }
                    _ => {
                        ui.label(
                            RichText::new(if name.is_empty() {
                                "Pick a variable with ⚙ (Edit layout)"
                            } else {
                                "No value yet"
                            })
                            .color(pal.muted),
                        );
                    }
                }
            }
            WidgetKind::Graph => {
                let h = ui.available_height();
                if let Some(t) = self.chart_styled(ui, ("dash", w.id), w.chart, &w.vars, h, &w.style.clone(), Some(crate::chartstyle::ChartTarget::Card(w.id))) {
                    if let Some(run) = &run {
                        self.scrub = Some(
                            run.times
                                .partition_point(|x| *x < t)
                                .min(run.times.len() - 1),
                        );
                    }
                }
            }
            WidgetKind::Slider => {
                egui::ScrollArea::vertical()
                    .id_salt(("sl", w.id))
                    .show(ui, |ui| {
                        for n in &w.vars {
                            let Some(v) = self.model.by_name(&expr::normalize(n)).cloned() else {
                                continue;
                            };
                            let mut nv = v.clone();
                            if self.slider_row_pub(ui, &mut nv) {
                                *self.model.var_mut(v.id).unwrap() = nv;
                                self.changed();
                            }
                            ui.add_space(4.0);
                        }
                        if w.vars.is_empty() {
                            ui.label(
                                RichText::new("Pick constants with ⚙ (Edit layout)")
                                    .color(pal.muted),
                            );
                        }
                    });
            }
            WidgetKind::Text => {
                if self.dash_edit {
                    let mut t = w.text.clone();
                    let h = ui.available_height();
                    ui.add_sized([ui.available_width(), h], egui::TextEdit::multiline(&mut t));
                    if t != w.text && edit.is_none() {
                        let mut n = w.clone();
                        n.text = t;
                        edit = Some((w.id, Some(n), 0));
                    }
                } else {
                    ui.label(RichText::new(&w.text).color(pal.muted));
                }
            }
        }
        edit
    }
}

impl App {
    /// A clean table picture for documents: the equations or the results of this sheet.
    pub fn table_export(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        origin: egui::Pos2,
    ) -> Option<Rect> {
        let sheet = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .cloned()?;
        let pal = self.pal;
        let p = egui::Painter::new(ctx.clone(), layer, Rect::EVERYTHING);
        // (text, colour, right-aligned)
        let mut headers: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<(String, egui::Color32, bool)>> = Vec::new();
        let ti = self.time_index();
        match sheet.table_mode {
            TableMode::Variables => {
                headers = vec![
                    "Name".into(),
                    "Type".into(),
                    "Equation / initial value".into(),
                    "Units".into(),
                    "Value".into(),
                ];
                for v in &self.model.vars {
                    let eq = if v.kind == Kind::Lookup && v.equation.trim().is_empty() {
                        format!("lookup table, {} points", v.table.len())
                    } else {
                        v.equation.clone()
                    };
                    let val = self
                        .value_now(&v.name)
                        .map(theme::fmt_num)
                        .unwrap_or_default();
                    rows.push(vec![
                        (v.name.clone(), pal.text, false),
                        (v.kind.label().into(), pal.kind(v.kind), false),
                        (eq, pal.text, false),
                        (v.units.clone(), pal.muted, false),
                        (val, pal.text, true),
                    ]);
                }
            }
            TableMode::Results => {
                let run = self.run.clone()?;
                let mut cols: Vec<String> = sheet
                    .columns
                    .iter()
                    .filter(|c| run.get(c).is_some())
                    .cloned()
                    .collect();
                if cols.is_empty() {
                    cols = self
                        .model
                        .vars
                        .iter()
                        .filter(|v| run.get(&v.name).is_some())
                        .map(|v| v.name.clone())
                        .collect();
                }
                headers.push(format!("Time ({})", run.time_units));
                headers.extend(cols.iter().cloned());
                let every = ((1.0 / self.model.settings.time_step).round() as usize).max(1);
                let step = every.max(run.times.len().div_ceil(400));
                for i in (0..run.times.len()).step_by(step) {
                    let mut r = vec![(theme::fmt_num(run.times[i]), pal.muted, true)];
                    for c in &cols {
                        r.push((
                            run.get(c).map(|v| theme::fmt_num(v[i])).unwrap_or_default(),
                            if Some(i) == ti { pal.accent } else { pal.text },
                            true,
                        ));
                    }
                    rows.push(r);
                }
            }
        }
        if rows.is_empty() {
            return None;
        }
        let font = egui::FontId::proportional(13.0);
        let bold = egui::FontId::proportional(13.5);
        let width = |t: &str, f: &egui::FontId| {
            p.layout_no_wrap(t.to_string(), f.clone(), pal.text)
                .size()
                .x
        };
        let n = headers.len();
        let mut widths: Vec<f32> = headers.iter().map(|h| width(h, &bold) + 24.0).collect();
        for r in &rows {
            for (i, c) in r.iter().enumerate() {
                widths[i] = widths[i].max(width(&c.0, &font) + 24.0);
            }
        }
        for w in widths.iter_mut() {
            *w = w.min(460.0);
        }
        let row_h = 26.0;
        let title_h = 34.0;
        let total_w: f32 = widths.iter().sum();
        let top = origin.y + title_h;
        p.text(
            origin + vec2(0.0, 4.0),
            Align2::LEFT_TOP,
            &sheet.name,
            egui::FontId::proportional(17.0),
            pal.text,
        );
        let table = Rect::from_min_size(
            egui::pos2(origin.x, top),
            vec2(total_w, row_h * (rows.len() + 1) as f32),
        );
        p.rect_filled(
            Rect::from_min_size(table.min, vec2(total_w, row_h)),
            CornerRadius {
                nw: 6,
                ne: 6,
                sw: 0,
                se: 0,
            },
            egui::Color32::from_rgb(238, 240, 247),
        );
        // cut long text to fit its column
        let fit = |t: &str, w: f32, f: &egui::FontId| -> String {
            if width(t, f) <= w - 16.0 {
                return t.to_string();
            }
            let mut s: String = t.to_string();
            while !s.is_empty() && width(&format!("{s}…"), f) > w - 16.0 {
                s.pop();
            }
            format!("{s}…")
        };
        let mut x = table.left();
        for (i, h) in headers.iter().enumerate() {
            let right = rows.first().is_some_and(|r| r[i].2);
            let pos = if right {
                egui::pos2(x + widths[i] - 10.0, top + row_h / 2.0)
            } else {
                egui::pos2(x + 10.0, top + row_h / 2.0)
            };
            p.text(
                pos,
                if right {
                    Align2::RIGHT_CENTER
                } else {
                    Align2::LEFT_CENTER
                },
                fit(h, widths[i], &bold),
                bold.clone(),
                pal.text,
            );
            x += widths[i];
        }
        for (ri, r) in rows.iter().enumerate() {
            let y = top + row_h * (ri + 1) as f32;
            if ri % 2 == 1 {
                p.rect_filled(
                    Rect::from_min_size(egui::pos2(table.left(), y), vec2(total_w, row_h)),
                    CornerRadius::ZERO,
                    egui::Color32::from_rgb(248, 249, 252),
                );
            }
            let mut x = table.left();
            for (i, c) in r.iter().enumerate() {
                let pos = if c.2 {
                    egui::pos2(x + widths[i] - 10.0, y + row_h / 2.0)
                } else {
                    egui::pos2(x + 10.0, y + row_h / 2.0)
                };
                p.text(
                    pos,
                    if c.2 {
                        Align2::RIGHT_CENTER
                    } else {
                        Align2::LEFT_CENTER
                    },
                    fit(&c.0, widths[i], &font),
                    font.clone(),
                    c.1,
                );
                x += widths[i];
            }
            p.line_segment(
                [egui::pos2(table.left(), y), egui::pos2(table.right(), y)],
                Stroke::new(1.0, egui::Color32::from_rgb(222, 226, 236)),
            );
        }
        let mut x = table.left();
        for w in widths.iter().take(n - 1) {
            x += w;
            p.line_segment(
                [egui::pos2(x, table.top()), egui::pos2(x, table.bottom())],
                Stroke::new(1.0, egui::Color32::from_rgb(232, 235, 242)),
            );
        }
        p.rect_stroke(
            table,
            CornerRadius::same(6),
            Stroke::new(1.0, egui::Color32::from_rgb(210, 214, 226)),
            StrokeKind::Inside,
        );
        Some(Rect::from_min_max(origin, table.max + vec2(2.0, 2.0)))
    }

    /// The whole dashboard, laid out at a document-friendly width.
    pub fn dashboard_export(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        origin: egui::Pos2,
    ) -> Option<Rect> {
        let sheet = self
            .model
            .sheets
            .iter()
            .find(|s| s.id == self.sheet)
            .cloned()?;
        if sheet.widgets.is_empty() {
            return None;
        }
        let width = 1400.0;
        // the same packing the dashboard uses: rows of four columns
        let (mut col, mut height, mut row_h) = (0u8, 0.0f32, 0.0f32);
        for w in &sheet.widgets {
            let span = w.span.clamp(1, 4);
            if col + span > 4 {
                col = 0;
                height += row_h + 14.0;
                row_h = 0.0;
            }
            col += span;
            row_h = row_h.max(w.height);
        }
        height += row_h;
        let area = Rect::from_min_size(origin, vec2(width, height + 100.0));
        let mut ui = self.export_ui(ctx, layer, area);
        self.dashboard(&mut ui);
        Some(area)
    }

    /// One dashboard card on its own.
    pub fn card_export(
        &mut self,
        ctx: &egui::Context,
        layer: egui::LayerId,
        origin: egui::Pos2,
        wid: u64,
    ) -> Option<Rect> {
        let w = self
            .model
            .sheets
            .iter()
            .flat_map(|s| s.widgets.iter())
            .find(|w| w.id == wid)
            .cloned()?;
        let pal = self.pal;
        let area = Rect::from_min_size(origin, vec2(340.0 * w.span.clamp(1, 4) as f32, w.height));
        let p = egui::Painter::new(ctx.clone(), layer, Rect::EVERYTHING);
        p.rect_filled(area, CornerRadius::same(12), pal.panel);
        p.rect_stroke(
            area,
            CornerRadius::same(12),
            Stroke::new(1.0, pal.border),
            StrokeKind::Inside,
        );
        let mut ui = self.export_ui(ctx, layer, area.shrink(14.0));
        self.card(&mut ui, &w);
        Some(area)
    }
}
