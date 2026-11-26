//! The right-hand panel: edit whatever is selected, or tune the model with sliders.

use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, pos2,
    vec2,
};
use egui_plot::{Line, LineStyle, Plot, PlotPoints, VLine};

use crate::app::{App, BottomTab, Sel};
use crate::expr::{self, FUNCTIONS};
use crate::model::{ItemValue, Kind, Rollup, Var};
use crate::theme::{self, Palette};

pub fn section(ui: &mut egui::Ui, pal: Palette, title: &str) {
    ui.add_space(10.0);
    ui.label(
        RichText::new(title.to_uppercase())
            .small()
            .strong()
            .color(pal.muted),
    );
    ui.add_space(2.0);
}

fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.gamma_multiply(0.14))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).small().strong().color(color));
        });
}

/// Insert `text` into `s` at char index `at` (or the end); returns the new cursor char index.
fn insert_at(s: &mut String, at: Option<usize>, text: &str) -> usize {
    let n = s.chars().count();
    let at = at.unwrap_or(n).min(n);
    let byte = s.char_indices().nth(at).map(|(b, _)| b).unwrap_or(s.len());
    let before = s[..byte].chars().last();
    let pad_l = if before.is_some_and(|c| !c.is_whitespace() && c != '(') {
        " "
    } else {
        ""
    };
    let ins = format!("{pad_l}{text}");
    s.insert_str(byte, &ins);
    at + ins.chars().count()
}

impl App {
    pub fn inspector(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let many = self.selected_items().len() > 1;
                match self.sel {
                    Some(Sel::Group(g)) if self.model.groups.iter().any(|x| x.id == g) => {
                        self.group_editor(ui, g)
                    }
                    Some(Sel::Module(m)) if !many && self.model.module(m).is_some() => {
                        self.module_editor(ui, m)
                    }
                    _ if many && !matches!(self.sel, Some(Sel::Link(_))) => self.multi_editor(ui),
                    Some(Sel::Var(id)) if self.model.var(id).is_some() => self.var_editor(ui, id),
                    Some(Sel::Link(id)) if self.model.links.iter().any(|l| l.id == id) => {
                        self.link_editor(ui, id)
                    }
                    Some(Sel::Note(id)) if self.model.notes.iter().any(|n| n.id == id) => {
                        self.note_editor(ui, id)
                    }
                    Some(Sel::Shape(id)) if self.model.shapes.iter().any(|n| n.id == id) => {
                        self.shape_editor(ui, id)
                    }
                    Some(Sel::Control(id)) if self.model.controls.iter().any(|n| n.id == id) => {
                        self.control_editor(ui, id)
                    }
                    _ => self.overview(ui),
                }
            });
    }

    /// Arrange buttons shared by the multi-selection and group panels.
    fn arrange_buttons(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let n = self.selected_items().len();
        section(ui, pal, "Align & distribute");
        ui.horizontal_wrapped(|ui| {
            for (i, op) in crate::arrange::Align::ALL.into_iter().enumerate() {
                let enabled = if i >= 6 { n >= 3 } else { n >= 2 };
                let (r, resp) = ui.allocate_exact_size(vec2(34.0, 30.0), Sense::click());
                ui.painter().rect_filled(
                    r,
                    CornerRadius::same(6),
                    if resp.hovered() && enabled {
                        pal.accent_soft
                    } else {
                        pal.surface
                    },
                );
                theme::draw_align(
                    ui.painter(),
                    r.shrink2(vec2(9.0, 7.0)),
                    op,
                    if enabled {
                        pal.text
                    } else {
                        pal.muted.gamma_multiply(0.4)
                    },
                    pal.accent,
                );
                if resp.on_hover_text(op.label()).clicked() && enabled {
                    self.align(op);
                }
            }
        });
    }

    fn multi_editor(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        let items = self.selected_items();
        badge(ui, "Selection", pal.accent);
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("{} items selected", items.len()))
                .size(18.0)
                .strong(),
        );
        ui.label(
            RichText::new("Shift-click to add or remove items, or drag a box on empty space.")
                .small()
                .color(pal.muted),
        );
        self.arrange_buttons(ui);
        section(ui, pal, "Organize");
        ui.horizontal_wrapped(|ui| {
            use crate::icons::{Ic, button};
            if button(ui, Ic::Group, Some("Group"), false, pal.text, pal.accent).on_hover_text("Select and move them together (⌘G)").clicked() {
                self.group_selection();
            }
            if button(ui, Ic::Module, Some("Make module"), false, pal.text, pal.accent).on_hover_text("Wrap them into a submodel; what crosses the edge becomes inputs and outputs (⇧⌘M)").clicked() {
                self.make_module();
            }
            if button(ui, Ic::Export, Some("Export as module…"), false, pal.text, pal.accent).on_hover_text("Save them as a module file to reuse in other models").clicked() {
                self.export_module();
            }
        });
        ui.horizontal_wrapped(|ui| {
            use crate::icons::{Ic, button};
            if button(ui, Ic::Copy, Some("Copy"), false, pal.text, pal.accent).on_hover_text("⌘C — also copies a picture for Office").clicked() {
                if self.copy(false) {
                    self.request_export(crate::export::ExportWhat::Selection, crate::export::ExportTo::Clipboard);
                }
            }
            if button(ui, Ic::Duplicate, Some("Duplicate"), false, pal.text, pal.accent).on_hover_text("⌘D").clicked() {
                self.duplicate();
            }
            if button(ui, Ic::Trash, Some("Delete"), false, pal.error, pal.error).on_hover_text("⌫").clicked() {
                self.delete_selection();
            }
        });
        let diagrams: Vec<(u64, String)> = self
            .model
            .sheets
            .iter()
            .filter(|s| s.kind == crate::model::SheetKind::Diagram && s.id != self.diagram_view())
            .map(|s| (s.id, s.name.clone()))
            .collect();
        if !diagrams.is_empty() {
            ui.menu_button("Move to sheet ▸", |ui| {
                for (sid, name) in diagrams {
                    if ui.button(name).clicked() {
                        self.checkpoint();
                        for s in self.selected_items() {
                            match s {
                                Sel::Var(id) => {
                                    if let Some(v) = self.model.var_mut(id) {
                                        v.view = sid;
                                    }
                                }
                                Sel::Note(id) => {
                                    if let Some(n) =
                                        self.model.notes.iter_mut().find(|n| n.id == id)
                                    {
                                        n.view = sid;
                                    }
                                }
                                Sel::Module(id) => {
                                    if let Some(m) =
                                        self.model.modules.iter_mut().find(|m| m.id == id)
                                    {
                                        m.view = sid;
                                    }
                                }
                                _ => {}
                            }
                        }
                        self.select_one(None);
                        self.status = "Moved to another sheet".into();
                        ui.close();
                    }
                }
            });
        }
        section(ui, pal, "In the selection");
        for s in items.iter().take(30) {
            let (name, col) = match s {
                Sel::Var(id) => self
                    .model
                    .var(*id)
                    .map(|v| (v.name.clone(), pal.kind(v.kind)))
                    .unwrap_or_default(),
                Sel::Note(_) => ("Note".into(), pal.muted),
                Sel::Module(id) => self
                    .model
                    .module(*id)
                    .map(|m| (format!("Module · {}", m.name), pal.accent))
                    .unwrap_or_default(),
                _ => continue,
            };
            ui.label(RichText::new(format!("•  {name}")).color(col));
        }
        if items.len() > 30 {
            ui.label(RichText::new(format!("…and {} more", items.len() - 30)).color(pal.muted));
        }
    }

    fn group_editor(&mut self, ui: &mut egui::Ui, gid: u64) {
        let pal = self.pal;
        let g = self
            .model
            .groups
            .iter()
            .find(|x| x.id == gid)
            .cloned()
            .unwrap();
        badge(ui, "Group", pal.muted);
        ui.add_space(6.0);
        let mut name = g.name.clone();
        let r = ui.add(
            egui::TextEdit::singleline(&mut name)
                .font(FontId::proportional(18.0))
                .desired_width(f32::INFINITY),
        );
        if r.gained_focus() {
            self.checkpoint();
        }
        ui.label(
            RichText::new(format!(
                "{} items · click a member again to pick it alone",
                g.members.len()
            ))
            .small()
            .color(pal.muted),
        );
        section(ui, pal, "Colour");
        let mut color = g.color;
        ui.horizontal(|ui| {
            if ui.selectable_label(color.is_none(), "Neutral").clicked() {
                color = None;
            }
            for i in 0..6 {
                let c = pal.series(i);
                let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                let on = color == Some([c.r(), c.g(), c.b()]);
                ui.painter().circle(
                    r.center(),
                    8.0,
                    c,
                    Stroke::new(if on { 2.5 } else { 0.0 }, pal.text),
                );
                if resp.clicked() {
                    color = Some([c.r(), c.g(), c.b()]);
                }
            }
        });
        if name != g.name || color != g.color {
            if let Some(x) = self.model.groups.iter_mut().find(|x| x.id == gid) {
                x.name = name;
                x.color = color;
            }
        }
        self.arrange_buttons(ui);
        section(ui, pal, "Organize");
        ui.horizontal_wrapped(|ui| {
            use crate::icons::{Ic, button};
            if button(ui, Ic::Ungroup, Some("Ungroup"), false, pal.text, pal.accent).on_hover_text("⇧⌘G").clicked() {
                self.ungroup();
            }
            if button(ui, Ic::Module, Some("Turn into a module"), false, pal.text, pal.accent).on_hover_text("Its parts move to their own sheet, with inputs and outputs").clicked() {
                self.make_module();
            }
            if button(ui, Ic::Export, Some("Export as module…"), false, pal.text, pal.accent).clicked() {
                self.export_module();
            }
            if button(ui, Ic::Copy, Some("Copy"), false, pal.text, pal.accent).clicked() {
                self.copy(false);
            }
        });
    }

    fn module_editor(&mut self, ui: &mut egui::Ui, mid: u64) {
        let pal = self.pal;
        let m = self.model.module(mid).cloned().unwrap();
        badge(ui, "Module", pal.accent);
        ui.add_space(6.0);
        let mut name = m.name.clone();
        let r = ui.add(
            egui::TextEdit::singleline(&mut name)
                .font(FontId::proportional(18.0))
                .desired_width(f32::INFINITY),
        );
        if r.gained_focus() {
            self.checkpoint();
        }
        let mut desc = m.description.clone();
        ui.add(
            egui::TextEdit::multiline(&mut desc)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text("What this part of the model does…"),
        );
        if name != m.name || desc != m.description {
            if let Some(x) = self.model.modules.iter_mut().find(|x| x.id == mid) {
                x.name = name.clone();
                x.description = desc;
            }
            if let Some(s) = self.model.sheet_mut(m.inner) {
                s.name = name;
            }
        }
        ui.horizontal_wrapped(|ui| {
            use crate::icons::{Ic, button};
            if button(ui, Ic::Back, Some("Open"), true, pal.text, pal.accent).on_hover_text("Open the module's own sheet (or double-click the box)").clicked() {
                self.open_module(mid);
            }
            if button(ui, Ic::Export, Some("Export…"), false, pal.text, pal.accent).on_hover_text("Save this module as a file to reuse in other models").clicked() {
                self.export_module();
            }
            if button(ui, Ic::Ungroup, Some("Dissolve"), false, pal.text, pal.accent).on_hover_text("Put its parts back on this sheet").clicked() {
                self.dissolve_module(mid);
            }
        });
        let (ins, outs) = self.module_ports(mid);
        for (title, list, col, hint) in [
            ("Inputs", &ins, pal.ok, "Used inside, defined outside"),
            ("Outputs", &outs, pal.aux, "Defined inside, used outside"),
        ] {
            section(ui, pal, &format!("{title} ({})", list.len()));
            ui.label(RichText::new(hint).small().color(pal.muted));
            for id in list {
                let Some(v) = self.model.var(*id).cloned() else {
                    continue;
                };
                ui.horizontal(|ui| {
                    ui.label(RichText::new("●").color(col));
                    if ui
                        .link(RichText::new(&v.name).color(pal.kind(v.kind)))
                        .on_hover_text("Go to it")
                        .clicked()
                    {
                        self.reveal_var(v.id);
                    }
                    if let Some(x) = self.value_now(&v.name) {
                        ui.label(
                            RichText::new(theme::fmt_num(x))
                                .monospace()
                                .small()
                                .color(pal.muted),
                        );
                    }
                });
            }
            if list.is_empty() {
                ui.label(RichText::new("none").color(pal.muted));
            }
        }
        section(ui, pal, "Inside");
        let parts: Vec<(String, Kind)> = self
            .model
            .vars
            .iter()
            .filter(|v| v.view == m.inner)
            .map(|v| (v.name.clone(), v.kind))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (n, k) in parts {
                ui.label(RichText::new(n).small().color(pal.kind(k)));
            }
        });
    }

    fn overview(&mut self, ui: &mut egui::Ui) {
        let pal = self.pal;
        ui.label(
            RichText::new(if self.model.title.is_empty() {
                "Untitled model"
            } else {
                &self.model.title
            })
            .size(18.0)
            .strong(),
        );
        ui.label(
            RichText::new("Nothing selected — click a part of the diagram to edit it.")
                .color(pal.muted),
        );

        // Key levers: when some variables are flagged, only they get controls here; every other
        // constant waits in a folded list
        let any_lever = self.has_levers();
        let scalar: Vec<(u64, bool)> = self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Constant && v.set.is_none())
            .map(|v| (v.id, v.lever))
            .collect();
        if any_lever {
            section(ui, pal, "Key levers");
            ui.label(RichText::new("The few controls that drive the model; the run updates instantly. They are also on the diagram.").small().color(pal.muted));
            self.lever_rows(ui);
            let others: Vec<u64> = scalar.iter().filter(|(_, l)| !*l).map(|(i, _)| *i).collect();
            if !others.is_empty() {
                egui::CollapsingHeader::new(RichText::new(format!("All other constants ({})", others.len())).color(pal.muted)).id_salt("other-consts").show(ui, |ui| {
                    for id in others {
                        let mut v = self.model.var(id).unwrap().clone();
                        if self.slider_row(ui, &mut v, true) {
                            *self.model.var_mut(id).unwrap() = v;
                            self.changed();
                        }
                    }
                });
            }
        } else {
            let consts: Vec<u64> = scalar.iter().map(|(i, _)| *i).collect();
            if !consts.is_empty() {
                section(ui, pal, "Sliders");
                ui.label(RichText::new("Drag to change a constant; the run updates instantly.").small().color(pal.muted));
                for id in consts {
                    let mut v = self.model.var(id).unwrap().clone();
                    if self.slider_row(ui, &mut v, true) {
                        *self.model.var_mut(id).unwrap() = v;
                        self.changed();
                    }
                }
            }
            // constants that are per a set: sliders for the item picked in the Showing bar
            let item_consts: Vec<(u64, String)> = self
                .model
                .vars
                .iter()
                .filter(|v| v.kind == Kind::Constant && v.set.is_some())
                .filter_map(|v| {
                    let sn = v.set.as_ref()?;
                    let item = self.focus.get(sn).filter(|i| self.model.sets.iter().any(|s| &s.name == sn && s.items.contains(i)))?;
                    Some((v.id, item.clone()))
                })
                .collect();
            if !item_consts.is_empty() {
                section(ui, pal, "Sliders for the selection");
                ui.label(RichText::new("Changes only the item chosen in the Showing bar.").small().color(pal.muted));
                for (id, item) in item_consts {
                    let base = self.model.var(id).unwrap().clone();
                    let mut tmp = base.clone();
                    tmp.set = None;
                    tmp.by_item.clear();
                    if let Some(o) = base.by_item.iter().find(|o| o.item == item).filter(|o| !o.equation.trim().is_empty()) {
                        tmp.equation = o.equation.clone();
                    }
                    tmp.name = format!("{} · {}", base.name, item);
                    if self.slider_row(ui, &mut tmp, true) {
                        let v = self.model.var_mut(id).unwrap();
                        match v.by_item.iter_mut().find(|o| o.item == item) {
                            Some(o) => o.equation = tmp.equation.clone(),
                            None => v.by_item.push(ItemValue { item: item.clone(), equation: tmp.equation.clone(), table: Vec::new() }),
                        }
                        if tmp.slider != base.slider {
                            v.slider = tmp.slider;
                        }
                        self.changed();
                    }
                }
            }
        }

        let stocks: Vec<Var> = self
            .model
            .vars
            .iter()
            .filter(|v| v.kind == Kind::Stock)
            .cloned()
            .collect();
        if !stocks.is_empty() {
            section(ui, pal, "Stocks over the run");
            for v in stocks {
                let resp = egui::Frame::new()
                    .fill(pal.surface)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.set_width(120.0);
                                ui.label(RichText::new(&v.name).strong());
                                let val = self
                                    .value_now(&v.name)
                                    .map(theme::fmt_num)
                                    .unwrap_or("–".into());
                                ui.label(
                                    RichText::new(format!("{val} {}", v.units))
                                        .monospace()
                                        .small()
                                        .color(pal.stock),
                                );
                            });
                            let (r, _) = ui.allocate_exact_size(
                                vec2(ui.available_width(), 34.0),
                                Sense::hover(),
                            );
                            if let Some(vals) = self.run.as_ref().and_then(|r| r.get(&v.name)) {
                                theme::sparkline(
                                    ui.painter(),
                                    r,
                                    vals,
                                    pal.stock,
                                    self.time_index(),
                                    true,
                                );
                            }
                        });
                    });
                if resp.response.interact(Sense::click()).clicked() {
                    self.sel = Some(Sel::Var(v.id));
                }
                ui.add_space(4.0);
            }
        }

        section(ui, pal, "About this model");
        ui.label(
            RichText::new(format!(
                "Runs from {} to {} {} in steps of {}.",
                theme::fmt_num(self.model.settings.initial_time),
                theme::fmt_num(self.model.settings.final_time),
                self.model.settings.time_units,
                self.model.settings.time_step
            ))
            .color(pal.muted),
        );
        if !self.problems.is_empty() {
            ui.add_space(4.0);
            if ui
                .link(
                    RichText::new(format!("⚠ {} problem(s) to fix", self.problems.len()))
                        .color(pal.error),
                )
                .clicked()
            {
                self.bottom_tab = BottomTab::Problems;
            }
        }
    }

    pub fn slider_row_pub(&mut self, ui: &mut egui::Ui, v: &mut Var) -> bool {
        self.slider_row(ui, v, true)
    }

    /// A labelled slider for a constant. Returns true when the value changed.
    fn slider_row(&mut self, ui: &mut egui::Ui, v: &mut Var, show_name: bool) -> bool {
        let pal = self.pal;
        let Ok(mut val) = v.equation.trim().parse::<f64>() else {
            if show_name {
                ui.label(RichText::new(format!("{} = {}", v.name, v.equation)).color(pal.muted));
            }
            return false;
        };
        let [lo, hi] = v.slider.unwrap_or_else(|| {
            let m = if val == 0.0 { 1.0 } else { val.abs() * 2.0 };
            [if val < 0.0 { -m } else { 0.0 }, m]
        });
        let mut changed = false;
        if show_name {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&v.name).color(pal.text));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(&v.units).small().color(pal.muted));
                });
            });
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().slider_width = (ui.available_width() - 70.0).max(80.0);
            let r = ui.add(
                egui::Slider::new(&mut val, lo.min(hi)..=hi.max(lo))
                    .clamping(egui::SliderClamping::Never)
                    .smart_aim(true),
            );
            if r.drag_started() || (r.changed() && !r.dragged()) {
                self.checkpoint();
            }
            if r.changed() {
                v.equation = trim_num(val);
                changed = true;
            }
        });
        changed
    }

    fn var_editor(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let orig = self.model.var(id).unwrap().clone();
        let mut v = orig.clone();
        let col = pal.kind(v.kind);

        // header
        ui.horizontal(|ui| {
            badge(ui, v.kind.label(), col);
            if matches!(v.kind, Kind::Aux | Kind::Constant | Kind::Lookup) {
                egui::ComboBox::from_id_salt(("kind", id))
                    .width(110.0)
                    .selected_text("Change type")
                    .show_ui(ui, |ui| {
                        for k in [Kind::Aux, Kind::Constant, Kind::Lookup] {
                            if ui.selectable_label(v.kind == k, k.label()).clicked() && v.kind != k
                            {
                                v.kind = k;
                                if k == Kind::Lookup && v.table.is_empty() {
                                    v.table = vec![[0.0, 0.0], [0.5, 0.7], [1.0, 1.0]];
                                }
                            }
                        }
                    });
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let lookup_fn = v.kind == Kind::Lookup && v.equation.trim().is_empty();
                if !lookup_fn {
                    let plotted = self.plotted.contains(&v.name);
                    if crate::icons::button(ui, crate::icons::Ic::Graph, Some("Graph"), plotted, pal.text, pal.accent)
                        .on_hover_text("Show in the trends graph")
                        .clicked()
                    {
                        if plotted {
                            self.plotted.retain(|n| n != &v.name)
                        } else {
                            self.plotted.push(v.name.clone())
                        }
                        self.bottom_tab = BottomTab::Trends;
                    }
                }
            });
        });
        ui.add_space(6.0);

        // name: committed when the field loses focus so references are renamed once
        let name_id = ui.scope_id().with(("name", id));
        let mut buf: String = ui
            .data_mut(|d| d.get_temp::<String>(name_id))
            .unwrap_or_else(|| v.name.clone());
        let r = ui.add(
            egui::TextEdit::singleline(&mut buf)
                .font(FontId::proportional(18.0))
                .desired_width(f32::INFINITY)
                .hint_text("Name"),
        );
        if self.focus_name {
            r.request_focus();
            ui.ctx().memory_mut(|m| m.request_focus(r.id));
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), r.id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(buf.chars().count()),
                )));
                st.store(ui.ctx(), r.id);
            }
            self.focus_name = false;
        }
        if r.has_focus() || r.lost_focus() {
            ui.data_mut(|d| d.insert_temp(name_id, buf.clone()));
        }
        if r.lost_focus() {
            ui.data_mut(|d| d.remove::<String>(name_id));
            let new = buf.trim().to_string();
            let nn = expr::normalize(&new);
            let old = expr::normalize(&v.name);
            if new.is_empty() {
                self.status = "A name can't be empty".into();
            } else if nn != old
                && (self.model.by_name(&nn).is_some()
                    || expr::TIME_VARS.iter().any(|(t, _)| *t == nn))
            {
                self.status = format!("'{new}' is already used");
            } else if new != v.name {
                self.checkpoint();
                for o in self.model.vars.iter_mut() {
                    o.equation = expr::rename_in(&o.equation, &old, &new);
                }
                for p in self.plotted.iter_mut() {
                    if expr::normalize(p) == old {
                        *p = new.clone();
                    }
                }
                v.equation = expr::rename_in(&v.equation, &old, &new);
                v.name = new;
            }
        }

        // ---- equation
        let label = match v.kind {
            Kind::Stock => "Initial value",
            Kind::Lookup => "Input",
            _ => "Equation",
        };
        section(ui, pal, label);
        if v.kind == Kind::Stock {
            let ins: Vec<String> = self
                .model
                .vars
                .iter()
                .filter(|f| f.kind == Kind::Flow && f.to == Some(id))
                .map(|f| f.name.clone())
                .collect();
            let outs: Vec<String> = self
                .model
                .vars
                .iter()
                .filter(|f| f.kind == Kind::Flow && f.from == Some(id))
                .map(|f| f.name.clone())
                .collect();
            let mut rate = String::new();
            for i in &ins {
                rate.push_str(&format!(" + {i}"));
            }
            for o in &outs {
                rate.push_str(&format!(" − {o}"));
            }
            egui::Frame::new()
                .fill(pal.stock.gamma_multiply(0.08))
                .corner_radius(CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(8, 5))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new("Changes each step by")
                            .small()
                            .color(pal.muted),
                    );
                    ui.label(
                        RichText::new(if rate.is_empty() {
                            "nothing yet — draw a flow into or out of it".to_string()
                        } else {
                            rate.trim_start_matches(" + ").trim().to_string()
                        })
                        .monospace()
                        .color(pal.stock),
                    );
                });
            ui.add_space(4.0);
        }
        if v.kind == Kind::Lookup {
            ui.label(
                RichText::new("Leave empty to use it as a function: name(x)")
                    .small()
                    .color(pal.muted),
            );
        }
        if v.kind == Kind::Constant {
            self.slider_row(ui, &mut v, false);
        }

        let eq_id = ui.scope_id().with(("eq", id));
        let mut insert: Option<String> = None;
        if let Some((vid, text)) = self.pending_insert.clone() {
            if vid == id {
                self.pending_insert = None;
                insert = Some(text);
            }
        }
        // the suggestion list takes the arrow keys, Tab, Enter and Esc while it's open
        let focused = ui.memory(|m| m.has_focus(eq_id));
        let ac_open = focused && self.ac.as_ref().is_some_and(|a| a.var == id && !a.items.is_empty());
        let (mut k_down, mut k_up, mut k_accept, mut k_esc) = (false, false, false, false);
        if ac_open {
            ui.input_mut(|i| {
                let none = egui::Modifiers::NONE;
                k_down = i.consume_key(none, egui::Key::ArrowDown);
                k_up = i.consume_key(none, egui::Key::ArrowUp);
                k_accept = i.consume_key(none, egui::Key::Tab) || i.consume_key(none, egui::Key::Enter);
                k_esc = i.consume_key(none, egui::Key::Escape);
            });
        }
        if focused && ui.input(|i| i.key_pressed(egui::Key::F1)) {
            self.open_functions(Some(id));
        }
        let out = egui::TextEdit::multiline(&mut v.equation)
            .id(eq_id)
            // while suggestions show, Tab picks one instead of moving to the next field
            .lock_focus(ac_open)
            .font(egui::TextStyle::Monospace)
            .desired_rows(if v.kind == Kind::Constant { 1 } else { 3 })
            .desired_width(f32::INFINITY)
            .hint_text(if v.kind == Kind::Lookup {
                "e.g. Fraction Infected"
            } else {
                "e.g. Infected / Illness Duration"
            })
            .show(ui);
        if out.response.gained_focus() {
            self.checkpoint();
        }
        if self.canvas.focus_equation {
            out.response.request_focus();
            self.canvas.focus_equation = false;
        }
        if let Some(cr) = out.cursor_range {
            self.eq_cursor = Some(cr.primary.index.0);
        }
        let hint = self.signature_hint(ui, &v.equation, out.response.has_focus().then_some(self.eq_cursor).flatten());
        let below = hint.map(|r| r.bottom()).unwrap_or(out.response.rect.bottom());
        let accept = self.autocomplete(ui, id, &v.equation, &out, (k_down, k_up, k_accept, k_esc), below);
        if let Some((text, st, en)) = accept {
            self.checkpoint();
            let chars: Vec<char> = v.equation.chars().collect();
            let (st, en) = (st.min(chars.len()), en.min(chars.len()));
            let mut e: String = chars[..st].iter().collect();
            e.push_str(&text);
            e.extend(chars[en..].iter());
            v.equation = e;
            let at = st + text.chars().count();
            self.eq_cursor = Some(at);
            if let Some(mut stt) = egui::TextEdit::load_state(ui.ctx(), eq_id) {
                stt.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
                stt.store(ui.ctx(), eq_id);
            }
            ui.memory_mut(|m| m.request_focus(eq_id));
        }

        // validation
        let problem = self
            .problems
            .iter()
            .find(|p| p.var.as_deref() == Some(orig.name.as_str()))
            .map(|p| p.message.clone());
        let parse_err = if v.equation.trim().is_empty() {
            None
        } else {
            expr::parse(&v.equation).err()
        };
        match (parse_err, problem) {
            (Some(e), _) | (None, Some(e)) => {
                ui.label(RichText::new(format!("⚠ {e}")).color(pal.error));
            }
            _ => {
                let val = self.value_now(&v.name).map(theme::fmt_num);
                let txt = match val {
                    Some(x) => format!("✓ Looks good · now {x} {}", v.units),
                    None => "✓ Looks good".into(),
                };
                ui.label(RichText::new(txt).color(pal.ok));
            }
        }

        // inputs from arrows
        let inputs: Vec<(String, Kind)> = self
            .model
            .links
            .iter()
            .filter(|l| l.to == id)
            .filter_map(|l| self.model.var(l.from))
            .map(|s| (s.name.clone(), s.kind))
            .collect();
        let mut used = Vec::new();
        if let Ok(e) = expr::parse(&v.equation) {
            expr::references(&e, &mut used);
        }
        if !inputs.is_empty() {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Inputs (click to insert)")
                    .small()
                    .color(pal.muted),
            );
            ui.horizontal_wrapped(|ui| {
                for (n, k) in &inputs {
                    let is_used = used.contains(&expr::normalize(n));
                    let txt = RichText::new(if is_used {
                        format!("✓ {n}")
                    } else {
                        n.clone()
                    })
                    .small()
                    .color(if is_used { pal.kind(*k) } else { pal.text });
                    let b = egui::Button::new(txt)
                        .fill(pal.kind(*k).gamma_multiply(0.1))
                        .stroke(Stroke::new(
                            1.0,
                            pal.kind(*k)
                                .gamma_multiply(if is_used { 0.6 } else { 0.25 }),
                        ))
                        .corner_radius(CornerRadius::same(10));
                    if ui
                        .add(b)
                        .on_hover_text(if is_used {
                            "Used in the equation"
                        } else {
                            "Linked by an arrow but not used yet"
                        })
                        .clicked()
                    {
                        insert = Some(n.clone());
                    }
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.menu_button(RichText::new("+ Variable").size(14.0), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        for (t, help) in expr::TIME_VARS {
                            if ui
                                .button(RichText::new(t.to_uppercase()).small())
                                .on_hover_text(*help)
                                .clicked()
                            {
                                insert = Some(t.to_uppercase());
                                ui.close();
                            }
                        }
                        ui.separator();
                        for o in self.model.vars.iter().filter(|o| o.id != id) {
                            let txt = if o.kind == Kind::Lookup && o.equation.trim().is_empty() {
                                format!("{}( )", o.name)
                            } else {
                                o.name.clone()
                            };
                            if ui
                                .button(RichText::new(txt).small().color(pal.kind(o.kind)))
                                .clicked()
                            {
                                insert = Some(
                                    if o.kind == Kind::Lookup && o.equation.trim().is_empty() {
                                        format!("{}(", o.name)
                                    } else {
                                        o.name.clone()
                                    },
                                );
                                ui.close();
                            }
                        }
                    });
            });
            if ui.button(RichText::new("ƒ  Functions…").size(14.0)).on_hover_text("Browse every function with explanations and examples (F1)").clicked() {
                self.open_functions(Some(id));
            }
            for op in ["+", "−", "×", "÷", "^", "( )"] {
                if ui.small_button(RichText::new(op).monospace()).clicked() {
                    insert = Some(
                        match op {
                            "−" => "-",
                            "×" => "*",
                            "÷" => "/",
                            "( )" => "(",
                            o => o,
                        }
                        .to_string(),
                    );
                }
            }
        });
        if let Some(text) = insert {
            self.checkpoint();
            let at = insert_at(&mut v.equation, self.eq_cursor, &format!("{text} "));
            self.eq_cursor = Some(at);
            if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), eq_id) {
                st.cursor.set_char_range(Some(egui::text::CCursorRange::one(
                    egui::text::CCursor::new(at),
                )));
                st.store(ui.ctx(), eq_id);
            }
            ui.memory_mut(|m| m.request_focus(eq_id));
        }
        // names used in the equation that have no arrow yet
        let missing: Vec<String> = used
            .iter()
            .filter_map(|n| self.model.by_name(n))
            .filter(|s| {
                s.id != id
                    && !self
                        .model
                        .links
                        .iter()
                        .any(|l| l.from == s.id && l.to == id)
            })
            .map(|s| s.name.clone())
            .collect();
        if !missing.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!("No arrow yet from {}", missing.join(", ")))
                        .small()
                        .color(pal.aux),
                );
                if ui.small_button("Add arrows").clicked() {
                    self.checkpoint();
                    for n in &missing {
                        if let Some(s) = self.model.by_name(&expr::normalize(n)).map(|s| s.id) {
                            self.model.add_link(s, id);
                        }
                    }
                }
            });
        }

        // ---- lookup table
        if v.kind == Kind::Lookup {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("LOOKUP TABLE").small().strong().color(pal.muted));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::icons::tool(ui, crate::icons::Ic::Table, "Copy the points (for Excel)", pal.muted, pal.accent).clicked() {
                        if let Some(d) = self.chart_data(&crate::export::ChartRef::Lookup(id)) {
                            ui.ctx().copy_text(d);
                            self.status = "Copied the lookup points — paste into any spreadsheet".into();
                        }
                    }
                    self.chart_copy_button(ui, crate::export::ChartRef::Lookup(id));
                });
            });
            ui.label(
                RichText::new(
                    "Drag points to reshape · double-click to add · right-click to remove",
                )
                .small()
                .color(pal.muted),
            );
            let marker = match expr::parse(&v.equation) {
                Ok(expr::Expr::Var(n)) => self.value_now(&n),
                _ => None,
            };
            if lookup_editor(ui, &mut v.table, pal, marker, id) {
                // checkpoint is taken when a drag starts inside the editor
            }
            if ui
                .data(|d| d.get_temp::<bool>(egui::Id::unique(("lk-start", id))))
                .unwrap_or(false)
            {
                ui.data_mut(|d| d.remove::<bool>(egui::Id::unique(("lk-start", id))));
                self.checkpoint();
            }
            egui::CollapsingHeader::new("Points as numbers")
                .default_open(false)
                .show(ui, |ui| {
                    let mut remove = None;
                    egui::Grid::new(("pts", id))
                        .num_columns(3)
                        .spacing([8.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(RichText::new("x (input)").small().color(pal.muted));
                            ui.label(RichText::new("y (output)").small().color(pal.muted));
                            ui.end_row();
                            let n = v.table.len();
                            for (i, p) in v.table.iter_mut().enumerate() {
                                ui.add(egui::DragValue::new(&mut p[0]).speed(0.01).max_decimals(4));
                                ui.add(egui::DragValue::new(&mut p[1]).speed(0.01).max_decimals(4));
                                if n > 2 && ui.small_button("×").clicked() {
                                    remove = Some(i);
                                }
                                ui.end_row();
                            }
                        });
                    if let Some(i) = remove {
                        v.table.remove(i);
                    }
                    if ui.button("Add point").clicked() {
                        let last = v.table.last().copied().unwrap_or([0.0, 0.0]);
                        let step = if v.table.len() > 1 {
                            last[0] - v.table[v.table.len() - 2][0]
                        } else {
                            1.0
                        };
                        v.table.push([last[0] + step.max(1e-6), last[1]]);
                    }
                });
        }

        // ---- flow ends
        if v.kind == Kind::Flow {
            section(ui, pal, "Connections");
            let stocks: Vec<(u64, String)> = self
                .model
                .vars
                .iter()
                .filter(|s| s.kind == Kind::Stock)
                .map(|s| (s.id, s.name.clone()))
                .collect();
            for (label, end) in [("Drains from", &mut v.from), ("Fills", &mut v.to)] {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).color(pal.muted));
                    let cur = end
                        .and_then(|e| stocks.iter().find(|s| s.0 == e))
                        .map(|s| s.1.clone())
                        .unwrap_or("Cloud (outside the model)".into());
                    egui::ComboBox::from_id_salt((label, id))
                        .selected_text(cur)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(end.is_none(), "Cloud (outside the model)")
                                .clicked()
                            {
                                *end = None;
                            }
                            for (sid, n) in &stocks {
                                if ui.selectable_label(*end == Some(*sid), n).clicked() {
                                    *end = Some(*sid);
                                }
                            }
                        });
                });
            }
            if v.from.is_some() && v.from == v.to {
                v.to = orig.to;
                v.from = orig.from;
            }
        }

        // ---- details
        let set_names: Vec<String> = self.model.sets.iter().map(|s| s.name.clone()).collect();
        section(ui, pal, "Details");
        egui::Grid::new(("details", id))
            .num_columns(2)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Units").color(pal.muted));
                let r = ui.add(
                    egui::TextEdit::singleline(&mut v.units)
                        .desired_width(f32::INFINITY)
                        .hint_text("e.g. People/Day"),
                );
                if r.gained_focus() {
                    self.checkpoint();
                }
                ui.end_row();
                if !set_names.is_empty() || v.set.is_some() {
                    ui.label(RichText::new("One for each").color(pal.muted));
                    ui.horizontal(|ui| {
                        let cur = v.set.clone().unwrap_or_else(|| "(just one)".into());
                        egui::ComboBox::from_id_salt(("per", id)).selected_text(cur).show_ui(ui, |ui| {
                            if ui.selectable_label(v.set.is_none(), "(just one)").clicked() {
                                self.checkpoint();
                                v.set = None;
                                v.by_item.clear();
                            }
                            for s in &set_names {
                                if ui.selectable_label(v.set.as_deref() == Some(s.as_str()), s).clicked() {
                                    self.checkpoint();
                                    v.set = Some(s.clone());
                                }
                            }
                        });
                        if v.set.is_some() && v.kind != Kind::Lookup {
                            ui.selectable_value(&mut v.rollup, Rollup::Total, "Total");
                            ui.selectable_value(&mut v.rollup, Rollup::Average, "Average");
                        }
                    });
                    ui.end_row();
                }
                if v.kind == Kind::Constant {
                    ui.label(RichText::new("Lever").color(pal.muted));
                    ui.checkbox(&mut v.lever, "Key lever").on_hover_text("Shown first, with the controls in the side panel; other constants fold away");
                    ui.end_row();
                }
                if v.kind == Kind::Constant && v.set.is_none() {
                    ui.label(RichText::new("Slider").color(pal.muted));
                    ui.horizontal(|ui| {
                        let mut s = v.slider.unwrap_or([0.0, 10.0]);
                        ui.add(egui::DragValue::new(&mut s[0]).speed(0.1).prefix("min "));
                        ui.add(egui::DragValue::new(&mut s[1]).speed(0.1).prefix("max "));
                        if Some(s) != v.slider && s[1] > s[0] {
                            v.slider = Some(s);
                        }
                    });
                    ui.end_row();
                }
                if v.kind == Kind::Stock {
                    ui.label(RichText::new("Limit").color(pal.muted));
                    ui.checkbox(&mut v.non_negative, "Never below zero");
                    ui.end_row();
                }
            });
        if let Some(set) = v.set.clone().and_then(|sn| self.model.sets.iter().find(|s| s.name == sn).cloned()) {
            let n_diff = v.by_item.iter().filter(|o| !o.equation.trim().is_empty() || !o.table.is_empty()).count();
            egui::CollapsingHeader::new(format!("Differs by {} ({} of {} items)", set.name, n_diff, set.items.len()))
                .id_salt(("byitem", id))
                .show(ui, |ui| {
                    ui.label(RichText::new("Leave a box empty to use the formula above for that item.").small().color(pal.muted));
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for it in &set.items {
                            let pos = v.by_item.iter().position(|o| o.item == *it);
                            let mut text = pos.map(|i| v.by_item[i].equation.clone()).unwrap_or_default();
                            let has_curve = pos.is_some_and(|i| !v.by_item[i].table.is_empty());
                            ui.horizontal(|ui| {
                                ui.add_sized([130.0, 18.0], egui::Label::new(RichText::new(it).small()).truncate());
                                let hint = if has_curve { "its own curve" } else { "" };
                                let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY).hint_text(hint));
                                if r.gained_focus() {
                                    self.checkpoint();
                                }
                                if r.changed() {
                                    match pos {
                                        Some(i) => v.by_item[i].equation = text.clone(),
                                        None => v.by_item.push(ItemValue { item: it.clone(), equation: text.clone(), table: Vec::new() }),
                                    }
                                }
                            });
                        }
                    });
                });
        }
        ui.label(RichText::new("Notes").color(pal.muted));
        let r = ui.add(
            egui::TextEdit::multiline(&mut v.comment)
                .desired_rows(2)
                .desired_width(f32::INFINITY)
                .hint_text("What this means, where the number comes from…"),
        );
        if r.gained_focus() {
            self.checkpoint();
        }

        // ---- trend
        let lookup_fn = v.kind == Kind::Lookup && v.equation.trim().is_empty();
        if !lookup_fn {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("BEHAVIOUR OVER TIME").small().strong().color(pal.muted));
                if self.run.is_some() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        self.chart_copy_button(ui, crate::export::ChartRef::Vars { kind: crate::model::ChartKind::Line, vars: vec![orig.name.clone()], title: orig.name.clone() });
                    });
                }
            });
            self.mini_trend(ui, &orig.name, col);
        }

        // write back
        let a = serde_json::to_string(&orig).unwrap_or_default();
        let b = serde_json::to_string(&v).unwrap_or_default();
        if a != b {
            if let Some(slot) = self.model.var_mut(id) {
                *slot = v;
            }
            self.changed();
        }
    }

    fn mini_trend(&mut self, ui: &mut egui::Ui, name: &str, col: Color32) {
        let pal = self.pal;
        let Some(run) = &self.run else {
            ui.label(RichText::new("Run the model to see this").color(pal.muted));
            return;
        };
        let Some(vals) = run.get(name) else {
            ui.label(RichText::new("Not simulated yet").color(pal.muted));
            return;
        };
        let _ = vals;
        let pts: Vec<[f64; 2]> = self.plot_pts(run, name).unwrap_or_default();
        let others: Vec<(String, Vec<[f64; 2]>, Color32, LineStyle, f32)> = self
            .compare_runs()
            .iter()
            .filter_map(|s| {
                let (c, st, w) = self.compare_style(s, col, true);
                s.run.get(name).map(|v| (s.run.name.clone(), s.run.times.iter().zip(v).map(|(t, v)| [*t, *v]).collect(), c, st, w))
            })
            .collect();
        let scrub_t = self.time_index().map(|i| run.times[i]);
        let plot = self.hold_axes(
            Plot::new(("mini", name))
                .height(150.0)
                .allow_drag(false)
                .allow_zoom(false)
                .allow_scroll(false)
                .allow_boxed_zoom(false)
                .show_x(true)
                .show_y(true),
            run,
            &[name.to_string()],
        );
        let r = plot.show(ui, |pu| {
            for (n, p, c, st, w) in others {
                pu.line(
                    Line::new(n, PlotPoints::from(p))
                        .color(c)
                        .style(st)
                        .width(w),
                );
            }
            self.draw_head(pu, &pts, col);
            pu.line(
                Line::new(name.to_string(), PlotPoints::from(pts))
                    .color(col)
                    .width(2.2),
            );
            if let Some(t) = scrub_t {
                pu.vline(
                    VLine::new("", t)
                        .color(pal.muted.gamma_multiply(0.6))
                        .width(1.0),
                );
            }
        });
        self.chart_menu(&r.response, crate::export::ChartRef::Vars { kind: crate::model::ChartKind::Line, vars: vec![name.to_string()], title: name.to_string() });
    }

    fn link_editor(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let l = self
            .model
            .links
            .iter()
            .find(|l| l.id == id)
            .unwrap()
            .clone();
        let from = self
            .model
            .var(l.from)
            .map(|v| v.name.clone())
            .unwrap_or_default();
        let to = self
            .model
            .var(l.to)
            .map(|v| v.name.clone())
            .unwrap_or_default();
        badge(ui, "Arrow", pal.link);
        ui.add_space(6.0);
        ui.label(
            RichText::new(format!("{from}  →  {to}"))
                .size(16.0)
                .strong(),
        );
        ui.label(RichText::new(format!("'{to}' depends on '{from}'.")).color(pal.muted));
        section(ui, pal, "Polarity");
        let mut pol = l.polarity.clone();
        ui.horizontal(|ui| {
            ui.selectable_value(&mut pol, "+".to_string(), "+ same direction");
            ui.selectable_value(&mut pol, "-".to_string(), "− opposite");
            ui.selectable_value(&mut pol, String::new(), "none");
        });
        section(ui, pal, "Shape");
        let mut e = l.clone();
        ui.horizontal(|ui| {
            for (label, b) in [("Straight", 0.0f32), ("Gentle", 30.0), ("Round", 70.0)] {
                if ui
                    .selectable_label((e.bend.abs() - b).abs() < 1.0, label)
                    .clicked()
                {
                    e.bend = if e.bend < 0.0 { -b } else { b };
                }
            }
            if ui
                .button("⇅ Flip")
                .on_hover_text("Bend the other way")
                .clicked()
            {
                e.bend = -e.bend;
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Route").color(pal.muted));
            if ui.selectable_label(e.shape.is_none(), "Usual").clicked() {
                e.shape = None;
            }
            for sh in crate::model::ArrowShape::ALL {
                if ui.selectable_label(e.shape == Some(sh), sh.label()).clicked() {
                    e.shape = Some(sh);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tip").color(pal.muted));
            if ui.selectable_label(e.head.is_none(), "Usual").clicked() {
                e.head = None;
            }
            for h in crate::model::ArrowHead::ALL {
                if ui.selectable_label(e.head == Some(h), h.label()).clicked() {
                    e.head = Some(h);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Turning points").color(pal.muted));
            ui.label(format!("{}", e.via.len()));
            if ui.button("+ Add one").on_hover_text("The arrow becomes a smooth path through its turning points. Drag the dots; drag a small circle to pull out a new one; double-click the arrow to add one there.").clicked() {
                let (a, b) = (self.model.var(e.from).map(|v| v.pos), self.model.var(e.to).map(|v| v.pos));
                if let (Some(a), Some(b)) = (a, b) {
                    // in the middle of the longest stretch, nudged sideways so the change shows
                    let mut cp = vec![a];
                    cp.extend(e.via.iter().copied());
                    cp.push(b);
                    let k = (0..cp.len() - 1).max_by(|&i, &j| {
                        let d = |i: usize| ((cp[i][0] - cp[i + 1][0]).powi(2) + (cp[i][1] - cp[i + 1][1]).powi(2)).sqrt();
                        d(i).total_cmp(&d(j))
                    }).unwrap_or(0);
                    let m = [(cp[k][0] + cp[k + 1][0]) / 2.0 + 30.0, (cp[k][1] + cp[k + 1][1]) / 2.0 - 30.0];
                    e.via.insert(k, m);
                }
            }
            if !e.via.is_empty() && ui.button("Remove all").clicked() {
                e.via.clear();
            }
        });
        ui.add(egui::Slider::new(&mut e.bend, -300.0..=300.0).text("curve"));
        ui.label(
            RichText::new("Tip: drag the round handle on the arrow to bend it.")
                .small()
                .color(pal.muted),
        );
        section(ui, pal, "Style");
        ui.horizontal(|ui| {
            ui.label(RichText::new("Line").color(pal.muted));
            for (i, label) in ["Thin", "Normal", "Bold"].into_iter().enumerate() {
                ui.selectable_value(&mut e.weight, i as u8, label);
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut e.dashed, "Dashed");
            ui.checkbox(&mut e.delay_mark, "Delay mark ||")
                .on_hover_text("Marks that the effect takes time to arrive");
        });
        ui.horizontal(|ui| {
            ui.label(RichText::new("Colour").color(pal.muted));
            if ui.selectable_label(e.color.is_none(), "Default").clicked() {
                e.color = None;
            }
            for i in 0..6 {
                let c = pal.series(i);
                let (r, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
                let on = e.color == Some([c.r(), c.g(), c.b()]);
                ui.painter().circle(
                    r.center(),
                    7.0,
                    c,
                    Stroke::new(if on { 2.5 } else { 0.0 }, pal.text),
                );
                if resp.clicked() {
                    e.color = Some([c.r(), c.g(), c.b()]);
                }
            }
        });
        e.polarity = pol;
        let a = serde_json::to_string(&l).unwrap_or_default();
        let b = serde_json::to_string(&e).unwrap_or_default();
        if a != b {
            if e.bend == l.bend {
                self.checkpoint();
            }
            if let Some(x) = self.model.links.iter_mut().find(|x| x.id == id) {
                *x = e;
            }
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button(format!("Select {from}")).clicked() {
                self.sel = Some(Sel::Var(l.from));
            }
            if ui.button(format!("Select {to}")).clicked() {
                self.sel = Some(Sel::Var(l.to));
            }
        });
        if ui
            .button(RichText::new("Delete arrow").color(pal.error))
            .clicked()
        {
            self.delete_selection();
        }
    }

    fn control_editor(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let Some(c) = self.model.controls.iter().find(|c| c.id == id).cloned() else { return };
        badge(ui, c.kind.label(), pal.muted);
        ui.add_space(6.0);
        match self.control_state(&c) {
            Some(st) => {
                ui.label(RichText::new(format!("Sets {} (now {})", st.name, crate::controls_ui::fmt_value(st.value))).color(pal.text));
            }
            None => {
                ui.label(RichText::new("Not tied to a constant yet. Pick one on the Control tab of the ribbon.").color(pal.muted));
            }
        }
        ui.label(RichText::new("Operate it right on the diagram: drag its track or dial, flip the switch, press − and +. Drag its edge to move it.").color(pal.muted));
        ui.add_space(8.0);
        if ui.button(RichText::new("Delete").color(pal.error)).clicked() {
            self.delete_selection();
        }
    }

    fn shape_editor(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        let Some(s) = self.model.shapes.iter().find(|s| s.id == id).cloned() else { return };
        badge(ui, s.kind.label(), pal.muted);
        ui.add_space(6.0);
        if s.kind == crate::model::ShapeKind::Picture {
            let name = s.image.and_then(|i| self.model.images.iter().find(|a| a.id == i)).map(|a| a.name.clone()).unwrap_or_else(|| "a missing picture".into());
            ui.label(RichText::new(name).color(pal.text));
        }
        ui.label(RichText::new("Colours, line width and size are on the Shape tab of the ribbon. Drag the squares to resize.").color(pal.muted));
        ui.add_space(8.0);
        if ui.button(RichText::new("Delete").color(pal.error)).clicked() {
            self.delete_selection();
        }
    }

    fn note_editor(&mut self, ui: &mut egui::Ui, id: u64) {
        let pal = self.pal;
        badge(ui, "Note", pal.muted);
        ui.add_space(6.0);
        let mut text = self
            .model
            .notes
            .iter()
            .find(|n| n.id == id)
            .unwrap()
            .text
            .clone();
        let r = ui.add(
            egui::TextEdit::multiline(&mut text)
                .desired_rows(5)
                .desired_width(f32::INFINITY),
        );
        if r.gained_focus() {
            self.checkpoint();
        }
        if r.changed() {
            if let Some(n) = self.model.notes.iter_mut().find(|n| n.id == id) {
                n.text = text;
            }
        }
        if ui
            .button(RichText::new("Delete note").color(pal.error))
            .clicked()
        {
            self.delete_selection();
        }
    }
}

/// One suggestion in the equation editor's drop-down.
#[derive(Clone)]
pub struct AcItem {
    pub label: String,
    pub insert: String,
    pub hint: String,
    pub color: Option<Color32>,
    pub function: bool,
}

pub struct Ac {
    pub var: u64,
    pub start: usize,
    pub end: usize,
    pub prefix: String,
    pub items: Vec<AcItem>,
    pub sel: usize,
}

impl App {
    /// Suggest functions and variables for the word being typed. Returns (text, start, end) to insert.
    fn autocomplete(&mut self, ui: &mut egui::Ui, id: u64, eq: &str, out: &egui::text_edit::TextEditOutput, keys: (bool, bool, bool, bool), below: f32) -> Option<(String, usize, usize)> {
        let pal = self.pal;
        let over_popup = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| self.ac_rect.is_some_and(|r| r.contains(p)));
        if out.response.has_focus() {
            let c = out.cursor_range.map(|r| r.primary.index.0).unwrap_or(0);
            let chars: Vec<char> = eq.chars().collect();
            let c = c.min(chars.len());
            let mut st = c;
            while st > 0 && !"+-*/^(),<>=!:&|\n".contains(chars[st - 1]) {
                st -= 1;
            }
            while st < c && chars[st] == ' ' {
                st += 1;
            }
            let prefix: String = chars[st..c].iter().collect();
            let np = expr::normalize(&prefix);
            let dismissed = self.ac_dismissed.as_ref().is_some_and(|(v, p)| *v == id && *p == prefix);
            let word_ends = c == chars.len() || !chars[c].is_alphanumeric();
            let mut items: Vec<(u8, AcItem)> = Vec::new();
            if !np.is_empty() && !np.starts_with(|ch: char| ch.is_ascii_digit()) && !dismissed && word_ends {
                let rank = |name: &str| -> Option<u8> {
                    let n = expr::normalize(name);
                    if n.starts_with(&np) {
                        Some(0)
                    } else if np.len() >= 2 && n.split(' ').any(|w| w.starts_with(&np)) {
                        Some(1)
                    } else {
                        None
                    }
                };
                for f in FUNCTIONS {
                    if let Some(r) = rank(f.name) {
                        items.push((r, AcItem { label: f.name.into(), insert: format!("{}(", f.name), hint: format!("{}  —  {}", f.signature, f.help), color: None, function: true }));
                    }
                }
                // the person's own functions, suggested like built-ins
                for f in &self.model.functions {
                    if let Some(r) = rank(&f.name) {
                        let sig = format!("{}({})", f.name, f.params.join(", "));
                        let help = if f.description.is_empty() { f.body.chars().take(60).collect::<String>() } else { f.description.clone() };
                        items.push((r, AcItem { label: f.name.clone(), insert: format!("{}(", f.name), hint: format!("{sig}  —  {help}"), color: Some(pal.accent), function: true }));
                    }
                }
                for (t, help) in expr::TIME_VARS {
                    if let Some(r) = rank(t) {
                        items.push((r, AcItem { label: t.to_uppercase(), insert: t.to_uppercase(), hint: help.to_string(), color: Some(pal.muted), function: false }));
                    }
                }
                for o in self.model.vars.iter().filter(|o| o.id != id) {
                    if let Some(r) = rank(&o.name) {
                        if expr::normalize(&o.name) == np {
                            continue;
                        }
                        let lookup_fn = o.kind == Kind::Lookup && o.equation.trim().is_empty();
                        let hint = if lookup_fn { "Lookup — call it with an input in brackets".to_string() } else { format!("{} · {}", o.kind.label(), o.equation.chars().take(60).collect::<String>()) };
                        items.push((r, AcItem { label: o.name.clone(), insert: if lookup_fn { format!("{}(", o.name) } else { o.name.clone() }, hint, color: Some(pal.kind(o.kind)), function: lookup_fn }));
                    }
                }
                items.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.label.len().cmp(&b.1.label.len())));
                items.truncate(8);
            }
            let prev = self.ac.as_ref().filter(|a| a.var == id && a.prefix == prefix).map(|a| a.sel).unwrap_or(0);
            self.ac = if items.is_empty() { None } else { Some(Ac { var: id, start: st, end: c, prefix, items: items.into_iter().map(|x| x.1).collect(), sel: prev }) };
        } else if !over_popup && self.ac.as_ref().is_some_and(|a| a.var == id) {
            self.ac = None;
        }
        let mut accept = None;
        let (down, up, enter, esc) = keys;
        if let Some(a) = self.ac.as_mut().filter(|a| a.var == id) {
            let n = a.items.len();
            if down {
                a.sel = (a.sel + 1).min(n - 1);
            }
            if up {
                a.sel = a.sel.saturating_sub(1);
            }
            a.sel = a.sel.min(n - 1);
            if enter {
                accept = Some((a.items[a.sel].insert.clone(), a.start, a.end));
            }
            if esc {
                self.ac_dismissed = Some((id, a.prefix.clone()));
            }
        }
        if esc || accept.is_some() {
            self.ac = None;
        }
        // the drop-down itself
        self.ac_rect = None;
        if let Some(a) = self.ac.as_ref().filter(|a| a.var == id) {
            let (items, sel) = (a.items.clone(), a.sel);
            let w = out.response.rect.width().max(380.0);
            let res = egui::Area::new(ui.scope_id().with("autocomplete")).order(egui::Order::Foreground).fixed_pos(egui::pos2(out.response.rect.left(), below + 4.0)).show(ui.ctx(), |ui| {
                let mut hovered = None;
                let mut clicked = None;
                egui::Frame::new()
                    .fill(pal.panel)
                    .stroke(Stroke::new(1.0, pal.border))
                    .corner_radius(CornerRadius::same(10))
                    .inner_margin(egui::Margin::same(6))
                    .shadow(egui::Shadow { offset: [0, 6], blur: 18, spread: 0, color: Color32::from_black_alpha(if pal.dark { 90 } else { 30 }) })
                    .show(ui, |ui| {
                        ui.set_width(w);
                        for (i, it) in items.iter().enumerate() {
                            let on = i == sel;
                            let resp = egui::Frame::new()
                                .fill(if on { pal.accent_soft } else { Color32::TRANSPARENT })
                                .corner_radius(CornerRadius::same(6))
                                .inner_margin(egui::Margin::symmetric(8, 4))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.horizontal(|ui| {
                                        if it.function {
                                            ui.label(RichText::new("ƒ").size(15.0).strong().color(pal.accent));
                                        } else {
                                            let (r, _) = ui.allocate_exact_size(vec2(15.0, 15.0), Sense::hover());
                                            ui.painter().circle_filled(r.center(), 4.5, it.color.unwrap_or(pal.muted));
                                        }
                                        ui.label(RichText::new(&it.label).size(15.0).strong().color(if on { pal.accent } else { pal.text }));
                                    });
                                    ui.label(RichText::new(&it.hint).size(12.5).color(pal.muted));
                                })
                                .response
                                .interact(Sense::click());
                            if resp.hovered() {
                                hovered = Some(i);
                            }
                            if resp.clicked() {
                                clicked = Some(i);
                            }
                        }
                        ui.add_space(2.0);
                        ui.label(RichText::new("↑↓ choose  ·  Tab or Enter insert  ·  Esc close  ·  F1 full reference").size(11.5).color(pal.muted));
                    });
                (hovered, clicked)
            });
            self.ac_rect = Some(res.response.rect);
            let (hovered, clicked) = res.inner;
            if let Some(a) = self.ac.as_mut() {
                if let Some(h) = hovered {
                    a.sel = h;
                }
                if let Some(c) = clicked {
                    accept = Some((a.items[c].insert.clone(), a.start, a.end));
                }
            }
            if accept.is_some() {
                self.ac = None;
            }
        }
        accept
    }

    /// When the cursor is inside a function call, show that function's arguments.
    fn signature_hint(&self, ui: &mut egui::Ui, eq: &str, cursor: Option<usize>) -> Option<egui::Rect> {
        let pal = self.pal;
        let c = cursor?;
        let chars: Vec<char> = eq.chars().collect();
        let c = c.min(chars.len());
        // walk back to the innermost unclosed bracket, counting commas on the way
        let (mut depth, mut commas, mut open) = (0i32, 0usize, None);
        for i in (0..c).rev() {
            match chars[i] {
                ')' => depth += 1,
                '(' if depth == 0 => {
                    open = Some(i);
                    break;
                }
                '(' => depth -= 1,
                ',' if depth == 0 => commas += 1,
                _ => {}
            }
        }
        let o = open?;
        let mut st = o;
        while st > 0 && (chars[st - 1].is_alphanumeric() || chars[st - 1] == ' ' || chars[st - 1] == '_') {
            st -= 1;
        }
        let name: String = chars[st..o].iter().collect();
        let f = crate::expr::function_info(&expr::normalize(&name).to_uppercase())?;
        let doc = crate::docs::find(f.name);
        // highlight the argument being typed
        let args_part = f.signature.split_once('(').map(|x| x.1.trim_end_matches(')')).unwrap_or("");
        let args: Vec<&str> = args_part.split(", ").collect();
        let r = egui::Frame::new().fill(pal.accent_soft).corner_radius(CornerRadius::same(8)).inner_margin(egui::Margin::symmetric(10, 6)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(RichText::new(format!("{}(", f.name)).monospace().size(13.5).color(pal.text));
                for (i, a) in args.iter().enumerate() {
                    let on = i == commas.min(args.len().saturating_sub(1));
                    let t = RichText::new(*a).monospace().size(13.5);
                    ui.label(if on { t.strong().color(pal.accent).underline() } else { t.color(pal.muted) });
                    if i + 1 < args.len() {
                        ui.label(RichText::new(", ").monospace().size(13.5).color(pal.muted));
                    }
                }
                ui.label(RichText::new(")").monospace().size(13.5).color(pal.text));
            });
            if let Some(d) = doc {
                let i = commas.min(d.params.len().saturating_sub(1));
                if let Some((pn, pd)) = d.params.get(i) {
                    ui.label(RichText::new(format!("{pn}: {pd}")).size(12.5).color(pal.text));
                } else {
                    ui.label(RichText::new(d.summary).size(12.5).color(pal.text));
                }
            }
        });
        Some(r.response.rect)
    }
}

fn trim_num(v: f64) -> String {
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

/// An editable graph of a lookup table. Returns true if the table changed.
fn lookup_editor(
    ui: &mut egui::Ui,
    table: &mut Vec<[f64; 2]>,
    pal: Palette,
    marker: Option<f64>,
    var_id: u64,
) -> bool {
    table.sort_by(|a, b| a[0].total_cmp(&b[0]));
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 200.0), Sense::click_and_drag());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::same(8), pal.surface);
    let plot = Rect::from_min_max(rect.min + vec2(40.0, 12.0), rect.max - vec2(12.0, 26.0));

    // bounds stay fixed while dragging so points don't run away
    let bid = egui::Id::unique(("lk-bounds", var_id));
    let did = egui::Id::unique(("lk-drag", var_id));
    let dragging: Option<usize> = ui.data(|d| d.get_temp(did));
    let bounds = match (dragging, ui.data(|d| d.get_temp::<[f64; 4]>(bid))) {
        (Some(_), Some(b)) => b,
        _ => {
            let (mut x0, mut x1, mut y0, mut y1) =
                (f64::INFINITY, f64::NEG_INFINITY, 0.0f64, f64::NEG_INFINITY);
            for q in table.iter() {
                x0 = x0.min(q[0]);
                x1 = x1.max(q[0]);
                y0 = y0.min(q[1]);
                y1 = y1.max(q[1]);
            }
            if !x0.is_finite() {
                (x0, x1, y1) = (0.0, 1.0, 1.0);
            }
            if x1 - x0 < 1e-9 {
                x1 = x0 + 1.0;
            }
            if y1 - y0 < 1e-9 {
                y1 = y0 + 1.0;
            }
            let pad = (y1 - y0) * 0.15;
            let b = [x0, x1, y0, y1 + pad];
            ui.data_mut(|d| d.insert_temp(bid, b));
            b
        }
    };
    let [x0, x1, y0, y1] = bounds;
    let to_s = |q: [f64; 2]| {
        pos2(
            plot.left() + ((q[0] - x0) / (x1 - x0)) as f32 * plot.width(),
            plot.bottom() - ((q[1] - y0) / (y1 - y0)) as f32 * plot.height(),
        )
    };
    let from_s = |s: Pos2| {
        [
            x0 + ((s.x - plot.left()) / plot.width()) as f64 * (x1 - x0),
            y0 + ((plot.bottom() - s.y) / plot.height()) as f64 * (y1 - y0),
        ]
    };

    for i in 0..=4 {
        let fx = i as f32 / 4.0;
        let x = plot.left() + fx * plot.width();
        let y = plot.bottom() - fx * plot.height();
        p.line_segment(
            [pos2(x, plot.top()), pos2(x, plot.bottom())],
            Stroke::new(1.0, pal.border),
        );
        p.line_segment(
            [pos2(plot.left(), y), pos2(plot.right(), y)],
            Stroke::new(1.0, pal.border),
        );
        p.text(
            pos2(x, plot.bottom() + 4.0),
            Align2::CENTER_TOP,
            theme::fmt_num(x0 + (x1 - x0) * fx as f64),
            FontId::monospace(9.5),
            pal.muted,
        );
        p.text(
            pos2(plot.left() - 4.0, y),
            Align2::RIGHT_CENTER,
            theme::fmt_num(y0 + (y1 - y0) * fx as f64),
            FontId::monospace(9.5),
            pal.muted,
        );
    }
    if let Some(mx) = marker {
        if mx >= x0 && mx <= x1 {
            let s = to_s([mx, expr::lookup(table, mx)]);
            p.line_segment(
                [pos2(s.x, plot.top()), pos2(s.x, plot.bottom())],
                Stroke::new(1.0, pal.accent.gamma_multiply(0.6)),
            );
            p.circle_filled(s, 4.0, pal.accent);
        }
    }
    let pts: Vec<Pos2> = table.iter().map(|q| to_s(*q)).collect();
    if pts.len() > 1 {
        p.add(egui::Shape::line(pts.clone(), Stroke::new(2.2, pal.lookup)));
    }
    let pointer = resp.hover_pos().or(resp.interact_pointer_pos());
    let nearest = pointer.and_then(|pp| {
        pts.iter()
            .enumerate()
            .map(|(i, s)| (i, s.distance(pp)))
            .filter(|(_, d)| *d < 10.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    });
    for (i, s) in pts.iter().enumerate() {
        let hot = Some(i) == nearest || Some(i) == dragging;
        p.circle(
            *s,
            if hot { 6.0 } else { 4.5 },
            pal.panel,
            Stroke::new(2.0, pal.lookup),
        );
    }
    p.rect_stroke(
        rect,
        CornerRadius::same(8),
        Stroke::new(1.0, pal.border),
        StrokeKind::Inside,
    );

    let mut changed = false;
    if resp.drag_started() {
        if let Some(i) = nearest {
            ui.data_mut(|d| {
                d.insert_temp(did, i);
                d.insert_temp(egui::Id::unique(("lk-start", var_id)), true);
            });
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(pp)) = (dragging, resp.interact_pointer_pos()) {
            if i < table.len() {
                let mut q = from_s(pp.clamp(plot.min, plot.max));
                let lo = if i > 0 {
                    table[i - 1][0]
                } else {
                    f64::NEG_INFINITY
                };
                let hi = if i + 1 < table.len() {
                    table[i + 1][0]
                } else {
                    f64::INFINITY
                };
                q[0] = q[0].clamp(lo, hi);
                // the end points keep their x so the input range stays put
                if i == 0 || i + 1 == table.len() {
                    q[0] = table[i][0];
                }
                table[i] = [round_sig(q[0]), round_sig(q[1])];
                changed = true;
            }
        }
    }
    if resp.drag_stopped() {
        ui.data_mut(|d| d.remove::<usize>(did));
    }
    if resp.double_clicked() && nearest.is_none() {
        if let Some(pp) = resp.interact_pointer_pos() {
            let q = from_s(pp);
            ui.data_mut(|d| d.insert_temp(egui::Id::unique(("lk-start", var_id)), true));
            table.push([round_sig(q[0]), round_sig(q[1])]);
            table.sort_by(|a, b| a[0].total_cmp(&b[0]));
            changed = true;
        }
    }
    if resp.secondary_clicked() && table.len() > 2 {
        if let Some(i) = nearest {
            ui.data_mut(|d| d.insert_temp(egui::Id::unique(("lk-start", var_id)), true));
            table.remove(i);
            changed = true;
        }
    }
    let _ = Color32::WHITE;
    changed
}

fn round_sig(v: f64) -> f64 {
    if v == 0.0 {
        return 0.0;
    }
    let mag = 10f64.powi(3 - v.abs().log10().ceil() as i32);
    (v * mag).round() / mag
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typing part of a name shows suggestions; Tab puts the full name in.
    #[test]
    fn autocomplete_suggests_and_inserts() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None);
        let id = app.model.by_name("infection rate").unwrap().id;
        app.select_one(Some(Sel::Var(id)));
        app.model.var_mut(id).unwrap().equation = "Contact Rate * SMOOTH(Infected, Ill".into();
        app.canvas.focus_equation = true;
        let mut frame = |app: &mut App, events: Vec<egui::Event>| {
            let input = egui::RawInput { events, focused: true, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| app.inspector(ui));
            out.textures_delta.clear();
        };
        for _ in 0..3 {
            frame(&mut app, vec![]);
        }
        // put the cursor at the end, as if the person had just typed
        frame(&mut app, vec![egui::Event::Key { key: egui::Key::End, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }]);
        frame(&mut app, vec![]);
        let ac = app.ac.as_ref().expect("suggestions are showing");
        assert_eq!(ac.items[0].label, "Illness Duration");
        frame(&mut app, vec![egui::Event::Key { key: egui::Key::Tab, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }]);
        frame(&mut app, vec![]);
        assert_eq!(app.model.var(id).unwrap().equation, "Contact Rate * SMOOTH(Infected, Illness Duration");
    }

    /// Draws the editor with its suggestion list into a picture (run with --ignored to look at it).
    #[test]
    #[ignore]
    fn picture_of_autocomplete() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None);
        let id = app.model.by_name("infection rate").unwrap().id;
        app.select_one(Some(Sel::Var(id)));
        app.model.var_mut(id).unwrap().equation = "Contact Rate * SMOOTH(Infected, Ill".into();
        app.canvas.focus_equation = true;
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, vec2(420.0, 760.0));
        let mut shapes = Vec::new();
        for i in 0..6 {
            let events = if i == 3 { vec![egui::Event::Key { key: egui::Key::End, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }] } else { vec![] };
            let input = egui::RawInput { events, focused: true, screen_rect: Some(screen), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                egui::Frame::new().fill(app.pal.panel).inner_margin(egui::Margin::same(14)).show(ui, |ui| app.inspector(ui));
            });
            out.textures_delta.clear();
            shapes = out.shapes;
        }
        let img = crate::export::rasterize(&shapes, screen, 2.0);
        std::fs::write("/tmp/fc_autocomplete.png", crate::export::png_bytes(&img, 2.0).unwrap()).unwrap();
    }
}
