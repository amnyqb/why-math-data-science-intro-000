//! The "Customize chart" window: type, variables, titles, axes, legend and colours of the
//! Trends graph and of any dashboard graph.

use egui::{Color32, RichText, Sense, Stroke, vec2};

use crate::app::App;
use crate::model::{ChartKind, ChartStyle, Kind, LegendPos, WidgetKind};

/// Which saved chart is being customized.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChartTarget {
    Trends,
    Card(u64),
}

/// What the window edits: kind, variables, style, and (for cards) the card's title.
#[derive(Clone, PartialEq)]
struct Spec {
    kind: ChartKind,
    vars: Vec<String>,
    style: ChartStyle,
    card_title: Option<String>,
}

impl App {
    pub fn customize_chart(&mut self, t: ChartTarget) {
        self.chart_editor = Some((t, false));
    }

    fn spec_of(&self, t: ChartTarget) -> Option<Spec> {
        match t {
            ChartTarget::Trends => Some(Spec { kind: self.trend_chart, vars: self.plotted.clone(), style: self.model.trend_style.clone(), card_title: None }),
            ChartTarget::Card(id) => self.model.sheets.iter().flat_map(|s| s.widgets.iter()).find(|w| w.id == id && w.kind == WidgetKind::Graph).map(|w| Spec { kind: w.chart, vars: w.vars.clone(), style: w.style.clone(), card_title: Some(w.title.clone()) }),
        }
    }

    fn set_spec(&mut self, t: ChartTarget, s: Spec) {
        match t {
            ChartTarget::Trends => {
                self.trend_chart = s.kind;
                self.plotted = s.vars;
                self.model.trend_style = s.style;
            }
            ChartTarget::Card(id) => {
                if let Some(w) = self.model.sheets.iter_mut().flat_map(|s| s.widgets.iter_mut()).find(|w| w.id == id) {
                    w.chart = s.kind;
                    w.vars = s.vars;
                    w.style = s.style;
                    if let Some(t) = s.card_title {
                        w.title = t;
                    }
                }
            }
        }
    }

    pub fn chart_editor_window(&mut self, ctx: &egui::Context) {
        let Some((target, mut touched)) = self.chart_editor else { return };
        let Some(orig) = self.spec_of(target) else {
            self.chart_editor = None;
            return;
        };
        let pal = self.pal;
        let mut s = orig.clone();
        let mut open = true;
        let mut reset = false;
        let set_items: std::collections::HashMap<String, Vec<String>> = self.model.vars.iter().filter(|v| v.set.is_some()).map(|v| (v.name.clone(), self.model.items_of(v))).collect();
        let vars_all: Vec<(String, Kind)> = self.model.vars.iter().filter(|v| !(v.kind == Kind::Lookup && v.equation.trim().is_empty())).map(|v| (v.name.clone(), v.kind)).collect();
        let title = match target {
            ChartTarget::Trends => "Customize chart · Trends".to_string(),
            ChartTarget::Card(_) => format!("Customize chart · {}", s.card_title.clone().unwrap_or_default()),
        };
        egui::Window::new(title).open(&mut open).collapsible(false).resizable(true).default_width(520.0).default_pos(ctx.content_rect().right_top() + vec2(-560.0, 90.0)).show(ctx, |ui| {
            egui::ScrollArea::vertical().max_height(ctx.content_rect().height() - 180.0).show(ui, |ui| {
                let head = |ui: &mut egui::Ui, t: &str| {
                    ui.add_space(8.0);
                    ui.label(RichText::new(t.to_uppercase()).small().strong().color(pal.muted));
                };
                head(ui, "Chart");
                egui::Grid::new("cs-chart").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Type");
                    egui::ComboBox::from_id_salt("cs-kind").width(260.0).selected_text(s.kind.label()).show_ui(ui, |ui| {
                        for k in ChartKind::ALL {
                            ui.selectable_value(&mut s.kind, k, k.label());
                        }
                    });
                    ui.end_row();
                    if let Some(t) = s.card_title.as_mut() {
                        ui.label("Card title");
                        ui.add(egui::TextEdit::singleline(t).desired_width(260.0));
                        ui.end_row();
                    }
                    ui.label("Title");
                    ui.add(egui::TextEdit::singleline(&mut s.style.title).desired_width(260.0).hint_text("none"));
                    ui.end_row();
                    ui.label("Subtitle");
                    ui.add(egui::TextEdit::singleline(&mut s.style.subtitle).desired_width(260.0).hint_text("none"));
                    ui.end_row();
                });

                head(ui, "Lines");
                let mut remove = None;
                for (i, n) in s.vars.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let c = s.style.color_for(n).map(|[r, g, b]| Color32::from_rgb(r, g, b)).unwrap_or(pal.series(i));
                        ui.menu_button(RichText::new("●").size(18.0).color(c), |ui| {
                            ui.label(RichText::new("Colour").small().color(pal.muted));
                            ui.horizontal(|ui| {
                                for k in 0..8 {
                                    let cc = pal.series(k);
                                    let (r, resp) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
                                    ui.painter().circle(r.center(), 8.0, cc, Stroke::new(if cc == c { 2.0 } else { 0.0 }, pal.text));
                                    if resp.clicked() {
                                        s.style.colors.retain(|(v, _)| v != n);
                                        s.style.colors.push((n.clone(), [cc.r(), cc.g(), cc.b()]));
                                        ui.close();
                                    }
                                }
                            });
                            if ui.button("Automatic colour").clicked() {
                                s.style.colors.retain(|(v, _)| v != n);
                                ui.close();
                            }
                        });
                        ui.label(RichText::new(n).size(14.0));
                        if crate::icons::tool(ui, crate::icons::Ic::Close, "Remove from the chart", pal.muted, pal.accent).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    s.vars.remove(i);
                }
                ui.menu_button(RichText::new("+ Add a variable").color(pal.accent), |ui| {
                    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        for (n, k) in &vars_all {
                            if !s.vars.contains(n) && ui.button(RichText::new(n).color(pal.kind(*k))).clicked() {
                                s.vars.push(n.clone());
                                ui.close();
                            }
                            if let Some(items) = set_items.get(n) {
                                ui.menu_button(RichText::new(format!("      ▸ one at a time ({})", items.len())).small().color(pal.muted), |ui| {
                                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                                        for it in items {
                                            let name = format!("{n}[{it}]");
                                            if !s.vars.contains(&name) && ui.button(RichText::new(it).color(pal.kind(*k))).clicked() {
                                                s.vars.push(name);
                                                ui.close();
                                            }
                                        }
                                    });
                                });
                            }
                        }
                    });
                });
                ui.checkbox(&mut s.style.compare, "Show scenarios and kept runs next to the current model");

                head(ui, "Axes");
                let units = self.run.as_ref().map(|r| r.time_units.clone()).unwrap_or_default();
                egui::Grid::new("cs-axes").num_columns(4).spacing([10.0, 8.0]).show(ui, |ui| {
                    ui.label("Horizontal title");
                    ui.add(egui::TextEdit::singleline(&mut s.style.x_label).desired_width(200.0).hint_text(format!("automatic: Time ({units})")));
                    ui.end_row();
                    ui.label("Vertical title");
                    ui.add(egui::TextEdit::singleline(&mut s.style.y_label).desired_width(200.0).hint_text("automatic: the variables' units"));
                    ui.end_row();
                    for (label, lo, hi) in [("Horizontal range", &mut s.style.x_min, &mut s.style.x_max), ("Vertical range", &mut s.style.y_min, &mut s.style.y_max)] {
                        ui.label(label);
                        ui.horizontal(|ui| {
                            for (txt, v) in [("from", lo), ("to", hi)] {
                                let mut auto = v.is_none();
                                if ui.checkbox(&mut auto, RichText::new(format!("{txt} auto")).small()).changed() {
                                    *v = if auto { None } else { Some(0.0) };
                                }
                                if let Some(x) = v.as_mut() {
                                    ui.add(egui::DragValue::new(x).speed(0.1));
                                }
                            }
                        });
                        ui.end_row();
                    }
                });

                head(ui, "Look");
                egui::Grid::new("cs-look").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                    ui.label("Legend");
                    egui::ComboBox::from_id_salt("cs-legend").selected_text(s.style.legend.label()).show_ui(ui, |ui| {
                        for l in LegendPos::ALL {
                            ui.selectable_value(&mut s.style.legend, l, l.label());
                        }
                    });
                    ui.end_row();
                    ui.label("Line width");
                    ui.add(egui::Slider::new(&mut s.style.line_width, 0.5..=3.0).fixed_decimals(1).suffix("×"));
                    ui.end_row();
                    ui.label("");
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut s.style.grid, "Grid");
                        ui.checkbox(&mut s.style.markers, "Point markers");
                        ui.checkbox(&mut s.style.time_marker, "Current-time line");
                    });
                    ui.end_row();
                });
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset to defaults").clicked() {
                        reset = true;
                    }
                    ui.label(RichText::new("Changes show on the chart right away.").small().color(pal.muted));
                });
            });
        });
        if reset {
            s.style = ChartStyle::default();
        }
        if s != orig {
            if !touched {
                self.checkpoint();
                touched = true;
            }
            self.set_spec(target, s);
        }
        self.chart_editor = if open { Some((target, touched)) } else { None };
    }
}
