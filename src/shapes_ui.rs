//! Shapes and pictures on a diagram sheet: drawing them, finding them under the pointer, their
//! resize handles, and making new ones.

use std::sync::Arc;

use egui::{Color32, CornerRadius, Pos2, Rect, Stroke, StrokeKind, pos2, vec2};
use dynamist_core::b64;
use dynamist_core::model::{ImageAsset, Shape, ShapeKind};

use crate::app::{App, Sel, Tool};
use crate::pictures::PicCallback;
#[cfg(not(feature = "gui"))]
use crate::rfd;
use crate::theme;

/// Handle squares are this wide on screen.
const HANDLE: f32 = 9.0;

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-6 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

impl App {
    /// The box of a shape in diagram units.
    pub fn shape_rect(&self, id: u64) -> Option<Rect> {
        let s = self.model.shapes.iter().find(|s| s.id == id)?;
        let (a, b) = s.bounds();
        Some(Rect::from_min_max(pos2(a[0], a[1]), pos2(b[0], b[1])))
    }

    fn shape_stroke_color(&self, s: &Shape) -> Color32 {
        s.stroke.map(rgb).unwrap_or(self.pal.text.gamma_multiply(0.65))
    }

    /// Draw every shape of this sheet, behind everything else.
    pub(crate) fn draw_shapes(&mut self, ui: &egui::Ui, p: &egui::Painter, origin: Pos2) {
        let view = self.diagram_view();
        let z = self.zoom;
        let shapes: Vec<Shape> = self.model.shapes.iter().filter(|s| s.view == view && self.shown(s.id)).cloned().collect();
        for s in &shapes {
            let (a, b) = s.bounds();
            let r = Rect::from_min_max(self.to_screen(origin, pos2(a[0], a[1])), self.to_screen(origin, pos2(b[0], b[1])));
            let col = self.shape_stroke_color(s);
            let stroke = Stroke::new((s.line_weight() * z).max(0.5), col);
            let fill = s.fill.map(rgb);
            match s.kind {
                ShapeKind::Rect | ShapeKind::Rounded => {
                    let rad = if s.kind == ShapeKind::Rounded { CornerRadius::same(((10.0 * z).round() as i32).clamp(0, 255) as u8) } else { CornerRadius::ZERO };
                    if let Some(f) = fill {
                        p.rect_filled(r, rad, f);
                    }
                    p.rect_stroke(r, rad, stroke, StrokeKind::Middle);
                }
                ShapeKind::Ellipse => {
                    let (c, rx, ry) = (r.center(), r.width() / 2.0, r.height() / 2.0);
                    let pts: Vec<Pos2> = (0..72)
                        .map(|i| {
                            let t = i as f32 / 72.0 * std::f32::consts::TAU;
                            pos2(c.x + rx * t.cos(), c.y + ry * t.sin())
                        })
                        .collect();
                    p.add(egui::Shape::convex_polygon(pts, fill.unwrap_or(Color32::TRANSPARENT), stroke));
                }
                ShapeKind::Line | ShapeKind::Arrow => {
                    let p0 = self.to_screen(origin, pos2(s.pos[0], s.pos[1]));
                    let p1 = self.to_screen(origin, pos2(s.pos[0] + s.size[0], s.pos[1] + s.size[1]));
                    if s.kind == ShapeKind::Arrow && p0.distance(p1) > 1.0 {
                        let dir = (p1 - p0).normalized();
                        let head = (10.0 + s.line_weight() * 2.0) * z;
                        p.line_segment([p0, p1 - dir * head * 0.7], stroke);
                        theme::arrow_head(p, p1, dir, head, col);
                    } else {
                        p.line_segment([p0, p1], stroke);
                    }
                }
                ShapeKind::Picture => {
                    let asset = s.image.and_then(|id| self.model.images.iter().find(|i| i.id == id)).cloned();
                    let drawn = asset.as_ref().and_then(|a| {
                        if self.exporting {
                            // the exporters cannot read a texture: they get the pixels instead
                            let d = self.pictures.get(a).ok()?;
                            let cb = egui::PaintCallback { rect: r, callback: Arc::new(PicCallback { rgba: d.rgba.clone(), w: d.w, h: d.h, png: d.png.clone() }) };
                            p.add(egui::Shape::Callback(cb));
                            Some(())
                        } else {
                            let tex = self.pictures.texture(ui.ctx(), a)?;
                            p.image(tex, r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                            Some(())
                        }
                    });
                    if drawn.is_none() {
                        // a picture that cannot be read: show where it is, so it can be deleted
                        p.rect_stroke(r, CornerRadius::ZERO, Stroke::new(1.0, self.pal.error), StrokeKind::Inside);
                        p.line_segment([r.left_top(), r.right_bottom()], Stroke::new(1.0, self.pal.error));
                        p.line_segment([r.right_top(), r.left_bottom()], Stroke::new(1.0, self.pal.error));
                    }
                }
            }
        }
        let _ = ui;
    }

    /// Which end or corner handles the selected shape shows (diagram units).
    fn shape_handles(&self) -> Vec<(u64, u8, Pos2)> {
        let sel = self.selected_items();
        let Some(&Sel::Shape(id)) = sel.first().filter(|_| sel.len() == 1) else {
            return Vec::new();
        };
        let Some(s) = self.model.shapes.iter().find(|s| s.id == id) else {
            return Vec::new();
        };
        if s.kind.is_line() {
            vec![(id, 0, pos2(s.pos[0], s.pos[1])), (id, 1, pos2(s.pos[0] + s.size[0], s.pos[1] + s.size[1]))]
        } else {
            let r = self.shape_rect(id).unwrap_or(Rect::NOTHING);
            vec![(id, 0, r.left_top()), (id, 1, r.right_top()), (id, 2, r.right_bottom()), (id, 3, r.left_bottom())]
        }
    }

    /// The handle of the selected shape under `d`, if any.
    pub(crate) fn shape_handle_hit(&self, d: Pos2) -> Option<(u64, u8)> {
        let tol = HANDLE * 0.9 / self.zoom;
        self.shape_handles().into_iter().find(|(_, _, p)| p.distance(d) <= tol).map(|(id, i, _)| (id, i))
    }

    pub(crate) fn draw_shape_handles(&self, p: &egui::Painter, origin: Pos2) {
        let pal = self.pal;
        let hs = self.shape_handles();
        if let Some((id, ..)) = hs.first() {
            if let Some(r) = self.shape_rect(*id) {
                let sr = Rect::from_min_max(self.to_screen(origin, r.min), self.to_screen(origin, r.max));
                p.add(egui::Shape::dashed_line(&[sr.left_top(), sr.right_top(), sr.right_bottom(), sr.left_bottom(), sr.left_top()], Stroke::new(1.0, pal.accent.gamma_multiply(0.7)), 5.0, 4.0));
            }
        }
        for (_, _, pt) in hs {
            let c = self.to_screen(origin, pt);
            let r = Rect::from_center_size(c, vec2(HANDLE, HANDLE));
            p.rect_filled(r, CornerRadius::same(2), Color32::WHITE);
            p.rect_stroke(r, CornerRadius::same(2), Stroke::new(1.5, pal.accent), StrokeKind::Inside);
        }
    }

    /// The topmost shape under `d`. Filled shapes and pictures are hit anywhere inside, the rest on their outline.
    pub(crate) fn shape_hit(&self, d: Pos2) -> Option<u64> {
        let view = self.diagram_view();
        let tol = 6.0 / self.zoom;
        for s in self.model.shapes.iter().rev().filter(|s| s.view == view) {
            let (a, b) = s.bounds();
            let r = Rect::from_min_max(pos2(a[0], a[1]), pos2(b[0], b[1]));
            let hit = match s.kind {
                ShapeKind::Line | ShapeKind::Arrow => dist_to_segment(d, pos2(s.pos[0], s.pos[1]), pos2(s.pos[0] + s.size[0], s.pos[1] + s.size[1])) <= tol,
                ShapeKind::Picture => r.contains(d),
                ShapeKind::Rect | ShapeKind::Rounded => {
                    if s.fill.is_some() {
                        r.expand(tol / 2.0).contains(d)
                    } else {
                        r.expand(tol).contains(d) && !r.shrink(tol).contains(d)
                    }
                }
                ShapeKind::Ellipse => {
                    let (c, rx, ry) = (r.center(), (r.width() / 2.0).max(1.0), (r.height() / 2.0).max(1.0));
                    let k = (((d.x - c.x) / rx).powi(2) + ((d.y - c.y) / ry).powi(2)).sqrt();
                    if s.fill.is_some() { k <= 1.0 + tol / rx.min(ry) } else { (k - 1.0).abs() <= tol / rx.min(ry) }
                }
            };
            if hit {
                return Some(s.id);
            }
        }
        None
    }

    /// Pull a handle of a shape to `d`.
    pub(crate) fn drag_shape_handle(&mut self, id: u64, handle: u8, d: Pos2) {
        let aspect = self
            .model
            .shapes
            .iter()
            .find(|s| s.id == id)
            .filter(|s| s.kind == ShapeKind::Picture)
            .and_then(|s| s.image)
            .and_then(|i| self.model.images.iter().find(|a| a.id == i))
            .map(|a| a.h as f32 / a.w.max(1) as f32);
        let Some(s) = self.model.shapes.iter_mut().find(|s| s.id == id) else { return };
        if s.kind.is_line() {
            if handle == 0 {
                let end = [s.pos[0] + s.size[0], s.pos[1] + s.size[1]];
                s.pos = [d.x, d.y];
                s.size = [end[0] - d.x, end[1] - d.y];
            } else {
                s.size = [d.x - s.pos[0], d.y - s.pos[1]];
            }
            return;
        }
        let (a, b) = s.bounds();
        // the corner opposite the one being dragged stays where it is
        let fixed = match handle {
            0 => pos2(b[0], b[1]),
            1 => pos2(a[0], b[1]),
            2 => pos2(a[0], a[1]),
            _ => pos2(b[0], a[1]),
        };
        let mut moving = d;
        if let Some(ar) = aspect {
            // a picture keeps its proportions: the wider pull wins
            let w = (d.x - fixed.x).abs().max(((d.y - fixed.y).abs()) / ar.max(0.01)).max(8.0);
            moving = pos2(fixed.x + w * (d.x - fixed.x).signum(), fixed.y + w * ar * (d.y - fixed.y).signum());
        }
        let r = Rect::from_two_pos(fixed, moving);
        s.pos = [r.min.x, r.min.y];
        s.size = [r.width().max(4.0), r.height().max(4.0)];
    }

    /// Finish drawing a shape dragged from `start` to `end`.
    pub(crate) fn create_shape(&mut self, start: Pos2, end: Pos2) {
        let kind = self.shape_kind;
        let view = self.diagram_view();
        let (pos, size) = if kind.is_line() {
            if start.distance(end) < 8.0 { ([start.x, start.y], [140.0, 0.0]) } else { ([start.x, start.y], [end.x - start.x, end.y - start.y]) }
        } else {
            let r = Rect::from_two_pos(start, end);
            if r.width() < 10.0 || r.height() < 10.0 { ([start.x, start.y], [140.0, 90.0]) } else { ([r.min.x, r.min.y], [r.width(), r.height()]) }
        };
        self.checkpoint();
        let id = self.model.new_id();
        self.model.shapes.push(Shape::new(id, view, kind, pos, size));
        self.select_one(Some(Sel::Shape(id)));
        self.tool = Tool::Select;
        self.changed();
    }

    /// Put a picture from PNG or JPEG bytes on the sheet, near the top left of what is showing.
    pub fn add_picture_bytes(&mut self, name: &str, bytes: &[u8]) -> Result<(), String> {
        let (w, h) = crate::pictures::probe(bytes)?;
        if bytes.len() > 12 * 1024 * 1024 {
            return Err("that picture is over 12 MB; make it smaller first".into());
        }
        self.checkpoint();
        let aid = self.model.new_id();
        self.model.images.push(ImageAsset { id: aid, name: name.to_string(), data: b64::encode(bytes), w, h });
        let width = (w as f32).min(320.0);
        let size = [width, width * h as f32 / w.max(1) as f32];
        let at = pos2(240.0 / self.zoom - self.pan.x, 160.0 / self.zoom - self.pan.y);
        let view = self.diagram_view();
        let sid = self.model.new_id();
        let mut s = Shape::new(sid, view, ShapeKind::Picture, [at.x, at.y], size);
        s.image = Some(aid);
        self.model.shapes.push(s);
        self.select_one(Some(Sel::Shape(sid)));
        self.changed();
        Ok(())
    }

    /// Ask for a picture file and add it.
    pub fn add_picture(&mut self) {
        let Some(path) = rfd::FileDialog::new().add_filter("Picture", &["png", "jpg", "jpeg"]).pick_file() else { return };
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "picture".into());
        match std::fs::read(&path) {
            Ok(bytes) => match self.add_picture_bytes(&name, &bytes) {
                Ok(()) => self.status = format!("Added {name}"),
                Err(e) => self.status = format!("Could not add {name}: {e}"),
            },
            Err(e) => self.status = format!("Could not read {name}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::headless::Headless;
    use dynamist_core::model::Model;

    /// A window with the example model and a few shapes, so the shape code can be driven directly.
    fn app_with_shapes() -> (egui::Context, App, [u64; 4]) {
        crate::app::HEADLESS.store(true, std::sync::atomic::Ordering::Relaxed);
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx, None);
        let mut m = Model::example();
        let view = m.first_diagram();
        let mut ids = [0u64; 4];
        for (i, (kind, pos, size, fill)) in [
            (ShapeKind::Rect, [0.0, 0.0], [100.0, 60.0], Some([255, 0, 0])),
            (ShapeKind::Rect, [300.0, 0.0], [100.0, 60.0], None),
            (ShapeKind::Ellipse, [0.0, 200.0], [100.0, 60.0], Some([0, 255, 0])),
            (ShapeKind::Line, [0.0, 400.0], [100.0, 0.0], None),
        ]
        .into_iter()
        .enumerate()
        {
            ids[i] = m.new_id();
            let mut s = Shape::new(ids[i], view, kind, pos, size);
            s.fill = fill;
            m.shapes.push(s);
        }
        app.finish_open(m);
        (ctx, app, ids)
    }

    #[test]
    fn what_is_under_the_pointer() {
        let (_c, app, ids) = app_with_shapes();
        // a filled rectangle is hit inside, an empty one only on its outline
        assert_eq!(app.shape_hit(pos2(50.0, 30.0)), Some(ids[0]));
        assert_eq!(app.shape_hit(pos2(350.0, 30.0)), None);
        assert_eq!(app.shape_hit(pos2(300.0, 30.0)), Some(ids[1]));
        // an ellipse is round: its corner is outside
        assert_eq!(app.shape_hit(pos2(50.0, 230.0)), Some(ids[2]));
        assert_eq!(app.shape_hit(pos2(2.0, 202.0)), None);
        // a line within a few points
        assert_eq!(app.shape_hit(pos2(50.0, 403.0)), Some(ids[3]));
        assert_eq!(app.shape_hit(pos2(50.0, 430.0)), None);
    }

    #[test]
    fn pulling_a_corner_keeps_the_opposite_one_and_a_line_end_moves_alone() {
        let (_c, mut app, ids) = app_with_shapes();
        app.drag_shape_handle(ids[0], 2, pos2(150.0, 90.0)); // bottom right
        let s = app.model.shapes.iter().find(|s| s.id == ids[0]).unwrap();
        assert_eq!((s.pos, s.size), ([0.0, 0.0], [150.0, 90.0]));
        app.drag_shape_handle(ids[0], 0, pos2(50.0, 40.0)); // top left, the far corner stays at 150, 90
        let s = app.model.shapes.iter().find(|s| s.id == ids[0]).unwrap();
        assert_eq!((s.pos, s.size), ([50.0, 40.0], [100.0, 50.0]));
        app.drag_shape_handle(ids[3], 0, pos2(20.0, 420.0)); // the start of the line
        let l = app.model.shapes.iter().find(|s| s.id == ids[3]).unwrap();
        assert_eq!(([l.pos[0] + l.size[0], l.pos[1] + l.size[1]]), [100.0, 400.0]);
    }

    #[test]
    fn drawing_moving_copying_and_deleting() {
        let (_c, mut app, ids) = app_with_shapes();
        let before = app.model.shapes.len();
        app.shape_kind = ShapeKind::Ellipse;
        app.create_shape(pos2(500.0, 500.0), pos2(620.0, 580.0));
        assert_eq!(app.model.shapes.len(), before + 1);
        let new = app.model.shapes.last().unwrap().clone();
        assert_eq!((new.pos, new.size, new.kind), ([500.0, 500.0], [120.0, 80.0], ShapeKind::Ellipse));
        // a click with no drag makes the standard size
        app.shape_kind = ShapeKind::Rect;
        app.create_shape(pos2(10.0, 600.0), pos2(10.0, 600.0));
        assert_eq!(app.model.shapes.last().unwrap().size, [140.0, 90.0]);
        // move, copy and paste, delete
        app.select_one(Some(Sel::Shape(ids[0])));
        app.move_item(Sel::Shape(ids[0]), vec2(5.0, 5.0), false);
        assert_eq!(app.model.shapes.iter().find(|s| s.id == ids[0]).unwrap().pos, [5.0, 5.0]);
        assert!(app.copy(false));
        app.paste();
        assert_eq!(app.model.shapes.len(), before + 3);
        app.delete_selection();
        assert_eq!(app.model.shapes.len(), before + 2);
        app.undo();
        assert_eq!(app.model.shapes.len(), before + 3);
    }

    /// A tiny PNG made on the spot.
    fn png(w: u32, h: u32, rgba: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(w, h, image::Rgba(rgba));
        let mut out = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn a_picture_is_kept_in_the_model_and_reaches_the_svg_and_the_png() {
        let (_c, mut app, _) = app_with_shapes();
        assert!(app.add_picture_bytes("red.png", &png(40, 20, [255, 0, 0, 255])).is_ok());
        let s = app.model.shapes.last().unwrap();
        assert_eq!(s.kind, ShapeKind::Picture);
        assert_eq!(s.size, [40.0, 20.0]);
        assert_eq!((app.model.images[0].w, app.model.images[0].h), (40, 20));
        assert!(app.add_picture_bytes("bad.png", b"not a picture").is_err());
        // saved with the model and read back
        let json = serde_json::to_string(&app.model).unwrap();
        let back: Model = serde_json::from_str(&json).unwrap();
        assert_eq!(back.images, app.model.images);
        // through the figure pipeline
        let mut h = Headless::new(back, None);
        let fig = h.sheet(None).unwrap();
        let svg = fig.svg(&Default::default()).unwrap();
        assert!(svg.text.contains("<image ") && svg.text.contains("data:image/png;base64,"), "the picture is missing from the SVG");
        assert!(svg.skipped.callbacks == 0 && svg.skipped.meshes == 0);
        let again = fig.svg(&Default::default()).unwrap();
        assert_eq!(svg.text, again.text, "SVG must be the same every time");
        assert!(fig.png(1.0).unwrap().len() > 1000);
    }
}
