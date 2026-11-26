//! Export any sheet, selection or chart as a high-resolution PNG with a transparent
//! background, to the clipboard (ready to paste into Word, PowerPoint or Excel) or a file.
//!
//! The sheet is drawn once more into an off-screen layer with a light theme and no
//! background. Those shapes are then rasterized here on the CPU at 2–4× resolution:
//! egui's own tessellator turns shapes into anti-aliased triangles, and text is drawn
//! from the font outlines so it stays sharp at any size.
#[cfg(not(feature = "gui"))]
use crate::rfd;

use std::sync::OnceLock;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use egui::emath::TSTransform;
use egui::epaint::{ClippedShape, Mesh, TessellationOptions, Tessellator, TextShape};
use egui::{Color32, FontFamily, Pos2, Rect, Shape, Vec2, vec2};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportWhat {
    /// Everything on the current sheet.
    Sheet,
    /// Only the selected items of a diagram.
    Selection,
    /// The graph in the results panel.
    Trends,
    /// One dashboard card.
    Card(u64),
    /// Any chart, described by App::export_chart.
    Chart,
}

/// Which chart to redraw for a picture (or to take data from).
#[derive(Clone, PartialEq, Debug)]
pub enum ChartRef {
    /// Variables over time, drawn as `kind`.
    Vars { kind: crate::model::ChartKind, vars: Vec<String>, title: String },
    /// A lookup table's curve.
    Lookup(u64),
    /// An optimize sheet's before/best graph.
    OptGoal(u64),
    /// An optimize sheet's progress graph.
    OptProgress(u64),
    /// A customized chart (Trends graph or a dashboard graph).
    Styled { kind: crate::model::ChartKind, vars: Vec<String>, style: crate::model::ChartStyle, target: Option<crate::chartstyle::ChartTarget> },
    /// One variable over time, baseline and every scenario.
    ScenarioLines(String),
    /// One number per scenario.
    ScenarioBars(String, crate::model::Measure),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ExportTo {
    Clipboard,
    File,
}

#[derive(Clone, Copy, Debug)]
pub struct ExportJob {
    pub what: ExportWhat,
    pub to: ExportTo,
}

/// Premultiplied RGBA, 0..1, in the same (gamma) space egui blends in.
pub struct Image {
    pub w: usize,
    pub h: usize,
    px: Vec<[f32; 4]>,
}

impl Image {
    fn new(w: usize, h: usize) -> Self {
        Image {
            w,
            h,
            px: vec![[0.0; 4]; w * h],
        }
    }
    #[inline]
    fn blend(&mut self, x: usize, y: usize, c: [f32; 4]) {
        let d = &mut self.px[y * self.w + x];
        let k = 1.0 - c[3];
        for i in 0..4 {
            d[i] = c[i] + d[i] * k;
        }
    }
    /// Straight (un-premultiplied) RGBA bytes, as PNG expects.
    pub fn rgba8(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.w * self.h * 4);
        for p in &self.px {
            let a = p[3].clamp(0.0, 1.0);
            if a <= 1e-6 {
                out.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                for i in 0..3 {
                    out.push(((p[i] / a).clamp(0.0, 1.0) * 255.0).round() as u8);
                }
                out.push((a * 255.0).round() as u8);
            }
        }
        out
    }
    /// The box (x0, y0, x1, y1, inclusive pixels) that `trimmed(pad)` keeps, or None if nothing is drawn.
    pub fn ink_box(&self, pad: usize) -> Option<(usize, usize, usize, usize)> {
        let (mut x0, mut y0, mut x1, mut y1) = (self.w, self.h, 0, 0);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.px[y * self.w + x][3] > 0.004 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        if x1 < x0 {
            return None;
        }
        Some((x0.saturating_sub(pad), y0.saturating_sub(pad), (x1 + pad).min(self.w - 1), (y1 + pad).min(self.h - 1)))
    }
    /// Trim fully transparent borders, keeping `pad` pixels.
    pub fn trimmed(self, pad: usize) -> Image {
        let (mut x0, mut y0, mut x1, mut y1) = (self.w, self.h, 0, 0);
        for y in 0..self.h {
            for x in 0..self.w {
                if self.px[y * self.w + x][3] > 0.004 {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        if x1 < x0 {
            return self;
        }
        let (x0, y0) = (x0.saturating_sub(pad), y0.saturating_sub(pad));
        let (x1, y1) = ((x1 + pad).min(self.w - 1), (y1 + pad).min(self.h - 1));
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut out = Image::new(w, h);
        for y in 0..h {
            out.px[y * w..(y + 1) * w]
                .copy_from_slice(&self.px[(y + y0) * self.w + x0..(y + y0) * self.w + x0 + w]);
        }
        out
    }
}

fn c4(c: Color32) -> [f32; 4] {
    [
        c.r() as f32 / 255.0,
        c.g() as f32 / 255.0,
        c.b() as f32 / 255.0,
        c.a() as f32 / 255.0,
    ]
}

/// Draw the shapes that fall inside `area` (in points) at `scale` pixels per point.
pub fn rasterize(shapes: &[ClippedShape], area: Rect, scale: f32) -> Image {
    let w = (area.width() * scale).ceil().max(1.0) as usize;
    let h = (area.height() * scale).ceil().max(1.0) as usize;
    let mut img = Image::new(w, h);
    let mut tess = Tessellator::new(scale, TessellationOptions::default(), [1, 1], Vec::new());
    for cs in shapes {
        draw_shape(
            &mut img,
            &mut tess,
            &cs.shape,
            cs.clip_rect,
            area,
            scale,
            cs.transform_after_tessellation,
        );
    }
    img
}

fn draw_shape(
    img: &mut Image,
    tess: &mut Tessellator,
    shape: &Shape,
    clip: Rect,
    area: Rect,
    s: f32,
    tr: TSTransform,
) {
    match shape {
        Shape::Noop => {}
        Shape::Callback(cb) => {
            if let Some(pic) = cb.callback.downcast_ref::<crate::pictures::PicCallback>() {
                draw_picture(img, pic, cb.rect, clip, area, s, tr);
            }
        }
        Shape::Vec(v) => {
            for x in v {
                draw_shape(img, tess, x, clip, area, s, tr);
            }
        }
        Shape::Text(t) => draw_text(img, t, clip, area, s, tr),
        other => {
            let mut mesh = Mesh::default();
            tess.tessellate_shape(other.clone(), &mut mesh);
            if tr != TSTransform::IDENTITY {
                for v in mesh.vertices.iter_mut() {
                    v.pos = tr * v.pos;
                }
            }
            draw_mesh(img, &mesh, clip, area, s);
        }
    }
}

/// A picture, scaled into `rect` (smooth, with its own transparency).
fn draw_picture(img: &mut Image, pic: &crate::pictures::PicCallback, rect: Rect, clip: Rect, area: Rect, s: f32, tr: TSTransform) {
    let Some(cl) = pixel_clip(img, clip, area, s) else {
        return;
    };
    let (a, b) = (tr * rect.min, tr * rect.max);
    let (x0, y0) = ((a.x - area.min.x) * s, (a.y - area.min.y) * s);
    let (x1, y1) = ((b.x - area.min.x) * s, (b.y - area.min.y) * s);
    if x1 - x0 < 1.0 || y1 - y0 < 1.0 || pic.w == 0 || pic.h == 0 {
        return;
    }
    let texel = |x: i64, y: i64| -> [f32; 4] {
        let (x, y) = (x.clamp(0, pic.w as i64 - 1) as usize, y.clamp(0, pic.h as i64 - 1) as usize);
        let p = &pic.rgba[(y * pic.w + x) * 4..][..4];
        let al = p[3] as f32 / 255.0;
        [p[0] as f32 / 255.0 * al, p[1] as f32 / 255.0 * al, p[2] as f32 / 255.0 * al, al]
    };
    for yy in (y0.floor() as i32).max(cl[1])..(y1.ceil() as i32).min(cl[3]) {
        for xx in (x0.floor() as i32).max(cl[0])..(x1.ceil() as i32).min(cl[2]) {
            // where this pixel's centre falls in the picture; averaging the four nearest texels
            let u = ((xx as f32 + 0.5 - x0) / (x1 - x0)) * pic.w as f32 - 0.5;
            let v = ((yy as f32 + 0.5 - y0) / (y1 - y0)) * pic.h as f32 - 0.5;
            let (iu, iv) = (u.floor(), v.floor());
            let (fu, fv) = (u - iu, v - iv);
            let mut c = [0.0f32; 4];
            for (dx, dy, w) in [(0, 0, (1.0 - fu) * (1.0 - fv)), (1, 0, fu * (1.0 - fv)), (0, 1, (1.0 - fu) * fv), (1, 1, fu * fv)] {
                let t = texel(iu as i64 + dx, iv as i64 + dy);
                for i in 0..4 {
                    c[i] += t[i] * w;
                }
            }
            img.blend(xx as usize, yy as usize, c);
        }
    }
}

/// Clip rect in image pixels.
fn pixel_clip(img: &Image, clip: Rect, area: Rect, s: f32) -> Option<[i32; 4]> {
    let c = clip.intersect(area);
    if !c.is_positive() {
        return None;
    }
    let x0 = (((c.min.x - area.min.x) * s).floor() as i32).max(0);
    let y0 = (((c.min.y - area.min.y) * s).floor() as i32).max(0);
    let x1 = (((c.max.x - area.min.x) * s).ceil() as i32).min(img.w as i32);
    let y1 = (((c.max.y - area.min.y) * s).ceil() as i32).min(img.h as i32);
    (x1 > x0 && y1 > y0).then_some([x0, y0, x1, y1])
}

fn draw_mesh(img: &mut Image, mesh: &Mesh, clip: Rect, area: Rect, s: f32) {
    let Some(cl) = pixel_clip(img, clip, area, s) else {
        return;
    };
    let v: Vec<([f32; 2], [f32; 4])> = mesh
        .vertices
        .iter()
        .map(|v| {
            (
                [(v.pos.x - area.min.x) * s, (v.pos.y - area.min.y) * s],
                c4(v.color),
            )
        })
        .collect();
    for t in mesh.indices.chunks_exact(3) {
        let (a, b, c) = (v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
        triangle(img, a, b, c, cl);
    }
}

fn edge(a: [f32; 2], b: [f32; 2], p: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}

/// Fill a triangle with colours interpolated across it (egui's feathering supplies the anti-aliasing).
fn triangle(
    img: &mut Image,
    a: ([f32; 2], [f32; 4]),
    mut b: ([f32; 2], [f32; 4]),
    mut c: ([f32; 2], [f32; 4]),
    cl: [i32; 4],
) {
    let mut area = edge(a.0, b.0, c.0);
    if area.abs() < 1e-9 {
        return;
    }
    if area < 0.0 {
        std::mem::swap(&mut b, &mut c);
        area = -area;
    }
    if a.1[3] <= 0.0 && b.1[3] <= 0.0 && c.1[3] <= 0.0 {
        return;
    }
    let x0 = (a.0[0].min(b.0[0]).min(c.0[0]).floor() as i32).max(cl[0]);
    let x1 = (a.0[0].max(b.0[0]).max(c.0[0]).ceil() as i32).min(cl[2]);
    let y0 = (a.0[1].min(b.0[1]).min(c.0[1]).floor() as i32).max(cl[1]);
    let y1 = (a.0[1].max(b.0[1]).max(c.0[1]).ceil() as i32).min(cl[3]);
    // top-left rule so shared edges aren't painted twice
    let tl = |p: [f32; 2], q: [f32; 2]| (p[1] == q[1] && q[0] < p[0]) || q[1] > p[1];
    let (t0, t1, t2) = (tl(b.0, c.0), tl(c.0, a.0), tl(a.0, b.0));
    for y in y0..y1 {
        for x in x0..x1 {
            let p = [x as f32 + 0.5, y as f32 + 0.5];
            let w0 = edge(b.0, c.0, p);
            let w1 = edge(c.0, a.0, p);
            let w2 = edge(a.0, b.0, p);
            if w0 < 0.0
                || w1 < 0.0
                || w2 < 0.0
                || (w0 == 0.0 && !t0)
                || (w1 == 0.0 && !t1)
                || (w2 == 0.0 && !t2)
            {
                continue;
            }
            let (l0, l1, l2) = (w0 / area, w1 / area, w2 / area);
            let col = [0, 1, 2, 3].map(|i| a.1[i] * l0 + b.1[i] * l1 + c.1[i] * l2);
            if col[3] > 0.0 {
                img.blend(x as usize, y as usize, col);
            }
        }
    }
}

// ---------------------------------------------------------------------------- text

struct Fonts {
    ubuntu: FontRef<'static>,
    hack: FontRef<'static>,
    noto: FontRef<'static>,
    icons: FontRef<'static>,
}

fn fonts() -> &'static Fonts {
    static F: OnceLock<Fonts> = OnceLock::new();
    F.get_or_init(|| Fonts {
        ubuntu: FontRef::try_from_slice(epaint_default_fonts::UBUNTU_LIGHT).expect("font"),
        hack: FontRef::try_from_slice(epaint_default_fonts::HACK_REGULAR).expect("font"),
        noto: FontRef::try_from_slice(epaint_default_fonts::NOTO_EMOJI_REGULAR).expect("font"),
        icons: FontRef::try_from_slice(epaint_default_fonts::EMOJI_ICON).expect("font"),
    })
}

/// The same fallback order egui uses for its built-in fonts.
pub(crate) fn font_for(family: Option<&FontFamily>, c: char) -> &'static FontRef<'static> {
    let f = fonts();
    let order: [&FontRef<'static>; 4] = match family {
        Some(FontFamily::Monospace) => [&f.hack, &f.ubuntu, &f.noto, &f.icons],
        _ => [&f.ubuntu, &f.noto, &f.icons, &f.hack],
    };
    order
        .into_iter()
        .find(|ft| ft.glyph_id(c).0 != 0)
        .unwrap_or(&f.ubuntu)
}

fn draw_text(img: &mut Image, t: &TextShape, clip: Rect, area: Rect, s: f32, tr: TSTransform) {
    let Some(cl) = pixel_clip(img, clip, area, s) else {
        return;
    };
    let galley = &t.galley;
    let job = &galley.job;
    let text: Vec<(usize, char)> = job.text.char_indices().collect();
    let rot = egui::emath::Rot2::from_angle(t.angle);
    // a ligature ("ff", "fl") is one glyph with the full width, followed by zero-width
    // placeholder glyphs for the letters it swallowed: draw those letters ourselves
    let is_placeholder = |g: &egui::epaint::text::Glyph| {
        g.advance_width == 0.0 && g.uv_rect.size == Vec2::ZERO && !g.chr.is_whitespace()
    };
    let mut k = 0usize;
    for placed in &galley.rows {
        let glyphs = &placed.row.glyphs;
        for (i, g) in glyphs.iter().enumerate() {
            let byte = text.get(k).map(|c| c.0).unwrap_or(0);
            k += 1;
            if is_placeholder(g) && i > 0 {
                continue;
            }
            let extra: Vec<char> = glyphs[i + 1..]
                .iter()
                .take_while(|n| is_placeholder(n))
                .map(|n| n.chr)
                .collect();
            let sec = job
                .sections
                .iter()
                .find(|s| s.byte_range.start.0 <= byte && byte < s.byte_range.end.0)
                .or(job.sections.first());
            if let Some(u) = sec.map(|s| s.format.underline).filter(|u| u.width > 0.0) {
                // underline: a thin bar under this letter, in the underline's colour
                let local: Vec2 = placed.pos.to_vec2() + g.pos.to_vec2();
                let world: Pos2 = tr * (t.pos + rot * local);
                let thick = (u.width * s * tr.scaling).max(1.0);
                let x0 = (world.x - area.min.x) * s;
                let y0 = (world.y - area.min.y) * s + g.font_face_ascent * 0.12 * s * tr.scaling + 1.0;
                let w = g.advance_width * s * tr.scaling;
                let c = c4(u.color);
                for yy in y0.floor() as i32..(y0 + thick).ceil() as i32 {
                    for xx in x0.floor() as i32..(x0 + w).ceil() as i32 {
                        if xx >= cl[0] && xx < cl[2] && yy >= cl[1] && yy < cl[3] {
                            img.blend(xx as usize, yy as usize, c);
                        }
                    }
                }
            }
            if g.chr.is_whitespace() {
                continue;
            }
            let italic = sec.is_some_and(|s| s.format.italics);
            let mut color = placed
                .row
                .visuals
                .mesh
                .vertices
                .get(g.first_vertex as usize)
                .map(|v| v.color)
                .unwrap_or(Color32::PLACEHOLDER);
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
            let mut x_off = 0.0f32;
            for ch in std::iter::once(g.chr).chain(extra.iter().copied()) {
                let font = font_for(family, ch);
                // match egui's metrics: the face's ascent decides the size
                let asc = font.ascent_unscaled();
                let hgt = font.height_unscaled();
                let px = (g.font_face_ascent * s * tr.scaling * hgt / asc.max(1.0)).max(1.0);
                let local: Vec2 = placed.pos.to_vec2() + g.pos.to_vec2() + vec2(x_off, 0.0);
                let world: Pos2 = tr * (t.pos + rot * local);
                if !ch.is_whitespace() {
                    glyph(img, font, ch, px, world, area, s, t.angle, color, cl, italic);
                }
                // advance in points for the next folded letter
                x_off += font
                    .as_scaled(PxScale::from(px))
                    .h_advance(font.glyph_id(ch))
                    / (s * tr.scaling);
            }
        }
        if placed.ends_with_newline {
            k += 1;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn glyph(
    img: &mut Image,
    font: &FontRef<'static>,
    ch: char,
    px: f32,
    baseline: Pos2,
    area: Rect,
    s: f32,
    angle: f32,
    color: Color32,
    cl: [i32; 4],
    italic: bool,
) {
    let col = c4(color);
    let scaled = font.as_scaled(PxScale::from(px));
    let gid = font.glyph_id(ch);
    let bx = (baseline.x - area.min.x) * s;
    let by = (baseline.y - area.min.y) * s;
    if angle == 0.0 {
        let gl = gid.with_scale_and_position(scaled.scale(), point(bx, by));
        let Some(og) = font.outline_glyph(gl) else {
            return;
        };
        let b = og.px_bounds();
        og.draw(|x, y, cov| {
            let (mut xx, yy) = (b.min.x as i32 + x as i32, b.min.y as i32 + y as i32);
            if italic {
                // a slant of about 12 degrees, about the baseline
                xx += ((by - yy as f32) * 0.21).round() as i32;
            }
            if xx >= cl[0] && xx < cl[2] && yy >= cl[1] && yy < cl[3] && cov > 0.0 {
                img.blend(xx as usize, yy as usize, col.map(|c| c * cov.min(1.0)));
            }
        });
        return;
    }
    // rotated text (e.g. a vertical axis label): draw upright, then turn it
    let gl = gid.with_scale_and_position(scaled.scale(), point(0.0, 0.0));
    let Some(og) = font.outline_glyph(gl) else {
        return;
    };
    let b = og.px_bounds();
    let (gw, gh) = (
        b.width().ceil() as usize + 1,
        b.height().ceil() as usize + 1,
    );
    let mut cov = vec![0.0f32; gw * gh];
    og.draw(|x, y, c| {
        if (x as usize) < gw && (y as usize) < gh {
            cov[y as usize * gw + x as usize] = c;
        }
    });
    let (sin, cos) = angle.sin_cos();
    let corners = [
        vec2(b.min.x, b.min.y),
        vec2(b.max.x, b.min.y),
        vec2(b.min.x, b.max.y),
        vec2(b.max.x, b.max.y),
    ];
    let rotp = |v: Vec2| vec2(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for c in corners {
        let r = rotp(c);
        x0 = x0.min(r.x);
        y0 = y0.min(r.y);
        x1 = x1.max(r.x);
        y1 = y1.max(r.y);
    }
    for yy in (by + y0).floor() as i32..(by + y1).ceil() as i32 {
        for xx in (bx + x0).floor() as i32..(bx + x1).ceil() as i32 {
            if xx < cl[0] || xx >= cl[2] || yy < cl[1] || yy >= cl[3] {
                continue;
            }
            // back into the upright glyph
            let d = vec2(xx as f32 + 0.5 - bx, yy as f32 + 0.5 - by);
            let u = vec2(d.x * cos + d.y * sin, -d.x * sin + d.y * cos);
            let (gx, gy) = ((u.x - b.min.x).floor(), (u.y - b.min.y).floor());
            if gx < 0.0 || gy < 0.0 || gx as usize >= gw || gy as usize >= gh {
                continue;
            }
            let c = cov[gy as usize * gw + gx as usize];
            if c > 0.0 {
                img.blend(xx as usize, yy as usize, col.map(|v| v * c.min(1.0)));
            }
        }
    }
}

// ---------------------------------------------------------------------------- output

/// PNG bytes with the resolution recorded, so Office places the picture at its real size.
pub fn png_bytes(img: &Image, scale: f32) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.w as u32, img.h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let ppm = (96.0 * scale / 0.0254).round() as u32;
        enc.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&img.rgba8())
            .map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Put the picture on the system clipboard.
pub fn to_clipboard(img: &Image, scale: f32) -> Result<(), String> {
    #[cfg(not(feature = "gui"))]
    {
        let _ = (img, scale);
        return Err("there is no clipboard in the headless build".into());
    }
    #[cfg(all(feature = "gui", target_os = "macos"))]
    {
        let png = png_bytes(img, scale)?;
        mac_clipboard(
            &png,
            img.w as f64 / scale as f64,
            img.h as f64 / scale as f64,
        );
        Ok(())
    }
    #[cfg(all(feature = "gui", not(target_os = "macos")))]
    {
        let _ = scale;
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        cb.set_image(arboard::ImageData {
            width: img.w,
            height: img.h,
            bytes: std::borrow::Cow::Owned(img.rgba8()),
        })
        .map_err(|e| e.to_string())
    }
}

/// On macOS, offer both PNG and a TIFF that knows its size in points, which Office prefers.
#[cfg(all(feature = "gui", target_os = "macos"))]
fn mac_clipboard(png: &[u8], w_pt: f64, h_pt: f64) {
    use objc2::AllocAnyThread;
    use objc2_app_kit::{NSImage, NSPasteboard, NSPasteboardTypePNG, NSPasteboardTypeTIFF};
    use objc2_foundation::{NSData, NSSize};
    let data = NSData::with_bytes(png);
    let pb = NSPasteboard::generalPasteboard();
    pb.clearContents();
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        image.setSize(NSSize::new(w_pt, h_pt));
        if let Some(tiff) = image.TIFFRepresentation() {
            pb.setData_forType(Some(&tiff), unsafe { NSPasteboardTypeTIFF });
        }
    }
    pb.setData_forType(Some(&data), unsafe { NSPasteboardTypePNG });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_chart_exports_as_picture_and_data() {
        use crate::model::ChartKind;
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None);
        let lookup = app.model.by_name("effect of awareness").unwrap().id;
        let charts = [
            ChartRef::Vars { kind: ChartKind::Bars, vars: vec!["Susceptible".into(), "Recovered".into()], title: "Bars".into() },
            ChartRef::Vars { kind: ChartKind::Donut, vars: vec!["Susceptible".into(), "Infected".into(), "Recovered".into()], title: "Donut".into() },
            ChartRef::Lookup(lookup),
        ];
        for (i, c) in charts.iter().enumerate() {
            let path = std::env::temp_dir().join(format!("fc_chart_{i}.png"));
            let _ = std::fs::remove_file(&path);
            for _ in 0..2 {
                let mut out = ctx.run_ui(Default::default(), |ui| {
                    app.export_path = Some(path.clone());
                    app.export_chart = Some(c.clone());
                    app.run_export(ui.ctx(), ExportJob { what: ExportWhat::Chart, to: ExportTo::File });
                });
                out.textures_delta.clear();
            }
            let bytes = std::fs::read(&path).unwrap_or_else(|_| panic!("chart {i} saved: {}", app.status));
            let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            assert!(w > 1000, "chart {i} is high resolution ({w} px wide)");
        }
        let data = app.chart_data(&charts[0]).unwrap();
        assert!(data.starts_with("Time (Day)\tSusceptible\tRecovered\n"));
        assert_eq!(data.lines().count(), 482);
        assert!(app.chart_data(&charts[2]).unwrap().starts_with("x (input)\tEffect of Awareness"));
    }

    #[test]
    fn rasterizes_shapes_and_text_with_transparency() {
        let ctx = egui::Context::default();
        let mut shapes = Vec::new();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let p = ui.painter();
            p.rect_filled(
                Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(40.0, 20.0)),
                4.0,
                Color32::from_rgb(79, 70, 229),
            );
            let g = p.layout_no_wrap(
                "Stock".into(),
                egui::FontId::proportional(14.0),
                Color32::BLACK,
            );
            shapes.push(ClippedShape {
                transform_after_tessellation: TSTransform::IDENTITY,
                clip_rect: Rect::EVERYTHING,
                shape: Shape::rect_filled(
                    Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(40.0, 20.0)),
                    4.0,
                    Color32::from_rgb(79, 70, 229),
                ),
            });
            shapes.push(ClippedShape {
                transform_after_tessellation: TSTransform::IDENTITY,
                clip_rect: Rect::EVERYTHING,
                shape: Shape::galley(Pos2::new(60.0, 10.0), g, Color32::BLACK),
            });
        });
        out.textures_delta.clear();
        let img = rasterize(
            &shapes,
            Rect::from_min_size(Pos2::ZERO, vec2(120.0, 40.0)),
            3.0,
        );
        assert_eq!((img.w, img.h), (360, 120));
        let rgba = img.rgba8();
        let at = |x: usize, y: usize| &rgba[(y * 360 + x) * 4..(y * 360 + x) * 4 + 4];
        assert_eq!(at(1, 1)[3], 0, "background stays transparent");
        assert_eq!(at(90, 60), &[79, 70, 229, 255], "filled shape is solid");
        // some dark text pixels were drawn to the right
        let inked = (180..330)
            .flat_map(|x| (30..90).map(move |y| (x, y)))
            .filter(|(x, y)| at(*x, *y)[3] > 200)
            .count();
        assert!(inked > 100, "text drawn: {inked}");
        assert!(
            png_bytes(&img, 3.0)
                .unwrap()
                .starts_with(&[0x89, b'P', b'N', b'G'])
        );
    }
}

// ---------------------------------------------------------------------------- driving an export

use crate::app::App;
use crate::model::SheetKind;
use crate::theme::{self, Palette};
use egui::{Id, LayerId, Order, UiBuilder, pos2};

impl App {
    pub fn request_export(&mut self, what: ExportWhat, to: ExportTo) {
        self.pending_export = Some(ExportJob { what, to });
    }

    /// A fresh off-screen Ui on the export layer.
    pub fn export_ui(&self, ctx: &egui::Context, layer: LayerId, area: Rect) -> egui::Ui {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            Id::unique("dynamist-export-ui"),
            UiBuilder::new().layer_id(layer).max_rect(area),
        );
        // graphs get no background of their own
        ui.visuals_mut().extreme_bg_color = Color32::TRANSPARENT;
        ui
    }

    /// Draw what was asked for off-screen and return the painted shapes with their area (in points).
    /// The app's PNG export and the headless SVG export both start here, so they show the same picture.
    pub fn render_shapes(&mut self, ctx: &egui::Context, what: ExportWhat) -> Option<(Vec<ClippedShape>, Rect)> {
        let layer = LayerId::new(Order::Background, Id::unique("dynamist-export"));
        let origin = pos2(-60000.0, -60000.0);
        // documents are white: always export in the light theme, without any page background
        let saved_pal = self.pal;
        let mut pal = Palette::light();
        pal.canvas = Color32::TRANSPARENT;
        if let Some(st) = &self.export_style {
            pal.set_colors(&st.colors);
        }
        self.pal = pal;
        if saved_pal.dark {
            theme::apply(ctx, &Palette::light());
        }
        let (saved_sel, saved_multi, saved_edit) = (self.sel, self.multi.clone(), self.dash_edit);
        self.exporting = true;
        self.dash_edit = false;
        // two passes: graphs size their axes from the pass before, so only the second is kept
        let mut area = None;
        for pass in 0..2 {
            if pass == 1 {
                ctx.graphics_mut(|g| *g.entry(layer) = Default::default());
            }
            area = match what {
                ExportWhat::Selection => self.diagram_export(ctx, layer, origin, true),
                ExportWhat::Trends => self.trends_export(ctx, layer, origin),
                ExportWhat::Card(id) => self.card_export(ctx, layer, origin, id),
                ExportWhat::Chart => self.chart_export(ctx, layer, origin),
                ExportWhat::Sheet => match self.active_sheet_kind() {
                    SheetKind::Diagram => self.diagram_export(ctx, layer, origin, false),
                    SheetKind::CausalLoop => self.cld_export(ctx, layer, origin),
                    SheetKind::Table => self.table_export(ctx, layer, origin),
                    SheetKind::Dashboard => self.dashboard_export(ctx, layer, origin),
                    SheetKind::Optimize => {
                        let area = Rect::from_min_size(origin, vec2(1440.0, 980.0));
                        let mut ui = self.export_ui(ctx, layer, area);
                        self.optimize_sheet(&mut ui);
                        Some(area)
                    }
                    SheetKind::Scenarios => {
                        let area = Rect::from_min_size(origin, vec2(1440.0, 1200.0));
                        let mut ui = self.export_ui(ctx, layer, area);
                        self.scenarios_sheet(&mut ui);
                        Some(area)
                    }
                    SheetKind::Entities => {
                        let area = Rect::from_min_size(origin, vec2(1500.0, 1400.0));
                        let mut ui = self.export_ui(ctx, layer, area);
                        self.entities_sheet(&mut ui);
                        Some(area)
                    }
                    SheetKind::Architecture => {
                        let area = Rect::from_min_size(origin, vec2(1500.0, 1500.0));
                        let mut ui = self.export_ui(ctx, layer, area);
                        self.architecture_sheet(&mut ui);
                        Some(area)
                    }
                },
            };
        }
        self.exporting = false;
        self.dash_edit = saved_edit;
        self.sel = saved_sel;
        self.multi = saved_multi;
        self.pal = saved_pal;
        if saved_pal.dark {
            theme::apply(ctx, &saved_pal);
        }
        let shapes: Vec<ClippedShape> = ctx.graphics_mut(|g| {
            let list = std::mem::take(g.entry(layer));
            list.all_entries().cloned().collect()
        });
        Some((shapes, area?))
    }

    /// Draw what was asked for off-screen, rasterize it, and hand it over.
    pub fn run_export(&mut self, ctx: &egui::Context, job: ExportJob) {
        let Some((shapes, area)) = self.render_shapes(ctx, job.what) else {
            self.status = "Nothing to export here yet".into();
            return;
        };
        let scale = self.export_scale;
        let img = rasterize(&shapes, area, scale).trimmed((10.0 * scale) as usize);
        let (w, h) = (img.w, img.h);
        match job.to {
            ExportTo::Clipboard => {
                self.status = match to_clipboard(&img, scale) {
                    Ok(()) => format!(
                        "Copied a transparent {scale:.0}× picture ({w}×{h} px) — paste it into Word, PowerPoint or Excel"
                    ),
                    Err(e) => format!("Couldn't copy the picture: {e}"),
                };
            }
            ExportTo::File => {
                let name = self
                    .model
                    .sheets
                    .iter()
                    .find(|s| s.id == self.sheet)
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "diagram".into());
                let chosen = match self.export_path.take() {
                    Some(p) => Some(p),
                    None => rfd::FileDialog::new()
                        .add_filter("PNG image", &["png"])
                        .set_file_name(format!("{name}.png"))
                        .save_file(),
                };
                let Some(path) = chosen else { return };
                self.status = match png_bytes(&img, scale)
                    .and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string()))
                {
                    Ok(()) => format!("Saved {} ({w}×{h} px, transparent)", path.display()),
                    Err(e) => format!("Couldn't save the picture: {e}"),
                };
            }
        }
    }

    pub fn export_chart_to(&mut self, chart: ChartRef, to: ExportTo) {
        self.export_chart = Some(chart);
        self.request_export(ExportWhat::Chart, to);
    }

    /// Right-click menu for any chart: copy or save it as a picture, or copy its numbers.
    pub fn chart_menu(&mut self, resp: &egui::Response, chart: ChartRef) {
        resp.context_menu(|ui| {
            use crate::icons::{Ic, menu_item};
            if menu_item(ui, Ic::Image, "Copy chart as picture", "", true) {
                self.export_chart_to(chart.clone(), ExportTo::Clipboard);
            }
            if menu_item(ui, Ic::Download, "Save chart as PNG…", "", true) {
                self.export_chart_to(chart.clone(), ExportTo::File);
            }
            if let ChartRef::Styled { target: Some(t), .. } = &chart {
                if menu_item(ui, Ic::Sliders, "Customize chart…", "", true) {
                    self.customize_chart(*t);
                }
            }
            let data = self.chart_data(&chart);
            if menu_item(ui, Ic::Table, "Copy chart data (for Excel)", "", data.is_some()) {
                if let Some(d) = data {
                    let rows = d.lines().count().saturating_sub(1);
                    ui.ctx().copy_text(d);
                    self.status = format!("Copied {rows} rows — paste into any spreadsheet");
                }
            }
        });
    }

    /// A small copy icon for a chart's header (right-click the chart for more).
    pub fn chart_copy_button(&mut self, ui: &mut egui::Ui, chart: ChartRef) {
        let pal = self.pal;
        if crate::icons::tool(ui, crate::icons::Ic::Image, "Copy this chart as a picture for Word or PowerPoint (right-click the chart for more)", pal.muted, pal.accent).clicked() {
            self.export_chart_to(chart, ExportTo::Clipboard);
        }
    }

    /// The numbers behind a chart, tab-separated for spreadsheets.
    pub fn chart_data(&self, chart: &ChartRef) -> Option<String> {
        let tsv = |head: Vec<String>, rows: Vec<Vec<String>>| {
            let mut s = head.join("\t");
            for r in rows {
                s.push('\n');
                s.push_str(&r.join("\t"));
            }
            s.push('\n');
            s
        };
        match chart {
            ChartRef::Vars { vars, .. } | ChartRef::Styled { vars, .. } => {
                let run = self.run.as_ref()?;
                let mut head = vec![format!("Time ({})", run.time_units)];
                head.extend(vars.iter().cloned());
                let rows = (0..run.times.len()).map(|i| std::iter::once(run.times[i].to_string()).chain(vars.iter().map(|n| run.get(n).map(|v| v[i].to_string()).unwrap_or_default())).collect()).collect();
                Some(tsv(head, rows))
            }
            ChartRef::Lookup(id) => {
                let v = self.model.var(*id)?;
                let mut t = v.table.clone();
                t.sort_by(|a, b| a[0].total_cmp(&b[0]));
                Some(tsv(vec!["x (input)".into(), v.name.clone()], t.iter().map(|p| vec![p[0].to_string(), p[1].to_string()]).collect()))
            }
            ChartRef::OptGoal(sheet) => {
                let (p, goal) = self.opt_result(*sheet)?;
                let (a, b) = (p.start_run.as_ref()?, p.best_run.as_ref()?);
                let (va, vb) = (a.get(&goal)?, b.get(&goal)?);
                Some(tsv(vec![format!("Time ({})", a.time_units), format!("{goal} before"), format!("{goal} best")], (0..a.times.len().min(vb.len())).map(|i| vec![a.times[i].to_string(), va[i].to_string(), vb[i].to_string()]).collect()))
            }
            ChartRef::ScenarioLines(var) => {
                let lineup = self.scenario_lineup();
                let base = &lineup.first()?.1;
                let mut head = vec![format!("Time ({})", base.time_units)];
                head.extend(lineup.iter().map(|x| x.0.clone()));
                let rows = (0..base.times.len()).map(|i| std::iter::once(base.times[i].to_string()).chain(lineup.iter().map(|(_, r, _)| r.get(var).and_then(|v| v.get(i)).map(|x| x.to_string()).unwrap_or_default())).collect()).collect();
                Some(tsv(head, rows))
            }
            ChartRef::ScenarioBars(var, m) => {
                let rows = self.scenario_lineup().into_iter().map(|(n, r, _)| vec![n, crate::optimize::measure(&r, var, *m, false).map(|x| x.to_string()).unwrap_or_default()]).collect();
                Some(tsv(vec!["Scenario".into(), format!("{} of {var}", m.label())], rows))
            }
            ChartRef::OptProgress(sheet) => {
                let (p, goal) = self.opt_result(*sheet)?;
                Some(tsv(vec!["Try".into(), format!("Best {goal}")], p.history.iter().map(|h| vec![h[0].to_string(), h[1].to_string()]).collect()))
            }
        }
    }

    /// Redraw the requested chart on its own, with a title, for a picture.
    fn chart_export(&mut self, ctx: &egui::Context, layer: LayerId, origin: Pos2) -> Option<Rect> {
        let chart = self.export_chart.clone()?;
        let area = Rect::from_min_size(origin, vec2(1100.0, 620.0));
        let mut ui = self.export_ui(ctx, layer, area);
        let pal = self.pal;
        let title = |ui: &mut egui::Ui, t: &str| {
            ui.label(egui::RichText::new(t).size(16.0).strong().color(pal.text));
        };
        match chart {
            ChartRef::Vars { kind, vars, title: t } => {
                title(&mut ui, &t);
                self.chart(&mut ui, "export-chart", kind, &vars, 560.0);
            }
            ChartRef::Lookup(id) => {
                let v = self.model.var(id)?.clone();
                title(&mut ui, &v.name);
                let mut t = v.table.clone();
                t.sort_by(|a, b| a[0].total_cmp(&b[0]));
                let input = if v.equation.trim().is_empty() { "input".to_string() } else { v.equation.clone() };
                egui_plot::Plot::new("export-lookup").height(560.0).x_axis_label(input).y_axis_label(v.units.clone()).include_y(0.0).show(&mut ui, |pu| {
                    pu.line(egui_plot::Line::new(v.name.clone(), egui_plot::PlotPoints::from(t.clone())).color(pal.lookup).width(2.4));
                    pu.points(egui_plot::Points::new("", t).radius(4.5).color(pal.lookup));
                });
            }
            ChartRef::OptGoal(sheet) => {
                let (p, goal) = self.opt_result(sheet)?;
                title(&mut ui, &format!("{goal} — before and best"));
                self.opt_goal_plot(&mut ui, &p, &goal, 560.0, "export-opt-goal");
            }
            ChartRef::Styled { kind, vars, style, .. } => {
                if style.title.is_empty() {
                    title(&mut ui, &vars.join(", "));
                }
                self.chart_styled(&mut ui, "export-chart", kind, &vars, 560.0, &style, None);
            }
            ChartRef::ScenarioLines(var) => {
                title(&mut ui, &format!("{var} — scenarios over time"));
                self.scenario_lines(&mut ui, &var, 560.0, "export-scen-lines");
            }
            ChartRef::ScenarioBars(var, m) => {
                title(&mut ui, &format!("{} of {var} by scenario", m.label()));
                self.scenario_bars(&mut ui, &var, m, 560.0, "export-scen-bars");
            }
            ChartRef::OptProgress(sheet) => {
                let (p, goal) = self.opt_result(sheet)?;
                title(&mut ui, &format!("Best {goal} by try"));
                self.opt_progress_plot(&mut ui, &p, 560.0, "export-opt-progress");
            }
        }
        Some(area)
    }

    fn trends_export(&mut self, ctx: &egui::Context, layer: LayerId, origin: Pos2) -> Option<Rect> {
        let names: Vec<String> = self.shown_vars().into_iter().map(|(n, _)| n).collect();
        if names.is_empty() || self.run.is_none() {
            return None;
        }
        let area = Rect::from_min_size(origin, vec2(1100.0, 600.0));
        let mut ui = self.export_ui(ctx, layer, area);
        ui.label(
            egui::RichText::new(names.join(", "))
                .size(16.0)
                .strong()
                .color(self.pal.text),
        );
        let kind = self.trend_chart;
        self.chart(&mut ui, "export-trend", kind, &names, 540.0);
        Some(area)
    }
}
