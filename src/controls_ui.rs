//! Input devices on a diagram: a slider, knob, switch or number input tied to a constant. Operating
//! one writes the number into the constant's equation and the model runs again, like the key levers.

use egui::{Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};
use dynamist_core::model::{Control, ControlKind, Kind};

use crate::app::{App, Sel, Tool};

/// Where the parts of a control are, in diagram units.
pub struct Parts {
    pub card: Rect,
    /// The track, dial, pill or value area: what is operated.
    pub main: Rect,
    pub minus: Option<Rect>,
    pub plus: Option<Rect>,
}

pub fn parts(c: &Control) -> Parts {
    let [w, h] = c.kind.size();
    let card = Rect::from_min_size(pos2(c.pos[0], c.pos[1]), vec2(w, h));
    match c.kind {
        ControlKind::Slider => {
            let track = Rect::from_min_max(pos2(card.left() + 14.0, card.bottom() - 19.0), pos2(card.right() - 14.0, card.bottom() - 15.0));
            Parts { card, main: track.expand2(vec2(8.0, 11.0)), minus: None, plus: None }
        }
        ControlKind::Knob => {
            let c0 = pos2(card.center().x, card.top() + 64.0);
            Parts { card, main: Rect::from_center_size(c0, vec2(68.0, 68.0)), minus: None, plus: None }
        }
        ControlKind::Switch => {
            let pill = Rect::from_min_size(pos2(card.right() - 12.0 - 42.0, card.center().y - 11.0), vec2(42.0, 22.0));
            Parts { card, main: pill.expand(4.0), minus: None, plus: None }
        }
        ControlKind::Number => {
            let plus = Rect::from_min_size(pos2(card.right() - 12.0 - 26.0, card.bottom() - 12.0 - 22.0), vec2(26.0, 22.0));
            let minus = plus.translate(vec2(-30.0, 0.0));
            let main = Rect::from_min_max(pos2(card.left() + 8.0, card.top() + 24.0), pos2(minus.left() - 6.0, card.bottom() - 4.0));
            Parts { card, main, minus: Some(minus), plus: Some(plus) }
        }
    }
}

/// What a control shows and sets right now.
#[derive(Clone, Debug)]
pub struct State {
    pub name: String,
    pub value: f64,
    pub lo: f64,
    pub hi: f64,
}

impl State {
    pub fn frac(&self) -> f32 {
        if self.hi > self.lo { (((self.value - self.lo) / (self.hi - self.lo)) as f32).clamp(0.0, 1.0) } else { 0.0 }
    }
    /// A step that suits the range: about a hundredth of it, as a power of ten.
    pub fn step(&self) -> f64 {
        let span = (self.hi - self.lo).abs().max(1e-9);
        10f64.powf((span / 100.0).log10().floor())
    }
    pub fn on(&self) -> bool {
        self.value > (self.lo + self.hi) / 2.0
    }
}

pub fn fmt_value(x: f64) -> String {
    let s = format!("{:.4}", x);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" { "0".into() } else { s.to_string() }
}

impl App {
    /// The constant a control sets, if it still exists and is a plain number.
    pub fn control_state(&self, c: &Control) -> Option<State> {
        let v = self.model.var(c.var)?;
        if v.kind != Kind::Constant || v.set.is_some() {
            return None;
        }
        let value = v.equation.trim().parse::<f64>().ok()?;
        let (lo, hi) = v.slider.map(|s| (s[0], s[1])).unwrap_or((0.0, (value.abs() * 2.0).max(1.0)));
        Some(State { name: v.name.clone(), value, lo, hi })
    }

    pub fn control_rect(&self, id: u64) -> Option<Rect> {
        self.model.controls.iter().find(|c| c.id == id).map(|c| parts(c).card)
    }

    /// Write a value into the constant (clamped to its range).
    pub fn set_control_value(&mut self, var: u64, x: f64) {
        let Some(st) = self.model.controls.iter().find(|c| c.var == var).and_then(|c| self.control_state(c)) else { return };
        let x = x.clamp(st.lo.min(st.hi), st.hi.max(st.lo));
        if let Some(v) = self.model.var_mut(var) {
            let text = fmt_value(x);
            if v.equation != text {
                v.equation = text;
                self.changed();
            }
        }
    }

    /// Operate the main part of a control at diagram point `d` (a click, or the start of a drag).
    pub fn operate_control(&mut self, id: u64, d: Pos2) {
        let Some(c) = self.model.controls.iter().find(|c| c.id == id).cloned() else { return };
        let Some(st) = self.control_state(&c) else { return };
        let p = parts(&c);
        match c.kind {
            ControlKind::Slider => {
                let t = ((d.x - (p.card.left() + 14.0)) / (p.card.width() - 28.0)).clamp(0.0, 1.0) as f64;
                let step = st.step();
                let x = ((st.lo + t * (st.hi - st.lo)) / step).round() * step;
                self.set_control_value(c.var, x);
            }
            ControlKind::Switch => {
                let x = if st.on() { st.lo } else { st.hi };
                self.set_control_value(c.var, x);
            }
            _ => {}
        }
    }

    /// Press − or + on a number input.
    pub fn step_control(&mut self, id: u64, up: bool) {
        let Some(c) = self.model.controls.iter().find(|c| c.id == id).cloned() else { return };
        let Some(st) = self.control_state(&c) else { return };
        let k = st.step() * 10.0;
        let x = ((st.value / k).round() + if up { 1.0 } else { -1.0 }) * k;
        self.set_control_value(c.var, x);
    }

    /// Dragging the main part: a knob goes with up and down, a number with left and right, a slider follows the pointer.
    pub fn drag_control(&mut self, id: u64, d: Pos2, delta: egui::Vec2) {
        let Some(c) = self.model.controls.iter().find(|c| c.id == id).cloned() else { return };
        let Some(st) = self.control_state(&c) else { return };
        let span = st.hi - st.lo;
        match c.kind {
            ControlKind::Slider => self.operate_control(id, d),
            ControlKind::Knob => self.set_control_value(c.var, st.value - delta.y as f64 / 150.0 * span),
            ControlKind::Number => self.set_control_value(c.var, st.value + delta.x as f64 / 200.0 * span),
            ControlKind::Switch => {}
        }
    }

    /// After a drag: round the value to a tidy step.
    pub fn settle_control(&mut self, id: u64) {
        let Some(c) = self.model.controls.iter().find(|c| c.id == id).cloned() else { return };
        let Some(st) = self.control_state(&c) else { return };
        let step = st.step();
        self.set_control_value(c.var, (st.value / step).round() * step);
    }

    /// Which part of which control is at `d`: (id, 0 body, 1 main, 2 minus, 3 plus). The last drawn wins.
    pub(crate) fn control_hit(&self, d: Pos2) -> Option<(u64, u8)> {
        let view = self.diagram_view();
        for c in self.model.controls.iter().rev().filter(|c| c.view == view) {
            let p = parts(c);
            if !p.card.contains(d) {
                continue;
            }
            if p.minus.is_some_and(|r| r.contains(d)) {
                return Some((c.id, 2));
            }
            if p.plus.is_some_and(|r| r.contains(d)) {
                return Some((c.id, 3));
            }
            if p.main.contains(d) && self.control_state(c).is_some() {
                return Some((c.id, 1));
            }
            return Some((c.id, 0));
        }
        None
    }

    /// Add a control where the pointer clicked, tied to the selected constant (or the first free one).
    pub(crate) fn create_control(&mut self, at: Pos2) {
        let kind = self.control_kind;
        let view = self.diagram_view();
        let free = |app: &App, v: &dynamist_core::model::Var| v.kind == Kind::Constant && v.set.is_none() && v.equation.trim().parse::<f64>().is_ok() && !app.model.controls.iter().any(|c| c.var == v.id);
        let picked = match self.sel {
            Some(Sel::Var(id)) => self.model.var(id).filter(|v| v.kind == Kind::Constant && v.set.is_none() && v.equation.trim().parse::<f64>().is_ok()).map(|v| v.id),
            _ => None,
        };
        let var = picked.or_else(|| self.model.vars.iter().find(|v| free(self, v)).map(|v| v.id)).unwrap_or(0);
        self.checkpoint();
        if let Some(v) = self.model.var_mut(var) {
            if v.slider.is_none() {
                let cur = v.equation.trim().parse::<f64>().unwrap_or(0.0);
                v.slider = Some([0.0, (cur.abs() * 2.0).max(1.0)]);
            }
        }
        let id = self.model.new_id();
        self.model.controls.push(Control::new(id, view, kind, var, [at.x, at.y]));
        self.select_one(Some(Sel::Control(id)));
        self.tool = Tool::Select;
        self.status = if var == 0 { "No constant to tie it to yet: add one, then pick it on the Control tab".into() } else { String::new() };
        self.changed();
    }

    /// Draw the controls of this sheet.
    pub(crate) fn draw_controls(&mut self, p: &egui::Painter, origin: Pos2) {
        let pal = self.pal;
        let z = self.zoom;
        let view = self.diagram_view();
        let controls: Vec<Control> = self.model.controls.iter().filter(|c| c.view == view && self.shown(c.id)).cloned().collect();
        let sc = |s: &App, r: Rect| Rect::from_min_max(s.to_screen(origin, r.min), s.to_screen(origin, r.max));
        for c in &controls {
            let pt = parts(c);
            let st = self.control_state(c);
            let selected = self.is_sel(Sel::Control(c.id));
            let card = sc(self, pt.card);
            p.rect_filled(card.translate(vec2(0.0, 2.0 * z)), CornerRadius::same((10.0 * z) as u8), Color32::from_black_alpha(if pal.dark { 60 } else { 14 }));
            p.rect_filled(card, CornerRadius::same((10.0 * z) as u8), pal.panel);
            p.rect_stroke(card, CornerRadius::same((10.0 * z) as u8), Stroke::new(if selected { 1.8 } else { 1.0 }, if selected { pal.accent } else { pal.border }), StrokeKind::Inside);
            let label = if !c.label.is_empty() { c.label.clone() } else { st.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "Pick a variable".into()) };
            let (txt, mute) = (if st.is_some() { pal.text } else { pal.muted }, pal.muted);
            let Some(st) = st else {
                p.text(card.center(), Align2::CENTER_CENTER, label, FontId::proportional(12.5 * z), mute);
                continue;
            };
            let value = fmt_value(st.value);
            let mono = FontId::monospace(12.5 * z);
            let accent = pal.accent;
            match c.kind {
                ControlKind::Slider => {
                    p.text(card.left_top() + vec2(12.0, 10.0) * z, Align2::LEFT_TOP, label, FontId::proportional(12.5 * z), txt);
                    p.text(card.right_top() + vec2(-12.0, 10.0) * z, Align2::RIGHT_TOP, value, mono, accent);
                    let tr = sc(self, Rect::from_min_max(pos2(pt.card.left() + 14.0, pt.card.bottom() - 19.0), pos2(pt.card.right() - 14.0, pt.card.bottom() - 15.0)));
                    p.rect_filled(tr, CornerRadius::same((2.0 * z) as u8), pal.border);
                    let x = tr.left() + tr.width() * st.frac();
                    p.rect_filled(Rect::from_min_max(tr.left_top(), pos2(x, tr.bottom())), CornerRadius::same((2.0 * z) as u8), accent);
                    p.circle_filled(pos2(x, tr.center().y), 7.0 * z, Color32::WHITE);
                    p.circle_stroke(pos2(x, tr.center().y), 7.0 * z, Stroke::new(2.0 * z, accent));
                }
                ControlKind::Knob => {
                    p.text(pos2(card.center().x, card.top() + 10.0 * z), Align2::CENTER_TOP, label, FontId::proportional(12.5 * z), txt);
                    let c0 = self.to_screen(origin, pos2(pt.card.center().x, pt.card.top() + 64.0));
                    let r = 26.0 * z;
                    // a 270 degree dial from lower left, round the top, to lower right
                    let (a0, sweep) = (135f32.to_radians(), 270f32.to_radians());
                    let arc = |frac: f32| -> Vec<Pos2> { (0..=48).map(|i| { let a = a0 + sweep * frac * i as f32 / 48.0; pos2(c0.x + r * a.cos(), c0.y + r * a.sin()) }).collect() };
                    p.add(egui::Shape::line(arc(1.0), Stroke::new(5.0 * z, pal.border)));
                    if st.frac() > 0.001 {
                        p.add(egui::Shape::line(arc(st.frac()), Stroke::new(5.0 * z, accent)));
                    }
                    let a = a0 + sweep * st.frac();
                    p.line_segment([c0, pos2(c0.x + (r - 6.0 * z) * a.cos(), c0.y + (r - 6.0 * z) * a.sin())], Stroke::new(2.5 * z, txt));
                    p.circle_filled(c0, 4.0 * z, txt);
                    p.text(pos2(card.center().x, card.bottom() - 8.0 * z), Align2::CENTER_BOTTOM, value, mono, accent);
                }
                ControlKind::Switch => {
                    p.text(pos2(card.left() + 12.0 * z, card.center().y), Align2::LEFT_CENTER, label, FontId::proportional(12.5 * z), txt);
                    let pill = sc(self, Rect::from_min_size(pos2(pt.card.right() - 12.0 - 42.0, pt.card.center().y - 11.0), vec2(42.0, 22.0)));
                    p.rect_filled(pill, CornerRadius::same((11.0 * z) as u8), if st.on() { accent } else { pal.border });
                    let kx = if st.on() { pill.right() - 11.0 * z } else { pill.left() + 11.0 * z };
                    p.circle_filled(pos2(kx, pill.center().y), 8.5 * z, Color32::WHITE);
                }
                ControlKind::Number => {
                    p.text(card.left_top() + vec2(12.0, 8.0) * z, Align2::LEFT_TOP, label, FontId::proportional(11.5 * z), mute);
                    p.text(card.left_bottom() + vec2(12.0, -9.0) * z, Align2::LEFT_BOTTOM, value, FontId::monospace(17.0 * z), accent);
                    for (r, sym) in [(pt.minus, "−"), (pt.plus, "+")] {
                        if let Some(r) = r {
                            let r = sc(self, r);
                            p.rect_filled(r, CornerRadius::same((6.0 * z) as u8), pal.surface);
                            p.rect_stroke(r, CornerRadius::same((6.0 * z) as u8), Stroke::new(1.0, pal.border), StrokeKind::Inside);
                            p.text(r.center(), Align2::CENTER_CENTER, sym, FontId::proportional(15.0 * z), txt);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::Headless;
    use dynamist_core::model::Model;

    fn app() -> (egui::Context, App) {
        crate::app::HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None);
        app.finish_open(Model::example());
        (ctx, app)
    }

    fn var_id(app: &App, name: &str) -> u64 {
        app.model.vars.iter().find(|v| v.name == name).unwrap().id
    }

    fn value(app: &App, name: &str) -> f64 {
        app.model.vars.iter().find(|v| v.name == name).unwrap().equation.parse().unwrap()
    }

    #[test]
    fn a_new_control_ties_to_the_selected_constant_and_gets_a_range() {
        let (_c, mut app) = app();
        let id = var_id(&app, "Illness Duration");
        app.model.var_mut(id).unwrap().slider = None;
        app.select_one(Some(Sel::Var(id)));
        app.control_kind = ControlKind::Knob;
        app.create_control(pos2(10.0, 10.0));
        let c = app.model.controls.last().unwrap();
        assert_eq!((c.kind, c.var), (ControlKind::Knob, id));
        // 5 is the value: the range reaches twice that
        assert_eq!(app.model.var(id).unwrap().slider, Some([0.0, 10.0]));
        // with nothing selected it takes a constant no control has yet
        app.select_one(None);
        app.create_control(pos2(300.0, 10.0));
        assert_ne!(app.model.controls.last().unwrap().var, id);
    }

    #[test]
    fn a_slider_sets_the_constant_and_stays_in_range() {
        let (_c, mut app) = app();
        let id = var_id(&app, "Contact Rate"); // slider range 0 to 15
        app.select_one(Some(Sel::Var(id)));
        app.create_control(pos2(0.0, 0.0));
        let c = app.model.controls.last().unwrap().clone();
        assert_eq!(c.var, id);
        let left = parts(&c).card.left() + 14.0;
        let width = parts(&c).card.width() - 28.0;
        app.operate_control(c.id, pos2(left + width * 0.5, 0.0));
        assert!((value(&app, "Contact Rate") - 7.5).abs() < 1e-9, "{}", value(&app, "Contact Rate"));
        app.operate_control(c.id, pos2(left + width * 5.0, 0.0)); // far past the end
        assert_eq!(value(&app, "Contact Rate"), 15.0);
        app.operate_control(c.id, pos2(left - 500.0, 0.0));
        assert_eq!(value(&app, "Contact Rate"), 0.0);
    }

    #[test]
    fn switch_steps_and_knob_each_do_their_job() {
        let (_c, mut app) = app();
        let id = var_id(&app, "Infectivity"); // 0.25 in 0 to 1
        app.select_one(Some(Sel::Var(id)));
        app.control_kind = ControlKind::Switch;
        app.create_control(pos2(0.0, 0.0));
        let sw = app.model.controls.last().unwrap().id;
        app.operate_control(sw, pos2(0.0, 0.0)); // 0.25 is below the middle: switch on
        assert_eq!(value(&app, "Infectivity"), 1.0);
        app.operate_control(sw, pos2(0.0, 0.0));
        assert_eq!(value(&app, "Infectivity"), 0.0);
        // a number steps by a tenth of the tidy step
        app.model.var_mut(id).unwrap().equation = "0.25".into();
        app.control_kind = ControlKind::Number;
        app.select_one(Some(Sel::Var(id)));
        app.create_control(pos2(0.0, 300.0));
        let num = app.model.controls.last().unwrap().id;
        app.step_control(num, true);
        let up = value(&app, "Infectivity");
        assert!(up > 0.25 && up <= 0.4, "{up}");
        app.step_control(num, false);
        app.step_control(num, false);
        assert!(value(&app, "Infectivity") < up);
        // dragging a knob up raises the value; it is tidied when the drag ends
        app.model.var_mut(id).unwrap().equation = "0.5".into();
        app.control_kind = ControlKind::Knob;
        app.select_one(Some(Sel::Var(id)));
        app.create_control(pos2(0.0, 600.0));
        let knob = app.model.controls.last().unwrap().id;
        app.drag_control(knob, pos2(0.0, 0.0), vec2(0.0, -30.0));
        assert!((value(&app, "Infectivity") - 0.7).abs() < 1e-9);
        app.drag_control(knob, pos2(0.0, 0.0), vec2(0.0, -0.4));
        app.settle_control(knob);
        assert!((value(&app, "Infectivity") - 0.7).abs() < 0.011 + 0.003);
    }

    #[test]
    fn what_is_under_the_pointer_and_what_happens_to_a_deleted_variable() {
        let (_c, mut app) = app();
        app.create_control(pos2(100.0, 100.0));
        let c = app.model.controls.last().unwrap().clone();
        let p = parts(&c);
        assert_eq!(app.control_hit(p.main.center()), Some((c.id, 1)));
        assert_eq!(app.control_hit(p.card.left_top() + vec2(3.0, 3.0)), Some((c.id, 0)));
        assert_eq!(app.control_hit(pos2(5000.0, 5000.0)), None);
        // the constant goes: the control stays, can be moved, but cannot be operated
        app.model.remove_var(c.var);
        assert!(app.control_state(&c).is_none());
        assert_eq!(app.control_hit(p.main.center()), Some((c.id, 0)));
    }

    #[test]
    fn controls_copy_move_delete_and_undo() {
        let (_c, mut app) = app();
        app.create_control(pos2(0.0, 0.0));
        let id = app.model.controls.last().unwrap().id;
        app.select_one(Some(Sel::Control(id)));
        app.move_item(Sel::Control(id), vec2(7.0, 9.0), false);
        assert_eq!(app.model.controls[0].pos, [7.0, 9.0]);
        assert!(app.copy(false));
        app.paste();
        assert_eq!(app.model.controls.len(), 2);
        app.delete_selection();
        assert_eq!(app.model.controls.len(), 1);
        app.undo();
        assert_eq!(app.model.controls.len(), 2);
    }

    #[test]
    fn a_control_reaches_the_svg_with_its_value() {
        let mut m = Model::example();
        let view = m.first_diagram();
        let cr = m.vars.iter().find(|v| v.name == "Contact Rate").unwrap().id;
        for (i, kind) in ControlKind::ALL.into_iter().enumerate() {
            let id = m.new_id();
            m.controls.push(Control::new(id, view, kind, cr, [40.0 + 200.0 * i as f32, 520.0]));
        }
        let back: Model = serde_json::from_str(&serde_json::to_string(&m).unwrap()).unwrap();
        assert_eq!(back.controls, m.controls);
        let mut h = Headless::new(m, None);
        let svg = h.sheet(None).unwrap().svg(&Default::default()).unwrap();
        assert!(svg.text.matches(">Contact Rate<").count() >= 4, "every control names its variable");
        assert!(svg.text.contains(">6<"), "the value is shown");
        assert_eq!(svg.skipped.callbacks + svg.skipped.meshes, 0);
    }
}
