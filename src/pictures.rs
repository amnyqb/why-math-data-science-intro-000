//! Pictures on a diagram sheet: decoding the stored file, the texture the window draws, and the
//! stand-in the exporters read (they cannot read a texture back).

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Context, TextureHandle, TextureId};
use dynamist_core::b64;
use dynamist_core::model::ImageAsset;

/// A picture as pixels, straight (not premultiplied) RGBA, plus the same re-encoded as a PNG for SVG.
pub struct Decoded {
    pub w: usize,
    pub h: usize,
    pub rgba: Arc<Vec<u8>>,
    pub png: Arc<Vec<u8>>,
}

/// What an exported sheet carries in place of a texture (egui's `PaintCallback` holds any value).
pub struct PicCallback {
    pub rgba: Arc<Vec<u8>>,
    pub w: usize,
    pub h: usize,
    pub png: Arc<Vec<u8>>,
}

#[derive(Default)]
pub struct PictureCache {
    /// asset id → (a fingerprint of the file, the decoded picture or why it failed)
    decoded: HashMap<u64, (u64, Result<Arc<Decoded>, String>)>,
    textures: HashMap<u64, (u64, TextureHandle)>,
}

/// A cheap stand-in for hashing the file: length and a few sampled bytes. Models reuse ids, so a cache
/// keyed by id alone would show the wrong picture after another model is opened.
fn fingerprint(a: &ImageAsset) -> u64 {
    let b = a.data.as_bytes();
    let mut h = b.len() as u64;
    let step = (b.len() / 64).max(1);
    for i in (0..b.len()).step_by(step) {
        h = h.wrapping_mul(1099511628211).wrapping_add(b[i] as u64);
    }
    h
}

/// Read a PNG or JPEG file's bytes: (pixel width, height).
pub fn probe(bytes: &[u8]) -> Result<(u32, u32), String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("not a PNG or JPEG picture ({e})"))?;
    Ok((img.width(), img.height()))
}

fn decode(a: &ImageAsset) -> Result<Arc<Decoded>, String> {
    let bytes = b64::decode(&a.data)?;
    let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.to_rgba8();
    let (w, h) = (img.width() as usize, img.height() as usize);
    let rgba = img.into_raw();
    let mut png = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut png, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut wr = enc.write_header().map_err(|e| e.to_string())?;
        wr.write_image_data(&rgba).map_err(|e| e.to_string())?;
    }
    Ok(Arc::new(Decoded { w, h, rgba: Arc::new(rgba), png: Arc::new(png) }))
}

impl PictureCache {
    pub fn get(&mut self, a: &ImageAsset) -> Result<Arc<Decoded>, String> {
        let fp = fingerprint(a);
        match self.decoded.get(&a.id) {
            Some((f, r)) if *f == fp => r.clone(),
            _ => {
                let r = decode(a);
                self.decoded.insert(a.id, (fp, r.clone()));
                r
            }
        }
    }

    /// The texture for the window to draw (made on first use).
    pub fn texture(&mut self, ctx: &Context, a: &ImageAsset) -> Option<TextureId> {
        let fp = fingerprint(a);
        if let Some((f, t)) = self.textures.get(&a.id) {
            if *f == fp {
                return Some(t.id());
            }
        }
        let d = self.get(a).ok()?;
        let img = egui::ColorImage::from_rgba_unmultiplied([d.w, d.h], &d.rgba);
        let t = ctx.load_texture(format!("picture-{}", a.id), img, egui::TextureOptions::LINEAR);
        let id = t.id();
        self.textures.insert(a.id, (fp, t));
        Some(id)
    }
}
