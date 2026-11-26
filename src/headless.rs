//! Draw sheets and charts with no window: the same `App` the desktop program runs, fed an
//! `egui::Context` and a fake `RawInput`. The shapes it paints are the shapes the app's own PNG
//! export rasterizes, so a figure made here is the picture the app shows (apart from anti-aliasing).
//!
//! Why this and not a second copy of the drawing code in `core/`: `canvas.rs` and `cld.rs` are
//! about 3 000 lines of geometry (pipes, valves, clouds, curved arrows, signs, markers) written
//! against egui painters. egui's `Context` itself needs no display (no GUI backend is linked in the
//! headless build, see the `gui` feature of this crate), so reusing it keeps one set of drawing
//! rules and costs only the egui crates, which are pure Rust. A second copy would drift.

use egui::epaint::{ClippedShape, TextShape};
use egui::{Color32, Context, FontId, Pos2, RawInput, Rect, Shape, vec2};
use dynamist_core::figstyle::FigStyle;
use dynamist_core::figure::{pick_sheet, sheet_names};
use dynamist_core::model::{ChartKind, ChartStyle, Model, SheetKind};

use crate::app::{App, HEADLESS};
use crate::export::{self, ChartRef, ExportWhat};
use crate::svg::{Svg, to_svg};

/// The app is drawn as on a retina display; text positions are laid out on a half-point grid.
pub const PIXELS_PER_POINT: f32 = 2.0;
/// Space kept around a figure, in points (the app's PNG export keeps the same).
pub const PAD: f32 = 10.0;

/// One drawn figure: the painted shapes, the area they are in, and the sheet's caption and source.
pub struct Figure {
    shapes: Vec<ClippedShape>,
    area: Rect,
    pub title: String,
}

pub struct Headless {
    ctx: Context,
    app: App,
    frame: u32,
}

/// What a chart shows.
#[derive(Clone, Debug)]
pub struct ChartSpec {
    pub vars: Vec<String>,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub kind: ChartKind,
    pub title: String,
    pub y_label: String,
    /// None: only the model's own run. Some(names): also those scenarios (empty = every enabled one).
    pub scenarios: Option<Vec<String>>,
    /// NAME=VALUE changes applied before the run.
    pub set: Vec<(String, String)>,
}

impl Default for ChartSpec {
    fn default() -> Self {
        ChartSpec { vars: Vec::new(), from: None, to: None, kind: ChartKind::Line, title: String::new(), y_label: String::new(), scenarios: None, set: Vec::new() }
    }
}

impl Headless {
    pub fn new(mut model: Model, style: Option<&FigStyle>) -> Headless {
        HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
        crate::canvas::set_raw_pipes(true);
        let ctx = Context::default();
        ctx.set_pixels_per_point(PIXELS_PER_POINT);
        let mut app = App::new(&ctx, None);
        app.animate = false;
        app.show_minimap = false;
        app.export_style = style.cloned();
        model.ensure_sheets();
        app.finish_open(model);
        let mut h = Headless { ctx, app, frame: 0 };
        h.frames(3);
        h
    }

    /// Bring the sheet of this name to the front of the window.
    pub fn show_sheet(&mut self, name: &str) -> bool {
        match self.app.model.sheets.iter().find(|s| s.name == name).map(|s| s.id) {
            Some(id) => {
                self.app.sheet = id;
                true
            }
            None => false,
        }
    }

    pub fn model(&self) -> &Model {
        &self.app.model
    }

    /// Run whole frames of the window so the sheet measures its words and boxes.
    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame += 1;
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))),
                time: Some(self.frame as f64 * 0.05),
                focused: true,
                ..Default::default()
            };
            let app = &mut self.app;
            let mut out = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
            out.textures_delta.clear();
        }
    }

    /// The whole window (ribbon, panels, sheet) as a PNG: for looking at the interface without a display.
    /// `tab` is a ribbon tab name ("Home", "Insert", …); `size` is in points.
    pub fn window_png(&mut self, tab: Option<&str>, size: (f32, f32), scale: f32) -> Result<Vec<u8>, String> {
        crate::canvas::set_raw_pipes(false);
        // "shape" and "control" name the contextual tab and what to select for it
        let want = tab.map(|t| t.to_ascii_lowercase()).filter(|t| t == "shape" || t == "control");
        let tab = if want.is_some() { Some("Text") } else { tab };
        if let Some(t) = tab {
            let found = crate::ribbon::RibbonTab::ALL.into_iter().chain([crate::ribbon::RibbonTab::Format]).find(|x| x.label().eq_ignore_ascii_case(t));
            self.app.ribbon_tab = found.ok_or_else(|| format!("no ribbon tab called “{t}”"))?;
            self.app.ribbon_open = true;
        }
        self.app.sel = None;
        if self.app.ribbon_tab == crate::ribbon::RibbonTab::Format {
            let m = &self.app.model;
            let first_note = m.notes.first().map(|n| crate::app::Sel::Note(n.id));
            let first_shape = m.shapes.first().map(|n| crate::app::Sel::Shape(n.id));
            let first_ctl = m.controls.first().map(|n| crate::app::Sel::Control(n.id));
            self.app.sel = match want.as_deref() {
                Some("shape") => first_shape,
                Some("control") => first_ctl,
                _ => first_note.or(first_shape).or(first_ctl),
            };
        }
        self.frames(2);
        let area = Rect::from_min_size(Pos2::ZERO, vec2(size.0, size.1));
        self.frame += 1;
        let input = RawInput { screen_rect: Some(area), time: Some(self.frame as f64 * 0.05), focused: true, ..Default::default() };
        let app = &mut self.app;
        let mut out = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        out.textures_delta.clear();
        let bg = ClippedShape { clip_rect: area, shape: Shape::rect_filled(area, 0.0, self.app.pal.canvas), transform_after_tessellation: Default::default() };
        let mut shapes = vec![bg];
        shapes.extend(out.shapes);
        let img = export::rasterize(&shapes, area, scale);
        export::png_bytes(&img, scale)
    }

    /// Draw a sheet (stock and flow or causal loop). `name` None = the first diagram.
    pub fn sheet(&mut self, name: Option<&str>) -> Result<Figure, String> {
        let m = &self.app.model;
        let s = pick_sheet(m, name, &[SheetKind::Diagram])?;
        if !matches!(s.kind, SheetKind::Diagram | SheetKind::CausalLoop) {
            return Err(format!("“{}” is a {} sheet; render draws diagrams and causal loop diagrams (use chart for results)", s.name, s.kind.label()));
        }
        let (id, title) = (s.id, s.name.clone());
        crate::canvas::set_raw_pipes(true);
        self.app.sheet = id;
        self.app.sel = None;
        self.app.multi.clear();
        self.frames(3);
        let (shapes, area) = self.app.render_shapes(&self.ctx, ExportWhat::Sheet).ok_or_else(|| format!("“{title}” has nothing to draw yet"))?;
        Ok(Figure { shapes, area, title })
    }

    /// Draw variables over time as a chart.
    pub fn chart(&mut self, spec: &ChartSpec) -> Result<Figure, String> {
        if spec.vars.is_empty() {
            return Err("chart needs --vars A,B".into());
        }
        for (k, v) in &spec.set {
            if !self.app.model.apply_change(k, v) {
                return Err(format!("--set {k}={v}: no such variable"));
            }
        }
        if let Some(names) = &spec.scenarios {
            for sc in self.app.model.scenarios.iter_mut() {
                sc.enabled = names.is_empty() || names.iter().any(|n| dynamist_core::expr::normalize(n) == dynamist_core::expr::normalize(&sc.name));
            }
            for n in names {
                if !self.app.model.scenarios.iter().any(|s| dynamist_core::expr::normalize(&s.name) == dynamist_core::expr::normalize(n)) {
                    return Err(format!("no scenario called “{n}” (scenarios: {})", self.app.model.scenarios.iter().map(|s| format!("“{}”", s.name)).collect::<Vec<_>>().join(", ")));
                }
            }
        }
        if !spec.set.is_empty() || spec.scenarios.is_some() {
            self.app.changed();
            self.app.simulate();
        }
        let run = match &self.app.run {
            Some(r) => r.clone(),
            None => {
                let why = self.app.run_error.clone().unwrap_or_else(|| self.app.problems.iter().map(|p| p.message.clone()).collect::<Vec<_>>().join("; "));
                return Err(format!("the model does not run: {why}"));
            }
        };
        let mut style = ChartStyle { title: spec.title.clone(), y_label: spec.y_label.clone(), compare: spec.scenarios.is_some(), time_marker: false, ..Default::default() };
        let (t0, t1) = (run.times.first().copied().unwrap_or(0.0), run.times.last().copied().unwrap_or(0.0));
        let (from, to) = (spec.from.unwrap_or(t0), spec.to.unwrap_or(t1));
        if from >= to {
            return Err(format!("--from {from} must be before --to {to}"));
        }
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for v in &spec.vars {
            let ys = run.get(v).ok_or_else(|| format!("no variable called “{v}” in the results"))?;
            for (t, y) in run.times.iter().zip(ys) {
                if *t >= from && *t <= to && y.is_finite() {
                    lo = lo.min(*y);
                    hi = hi.max(*y);
                }
            }
        }
        if spec.from.is_some() || spec.to.is_some() {
            style.x_min = Some(from);
            style.x_max = Some(to);
            if lo.is_finite() && hi.is_finite() {
                let pad = ((hi - lo) * 0.06).max(1e-9);
                // a quantity that starts near zero is shown from zero, so a short window does not exaggerate it
                let floor = if lo >= 0.0 && lo < 0.35 * hi { 0.0 } else { lo - pad };
                style.y_min = Some(floor);
                style.y_max = Some(hi + pad);
            }
        }
        let title = spec.vars.join(", ");
        self.app.export_chart = Some(ChartRef::Styled { kind: spec.kind, vars: spec.vars.clone(), style, target: None });
        self.frames(1);
        let (shapes, area) = self.app.render_shapes(&self.ctx, ExportWhat::Chart).ok_or("the chart has nothing to draw")?;
        Ok(Figure { shapes, area, title })
    }

    /// The caption and source of a sheet (empty strings if none).
    pub fn caption_of(&self, name: Option<&str>) -> (String, String) {
        match pick_sheet(&self.app.model, name, &[SheetKind::Diagram]) {
            Ok(s) => (s.caption.clone(), s.source.clone()),
            Err(_) => Default::default(),
        }
    }

    pub fn sheet_list(&self) -> String {
        sheet_names(&self.app.model)
    }

    /// Add the caption (and source) under a figure, laid out with the same fonts as the rest.
    pub fn add_caption(&mut self, fig: &mut Figure, caption: &str, source: &str, style: &FigStyle) {
        if caption.trim().is_empty() && source.trim().is_empty() {
            return;
        }
        let Some(ink) = fig.ink(2.0) else { return };
        let pal = {
            let mut p = crate::theme::Palette::light();
            p.set_colors(&style.colors);
            p
        };
        let size = style.caption_size.unwrap_or(13.0);
        let width = ink.width().max(320.0);
        let (x, y0) = (ink.min.x, ink.max.y + 6.0);
        let mut y_end = y0;
        let mut made: Vec<ClippedShape> = Vec::new();
        let (cap, src) = (caption.trim().to_string(), source.trim().to_string());
        let mut out = self.ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1000.0))), time: Some(self.frame as f64 * 0.05 + 1.0), focused: true, ..Default::default() }, |ui| {
            let painter = ui.painter();
            made.clear();
            let mut y = y0;
            for (text, font, color) in [(cap.clone(), size, pal.text), (if src.is_empty() { src.clone() } else { format!("Source: {src}") }, size - 2.0, pal.muted)] {
                if text.is_empty() {
                    continue;
                }
                let g = painter.layout(text, FontId::proportional(font), color, width);
                let h = g.size().y;
                made.push(ClippedShape { transform_after_tessellation: Default::default(), clip_rect: Rect::EVERYTHING, shape: Shape::Text(TextShape::new(Pos2::new(x, y), g, Color32::PLACEHOLDER)) });
                y += h + 3.0;
            }
            y_end = y;
        });
        out.textures_delta.clear();
        fig.area = fig.area.union(Rect::from_min_max(Pos2::new(ink.min.x, ink.min.y), Pos2::new(ink.min.x + width, y_end + 6.0)));
        fig.shapes.extend(made);
    }
}

impl Figure {
    /// The box that holds everything drawn, plus `PAD`, in the shapes' coordinates. `scale` is the
    /// resolution of the measuring pass.
    fn ink(&self, scale: f32) -> Option<Rect> {
        let img = export::rasterize(&self.shapes, self.area, scale);
        let (x0, y0, x1, y1) = img.ink_box(0)?;
        Some(Rect::from_min_max(
            self.area.min + vec2(x0 as f32 / scale, y0 as f32 / scale),
            self.area.min + vec2((x1 + 1) as f32 / scale, (y1 + 1) as f32 / scale),
        ))
    }

    /// The figure as SVG text.
    pub fn svg(&self, style: &FigStyle) -> Result<Svg, String> {
        let ink = self.ink(4.0).ok_or("nothing was drawn")?;
        let view = ink.expand(PAD);
        Ok(to_svg(&self.shapes, view, style, &self.title))
    }

    /// The figure as a PNG at `scale` pixels per point (the app's own rasterizer; transparent).
    pub fn png(&self, scale: f32) -> Result<Vec<u8>, String> {
        let img = export::rasterize(&self.shapes, self.area, scale).trimmed((PAD * scale) as usize);
        export::png_bytes(&img, scale)
    }
}
