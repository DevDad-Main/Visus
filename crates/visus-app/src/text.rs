use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ab_glyph::{Font, FontRef, GlyphId, ScaleFont, point};
use tiny_skia::Pixmap;

static INTER: &[u8] = include_bytes!("../assets/Inter.ttf");

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Pre-rasterised glyph coverage, offset from the baseline origin.
struct Cached {
    left: i32,
    top: i32,
    w: usize,
    cov: Vec<u8>,
}

pub struct Text {
    font: FontRef<'static>,
    /// Keyed by glyph, size and quarter-pixel horizontal phase.
    cache: RefCell<HashMap<(GlyphId, u32, u8), Option<Rc<Cached>>>>,
}

impl Text {
    pub fn new() -> Self {
        Self { font: FontRef::try_from_slice(INTER).expect("bundled font"), cache: RefCell::new(HashMap::new()) }
    }

    pub fn width(&self, s: &str, px: f32) -> f32 {
        let f = self.font.as_scaled(px);
        let mut w = 0.0;
        let mut prev = None;
        for c in s.chars() {
            let id = f.glyph_id(c);
            if let Some(p) = prev {
                w += f.kern(p, id);
            }
            w += f.h_advance(id);
            prev = Some(id);
        }
        w
    }

    /// Shortens with an ellipsis so the text fits in `max` pixels.
    pub fn vertical_bounds(&self, text: &str, size: f32) -> (f32, f32) {
        let font = self.font.as_scaled(size);
        let mut top = f32::INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        for character in text.chars() {
            let glyph = font.glyph_id(character).with_scale_and_position(size, point(0.0, 0.0));
            if let Some(outline) = self.font.outline_glyph(glyph) {
                let bounds = outline.px_bounds();
                top = top.min(bounds.min.y);
                bottom = bottom.max(bounds.max.y);
            }
        }
        if top.is_finite() { (top, bottom) } else { (-size * 0.7, 0.0) }
    }

    pub fn fit(&self, s: &str, px: f32, max: f32) -> String {
        if self.width(s, px) <= max {
            return s.to_string();
        }
        let mut out: String = s.to_string();
        while !out.is_empty() {
            out.pop();
            let cand = format!("{}…", out.trim_end());
            if self.width(&cand, px) <= max {
                return cand;
            }
        }
        String::new()
    }

    /// Draws `s` with its baseline at `y`. All units are physical pixels.
    pub fn draw(&self, pm: &mut Pixmap, s: &str, x: f32, y: f32, px: f32, rgba: [u8; 4], align: Align) {
        if rgba[3] == 0 {
            return;
        }
        let w = self.width(s, px);
        let mut cx = match align {
            Align::Left => x,
            Align::Center => x - w / 2.0,
            Align::Right => x - w,
        };
        let f = self.font.as_scaled(px);
        let (pw, ph) = (pm.width() as i32, pm.height() as i32);
        let pixels = pm.pixels_mut();
        let mut prev = None;
        for c in s.chars() {
            let id = f.glyph_id(c);
            if let Some(p) = prev {
                cx += f.kern(p, id);
            }
            let (gx0, phase) = (cx.floor(), ((cx - cx.floor()) * 4.0) as u8);
            cx += f.h_advance(id);
            prev = Some(id);
            let Some(g) = self.glyph(id, px, phase) else { continue };
            let (ox, oy) = (gx0 as i32 + g.left, y.round() as i32 + g.top);
            for (i, cov) in g.cov.iter().enumerate() {
                if *cov == 0 {
                    continue;
                }
                let (px_x, px_y) = (ox + (i % g.w) as i32, oy + (i / g.w) as i32);
                if px_x < 0 || px_y < 0 || px_x >= pw || px_y >= ph {
                    continue;
                }
                let a = *cov as f32 / 255.0 * rgba[3] as f32 / 255.0;
                let dst = &mut pixels[(px_y * pw + px_x) as usize];
                let inv = 1.0 - a;
                let mix = |s: u8, d: u8| (s as f32 * a + d as f32 * inv).round().min(255.0) as u8;
                // Source is premultiplied: colour * alpha.
                let r = mix(rgba[0], dst.red());
                let g2 = mix(rgba[1], dst.green());
                let bl = mix(rgba[2], dst.blue());
                let al = (255.0 * a + dst.alpha() as f32 * inv).round().min(255.0) as u8;
                if let Some(px) = tiny_skia::PremultipliedColorU8::from_rgba(r.min(al), g2.min(al), bl.min(al), al) {
                    *dst = px;
                }
            }
        }
    }

    fn glyph(&self, id: GlyphId, px: f32, phase: u8) -> Option<Rc<Cached>> {
        let key = (id, px.to_bits(), phase);
        if let Some(hit) = self.cache.borrow().get(&key) {
            return hit.clone();
        }
        let glyph = id.with_scale_and_position(px, point(phase as f32 / 4.0, 0.0));
        let made = self.font.outline_glyph(glyph).map(|o| {
            let b = o.px_bounds();
            let (w, h) = (b.width().ceil() as usize + 1, b.height().ceil() as usize + 1);
            let mut cov = vec![0u8; w * h];
            o.draw(|x, y, c| {
                if (x as usize) < w && (y as usize) < h {
                    cov[y as usize * w + x as usize] = (c * 255.0) as u8;
                }
            });
            Rc::new(Cached { left: b.min.x as i32, top: b.min.y as i32, w, cov })
        });
        let mut cache = self.cache.borrow_mut();
        if cache.len() > 4000 {
            cache.clear();
        }
        cache.insert(key, made.clone());
        made
    }
}
