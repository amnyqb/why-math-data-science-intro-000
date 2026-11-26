//! Painted shapes → SVG. The app draws a sheet with egui; `App::render_shapes` returns the shapes it
//! painted, and `export::rasterize` turns them into a PNG. This module turns the very same shapes into
//! an SVG, so the picture is the same one apart from anti-aliasing:
//!
//! * rectangles, circles, lines, paths and Bézier curves become the matching SVG elements;
//! * text stays text: one `<text>` per run, with an `x` for every letter so the layout egui
//!   measured is kept whatever font the viewer substitutes;
//! * nothing depends on the machine: numbers are printed with two decimals, ids are counted, and
//!   there is no timestamp, so the same shapes always give the same bytes.

use std::collections::HashMap;
use std::fmt::Write;

use ab_glyph::{Font, PxScale, ScaleFont};
use egui::epaint::{ClippedShape, ColorMode, Mesh, PathShape, StrokeKind, TextShape};
use egui::emath::TSTransform;
use egui::{Color32, FontFamily, Pos2, Rect, Shape, Vec2};
use dynamist_core::figstyle::FigStyle;

/// What the converter could not draw (it is left out, never guessed).
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Skipped {
    pub meshes: usize,
    pub bands: usize,
    pub callbacks: usize,
}

pub struct Svg {
    pub text: String,
    pub width: f32,
    pub height: f32,
    pub skipped: Skipped,
}

const DEFAULT_FONT: &str = "Ubuntu Light, Ubuntu, Helvetica, Arial, sans-serif";
const DEFAULT_MONO: &str = "Hack, Menlo, Consolas, monospace";

fn num(x: f32) -> String {
    let x = if x.is_finite() { x } else { 0.0 };
    let r = (x * 100.0).round() / 100.0;
    if r == 0.0 {
        return "0".into();
    }
    let mut s = format!("{r:.2}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

fn hex(c: Color32) -> String {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    format!("#{r:02x}{g:02x}{b:02x}")
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

struct W<'a> {
    out: String,
    defs: String,
    off: Vec2,
    style: &'a FigStyle,
    line_scale: f32,
    text_scale: f32,
    clips: HashMap<[i32; 4], usize>,
    blurs: usize,
    skipped: Skipped,
}

impl W<'_> {
    fn p(&self, p: Pos2) -> String {
        format!("{} {}", num(p.x - self.off.x), num(p.y - self.off.y))
    }
    fn xy(&self, p: Pos2) -> String {
        format!("{},{}", num(p.x - self.off.x), num(p.y - self.off.y))
    }
    fn fill(&self, c: Color32) -> String {
        if c.a() == 0 {
            return " fill=\"none\"".into();
        }
        let mut s = format!(" fill=\"{}\"", hex(c));
        if c.a() < 255 {
            let _ = write!(s, " fill-opacity=\"{}\"", num(c.a() as f32 / 255.0));
        }
        s
    }
    fn stroke(&self, w: f32, c: Color32) -> String {
        if w <= 0.0 || c.a() == 0 {
            return String::new();
        }
        let mut s = format!(" stroke=\"{}\" stroke-width=\"{}\"", hex(c), num(w * self.line_scale));
        if c.a() < 255 {
            let _ = write!(s, " stroke-opacity=\"{}\"", num(c.a() as f32 / 255.0));
        }
        s
    }
    fn path_stroke(&self, s: &egui::epaint::PathStroke) -> String {
        match &s.color {
            ColorMode::Solid(c) => self.stroke(s.width, *c),
            ColorMode::UV(_) => String::new(),
        }
    }

    fn clip_id(&mut self, clip: Rect) -> usize {
        let key = [clip.min.x as i32, clip.min.y as i32, clip.max.x as i32, clip.max.y as i32];
        if let Some(i) = self.clips.get(&key) {
            return *i;
        }
        let i = self.clips.len() + 1;
        self.clips.insert(key, i);
        let _ = writeln!(
            self.defs,
            "<clipPath id=\"c{i}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath>",
            num(clip.min.x - self.off.x),
            num(clip.min.y - self.off.y),
            num(clip.width()),
            num(clip.height())
        );
        i
    }

    fn rect_el(&mut self, r: &egui::epaint::RectShape) {
        let cr = r.corner_radius;
        let (nw, ne, se, sw) = (cr.nw as f32, cr.ne as f32, cr.se as f32, cr.sw as f32);
        let rot = if r.angle != 0.0 {
            let c = r.rect.center();
            format!(" transform=\"rotate({} {} {})\"", num(r.angle.to_degrees()), num(c.x - self.off.x), num(c.y - self.off.y))
        } else {
            String::new()
        };
        let filter = if r.blur_width > 0.0 {
            self.blurs += 1;
            let _ = writeln!(self.defs, "<filter id=\"b{}\" x=\"-50%\" y=\"-50%\" width=\"200%\" height=\"200%\"><feGaussianBlur stdDeviation=\"{}\"/></filter>", self.blurs, num(r.blur_width / 2.0));
            format!(" filter=\"url(#b{})\"", self.blurs)
        } else {
            String::new()
        };
        let shape = |w: &Self, rect: Rect, radii: [f32; 4], extra: &str| -> String {
            let (x, y, wd, ht) = (num(rect.min.x - w.off.x), num(rect.min.y - w.off.y), num(rect.width()), num(rect.height()));
            if radii.iter().all(|q| (*q - radii[0]).abs() < 1e-3) {
                if radii[0] > 0.0 {
                    format!("<rect x=\"{x}\" y=\"{y}\" width=\"{wd}\" height=\"{ht}\" rx=\"{}\"{extra}{rot}{filter}/>", num(radii[0]))
                } else {
                    format!("<rect x=\"{x}\" y=\"{y}\" width=\"{wd}\" height=\"{ht}\"{extra}{rot}{filter}/>")
                }
            } else {
                let (x0, y0, x1, y1) = (rect.min.x - w.off.x, rect.min.y - w.off.y, rect.max.x - w.off.x, rect.max.y - w.off.y);
                let [a, b, c, d] = radii; // nw ne se sw
                format!(
                    "<path d=\"M{} {}H{}A{} {} 0 0 1 {} {}V{}A{} {} 0 0 1 {} {}H{}A{} {} 0 0 1 {} {}V{}A{} {} 0 0 1 {} {}Z\"{extra}{rot}{filter}/>",
                    num(x0 + a), num(y0), num(x1 - b), num(b), num(b), num(x1), num(y0 + b), num(y1 - c), num(c), num(c), num(x1 - c), num(y1),
                    num(x0 + d), num(d), num(d), num(x0), num(y1 - d), num(y0 + a), num(a), num(a), num(x0 + a), num(y0)
                )
            }
        };
        if r.fill.a() > 0 {
            let f = self.fill(r.fill);
            let el = shape(self, r.rect, [nw, ne, se, sw], &f);
            self.out.push_str(&el);
            self.out.push('\n');
        }
        let sw_ = r.stroke.width;
        if sw_ > 0.0 && r.stroke.color.a() > 0 {
            let half = sw_ / 2.0;
            let (rect, grow) = match r.stroke_kind {
                StrokeKind::Inside => (r.rect.shrink(half), -half),
                StrokeKind::Middle => (r.rect, 0.0),
                StrokeKind::Outside => (r.rect.expand(half), half),
            };
            let radii = [(nw + grow).max(0.0), (ne + grow).max(0.0), (se + grow).max(0.0), (sw + grow).max(0.0)];
            let st = format!(" fill=\"none\"{}", self.stroke(sw_, r.stroke.color));
            let el = shape(self, rect, radii, &st);
            self.out.push_str(&el);
            self.out.push('\n');
        }
    }

    fn poly(&mut self, points: &[Pos2], closed: bool, fill: Color32, stroke: String) {
        if points.len() < 2 {
            return;
        }
        let pts = points.iter().map(|p| self.xy(*p)).collect::<Vec<_>>().join(" ");
        let tag = if closed { "polygon" } else { "polyline" };
        let f = if closed && fill.a() > 0 { self.fill(fill) } else { " fill=\"none\"".into() };
        let _ = writeln!(self.out, "<{tag} points=\"{pts}\"{f}{stroke} stroke-linejoin=\"round\"/>");
    }

    fn shape(&mut self, s: &Shape) {
        match s {
            Shape::Noop => {}
            Shape::Vec(v) => v.iter().for_each(|x| self.shape(x)),
            Shape::Circle(c) => {
                let st = self.stroke(c.stroke.width, c.stroke.color);
                let f = self.fill(c.fill);
                let _ = writeln!(self.out, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"{f}{st}/>", num(c.center.x - self.off.x), num(c.center.y - self.off.y), num(c.radius));
            }
            Shape::Ellipse(e) => {
                let st = self.stroke(e.stroke.width, e.stroke.color);
                let f = self.fill(e.fill);
                let rot = if e.angle != 0.0 { format!(" transform=\"rotate({} {} {})\"", num(e.angle.to_degrees()), num(e.center.x - self.off.x), num(e.center.y - self.off.y)) } else { String::new() };
                let _ = writeln!(self.out, "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\"{f}{st}{rot}/>", num(e.center.x - self.off.x), num(e.center.y - self.off.y), num(e.radius.x), num(e.radius.y));
            }
            Shape::LineSegment { points, stroke } => {
                let st = self.stroke(stroke.width, stroke.color);
                if st.is_empty() {
                    return;
                }
                let _ = writeln!(self.out, "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"{st}/>", num(points[0].x - self.off.x), num(points[0].y - self.off.y), num(points[1].x - self.off.x), num(points[1].y - self.off.y));
            }
            Shape::Path(PathShape { points, closed, fill, stroke }) => {
                let st = self.path_stroke(stroke);
                self.poly(points, *closed, *fill, st);
            }
            Shape::Rect(r) => self.rect_el(r),
            Shape::QuadraticBezier(b) => {
                let st = self.path_stroke(&b.stroke);
                let f = if b.closed && b.fill.a() > 0 { self.fill(b.fill) } else { " fill=\"none\"".into() };
                let z = if b.closed { "Z" } else { "" };
                let _ = writeln!(self.out, "<path d=\"M{}Q{} {}{z}\"{f}{st}/>", self.p(b.points[0]), self.p(b.points[1]), self.p(b.points[2]));
            }
            Shape::CubicBezier(b) => {
                let st = self.path_stroke(&b.stroke);
                let f = if b.closed && b.fill.a() > 0 { self.fill(b.fill) } else { " fill=\"none\"".into() };
                let z = if b.closed { "Z" } else { "" };
                let _ = writeln!(self.out, "<path d=\"M{}C{} {} {}{z}\"{f}{st}/>", self.p(b.points[0]), self.p(b.points[1]), self.p(b.points[2]), self.p(b.points[3]));
            }
            Shape::Text(t) => self.text(t),
            Shape::Mesh(m) => self.mesh(m),
            Shape::Band(_) => self.skipped.bands += 1,
            Shape::Callback(cb) => match cb.callback.downcast_ref::<crate::pictures::PicCallback>() {
                Some(pic) => {
                    let r = cb.rect;
                    let href = dynamist_core::b64::encode(&pic.png);
                    let _ = writeln!(self.out, "<image x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" href=\"data:image/png;base64,{href}\"/>", num(r.min.x - self.off.x), num(r.min.y - self.off.y), num(r.width()), num(r.height()));
                }
                None => self.skipped.callbacks += 1,
            },
        }
    }

    /// Coloured triangles (gradients and the like). Textured meshes (images, glyph atlas) are skipped.
    fn mesh(&mut self, m: &Mesh) {
        if m.texture_id != egui::TextureId::default() && !m.is_empty() {
            self.skipped.meshes += 1;
            return;
        }
        for tri in m.indices.chunks_exact(3) {
            let v: Vec<_> = tri.iter().map(|i| m.vertices[*i as usize]).collect();
            let avg = |f: fn(&Color32) -> u8| ((f(&v[0].color) as u32 + f(&v[1].color) as u32 + f(&v[2].color) as u32) / 3) as u8;
            let c = Color32::from_rgba_premultiplied(avg(|c| c.r()), avg(|c| c.g()), avg(|c| c.b()), avg(|c| c.a()));
            if c.a() == 0 {
                continue;
            }
            let pts: Vec<Pos2> = v.iter().map(|x| x.pos).collect();
            self.poly(&pts, true, c, String::new());
        }
    }

    fn family(&self, fam: Option<&FontFamily>) -> String {
        match fam {
            Some(FontFamily::Monospace) => match &self.style.mono_font {
                Some(f) => format!("{}, {DEFAULT_MONO}", esc(f)),
                None => DEFAULT_MONO.into(),
            },
            _ => match &self.style.font {
                Some(f) => format!("{}, {DEFAULT_FONT}", esc(f)),
                None => DEFAULT_FONT.into(),
            },
        }
    }

    fn text(&mut self, t: &TextShape) {
        let galley = &t.galley;
        let job = &galley.job;
        let chars: Vec<(usize, char)> = job.text.char_indices().collect();
        let is_placeholder = |g: &egui::epaint::text::Glyph| g.advance_width == 0.0 && g.uv_rect.size == Vec2::ZERO && !g.chr.is_whitespace();
        struct Run {
            fam: String,
            italic: bool,
            underline: bool,
            size: f32,
            color: Color32,
            y: f32,
            text: String,
            xs: Vec<f32>,
        }
        let mut runs: Vec<Run> = Vec::new();
        let mut k = 0usize;
        for placed in &galley.rows {
            let glyphs = &placed.row.glyphs;
            for (i, g) in glyphs.iter().enumerate() {
                let byte = chars.get(k).map(|c| c.0).unwrap_or(0);
                k += 1;
                if is_placeholder(g) && i > 0 {
                    continue;
                }
                let extra: Vec<char> = glyphs[i + 1..].iter().take_while(|n| is_placeholder(n)).map(|n| n.chr).collect();
                let sec = job.sections.iter().find(|s| s.byte_range.start.0 <= byte && byte < s.byte_range.end.0).or(job.sections.first());
                let mut color = placed.row.visuals.mesh.vertices.get(g.first_vertex as usize).map(|v| v.color).unwrap_or(Color32::PLACEHOLDER);
                if color == Color32::PLACEHOLDER {
                    color = sec.map(|s| s.format.color).unwrap_or(Color32::PLACEHOLDER);
                }
                if color == Color32::PLACEHOLDER {
                    color = t.fallback_color;
                }
                if let Some(o) = t.override_text_color {
                    color = o;
                }
                if t.opacity_factor < 1.0 {
                    color = color.gamma_multiply(t.opacity_factor);
                }
                let family = sec.map(|s| &s.format.font_id.family);
                let italic = sec.is_some_and(|s| s.format.italics);
                let underline = sec.is_some_and(|s| s.format.underline.width > 0.0);
                let mut x_off = 0.0f32;
                for ch in std::iter::once(g.chr).chain(extra.iter().copied()) {
                    let font = crate::export::font_for(family, ch);
                    let px = g.font_face_ascent * font.height_unscaled() / font.ascent_unscaled().max(1.0);
                    let x = placed.pos.x + g.pos.x + x_off;
                    let y = placed.pos.y + g.pos.y;
                    let fam = self.family(family);
                    let size = px * self.text_scale;
                    let same = runs.last().is_some_and(|r| r.fam == fam && r.italic == italic && r.underline == underline && (r.size - size).abs() < 1e-3 && r.color == color && (r.y - y).abs() < 1e-3);
                    if !same {
                        runs.push(Run { fam, italic, underline, size, color, y, text: String::new(), xs: Vec::new() });
                    }
                    let r = runs.last_mut().unwrap();
                    r.text.push(ch);
                    r.xs.push(x);
                    x_off += font.as_scaled(PxScale::from(px)).h_advance(font.glyph_id(ch));
                }
            }
            if placed.ends_with_newline {
                k += 1;
            }
        }
        if runs.is_empty() {
            return;
        }
        let rot = t.angle != 0.0;
        if rot {
            let _ = writeln!(self.out, "<g transform=\"translate({} {}) rotate({})\">", num(t.pos.x - self.off.x), num(t.pos.y - self.off.y), num(t.angle.to_degrees()));
        }
        for r in runs {
            let (base_x, base_y) = if rot { (0.0, 0.0) } else { (t.pos.x - self.off.x, t.pos.y - self.off.y) };
            let xs = r.xs.iter().map(|x| num(x + base_x)).collect::<Vec<_>>().join(" ");
            let ital = format!("{}{}", if r.italic { " font-style=\"italic\"" } else { "" }, if r.underline { " text-decoration=\"underline\"" } else { "" });
            let f = self.fill(r.color);
            let _ = writeln!(self.out, "<text x=\"{xs}\" y=\"{}\" font-family=\"{}\" font-size=\"{}\"{ital}{f} xml:space=\"preserve\">{}</text>", num(r.y + base_y), r.fam, num(r.size), esc(&r.text));
        }
        if rot {
            self.out.push_str("</g>\n");
        }
    }
}

/// The picture of `shapes` inside `view` (points, in the shapes' own coordinates) as an SVG document.
/// `title` becomes the `<title>`. The style gives fonts, line and text scale, the figure width and a
/// background.
pub fn to_svg(shapes: &[ClippedShape], view: Rect, style: &FigStyle, title: &str) -> Svg {
    let mut w = W {
        out: String::new(),
        defs: String::new(),
        off: view.min.to_vec2(),
        style,
        line_scale: style.line_scale.unwrap_or(1.0),
        text_scale: style.text_scale.unwrap_or(1.0),
        clips: HashMap::new(),
        blurs: 0,
        skipped: Skipped::default(),
    };
    let mut open_clip: Option<usize> = None;
    let mut open_tr = false;
    for cs in shapes {
        let tr = cs.transform_after_tessellation;
        let moved = tr != TSTransform::IDENTITY;
        let needs_clip = !cs.clip_rect.contains_rect(view);
        let want = if needs_clip && cs.clip_rect.intersect(view).is_positive() { Some(w.clip_id(cs.clip_rect.intersect(view))) } else { None };
        if needs_clip && want.is_none() {
            continue;
        }
        if want != open_clip || open_tr || moved {
            if open_tr {
                w.out.push_str("</g>\n");
                open_tr = false;
            }
            if open_clip.is_some() && want != open_clip {
                w.out.push_str("</g>\n");
                open_clip = None;
            }
            if let (Some(c), None) = (want, open_clip) {
                let _ = writeln!(w.out, "<g clip-path=\"url(#c{c})\">");
                open_clip = Some(c);
            }
        }
        if moved {
            let _ = writeln!(w.out, "<g transform=\"translate({} {}) scale({})\">", num(tr.translation.x - w.off.x), num(tr.translation.y - w.off.y), num(tr.scaling));
            open_tr = true;
            let saved = w.off;
            w.off = Vec2::ZERO;
            w.shape(&cs.shape);
            w.off = saved;
        } else {
            w.shape(&cs.shape);
        }
    }
    if open_tr {
        w.out.push_str("</g>\n");
    }
    if open_clip.is_some() {
        w.out.push_str("</g>\n");
    }
    let (wd, ht) = (view.width(), view.height());
    let mut head = String::new();
    let size = match style.width_mm {
        Some(mm) => format!("width=\"{}mm\" height=\"{}mm\"", num(mm), num(mm * ht / wd.max(1.0))),
        None => format!("width=\"{}\" height=\"{}\"", num(wd), num(ht)),
    };
    let _ = writeln!(head, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(head, "<svg xmlns=\"http://www.w3.org/2000/svg\" {size} viewBox=\"0 0 {} {}\">", num(wd), num(ht));
    let _ = writeln!(head, "<title>{}</title>", esc(title));
    if !w.defs.is_empty() {
        let _ = write!(head, "<defs>\n{}</defs>\n", w.defs);
    }
    if let Some(bg) = &style.background {
        if let Ok([r, g, b]) = dynamist_core::figstyle::parse_hex(bg) {
            let _ = writeln!(head, "<rect width=\"100%\" height=\"100%\" fill=\"#{r:02x}{g:02x}{b:02x}\"/>");
        }
    }
    let text = format!("{head}{}</svg>\n", w.out);
    Svg { text, width: wd, height: ht, skipped: w.skipped }
}
