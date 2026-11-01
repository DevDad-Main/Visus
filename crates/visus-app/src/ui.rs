use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode,
    Stroke, Transform, LineCap,
};
use visus_core::{Alert, EyeState, Lyrics, Notch, Page, Rect, Shape, current_index};

use crate::text::{Align, Text};

const GREEN: [u8; 3] = [48, 209, 88];
const BLUE: [u8; 3] = [90, 200, 250];
const ORANGE: [u8; 3] = [255, 159, 10];
const WHITE: [u8; 3] = [255, 255, 255];
const GRAY: [u8; 3] = [160, 160, 168];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Prev,
    PlayPause,
    Next,
    Lyrics,
    TimerToggle,
    TimerReset,
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect { x, y, w, h }
}

/// Clickable regions in window-logical coordinates for the current frame.
pub fn buttons(notch: &Notch) -> Vec<(Button, Rect)> {
    let s = notch.shape();
    if !notch.is_open() || s.rect.h < 200.0 {
        return Vec::new();
    }
    let (x0, cx) = (s.rect.x, s.rect.x + s.rect.w / 2.0);
    match notch.page {
        Page::Music if notch.lyrics_open() => {
            let b = |dx: f32| rect(x0 + dx - 24.0, 188.0, 48.0, 40.0);
            vec![
                (Button::Lyrics, b(60.0)),
                (Button::Prev, b(140.0)),
                (Button::PlayPause, b(200.0)),
                (Button::Next, b(260.0)),
            ]
        }
        Page::Music => vec![
            (Button::Lyrics, rect(x0 + 70.0 - 24.0, 190.0, 48.0, 40.0)),
            (Button::Prev, rect(cx - 84.0 - 24.0, 190.0, 48.0, 40.0)),
            (Button::PlayPause, rect(cx - 24.0, 190.0, 48.0, 40.0)),
            (Button::Next, rect(cx + 84.0 - 24.0, 190.0, 48.0, 40.0)),
        ],
        Page::Timer => vec![
            (Button::TimerToggle, rect(x0 + 330.0, 142.0, 150.0, 44.0)),
            (Button::TimerReset, rect(x0 + 496.0, 142.0, 100.0, 44.0)),
        ],
        _ => Vec::new(),
    }
}

/// Radius of the concave fillets where the pill melts into the top edge of the screen.
const FLARE: f32 = 14.0;

/// Horizontal overhang of the flares on each side; used to size the input region.
pub fn flare(s: &Shape) -> f32 {
    let r = s.bottom_radius.min(s.rect.w / 2.0).min(s.rect.h);
    FLARE.min((s.rect.h - r).max(0.0))
}

pub fn pill_path(s: &Shape) -> Option<Path> {
    const K: f32 = 0.5523;
    let r = s.bottom_radius.min(s.rect.w / 2.0).min(s.rect.h);
    let f = flare(s);
    let (x, y, w, h) = (s.rect.x, s.rect.y, s.rect.w, s.rect.h);
    let mut p = PathBuilder::new();
    p.move_to(x - f, y);
    p.line_to(x + w + f, y);
    p.cubic_to(x + w + f - f * K, y, x + w, y + f - f * K, x + w, y + f);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - r + r * K, x + w - r + r * K, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + r - r * K, y + h, x, y + h - r + r * K, x, y + h - r);
    p.line_to(x, y + f);
    p.cubic_to(x, y + f - f * K, x - f + f * K, y, x - f, y);
    p.close();
    p.finish()
}

fn rrect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    const K: f32 = 0.5523;
    let k = r * (1.0 - K);
    let mut p = PathBuilder::new();
    p.move_to(x + r, y);
    p.line_to(x + w - r, y);
    p.cubic_to(x + w - k, y, x + w, y + k, x + w, y + r);
    p.line_to(x + w, y + h - r);
    p.cubic_to(x + w, y + h - k, x + w - k, y + h, x + w - r, y + h);
    p.line_to(x + r, y + h);
    p.cubic_to(x + k, y + h, x, y + h - k, x, y + h - r);
    p.line_to(x, y + r);
    p.cubic_to(x, y + k, x + k, y, x + r, y);
    p.close();
    p.finish()
}

fn color(rgb: [u8; 3], a: f32) -> [u8; 4] {
    [rgb[0], rgb[1], rgb[2], (a.clamp(0.0, 1.0) * 255.0) as u8]
}

struct Ctx<'a> {
    pm: &'a mut Pixmap,
    text: &'a Text,
    scale: f32,
    /// Physical-pixel x offset so scaling happens around the pill's centre line.
    ox: f32,
}

impl Ctx<'_> {
    fn xf(&self) -> Transform {
        Transform::from_row(self.scale, 0.0, 0.0, self.scale, self.ox, 0.0)
    }

    fn paint(c: [u8; 4]) -> Paint<'static> {
        let mut p = Paint::default();
        p.set_color(Color::from_rgba8(c[0], c[1], c[2], c[3]));
        p.anti_alias = true;
        p
    }

    fn fill(&mut self, path: &Path, c: [u8; 4]) {
        let t = self.xf();
        self.pm.fill_path(path, &Self::paint(c), FillRule::Winding, t, None);
    }

    fn stroke(&mut self, path: &Path, c: [u8; 4], width: f32) {
        let t = self.xf();
        let s = Stroke { width, line_cap: LineCap::Round, ..Default::default() };
        self.pm.stroke_path(path, &Self::paint(c), &s, t, None);
    }

    fn circle(&mut self, x: f32, y: f32, r: f32, c: [u8; 4]) {
        if let Some(p) = PathBuilder::from_circle(x, y, r) {
            self.fill(&p, c);
        }
    }

    fn rrect(&mut self, r: Rect, radius: f32, c: [u8; 4]) {
        if let Some(p) = rrect(r.x, r.y, r.w, r.h, radius) {
            self.fill(&p, c);
        }
    }

    /// Ring with a progress arc starting at 12 o'clock.
    fn ring(&mut self, x: f32, y: f32, r: f32, width: f32, progress: f32, rgb: [u8; 3], a: f32) {
        if let Some(p) = PathBuilder::from_circle(x, y, r) {
            self.stroke(&p, color(WHITE, 0.16 * a), width);
        }
        if progress <= 0.001 {
            return;
        }
        let steps = (progress * 72.0).ceil().max(2.0) as i32;
        let mut pb = PathBuilder::new();
        for i in 0..=steps {
            let ang = -std::f32::consts::FRAC_PI_2 + progress * std::f32::consts::TAU * i as f32 / steps as f32;
            let (px, py) = (x + r * ang.cos(), y + r * ang.sin());
            if i == 0 { pb.move_to(px, py) } else { pb.line_to(px, py) }
        }
        if let Some(p) = pb.finish() {
            self.stroke(&p, color(rgb, a), width);
        }
    }

    fn t(&mut self, s: &str, x: f32, y: f32, px: f32, c: [u8; 4], align: Align) {
        let k = self.scale;
        self.text.draw(self.pm, s, x * k + self.ox, y * k, px * k, c, align);
    }

    fn text_width(&self, s: &str, px: f32) -> f32 {
        self.text.width(s, px * self.scale) / self.scale
    }

    fn fit(&self, s: &str, px: f32, max: f32) -> String {
        self.text.fit(s, px * self.scale, max * self.scale)
    }

    fn triangle(&mut self, pts: [(f32, f32); 3], c: [u8; 4]) {
        let mut pb = PathBuilder::new();
        pb.move_to(pts[0].0, pts[0].1);
        pb.line_to(pts[1].0, pts[1].1);
        pb.line_to(pts[2].0, pts[2].1);
        pb.close();
        if let Some(p) = pb.finish() {
            self.fill(&p, c);
        }
    }
}

fn mmss(d: std::time::Duration) -> String {
    let s = d.as_secs();
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn hue_rgb(h: f32) -> [u8; 3] {
    let k = |n: f32| {
        let kk = (n + h * 6.0) % 6.0;
        (0.62 - 0.45 * (kk.min(4.0 - kk)).clamp(0.0, 1.0)).clamp(0.0, 1.0)
    };
    [(k(5.0) * 255.0) as u8, (k(3.0) * 255.0) as u8, (k(1.0) * 255.0) as u8]
}

fn hash(s: &str) -> u32 {
    s.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

pub fn draw(pm: &mut Pixmap, text: &Text, notch: &Notch, scale: f32) {
    let shape = notch.shape();
    let user = notch.settings().scale;
    let centre = shape.rect.x + shape.rect.w / 2.0;
    let mut cx = Ctx { pm, text, scale: scale * user, ox: centre * (1.0 - user) * scale };

    if let Some(path) = pill_path(&shape) {
        cx.fill(&path, [0, 0, 0, 255]);
    }

    let r = shape.rect;
    let ch = notch.settings().closed_height;
    let closed_a = (1.0 - (r.h - ch) / 20.0).clamp(0.0, 1.0);
    let peek_a = ((r.h - (ch + 12.0)) / 24.0).clamp(0.0, 1.0) * (1.0 - ((r.h - 100.0) / 60.0).clamp(0.0, 1.0));
    let open_a = ((r.h - 150.0) / 70.0).clamp(0.0, 1.0);

    if closed_a > 0.0 {
        if notch.eye_state() == EyeState::Hidden {
            draw_closed(&mut cx, notch, closed_a);
        } else {
            draw_eye(&mut cx, notch, closed_a);
        }
    }
    if let (Some(alert), true) = (notch.alert_kind(), peek_a > 0.0) {
        draw_alert(&mut cx, &shape, alert, peek_a);
    }
    if open_a > 0.0 {
        draw_open(&mut cx, notch, open_a);
    }
}

/// An almond-shaped eye with iris, pupil and highlight; blinks, looks around and dozes off.
fn draw_eye(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let fade = notch.eye_fade() * a;
    let asleep = notch.eye_state() == EyeState::Asleep;
    let (lx, ly) = notch.eye_look();
    let ey = r.h * 0.23;
    let ex = ey * 1.9;
    let (x, y) = (r.x + r.w / 2.0 + lx * 5.0, r.h / 2.0 + ly * 1.5);
    let open = notch.eye_openness();
    let iris_rgb = notch.settings().eye_iris;
    let sclera = notch.settings().eye_sclera.0;
    let [gr, gg, gb] = iris_rgb.0;

    // Soft glow behind the eye, kept inside the pill.
    let glow_r = r.h * 0.5;
    let glow = RadialGradient::new(
        Point::from_xy(x, y),
        Point::from_xy(x, y),
        glow_r,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(gr, gg, gb, (fade * if asleep { 40.0 } else { 130.0 }) as u8)),
            GradientStop::new(1.0, Color::from_rgba8(gr, gg, gb, 0)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    if let (Some(shader), Some(path)) = (glow, PathBuilder::from_circle(x, y, glow_r)) {
        let mut p = Paint::default();
        p.shader = shader;
        p.anti_alias = true;
        let t = cx.xf();
        cx.pm.fill_path(&path, &p, FillRule::Winding, t, None);
    }

    // A closed lid is a downward arc; a cubic with both controls at h peaks at 0.75h.
    if asleep || open < 0.12 {
        let mut pb = PathBuilder::new();
        pb.move_to(x - ex, y - 1.0);
        pb.cubic_to(x - ex * 0.45, y + ey * 0.9, x + ex * 0.45, y + ey * 0.9, x + ex, y - 1.0);
        if let Some(lid) = pb.finish() {
            cx.stroke(&lid, color(WHITE, fade * if asleep { 0.55 } else { 0.9 }), 2.2);
        }
        if asleep {
            cx.t("z", x + ex + 8.0, y - 3.0, 11.0, color(WHITE, fade * 0.5), Align::Left);
            cx.t("z", x + ex + 16.0, y - 10.0, 8.0, color(WHITE, fade * 0.35), Align::Left);
        }
        return;
    }

    let h = ey * open / 0.75;
    let mut pb = PathBuilder::new();
    pb.move_to(x - ex, y);
    pb.cubic_to(x - ex * 0.45, y - h, x + ex * 0.45, y - h, x + ex, y);
    pb.cubic_to(x + ex * 0.45, y + h, x - ex * 0.45, y + h, x - ex, y);
    pb.close();
    let Some(almond) = pb.finish() else { return };
    cx.fill(&almond, color(sclera, fade));

    // Iris, pupil and highlight are clipped to the almond so lids cover them while blinking.
    let Some(mut mask) = tiny_skia::Mask::new(cx.pm.width(), cx.pm.height()) else { return };
    mask.fill_path(&almond, FillRule::Winding, true, cx.xf());
    let light = iris_rgb.lighten(0.55);
    let ir = ey * 0.95;
    let (ix, iy) = (x + lx * (ex - ir) * 0.9, y + ly * ey * 0.25);
    let iris = RadialGradient::new(
        Point::from_xy(ix, iy),
        Point::from_xy(ix, iy),
        ir,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(light[0], light[1], light[2], (fade * 255.0) as u8)),
            GradientStop::new(1.0, Color::from_rgba8(gr, gg, gb, (fade * 255.0) as u8)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    let t = cx.xf();
    if let (Some(shader), Some(c)) = (iris, PathBuilder::from_circle(ix, iy, ir)) {
        let mut p = Paint::default();
        p.shader = shader;
        p.anti_alias = true;
        cx.pm.fill_path(&c, &p, FillRule::Winding, t, Some(&mask));
    }
    let mut ink = Paint::default();
    ink.anti_alias = true;
    ink.set_color(Color::from_rgba8(0, 0, 0, (fade * 255.0) as u8));
    if let Some(c) = PathBuilder::from_circle(ix, iy, ir * 0.45) {
        cx.pm.fill_path(&c, &ink, FillRule::Winding, t, Some(&mask));
    }
    ink.set_color(Color::from_rgba8(255, 255, 255, (fade * 230.0) as u8));
    if let Some(c) = PathBuilder::from_circle(ix - ir * 0.3, iy - ir * 0.35, ir * 0.17) {
        cx.pm.fill_path(&c, &ink, FillRule::Winding, t, Some(&mask));
    }
}

fn draw_closed(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let mid = r.y + r.h / 2.0;
    let timer = notch.timer();
    let progress = if timer.is_running() { 1.0 - timer.progress() } else { 1.0 };
    cx.ring(r.x + 28.0, mid, 8.0, 2.5, progress, GREEN, a);
    let clock = chrono::Local::now().format("%H:%M").to_string();
    cx.t(&clock, r.x + r.w / 2.0, mid + 7.0, 20.0, color(WHITE, a), Align::Center);
    // Equalizer glyph; flat when nothing plays.
    let heights: [f32; 4] = if notch.media().playing { [8.0, 14.0, 10.0, 6.0] } else { [5.0, 5.0, 5.0, 5.0] };
    for (i, h) in heights.iter().enumerate() {
        let x = r.x + r.w - 38.0 + i as f32 * 6.0;
        cx.rrect(rect(x, mid - h / 2.0, 2.5, *h), 1.25, color(WHITE, a));
    }
}

fn draw_alert(cx: &mut Ctx, shape: &Shape, alert: Alert, a: f32) {
    let r = shape.rect;
    let mid = r.h / 2.0;
    let (ix, rx) = (r.x + 46.0, r.x + r.w - 36.0);
    let (title, sub, accent, ring, value): (&str, String, [u8; 3], f32, Option<String>) = match alert {
        Alert::Charging { percent } => ("Charging", "Charger connected".into(), GREEN, percent as f32 / 100.0, Some(format!("{percent}%"))),
        Alert::Unplugged { percent } => ("On battery", "Charger disconnected".into(), ORANGE, percent as f32 / 100.0, Some(format!("{percent}%"))),
        Alert::Headphones => ("Headphones", "Connected".into(), BLUE, 1.0, None),
        Alert::CapsLock(on) => ("Caps Lock", if on { "On" } else { "Off" }.into(), if on { ORANGE } else { GRAY }, if on { 1.0 } else { 0.0 }, None),
        Alert::Volume { level, muted } => {
            let pct = (level * 100.0).round() as i32;
            ("Volume", if muted { "Muted".into() } else { format!("{pct}%") }, BLUE, if muted { 0.0 } else { level }, None)
        }
        Alert::TimerDone => ("Focus complete", "Time for a break".into(), GREEN, 1.0, None),
    };
    cx.ring(ix, mid, 16.0, 3.5, ring, accent, a);
    cx.t(title, r.x + 88.0, mid - 2.0, 23.0, color(WHITE, a), Align::Left);
    cx.t(&sub, r.x + 88.0, mid + 22.0, 17.0, color(GRAY, a), Align::Left);
    if let Some(v) = value {
        cx.t(&v, rx, mid + 13.0, 34.0, color(WHITE, a), Align::Right);
    }
}

fn draw_open(cx: &mut Ctx, notch: &Notch, a: f32) {
    let shape = notch.shape();
    let r = shape.rect;
    let mid = r.x + r.w / 2.0;

    let now = chrono::Local::now();
    let header = format!("{}  {}", now.format("%a %-d %b %Y"), now.format("%H:%M"));
    cx.t(&header, mid, 44.0, 22.0, color(WHITE, a), Align::Center);

    match notch.page {
        Page::Music => draw_music(cx, notch, a),
        Page::Timer => draw_timer(cx, notch, a),
        Page::Stats => draw_stats(cx, notch, a),
        p => {
            let name = match p {
                Page::Shelf => "Shelf",
                _ => "To-do",
            };
            cx.t(name, mid, 118.0, 32.0, color(WHITE, a), Align::Center);
            cx.t("Coming soon", mid, 150.0, 20.0, color(GRAY, a), Align::Center);
        }
    }

    let n = Page::ALL.len() as f32;
    let y = r.h - 12.0;
    for (i, p) in Page::ALL.iter().enumerate() {
        let active = *p == notch.page;
        let x = mid + (i as f32 - (n - 1.0) / 2.0) * 14.0;
        if active {
            cx.rrect(rect(x - 6.0, y - 3.0, 12.0, 6.0), 3.0, color(WHITE, a));
        } else {
            cx.circle(x, y, 3.0, color(WHITE, 0.4 * a));
        }
    }
}

fn draw_music(cx: &mut Ctx, notch: &Notch, a: f32) {
    if notch.lyrics_open() {
        draw_music_lyrics(cx, notch, a);
        return;
    }
    let r = notch.shape().rect;
    let (x0, mid) = (r.x, r.x + r.w / 2.0);
    let m = notch.media();

    draw_art(cx, rect(x0 + 39.0, 62.0, 96.0, 96.0), 18.0, m, a);

    let (title, artist) = if m.has_session && !m.title.is_empty() {
        (m.title.clone(), m.artist.clone())
    } else {
        ("Nothing playing".to_string(), "Play something in any player".to_string())
    };
    let tx = x0 + 160.0;
    let max = r.w - 160.0 - 60.0;
    let t = cx.fit(&title, 28.0, max);
    cx.t(&t, tx, 100.0, 28.0, color(WHITE, a), Align::Left);
    let ar = cx.fit(&artist, 20.0, max);
    cx.t(&ar, tx, 128.0, 20.0, color(GRAY, a), Align::Left);

    // Progress
    let pos = notch.media_position();
    let frac = if m.duration.is_zero() { 0.0 } else { (pos.as_secs_f32() / m.duration.as_secs_f32()).clamp(0.0, 1.0) };
    let (bx, bw, by) = (x0 + 136.0 + 24.0, 400.0, 172.0);
    if m.has_session {
        cx.t(&mmss(pos), x0 + 44.0, by + 7.0, 20.0, color(WHITE, a), Align::Left);
    }
    cx.rrect(rect(bx, by - 2.5, bw, 5.0), 2.5, color(WHITE, 0.25 * a));
    if frac > 0.0 {
        cx.rrect(rect(bx, by - 2.5, (bw * frac).max(5.0), 5.0), 2.5, color(WHITE, a));
    }
    if m.has_session {
        let left = m.duration.saturating_sub(pos);
        cx.t(&format!("-{}", mmss(left)), r.x + r.w - 44.0, by + 7.0, 20.0, color(WHITE, a), Align::Right);
    }

    // Transport
    let cy = 210.0;
    let white = color(WHITE, a);
    for (dx, dir) in [(-84.0, -1.0), (84.0, 1.0)] {
        let x = mid + dx;
        for off in [-7.0, 5.0] {
            let b = x + off;
            cx.triangle([(b - 6.0 * dir, cy - 9.0), (b - 6.0 * dir, cy + 9.0), (b + 6.0 * dir, cy)], white);
        }
    }
    if m.playing {
        cx.rrect(rect(mid - 8.0, cy - 10.0, 5.0, 20.0), 1.5, white);
        cx.rrect(rect(mid + 3.0, cy - 10.0, 5.0, 20.0), 1.5, white);
    } else {
        cx.triangle([(mid - 7.0, cy - 11.0), (mid - 7.0, cy + 11.0), (mid + 10.0, cy)], white);
    }
    draw_lyrics_icon(cx, x0 + 70.0, cy, false, a);
}

/// Speech bubble with two dots; filled when the lyrics pane is open.
fn draw_lyrics_icon(cx: &mut Ctx, x: f32, y: f32, active: bool, a: f32) {
    let c = color(WHITE, a);
    let body = rect(x - 11.0, y - 9.0, 22.0, 16.0);
    if active {
        cx.rrect(body, 5.0, c);
    } else if let Some(p) = rrect(body.x, body.y, body.w, body.h, 5.0) {
        cx.stroke(&p, c, 1.8);
    }
    cx.triangle([(x - 6.0, y + 6.0), (x - 1.0, y + 6.0), (x - 6.0, y + 12.0)], c);
    let dot = if active { color([0, 0, 0], a) } else { c };
    cx.circle(x - 4.0, y - 1.0, 1.7, dot);
    cx.circle(x + 4.0, y - 1.0, 1.7, dot);
}

fn draw_music_lyrics(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let x0 = r.x;
    let m = notch.media();
    let white = color(WHITE, a);

    // Left column: compact now-playing.
    draw_art(cx, rect(x0 + 36.0, 62.0, 72.0, 72.0), 14.0, m, a);
    let (title, artist) = if m.has_session && !m.title.is_empty() {
        (m.title.clone(), m.artist.clone())
    } else {
        ("Nothing playing".to_string(), String::new())
    };
    let t = cx.fit(&title, 21.0, 210.0);
    cx.t(&t, x0 + 124.0, 92.0, 21.0, white, Align::Left);
    let ar = cx.fit(&artist, 16.0, 210.0);
    cx.t(&ar, x0 + 124.0, 116.0, 16.0, color(GRAY, a), Align::Left);

    let pos = notch.media_position();
    let frac = if m.duration.is_zero() { 0.0 } else { (pos.as_secs_f32() / m.duration.as_secs_f32()).clamp(0.0, 1.0) };
    let (bx, bw, by) = (x0 + 36.0, 300.0, 162.0);
    cx.rrect(rect(bx, by - 2.0, bw, 4.0), 2.0, color(WHITE, 0.25 * a));
    if frac > 0.0 {
        cx.rrect(rect(bx, by - 2.0, (bw * frac).max(4.0), 4.0), 2.0, white);
    }
    if m.has_session {
        cx.t(&mmss(pos), bx, by + 20.0, 14.0, color(GRAY, a), Align::Left);
        cx.t(&format!("-{}", mmss(m.duration.saturating_sub(pos))), bx + bw, by + 20.0, 14.0, color(GRAY, a), Align::Right);
    }

    let cy = 208.0;
    for (dx, dir) in [(140.0, -1.0), (260.0, 1.0)] {
        let x = x0 + dx;
        for off in [-7.0, 5.0] {
            let b = x + off;
            cx.triangle([(b - 6.0 * dir, cy - 8.0), (b - 6.0 * dir, cy + 8.0), (b + 6.0 * dir, cy)], white);
        }
    }
    let px = x0 + 200.0;
    if m.playing {
        cx.rrect(rect(px - 8.0, cy - 10.0, 5.0, 20.0), 1.5, white);
        cx.rrect(rect(px + 3.0, cy - 10.0, 5.0, 20.0), 1.5, white);
    } else {
        cx.triangle([(px - 7.0, cy - 11.0), (px - 7.0, cy + 11.0), (px + 10.0, cy)], white);
    }
    draw_lyrics_icon(cx, x0 + 60.0, cy, true, a);

    // Divider and lyric pane.
    cx.rrect(rect(x0 + 368.0, 58.0, 1.0, 150.0), 0.5, color(WHITE, 0.15 * a));
    let (lx, lw) = (x0 + 392.0, r.w - 392.0 - 28.0);
    let note = |cx: &mut Ctx, s: &str| cx.t(s, lx + lw / 2.0, 138.0, 19.0, color(GRAY, a), Align::Center);
    match notch.lyrics() {
        Lyrics::Ready(lines) => {
            let idx = current_index(lines, pos);
            // Before the first line, show the opening lines as upcoming.
            let focus = idx.map_or(-1, |i| i as i32);
            // One line behind, the current line, then three upcoming; baselines sit below the header.
            for off in -1..=3i32 {
                let i = focus + off;
                let Some(line) = usize::try_from(i).ok().and_then(|i| lines.get(i)) else { continue };
                let alpha = [0.4, 1.0, 0.55, 0.35, 0.2][(off + 1) as usize];
                let mut size = if off == 0 { 23.0 } else { 18.0 };
                let text = if line.text.is_empty() { "\u{266a}" } else { line.text.as_str() };
                // Shrink a little before resorting to an ellipsis.
                while size > 15.0 && cx.text_width(text, size) > lw {
                    size -= 1.0;
                }
                let fitted = cx.fit(text, size, lw);
                let y = [96.0, 130.0, 164.0, 194.0, 222.0][(off + 1) as usize];
                cx.t(&fitted, lx, y, size, color(WHITE, alpha * a), Align::Left);
            }
        }
        Lyrics::Loading => note(cx, "Searching for lyrics\u{2026}"),
        Lyrics::NotFound => note(cx, "No synced lyrics found"),
        Lyrics::None => note(cx, "Play something to see lyrics"),
    }
}

fn draw_timer(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let x0 = r.x;
    let t = notch.timer();
    cx.ring(x0 + 170.0, 120.0, 52.0, 9.0, 1.0 - t.progress(), GREEN, a);
    cx.t(&mmss(t.remaining()), x0 + 170.0, 130.0, 30.0, color(WHITE, a), Align::Center);

    cx.t("Focus", x0 + 330.0, 98.0, 30.0, color(WHITE, a), Align::Left);
    let mins = t.total().as_secs() / 60;
    cx.t(&format!("{mins} minute session"), x0 + 330.0, 124.0, 19.0, color(GRAY, a), Align::Left);

    for (b, rc) in buttons(notch) {
        let label = match b {
            Button::TimerToggle => if t.is_running() { "Pause" } else { "Start" },
            Button::TimerReset => "Reset",
            _ => continue,
        };
        let fill = if b == Button::TimerToggle { color(GREEN, 0.9 * a) } else { color(WHITE, 0.14 * a) };
        cx.rrect(rc, 22.0, fill);
        let ink = if b == Button::TimerToggle { color([0, 0, 0], a) } else { color(WHITE, a) };
        cx.t(label, rc.x + rc.w / 2.0, rc.y + 29.0, 20.0, ink, Align::Center);
    }
}

/// Handles a left click at window-logical coordinates; returns true if a button was hit.
pub fn click(notch: &mut Notch, x: f32, y: f32) -> bool {
    use visus_core::MediaCommand;
    for (b, r) in buttons(notch) {
        if r.contains(x, y) {
            match b {
                Button::Prev => visus_platform::media_command(MediaCommand::Previous),
                Button::PlayPause => visus_platform::media_command(MediaCommand::PlayPause),
                Button::Next => visus_platform::media_command(MediaCommand::Next),
                Button::Lyrics => notch.toggle_lyrics(),
                Button::TimerToggle => notch.toggle_timer(),
                Button::TimerReset => notch.reset_timer(),
            }
            return true;
        }
    }
    false
}

/// How long the app may sleep while nothing animates: the next clock minute or content refresh.
pub fn idle_wake(notch: &Notch) -> Option<std::time::Duration> {
    use std::time::Duration;
    let mut wake = notch.refresh_interval();
    if !notch.is_hidden() {
        let secs = chrono::Timelike::second(&chrono::Local::now()) as u64;
        let minute = Duration::from_secs(60 - secs) + Duration::from_millis(20);
        wake = Some(wake.map_or(minute, |w| w.min(minute)));
    }
    wake
}

/// Window-logical position -> pill-logical position (undoes the user size scale).
pub fn to_logical(notch: &Notch, x: f32, y: f32) -> (f32, f32) {
    let sh = notch.shape().rect;
    let centre = sh.x + sh.w / 2.0;
    let k = notch.settings().scale;
    (centre + (x - centre) / k, y / k)
}

/// Area the pill (including flares) covers on screen, in window-logical pixels.
pub fn hit_rect(notch: &Notch) -> Rect {
    let s = notch.shape();
    let f = flare(&s);
    let k = notch.settings().scale;
    let centre = s.rect.x + s.rect.w / 2.0;
    let half = s.rect.w / 2.0 + f;
    Rect { x: centre - half * k, y: 0.0, w: half * 2.0 * k, h: s.rect.h * k }
}

fn rate(bytes_per_sec: u64) -> String {
    let b = bytes_per_sec as f64;
    if b >= 1024.0 * 1024.0 {
        format!("{:.1} MB/s", b / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB/s", b / 1024.0)
    }
}

fn draw_stats(cx: &mut Ctx, notch: &Notch, a: f32) {
    let x0 = notch.shape().rect.x;
    let st = notch.stats();
    let gauges = [
        ("CPU", st.cpu, format!("{:.0}%", st.cpu * 100.0), GREEN, x0 + 130.0),
        ("Memory", st.ram, format!("{:.0}%", st.ram * 100.0), BLUE, x0 + 300.0),
    ];
    for (label, v, text, rgb, x) in gauges {
        cx.ring(x, 122.0, 42.0, 8.0, v, rgb, a);
        cx.t(&text, x, 131.0, 26.0, color(WHITE, a), Align::Center);
        cx.t(label, x, 198.0, 18.0, color(GRAY, a), Align::Center);
    }
    let detail = format!("{:.1} / {:.1} GB", st.ram_used_gb, st.ram_total_gb);
    cx.t(&detail, x0 + 300.0, 218.0, 15.0, color(GRAY, 0.8 * a), Align::Center);

    cx.t("Network", x0 + 480.0, 98.0, 18.0, color(GRAY, a), Align::Left);
    cx.t(&format!("\u{2193} {}", rate(st.rx_bytes_per_sec)), x0 + 480.0, 138.0, 26.0, color(WHITE, a), Align::Left);
    cx.t(&format!("\u{2191} {}", rate(st.tx_bytes_per_sec)), x0 + 480.0, 174.0, 26.0, color(WHITE, 0.75 * a), Align::Left);
}

thread_local! {
    /// Premultiplied copy of the current cover, rebuilt only when the cover changes.
    static ART_PIXMAP: std::cell::RefCell<Option<(u64, Pixmap)>> = const { std::cell::RefCell::new(None) };
}

/// Cover art in a rounded square, or a gradient seeded from the title when there is none.
fn draw_art(cx: &mut Ctx, r: Rect, radius: f32, m: &visus_core::MediaInfo, a: f32) {
    let Some(path) = rrect(r.x, r.y, r.w, r.h, radius) else { return };
    let xf = cx.xf();

    if let Some(art) = m.art.as_ref() {
        ART_PIXMAP.with(|cell| {
            let mut cell = cell.borrow_mut();
            if cell.as_ref().is_none_or(|(id, _)| *id != art.id) {
                let Some(mut pm) = Pixmap::new(art.size, art.size) else { return };
                for (dst, src) in pm.pixels_mut().iter_mut().zip(art.rgba.chunks_exact(4)) {
                    let al = src[3] as u32;
                    let mul = |c: u8| ((c as u32 * al + 127) / 255) as u8;
                    if let Some(p) = tiny_skia::PremultipliedColorU8::from_rgba(mul(src[0]), mul(src[1]), mul(src[2]), src[3]) {
                        *dst = p;
                    }
                }
                *cell = Some((art.id, pm));
            }
            let Some((_, pm)) = cell.as_ref() else { return };
            let k = r.w / art.size as f32;
            let pattern = tiny_skia::Pattern::new(
                pm.as_ref(),
                SpreadMode::Pad,
                tiny_skia::FilterQuality::Bilinear,
                a,
                Transform::from_row(k, 0.0, 0.0, k, r.x, r.y),
            );
            let mut p = Paint::default();
            p.shader = pattern;
            p.anti_alias = true;
            cx.pm.fill_path(&path, &p, FillRule::Winding, xf, None);
        });
        return;
    }

    let h = if m.has_session { (hash(&m.title) % 360) as f32 / 360.0 } else { 0.62 };
    let (c1, c2) = (hue_rgb(h), hue_rgb((h + 0.12) % 1.0));
    let grad = LinearGradient::new(
        Point::from_xy(r.x, r.y),
        Point::from_xy(r.x + r.w, r.y + r.h),
        vec![
            GradientStop::new(0.0, Color::from_rgba8(c1[0], c1[1], c1[2], (a * 255.0) as u8)),
            GradientStop::new(1.0, Color::from_rgba8(c2[0], c2[1], c2[2], (a * 255.0) as u8)),
        ],
        SpreadMode::Pad,
        Transform::identity(),
    );
    if let Some(shader) = grad {
        let mut p = Paint::default();
        p.shader = shader;
        p.anti_alias = true;
        cx.pm.fill_path(&path, &p, FillRule::Winding, xf, None);
    }
}
