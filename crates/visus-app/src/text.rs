use ab_glyph::{Font, FontRef, ScaleFont, point};
use tiny_skia::Pixmap;

static INTER: &[u8] = include_bytes!("../assets/Inter.ttf");

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

pub struct Text {
    font: FontRef<'static>,
}

impl Text {
    pub fn new() -> Self {
        Self { font: FontRef::try_from_slice(INTER).expect("bundled font") }
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
            let glyph = id.with_scale_and_position(px, point(cx, y));
            cx += f.h_advance(id);
            prev = Some(id);
            let Some(outlined) = self.font.outline_glyph(glyph) else { continue };
            let b = outlined.px_bounds();
            outlined.draw(|gx, gy, cov| {
                let (px_x, px_y) = (b.min.x as i32 + gx as i32, b.min.y as i32 + gy as i32);
                if px_x < 0 || px_y < 0 || px_x >= pw || px_y >= ph {
                    return;
                }
                let a = cov * rgba[3] as f32 / 255.0;
                let dst = &mut pixels[(px_y * pw + px_x) as usize];
                let inv = 1.0 - a;
                let mix = |s: u8, d: u8| (s as f32 * a + d as f32 * inv).round().min(255.0) as u8;
                // Source is premultiplied: colour * alpha.
                let r = mix(rgba[0], dst.red());
                let g = mix(rgba[1], dst.green());
                let bl = mix(rgba[2], dst.blue());
                let al = (255.0 * a + dst.alpha() as f32 * inv).round().min(255.0) as u8;
                if let Some(px) = tiny_skia::PremultipliedColorU8::from_rgba(r.min(al), g.min(al), bl.min(al), al) {
                    *dst = px;
                }
            });
        }
    }
}
