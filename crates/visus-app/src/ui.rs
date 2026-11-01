use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode,
    Stroke, Transform, LineCap,
};
use visus_core::{Alert, AppKind, EyeState, EyeStyle, Kind, Lyrics, Notch, Outcome, Page, Rect, Rgb, Shape, UpdateStatus, current_index, weather_label};

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
    Palette,
    TimerToggle,
    TimerReset,
    TimerPreset(u32),
    TimerMinus,
    TimerPlus,
    TodoToggle(usize),
    TodoDelete(usize),
    TodoAdd,
    ShelfOpen(usize),
    ShelfRemove(usize),
    Toggle(Toggle),
    Size(u8),
    EyeColour(u8),
    Style(u8),
    Update,
    Location,
    SettingsOpen,
    SettingsClose,
    EyeTime(i8),
    DriveToggle(u8),
    FolderRemove(usize),
    FolderAdd,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    Minimal,
    Eye,
    Fullscreen,
    Autostart,
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
    let mut btns = page_buttons(notch, x0, cx);
    btns.push((Button::Palette, rect(x0 + s.rect.w - 76.0, 10.0, 44.0, 36.0)));
    btns.push((Button::SettingsOpen, rect(x0 + s.rect.w - 124.0, 10.0, 44.0, 36.0)));
    btns
}

const TODO_ROWS: usize = 4;
const TODO_ROW_H: f32 = 34.0;
const TODO_TOP: f32 = 60.0;

/// Indices of the tasks shown, open tasks first.
fn todo_order(notch: &Notch) -> Vec<usize> {
    let t = notch.todos();
    let mut idx: Vec<usize> = (0..t.len()).filter(|i| !t[*i].done).collect();
    idx.extend((0..t.len()).filter(|i| t[*i].done));
    idx.truncate(TODO_ROWS);
    idx
}

fn todo_buttons(notch: &Notch, x0: f32) -> Vec<(Button, Rect)> {
    let w = notch.shape().rect.w - 72.0;
    let order = todo_order(notch);
    let mut v = Vec::new();
    for (slot, i) in order.iter().enumerate() {
        let y = TODO_TOP + slot as f32 * TODO_ROW_H;
        v.push((Button::TodoToggle(*i), rect(x0 + 36.0, y, w - 44.0, 30.0)));
        v.push((Button::TodoDelete(*i), rect(x0 + 36.0 + w - 40.0, y, 40.0, 30.0)));
    }
    let y = TODO_TOP + TODO_ROWS as f32 * TODO_ROW_H + 2.0;
    v.push((Button::TodoAdd, rect(x0 + 36.0, y, w, 30.0)));
    v
}

fn draw_todo(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let (x0, w) = (r.x, r.w - 72.0);
    let todos = notch.todos();
    for (slot, i) in todo_order(notch).into_iter().enumerate() {
        let t = &todos[i];
        let y = TODO_TOP + slot as f32 * TODO_ROW_H;
        let (bx, by) = (x0 + 52.0, y + 15.0);
        if t.done {
            cx.circle(bx, by, 9.0, color(GREEN, a));
            let mut pb = PathBuilder::new();
            pb.move_to(bx - 4.0, by);
            pb.line_to(bx - 1.0, by + 3.5);
            pb.line_to(bx + 4.5, by - 3.5);
            if let Some(p) = pb.finish() {
                cx.stroke(&p, color([0, 0, 0], a), 2.0);
            }
        } else if let Some(p) = PathBuilder::from_circle(bx, by, 8.5) {
            cx.stroke(&p, color(WHITE, 0.55 * a), 1.8);
        }
        let room = w - 120.0;
        let text = cx.fit(&t.text, 18.0, room);
        let ink = if t.done { color(GRAY, 0.8 * a) } else { color(WHITE, a) };
        cx.t(&text, x0 + 74.0, y + 21.0, 18.0, ink, Align::Left);
        if t.done {
            let tw = cx.text_width(&text, 18.0);
            cx.rrect(rect(x0 + 74.0, y + 15.0, tw, 1.5), 0.7, color(GRAY, 0.8 * a));
        }
        if let Some(d) = t.due {
            cx.t(&format!("{:02}:{:02}", d / 60, d % 60), x0 + 36.0 + w - 52.0, y + 20.0, 14.0, color(ORANGE, a), Align::Right);
        }
        let (dx, dy) = (x0 + 36.0 + w - 20.0, y + 15.0);
        let mut pb = PathBuilder::new();
        pb.move_to(dx - 4.0, dy - 4.0);
        pb.line_to(dx + 4.0, dy + 4.0);
        pb.move_to(dx + 4.0, dy - 4.0);
        pb.line_to(dx - 4.0, dy + 4.0);
        if let Some(p) = pb.finish() {
            cx.stroke(&p, color(GRAY, 0.8 * a), 1.6);
        }
    }

    let y = TODO_TOP + TODO_ROWS as f32 * TODO_ROW_H + 2.0;
    let field = rect(x0 + 36.0, y, w, 30.0);
    match notch.todo_input() {
        Some(input) => {
            cx.rrect(field, 12.0, color(WHITE, 0.12 * a));
            if input.is_empty() {
                cx.t("Type a task\u{2026}  add @14:30 for a reminder", field.x + 14.0, y + 20.0, 15.0, color(GRAY, 0.9 * a), Align::Left);
                cx.rrect(rect(field.x + 12.0, y + 7.0, 1.6, 16.0), 0.8, color(WHITE, a));
            } else {
                let mut shown = input;
                while shown.chars().count() > 1 && cx.text_width(shown, 16.0) > w - 40.0 {
                    shown = &shown[shown.chars().next().map_or(1, char::len_utf8)..];
                }
                cx.t(shown, field.x + 14.0, y + 20.0, 16.0, color(WHITE, a), Align::Left);
                let cxp = field.x + 14.0 + cx.text_width(shown, 16.0) + 1.0;
                cx.rrect(rect(cxp, y + 7.0, 1.6, 16.0), 0.8, color(WHITE, a));
            }
        }
        None => {
            cx.rrect(field, 12.0, color(WHITE, 0.07 * a));
            cx.t("+  Add task", field.x + 14.0, y + 20.0, 15.0, color(GRAY, a), Align::Left);
        }
    }
}

const TIMER_PRESETS: [u32; 6] = [5, 10, 15, 25, 45, 60];

fn timer_buttons(x0: f32) -> Vec<(Button, Rect)> {
    let mut v: Vec<(Button, Rect)> = TIMER_PRESETS
        .iter()
        .enumerate()
        .map(|(i, m)| (Button::TimerPreset(*m), rect(x0 + 330.0 + i as f32 * 58.0, 62.0, 52.0, 30.0)))
        .collect();
    v.push((Button::TimerToggle, rect(x0 + 330.0, 164.0, 150.0, 44.0)));
    v.push((Button::TimerReset, rect(x0 + 496.0, 164.0, 100.0, 44.0)));
    v.push((Button::TimerMinus, rect(x0 + 118.0, 188.0, 34.0, 34.0)));
    v.push((Button::TimerPlus, rect(x0 + 188.0, 188.0, 34.0, 34.0)));
    v
}

fn page_buttons(notch: &Notch, x0: f32, cx: f32) -> Vec<(Button, Rect)> {
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
        Page::Timer => timer_buttons(x0),
        Page::Todo => todo_buttons(notch, x0),
        Page::Shelf => shelf_buttons(notch, x0),
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
    /// Physical-pixel y offset; lets popup content reuse page-style coordinates.
    oy: f32,
}

impl Ctx<'_> {
    fn xf(&self) -> Transform {
        Transform::from_row(self.scale, 0.0, 0.0, self.scale, self.ox, self.oy)
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
        self.text.draw(self.pm, s, x * k + self.ox, y * k + self.oy, px * k, c, align);
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
    let mut cx = Ctx { pm, text, scale: scale * user, ox: centre * (1.0 - user) * scale, oy: 0.0 };

    if let Some(path) = pill_path(&shape) {
        cx.fill(&path, [0, 0, 0, 255]);
    }

    let r = shape.rect;
    let ch = notch.settings().closed_height;
    let closed_a = if notch.settings().minimal { 0.0 } else { (1.0 - (r.h - ch) / 20.0).clamp(0.0, 1.0) };
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
    draw_palette(&mut cx, notch);
    draw_settings_panel(&mut cx, notch);
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
    let sauron = notch.settings().eye_style == EyeStyle::Sauron;
    let [gr, gg, gb] = if sauron { [255, 95, 20] } else { iris_rgb.0 };

    let t = notch.awake_time();
    let flick = if sauron { 0.8 + 0.2 * (t * 11.0).sin().abs() * (t * 5.3).cos().abs().max(0.4) } else { 1.0 };

    // Sauron's gaze: a soft beam sweeping across the inside of the pill like a searchlight.
    if sauron && !asleep {
        let bx = r.x + r.w / 2.0 + (t * 0.8).sin() * r.w * 0.34;
        let beam = LinearGradient::new(
            Point::from_xy(bx - 46.0, 0.0),
            Point::from_xy(bx + 46.0, 0.0),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 120, 30, 0)),
                GradientStop::new(0.5, Color::from_rgba8(255, 130, 40, (fade * 70.0) as u8)),
                GradientStop::new(1.0, Color::from_rgba8(255, 120, 30, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        );
        if let (Some(shader), Some(pill), Some(mut mask)) = (beam, pill_path(&notch.shape()), tiny_skia::Mask::new(cx.pm.width(), cx.pm.height())) {
            mask.fill_path(&pill, FillRule::Winding, true, cx.xf());
            let mut p = Paint::default();
            p.shader = shader;
            if let Some(band) = tiny_skia::Rect::from_xywh(bx - 46.0, 0.0, 92.0, r.h) {
                let t = cx.xf();
                cx.pm.fill_rect(band, &p, t, Some(&mask));
            }
        }
    }

    // Soft glow behind the eye, kept inside the pill.
    let glow_r = r.h * 0.5;
    let glow = RadialGradient::new(
        Point::from_xy(x, y),
        Point::from_xy(x, y),
        glow_r,
        vec![
            GradientStop::new(0.0, Color::from_rgba8(gr, gg, gb, (fade * flick * if asleep { 40.0 } else { 130.0 }) as u8)),
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
            // A closed Sauron lid glows ember-orange instead of going grey.
            let tint = if sauron { [255, 110, 25] } else { WHITE };
            cx.stroke(&lid, color(tint, fade * if asleep { 0.6 } else { 1.0 }), 2.4);
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
    if sauron {
        // A burning lens: bright core fading to dark red at the lids.
        let fire = RadialGradient::new(
            Point::from_xy(x, y),
            Point::from_xy(x, y),
            ex,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 225, 110, (fade * 255.0) as u8)),
                GradientStop::new(0.55, Color::from_rgba8(255, 120, 20, (fade * 255.0) as u8)),
                GradientStop::new(1.0, Color::from_rgba8(140, 20, 0, (fade * 255.0) as u8)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        );
        if let Some(shader) = fire {
            let mut p = Paint::default();
            p.shader = shader;
            p.anti_alias = true;
            let t = cx.xf();
            cx.pm.fill_path(&almond, &p, FillRule::Winding, t, None);
        }
    } else {
        cx.fill(&almond, color(sclera, fade));
    }

    // Iris, pupil and highlight are clipped to the almond so lids cover them while blinking.
    let Some(mut mask) = tiny_skia::Mask::new(cx.pm.width(), cx.pm.height()) else { return };
    mask.fill_path(&almond, FillRule::Winding, true, cx.xf());
    if sauron {
        // Vertical slit pupil that follows the gaze.
        let (px, ry) = (x + lx * ex * 0.35, h * 0.78);
        if let Some(oval) = tiny_skia::Rect::from_xywh(px - ey * 0.1, y - ry, ey * 0.2, ry * 2.0).and_then(PathBuilder::from_oval) {
            let mut ink = Paint::default();
            ink.anti_alias = true;
            ink.set_color(Color::from_rgba8(0, 0, 0, (fade * 255.0) as u8));
            let t = cx.xf();
            cx.pm.fill_path(&oval, &ink, FillRule::Winding, t, Some(&mask));
        }
        return;
    }
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
    let timer_active = timer.is_running() || timer.progress() > 0.0;
    // Left slot: the focus timer while one is active, otherwise the battery level.
    if timer_active {
        let left = timer.remaining().as_secs_f32() / timer.total().as_secs_f32();
        cx.ring(r.x + 28.0, mid, 8.0, 2.5, left, GREEN, a);
    } else if let Some((charging, pct)) = notch.battery() {
        let tint = if charging || pct > 20 { GREEN } else { ORANGE };
        cx.ring(r.x + 28.0, mid, 8.0, 2.5, pct as f32 / 100.0, tint, a);
        if charging {
            cx.triangle([(r.x + 29.5, mid - 4.5), (r.x + 25.0, mid + 0.5), (r.x + 28.5, mid + 0.5)], color(WHITE, a));
            cx.triangle([(r.x + 26.5, mid + 4.5), (r.x + 31.0, mid - 0.5), (r.x + 27.5, mid - 0.5)], color(WHITE, a));
        }
    }
    let clock = chrono::Local::now().format("%H:%M").to_string();
    cx.t(&clock, r.x + r.w / 2.0, mid + 7.0, 20.0, color(WHITE, a), Align::Center);
    if notch.visualizer_active() {
        let t = chrono::Local::now().timestamp_millis() as f32 / 1000.0;
        for i in 0..5 {
            let f = i as f32;
            let wave = 0.5 + 0.5 * (t * (5.0 + f * 1.7) + f * 1.3).sin() * (0.6 + 0.4 * (t * (2.3 + f) + f).sin());
            let h = 4.0 + 12.0 * wave;
            let x = r.x + r.w - 42.0 + f * 5.5;
            cx.rrect(rect(x, mid - h / 2.0, 2.5, h), 1.25, color(WHITE, a));
        }
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
        Alert::Online => ("Back online", "Connected to the internet".into(), GREEN, 1.0, None),
        Alert::Offline => ("Offline", "Connection lost".into(), ORANGE, 0.0, None),
        Alert::Language(l) => ("Keyboard", String::from_utf8_lossy(&l).to_uppercase(), BLUE, 1.0, None),
        Alert::TodoDue => ("Time's up!", "A task is due".into(), ORANGE, 1.0, None),
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
    magnifier(cx, r.x + r.w - 62.0, 25.0, color(WHITE, 0.85 * a));
    gear(cx, r.x + r.w - 102.0, 25.0, color(WHITE, 0.85 * a));

    match notch.page {
        Page::Music => draw_music(cx, notch, a),
        Page::Timer => draw_timer(cx, notch, a),
        Page::Stats => draw_stats(cx, notch, a),
        Page::Todo => draw_todo(cx, notch, a),
        Page::Shelf => draw_shelf(cx, notch, a),
        Page::Calendar => draw_calendar(cx, notch, a),
        Page::Weather => draw_weather(cx, notch, a),
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
    let x0 = notch.shape().rect.x;
    let t = notch.timer();
    // The arc shrinks as time runs out.
    cx.ring(x0 + 170.0, 118.0, 52.0, 9.0, t.remaining().as_secs_f32() / t.total().as_secs_f32(), GREEN, a);
    cx.t(&mmss(t.remaining()), x0 + 170.0, 128.0, 30.0, color(WHITE, a), Align::Center);

    let mins = t.total().as_secs() / 60;
    for (b, rc) in timer_buttons(x0) {
        match b {
            Button::TimerPreset(m) => {
                let active = u64::from(m) == mins;
                cx.rrect(rc, 15.0, if active { color(GREEN, 0.9 * a) } else { color(WHITE, 0.12 * a) });
                let ink = if active { color([0, 0, 0], a) } else { color(WHITE, a) };
                cx.t(&format!("{m}m"), rc.x + rc.w / 2.0, rc.y + 20.0, 15.0, ink, Align::Center);
            }
            Button::TimerMinus | Button::TimerPlus => {
                cx.rrect(rc, 17.0, color(WHITE, 0.12 * a));
                let (cx_, cy) = (rc.x + rc.w / 2.0, rc.y + rc.h / 2.0);
                cx.rrect(rect(cx_ - 6.0, cy - 1.0, 12.0, 2.0), 1.0, color(WHITE, a));
                if b == Button::TimerPlus {
                    cx.rrect(rect(cx_ - 1.0, cy - 6.0, 2.0, 12.0), 1.0, color(WHITE, a));
                }
            }
            Button::TimerToggle | Button::TimerReset => {
                let go = b == Button::TimerToggle;
                let label = if go { if t.is_running() { "Pause" } else { "Start" } } else { "Reset" };
                cx.rrect(rc, 22.0, if go { color(GREEN, 0.9 * a) } else { color(WHITE, 0.14 * a) });
                let ink = if go { color([0, 0, 0], a) } else { color(WHITE, a) };
                cx.t(label, rc.x + rc.w / 2.0, rc.y + 29.0, 20.0, ink, Align::Center);
            }
            _ => {}
        }
    }
    cx.t("Focus", x0 + 330.0, 128.0, 26.0, color(WHITE, a), Align::Left);
    cx.t(&format!("{mins} minute session"), x0 + 330.0, 150.0, 16.0, color(GRAY, a), Align::Left);
}

/// Handles a left click at pill-logical coordinates; the host runs the returned outcome.
pub fn click(notch: &mut Notch, x: f32, y: f32) -> Outcome {
    use visus_core::MediaCommand;
    if let Some(layout) = palette_layout(notch) {
        if let Some(i) = layout.rows.iter().position(|r| r.contains(x, y)) {
            notch.palette_select(i);
            return notch.palette_activate();
        }
        return Outcome::None;
    }
    let list = if notch.settings_is_open() { settings_buttons(notch) } else { buttons(notch) };
    for (b, r) in list {
        if !r.contains(x, y) {
            continue;
        }
        match b {
            Button::Prev => visus_platform::media_command(MediaCommand::Previous),
            Button::PlayPause => visus_platform::media_command(MediaCommand::PlayPause),
            Button::Next => visus_platform::media_command(MediaCommand::Next),
            Button::Lyrics => notch.toggle_lyrics(),
            Button::Palette => notch.open_palette(),
            Button::TimerToggle => notch.toggle_timer(),
            Button::TimerReset => notch.reset_timer(),
            Button::TimerPreset(m) => notch.set_timer_minutes(m),
            Button::TimerMinus => notch.adjust_timer(-1),
            Button::TimerPlus => notch.adjust_timer(1),
            Button::TodoToggle(i) => notch.todo_toggle(i),
            Button::TodoDelete(i) => notch.todo_delete(i),
            Button::TodoAdd => notch.begin_todo_input(),
            Button::ShelfOpen(i) => {
                if let Some(p) = notch.shelf().get(i) {
                    return Outcome::OpenPath(p.clone());
                }
            }
            Button::ShelfRemove(i) => notch.shelf_remove(i),
            Button::Toggle(t) => match t {
                Toggle::Minimal => notch.update_settings(|s| s.minimal = !s.minimal),
                Toggle::Eye => notch.update_settings(|s| s.mascot = !s.mascot),
                Toggle::Fullscreen => notch.update_settings(|s| s.hide_in_fullscreen = !s.hide_in_fullscreen),
                Toggle::Autostart => {
                    let _ = visus_platform::set_autostart(!visus_platform::autostart_enabled());
                }
            },
            Button::Size(i) => notch.update_settings(|s| s.scale = SIZES[i as usize % SIZES.len()].1),
            Button::EyeColour(i) => notch.update_settings(|s| s.eye_iris = Rgb(EYE_COLOURS[i as usize % EYE_COLOURS.len()])),
            Button::Style(i) => notch.update_settings(|s| s.eye_style = if i == 0 { EyeStyle::Classic } else { EyeStyle::Sauron }),
            Button::Location => notch.begin_location_input(),
            Button::SettingsOpen => notch.open_settings(),
            Button::SettingsClose => notch.close_settings(),
            Button::EyeTime(d) => notch.adjust_eye_delay(d as i32),
            Button::DriveToggle(i) => {
                if let Some(d) = visus_platform::list_drives().get(i as usize) {
                    notch.toggle_app_folder(d);
                    return Outcome::Rescan;
                }
            }
            Button::FolderRemove(i) => {
                notch.remove_app_folder(i);
                return Outcome::Rescan;
            }
            Button::FolderAdd => notch.begin_folder_input(),
            Button::Update => {
                return match notch.update_status() {
                    UpdateStatus::Available { .. } => {
                        notch.set_update_status(UpdateStatus::Installing);
                        Outcome::InstallUpdate
                    }
                    UpdateStatus::Checking | UpdateStatus::Installing => Outcome::None,
                    _ => {
                        notch.set_update_status(UpdateStatus::Checking);
                        Outcome::CheckUpdates
                    }
                };
            }
        }
        break;
    }
    Outcome::None
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
    let pill = Rect { x: centre - half * k, y: 0.0, w: half * 2.0 * k, h: s.rect.h * k };
    // While the launcher is visible the panel is part of the interactive area too.
    let extra = palette_layout(notch).map(|l| l.panel).or_else(|| settings_panel(notch));
    let Some(p) = extra else { return pill };
    let (px, pw) = (centre + (p.x - centre) * k, p.w * k);
    let (x, right) = (pill.x.min(px), (pill.x + pill.w).max(px + pw));
    Rect { x, y: 0.0, w: right - x, h: (p.y + p.h) * k }
}

const PANEL_W: f32 = 520.0;
const ROW_H: f32 = 48.0;
const PANEL_TOP_PAD: f32 = 66.0;
const PANEL_FOOTER: f32 = 40.0;

pub struct PaletteLayout {
    pub panel: Rect,
    pub search: Rect,
    pub rows: Vec<Rect>,
    /// Full-height panel, used to place content while the panel is still growing.
    pub full_h: f32,
}

/// Panel geometry in pill-logical coordinates; None while the launcher is fully hidden.
pub fn palette_layout(notch: &Notch) -> Option<PaletteLayout> {
    let p = notch.palette_progress();
    if p < 0.01 && !notch.palette_is_open() {
        return None;
    }
    let s = notch.shape().rect;
    let centre = s.x + s.w / 2.0;
    let n = notch.palette().results.len().max(1) as f32;
    let full_h = PANEL_TOP_PAD + n * ROW_H + PANEL_FOOTER;
    let p = p.clamp(0.0, 1.04);
    // Closing sucks the panel up into the pill: it narrows faster than it shortens.
    let w = if notch.palette_is_open() { PANEL_W * (0.72 + 0.28 * p.min(1.0)) } else { PANEL_W * (0.06 + 0.94 * p.min(1.0).powi(2)) };
    let panel = Rect { x: centre - w / 2.0, y: s.h + 10.0, w, h: full_h * p };
    let search = Rect { x: panel.x + 16.0, y: panel.y + 14.0, w: w - 32.0, h: 40.0 };
    let rows = (0..notch.palette().results.len())
        .map(|i| Rect { x: panel.x + 10.0, y: panel.y + PANEL_TOP_PAD + i as f32 * ROW_H, w: w - 20.0, h: ROW_H - 4.0 })
        .collect();
    Some(PaletteLayout { panel, search, rows, full_h })
}

fn magnifier(cx: &mut Ctx, x: f32, y: f32, c: [u8; 4]) {
    if let Some(p) = PathBuilder::from_circle(x, y, 6.5) {
        cx.stroke(&p, c, 2.0);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(x + 5.2, y + 5.2);
    pb.line_to(x + 10.5, y + 10.5);
    if let Some(p) = pb.finish() {
        cx.stroke(&p, c, 2.2);
    }
}

fn draw_palette(cx: &mut Ctx, notch: &Notch) {
    let Some(l) = palette_layout(notch) else { return };
    let p = notch.palette_progress().clamp(0.0, 1.0);
    let ca = ((p - 0.5) / 0.5).clamp(0.0, 1.0);
    let p = if notch.palette_is_open() { p } else { (p / 0.35).min(1.0) };
    let pal = notch.palette();

    // Soft shadow, then the body and a hairline edge so it reads as glass-black beneath the pill.
    for (grow, alpha) in [(12.0, 0.06), (4.0, 0.12)] {
        cx.rrect(rect(l.panel.x - grow, l.panel.y - grow * 0.4, l.panel.w + 2.0 * grow, l.panel.h + 2.0 * grow), 30.0 + grow, color([0, 0, 0], alpha * p));
    }
    cx.rrect(l.panel, 28.0, color([0, 0, 0], p));
    if let Some(path) = rrect(l.panel.x, l.panel.y, l.panel.w, l.panel.h, 28.0) {
        cx.stroke(&path, color(WHITE, 0.09 * p), 1.0);
    }
    if ca <= 0.0 {
        return;
    }

    // Search field.
    cx.rrect(l.search, 14.0, color(WHITE, 0.08 * ca));
    magnifier(cx, l.search.x + 22.0, l.search.y + 18.0, color(GRAY, ca));
    let tx = l.search.x + 44.0;
    let base = l.search.y + 26.0;
    let max = l.search.w - 64.0;
    if pal.query.is_empty() {
        cx.t("Search apps, commands, math\u{2026}", tx, base, 17.0, color(GRAY, 0.8 * ca), Align::Left);
        cx.rrect(rect(tx - 2.0, l.search.y + 10.0, 1.6, 20.0), 0.8, color(WHITE, 0.9 * ca));
    } else {
        // Keep the tail of long queries visible.
        let mut shown = pal.query.as_str();
        while shown.chars().count() > 1 && cx.text_width(shown, 17.0) > max {
            let skip = shown.chars().next().map_or(1, char::len_utf8);
            shown = &shown[skip..];
        }
        cx.t(shown, tx, base, 17.0, color(WHITE, ca), Align::Left);
        let caret_x = tx + cx.text_width(shown, 17.0) + 1.0;
        cx.rrect(rect(caret_x, l.search.y + 10.0, 1.6, 20.0), 0.8, color(WHITE, 0.9 * ca));
    }

    // Results.
    for (i, (item, row)) in pal.results.iter().zip(&l.rows).enumerate() {
        let selected = i == pal.selected;
        if selected {
            cx.rrect(*row, 13.0, color(WHITE, 0.11 * ca));
        }
        let tile = rect(row.x + 10.0, row.y + 7.0, 30.0, 30.0);
        if let Some(art) = pal.icon_for(item) {
            cx.rrect(tile, 9.0, color(WHITE, 0.06 * ca));
            draw_image(cx, tile, 8.0, art, ca);
        } else {
            let kind = item.app.as_ref().map_or(AppKind::Generic, |a| a.kind);
            let (tint, glyph) = match item.kind {
                Kind::Visus => ([96, 120, 255], "V".to_string()),
                Kind::Calc => (GREEN, "=".to_string()),
                Kind::Web => (BLUE, "\u{2197}".to_string()),
                Kind::App => (kind_tint(kind, &item.title), item.title.chars().next().map_or("?".into(), |c| c.to_uppercase().to_string())),
            };
            cx.rrect(tile, 9.0, color(tint, ca));
            if item.kind != Kind::App || !draw_kind_glyph(cx, tile, kind, ca) {
                cx.t(&glyph, tile.x + 15.0, tile.y + 21.0, 16.0, color(WHITE, ca), Align::Center);
            }
        }

        let text_x = row.x + 54.0;
        let room = row.w - 54.0 - 40.0;
        let title = cx.fit(&item.title, 17.0, room);
        let sub = cx.fit(&item.subtitle, 12.5, room);
        cx.t(&title, text_x, row.y + 20.0, 17.0, color(WHITE, ca), Align::Left);
        cx.t(&sub, text_x, row.y + 36.0, 12.5, color(GRAY, 0.9 * ca), Align::Left);
        if selected && item.action.is_some() {
            cx.t("\u{21b5}", row.x + row.w - 22.0, row.y + 27.0, 17.0, color(GRAY, ca), Align::Center);
        }
    }

    let foot_y = l.panel.y + l.full_h - 14.0;
    cx.t("\u{2191}\u{2193} navigate    \u{21b5} open    esc close", l.panel.x + l.panel.w / 2.0, foot_y, 12.0, color(GRAY, 0.65 * ca), Align::Center);
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
    /// Premultiplied copies of recently drawn images (covers, app icons), keyed by content id.
    static ART_PIXMAPS: std::cell::RefCell<std::collections::HashMap<u64, Pixmap>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

/// Draws a square image into a rounded rectangle.
fn draw_image(cx: &mut Ctx, r: Rect, radius: f32, art: &visus_core::Art, a: f32) {
    let Some(path) = rrect(r.x, r.y, r.w, r.h, radius) else { return };
    let xf = cx.xf();
    ART_PIXMAPS.with(|cell| {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&art.id) {
            let Some(mut pm) = Pixmap::new(art.size, art.size) else { return };
            for (dst, src) in pm.pixels_mut().iter_mut().zip(art.rgba.chunks_exact(4)) {
                let al = src[3] as u32;
                let mul = |c: u8| ((c as u32 * al + 127) / 255) as u8;
                if let Some(p) = tiny_skia::PremultipliedColorU8::from_rgba(mul(src[0]), mul(src[1]), mul(src[2]), src[3]) {
                    *dst = p;
                }
            }
            if map.len() > 40 {
                map.clear();
            }
            map.insert(art.id, pm);
        }
        let Some(pm) = map.get(&art.id) else { return };
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
}

/// Cover art in a rounded square, or a gradient seeded from the title when there is none.
fn draw_art(cx: &mut Ctx, r: Rect, radius: f32, m: &visus_core::MediaInfo, a: f32) {
    if let Some(art) = m.art.as_ref() {
        draw_image(cx, r, radius, art, a);
        return;
    }
    let Some(path) = rrect(r.x, r.y, r.w, r.h, radius) else { return };
    let xf = cx.xf();
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

/// Pill plus launcher panel at their final sizes; used to size the OS window region once per animation.
pub fn region_rect(notch: &Notch) -> Rect {
    let now = hit_rect(notch);
    let (tw, th) = notch.target_size();
    let k = notch.settings().scale;
    let s = notch.shape().rect;
    let centre = s.x + s.w / 2.0;
    let half = (tw / 2.0 + 14.0) * k;
    let mut left = now.x.min(centre - half);
    let mut right = (now.x + now.w).max(centre + half);
    let mut bottom = now.y + now.h.max(th * k);
    if notch.palette_is_open() || notch.palette_progress() > 0.01 {
        let n = notch.palette().results.len().max(1) as f32;
        let full = PANEL_TOP_PAD + n * ROW_H + PANEL_FOOTER;
        left = left.min(centre - PANEL_W / 2.0 * k);
        right = right.max(centre + PANEL_W / 2.0 * k);
        bottom = bottom.max((th + 10.0 + full) * k);
    }
    if notch.settings_is_open() || notch.settings_progress() > 0.01 {
        left = left.min(centre - SETTINGS_W / 2.0 * k);
        right = right.max(centre + SETTINGS_W / 2.0 * k);
        bottom = bottom.max((th + 10.0 + SETTINGS_H) * k);
    }
    Rect { x: left, y: 0.0, w: right - left, h: bottom }
}

/// Window-logical y below which nothing is drawn (shadow margin included); lets hosts skip empty rows.
pub fn content_bottom(notch: &Notch) -> f32 {
    let r = region_rect(notch);
    r.y + r.h + 28.0
}

const SIZES: [(&str, f32); 3] = [("Small", 0.8), ("Default", 1.0), ("Large", 1.15)];
const EYE_COLOURS: [[u8; 3]; 6] = [[40, 110, 230], [48, 209, 88], [255, 159, 10], [255, 69, 58], [191, 90, 242], [160, 160, 168]];

fn file_name(p: &str) -> String {
    std::path::Path::new(p).file_name().map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned())
}

fn shelf_buttons(notch: &Notch, x0: f32) -> Vec<(Button, Rect)> {
    let w = notch.shape().rect.w - 72.0;
    let mut v = Vec::new();
    for i in 0..notch.shelf().len().min(4) {
        let y = 60.0 + i as f32 * 40.0;
        v.push((Button::ShelfOpen(i), rect(x0 + 36.0, y, w - 44.0, 36.0)));
        v.push((Button::ShelfRemove(i), rect(x0 + 36.0 + w - 40.0, y, 40.0, 36.0)));
    }
    v
}

fn draw_shelf(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let (x0, w) = (r.x, r.w - 72.0);
    if notch.shelf().is_empty() {
        cx.t("Drop files here", r.x + r.w / 2.0, 130.0, 26.0, color(WHITE, a), Align::Center);
        cx.t("Drag them onto the pill to keep them close", r.x + r.w / 2.0, 158.0, 16.0, color(GRAY, a), Align::Center);
        return;
    }
    for (i, path) in notch.shelf().iter().take(4).enumerate() {
        let y = 60.0 + i as f32 * 40.0;
        let name = file_name(path);
        let ext = std::path::Path::new(path).extension().map_or(String::new(), |e| e.to_string_lossy().to_uppercase());
        let tint = hue_rgb((hash(&ext) % 360) as f32 / 360.0);
        cx.rrect(rect(x0 + 36.0, y + 3.0, 30.0, 30.0), 8.0, color(tint, a));
        cx.t(&ext.chars().take(3).collect::<String>(), x0 + 51.0, y + 23.0, 11.0, color(WHITE, a), Align::Center);
        let room = w - 130.0;
        let title = cx.fit(&name, 17.0, room);
        cx.t(&title, x0 + 78.0, y + 16.0, 17.0, color(WHITE, a), Align::Left);
        let dir = std::path::Path::new(path).parent().map_or(String::new(), |d| d.to_string_lossy().into_owned());
        let dir = cx.fit(&dir, 12.0, room);
        cx.t(&dir, x0 + 78.0, y + 31.0, 12.0, color(GRAY, 0.9 * a), Align::Left);
        let (dx, dy) = (x0 + 36.0 + w - 20.0, y + 18.0);
        let mut pb = PathBuilder::new();
        pb.move_to(dx - 4.0, dy - 4.0);
        pb.line_to(dx + 4.0, dy + 4.0);
        pb.move_to(dx + 4.0, dy - 4.0);
        pb.line_to(dx - 4.0, dy + 4.0);
        if let Some(p) = pb.finish() {
            cx.stroke(&p, color(GRAY, 0.8 * a), 1.6);
        }
    }
}

fn draw_calendar(cx: &mut Ctx, notch: &Notch, a: f32) {
    use chrono::{Datelike, NaiveDate};
    let x0 = notch.shape().rect.x;
    let today = chrono::Local::now().date_naive();
    cx.t(&today.format("%A").to_string(), x0 + 130.0, 92.0, 18.0, color(GRAY, a), Align::Center);
    cx.t(&today.day().to_string(), x0 + 130.0, 160.0, 68.0, color(WHITE, a), Align::Center);
    cx.t(&today.format("%B %Y").to_string(), x0 + 130.0, 196.0, 18.0, color(GRAY, a), Align::Center);
    let due = notch.todos().iter().filter(|t| !t.done && t.due.is_some()).count();
    if due > 0 {
        cx.t(&format!("{due} reminder{} today", if due == 1 { "" } else { "s" }), x0 + 130.0, 220.0, 14.0, color(ORANGE, a), Align::Center);
    }

    let first = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap_or(today);
    let lead = first.weekday().num_days_from_monday() as f32;
    let days = if today.month() == 12 {
        NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
    }
    .map_or(30, |n| n.signed_duration_since(first).num_days() as u32);
    let (gx, col_w, row_h) = (x0 + 290.0, 58.0, 21.5);
    for (i, d) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
        cx.t(d, gx + i as f32 * col_w + col_w / 2.0, 80.0, 13.0, color(GRAY, 0.8 * a), Align::Center);
    }
    for day in 1..=days {
        let idx = lead + (day - 1) as f32;
        let (col, row) = (idx % 7.0, (idx / 7.0).floor());
        let (px, py) = (gx + col * col_w + col_w / 2.0, 92.0 + row * row_h);
        if day == today.day() {
            cx.circle(px, py + 6.0, 10.5, color(GREEN, a));
        }
        let ink = if day == today.day() { color([0, 0, 0], a) } else { color(WHITE, a) };
        cx.t(&day.to_string(), px, py + 11.0, 14.0, ink, Align::Center);
    }
}

fn draw_weather(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let mid = r.x + r.w / 2.0;
    if notch.settings().weather_lat.is_none() || notch.settings().weather_lon.is_none() {
        cx.t("Where are you?", mid, 118.0, 26.0, color(WHITE, a), Align::Center);
        cx.t("Set your city in settings (gear)", mid, 148.0, 16.0, color(GRAY, a), Align::Center);
        return;
    }
    match notch.weather() {
        Some(w) => {
            cx.t(&format!("{:.0}\u{b0}", w.temp_c), mid - 90.0, 160.0, 84.0, color(WHITE, a), Align::Center);
            cx.t(weather_label(w.code), mid + 70.0, 120.0, 28.0, color(WHITE, a), Align::Left);
            cx.t(&format!("Wind {:.0} km/h", w.wind_kph), mid + 70.0, 150.0, 17.0, color(GRAY, a), Align::Left);
        }
        None => cx.t("Loading\u{2026}", mid, 140.0, 22.0, color(GRAY, a), Align::Center),
    }
}

fn settings_buttons_raw(x0: f32) -> Vec<(Button, Rect)> {
    let mut v = Vec::new();
    for (i, t) in [Toggle::Minimal, Toggle::Eye, Toggle::Fullscreen, Toggle::Autostart].into_iter().enumerate() {
        v.push((Button::Toggle(t), rect(x0 + 36.0, 58.0 + i as f32 * 30.0, 300.0, 28.0)));
    }
    for i in 0..SIZES.len() {
        v.push((Button::Size(i as u8), rect(x0 + 380.0 + i as f32 * 76.0, 70.0, 70.0, 28.0)));
    }
    for i in 0..EYE_COLOURS.len() {
        v.push((Button::EyeColour(i as u8), rect(x0 + 380.0 + i as f32 * 32.0 - 2.0, 138.0, 30.0, 30.0)));
    }
    for i in 0..2 {
        v.push((Button::Style(i), rect(x0 + 380.0 + i as f32 * 96.0, 178.0, 90.0, 28.0)));
    }
    v.push((Button::Location, rect(x0 + 36.0, 182.0, 300.0, 26.0)));
    v.push((Button::Update, rect(x0 + 36.0, 214.0, 130.0, 26.0)));
    v.push((Button::SettingsClose, rect(x0 + SETTINGS_W - 56.0, 40.0, 36.0, 28.0)));
    v.push((Button::EyeTime(-1), rect(x0 + 190.0, 250.0, 30.0, 26.0)));
    v.push((Button::EyeTime(1), rect(x0 + 276.0, 250.0, 30.0, 26.0)));
    for i in 0..visus_platform::list_drives().len().min(7) {
        v.push((Button::DriveToggle(i as u8), rect(x0 + 380.0 + i as f32 * 52.0, 270.0, 46.0, 26.0)));
    }
    v
}

const SETTINGS_W: f32 = 744.0;
const SETTINGS_H: f32 = 348.0;

/// Popup geometry in pill-logical coordinates; None while fully hidden.
fn settings_panel(notch: &Notch) -> Option<Rect> {
    let p = notch.settings_progress();
    if p < 0.01 && !notch.settings_is_open() {
        return None;
    }
    let s = notch.shape().rect;
    let centre = s.x + s.w / 2.0;
    let p = p.clamp(0.0, 1.04);
    let w = SETTINGS_W * (0.75 + 0.25 * p.min(1.0));
    Some(Rect { x: centre - w / 2.0, y: s.h + 10.0, w, h: SETTINGS_H * p })
}

/// Settings controls placed in the popup (page-style coordinates shifted below the pill).
fn settings_buttons(notch: &Notch) -> Vec<(Button, Rect)> {
    let s = notch.shape().rect;
    let x0 = s.x + s.w / 2.0 - SETTINGS_W / 2.0;
    let oy = s.h + 10.0 - 36.0;
    let mut v = settings_buttons_raw(x0);
    let s = notch.settings();
    let (nf, shown) = (s.app_folders.len(), s.app_folders.len().min(2));
    let _ = nf;
    for i in 0..shown {
        v.push((Button::FolderRemove(i), rect(x0 + 36.0 + 260.0, 298.0 + i as f32 * 26.0, 40.0, 24.0)));
    }
    v.push((Button::FolderAdd, rect(x0 + 36.0, 352.0, 300.0, 26.0)));
    v.into_iter().map(|(b, mut r)| {
        r.y += oy;
        (b, r)
    }).collect()
}

fn gear(cx: &mut Ctx, x: f32, y: f32, c: [u8; 4]) {
    if let Some(p) = PathBuilder::from_circle(x, y, 5.0) {
        cx.stroke(&p, c, 2.0);
    }
    for k in 0..6 {
        let ang = k as f32 * std::f32::consts::FRAC_PI_3;
        cx.circle(x + 8.2 * ang.cos(), y + 8.2 * ang.sin(), 1.6, c);
    }
}

fn draw_settings_panel(cx: &mut Ctx, notch: &Notch) {
    let Some(panel) = settings_panel(notch) else { return };
    let p = notch.settings_progress().clamp(0.0, 1.0);
    let ca = ((p - 0.5) / 0.5).clamp(0.0, 1.0);
    for (grow, alpha) in [(12.0, 0.06), (4.0, 0.12)] {
        cx.rrect(rect(panel.x - grow, panel.y - grow * 0.4, panel.w + 2.0 * grow, panel.h + 2.0 * grow), 30.0 + grow, color([0, 0, 0], alpha * p));
    }
    cx.rrect(panel, 28.0, color([0, 0, 0], p));
    if let Some(path) = rrect(panel.x, panel.y, panel.w, panel.h, 28.0) {
        cx.stroke(&path, color(WHITE, 0.09 * p), 1.0);
    }
    if ca <= 0.0 {
        return;
    }
    let s = notch.shape().rect;
    let x0 = s.x + s.w / 2.0 - SETTINGS_W / 2.0;
    cx.oy = (s.h + 10.0 - 36.0) * cx.scale;
    draw_settings(cx, notch, x0, ca);
    cx.oy = 0.0;
}

fn draw_settings(cx: &mut Ctx, notch: &Notch, x0: f32, a: f32) {
    let s = notch.settings();
    let autostart = visus_platform::autostart_enabled();
    let toggles = [("Minimal notch", s.minimal), ("Idle eye", s.mascot), ("Hide in full screen", s.hide_in_fullscreen), ("Start at login", autostart)];
    for (i, (label, on)) in toggles.iter().enumerate() {
        let y = 58.0 + i as f32 * 30.0;
        cx.t(label, x0 + 36.0, y + 20.0, 17.0, color(WHITE, a), Align::Left);
        let sw = rect(x0 + 290.0, y + 2.0, 44.0, 24.0);
        cx.rrect(sw, 12.0, if *on { color(GREEN, a) } else { color(WHITE, 0.18 * a) });
        cx.circle(if *on { sw.x + 32.0 } else { sw.x + 12.0 }, sw.y + 12.0, 9.0, color(WHITE, a));
    }

    cx.t("Size", x0 + 380.0, 62.0, 14.0, color(GRAY, a), Align::Left);
    for (i, (label, k)) in SIZES.iter().enumerate() {
        let on = (s.scale - k).abs() < 0.01;
        let rc = rect(x0 + 380.0 + i as f32 * 76.0, 70.0, 70.0, 28.0);
        cx.rrect(rc, 14.0, if on { color(WHITE, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        cx.t(label, rc.x + 35.0, rc.y + 19.0, 14.0, if on { color([0, 0, 0], a) } else { color(WHITE, a) }, Align::Center);
    }
    cx.t("Eye colour", x0 + 380.0, 130.0, 14.0, color(GRAY, a), Align::Left);
    for (i, c) in EYE_COLOURS.iter().enumerate() {
        let (px, py) = (x0 + 380.0 + i as f32 * 32.0 + 13.0, 153.0);
        if s.eye_iris.0 == *c {
            cx.circle(px, py, 14.0, color(WHITE, 0.9 * a));
        }
        cx.circle(px, py, 11.0, color(*c, a));
    }
    for (i, label) in ["Classic", "Sauron"].iter().enumerate() {
        let on = (s.eye_style == EyeStyle::Sauron) == (i == 1);
        let rc = rect(x0 + 380.0 + i as f32 * 96.0, 178.0, 90.0, 28.0);
        cx.rrect(rc, 14.0, if on { color(WHITE, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        cx.t(label, rc.x + 45.0, rc.y + 19.0, 14.0, if on { color([0, 0, 0], a) } else { color(WHITE, a) }, Align::Center);
    }

    let (label, status) = match notch.update_status() {
        UpdateStatus::Idle => ("Check for updates".to_string(), String::new()),
        UpdateStatus::Checking => ("Checking\u{2026}".to_string(), String::new()),
        UpdateStatus::UpToDate => ("Check again".to_string(), "Up to date".to_string()),
        UpdateStatus::Available { version } => (format!("Install {version}"), "Update available".to_string()),
        UpdateStatus::Installing => ("Installing\u{2026}".to_string(), String::new()),
        UpdateStatus::Failed(e) => ("Try again".to_string(), e.clone()),
    };
    draw_location_row(cx, notch, rect(x0 + 36.0, 182.0, 300.0, 26.0), a);
    let rc = rect(x0 + 36.0, 214.0, 130.0, 26.0);
    cx.rrect(rc, 13.0, color(GREEN, 0.9 * a));
    let fitted = cx.fit(&label, 13.0, 120.0);
    cx.t(&fitted, rc.x + 65.0, rc.y + 18.0, 13.0, color([0, 0, 0], a), Align::Center);
    let st = cx.fit(&format!("v{} {}", env!("CARGO_PKG_VERSION"), status), 12.0, 180.0);
    cx.t(&st, x0 + 176.0, 232.0, 12.0, color(GRAY, a), Align::Left);

    // Close button.
    let (kx, ky) = (x0 + SETTINGS_W - 38.0, 54.0);
    let mut pb = PathBuilder::new();
    pb.move_to(kx - 5.0, ky - 5.0);
    pb.line_to(kx + 5.0, ky + 5.0);
    pb.move_to(kx + 5.0, ky - 5.0);
    pb.line_to(kx - 5.0, ky + 5.0);
    if let Some(p) = pb.finish() {
        cx.stroke(&p, color(GRAY, a), 1.8);
    }

    // Eye delay.
    cx.t("Eye appears after", x0 + 36.0, 269.0, 17.0, color(WHITE, a), Align::Left);
    for (dx, plus) in [(190.0, false), (276.0, true)] {
        let rc = rect(x0 + dx, 250.0, 30.0, 26.0);
        cx.rrect(rc, 13.0, color(WHITE, 0.12 * a));
        let (mx, my) = (rc.x + 15.0, rc.y + 13.0);
        cx.rrect(rect(mx - 5.0, my - 1.0, 10.0, 2.0), 1.0, color(WHITE, a));
        if plus {
            cx.rrect(rect(mx - 1.0, my - 5.0, 2.0, 10.0), 1.0, color(WHITE, a));
        }
    }
    let secs = s.idle_secs;
    let value = if secs >= 60 { format!("{} min", secs / 60) } else { format!("{secs} s") };
    cx.t(&value, x0 + 248.0, 269.0, 16.0, color(WHITE, a), Align::Center);

    // Drives and folders the launcher searches besides the Start Menu.
    cx.t("Search drives for apps", x0 + 380.0, 262.0, 14.0, color(GRAY, a), Align::Left);
    for (i, d) in visus_platform::list_drives().iter().take(7).enumerate() {
        let on = s.app_folders.iter().any(|f| f == d);
        let rc = rect(x0 + 380.0 + i as f32 * 52.0, 270.0, 46.0, 26.0);
        cx.rrect(rc, 13.0, if on { color(GREEN, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        cx.t(d.trim_end_matches('\\'), rc.x + 23.0, rc.y + 18.0, 14.0, if on { color([0, 0, 0], a) } else { color(WHITE, a) }, Align::Center);
    }
    cx.t("Scans up to 3 folders deep", x0 + 380.0, 316.0, 12.0, color(GRAY, 0.8 * a), Align::Left);
    cx.t("Extra app folders", x0 + 36.0, 292.0, 14.0, color(GRAY, a), Align::Left);
    for (i, f) in s.app_folders.iter().take(2).enumerate() {
        let y = 298.0 + i as f32 * 26.0;
        let txt = cx.fit(f, 14.0, 240.0);
        cx.t(&txt, x0 + 36.0, y + 17.0, 14.0, color(WHITE, a), Align::Left);
        let (dx, dy) = (x0 + 36.0 + 280.0, y + 12.0);
        let mut pb = PathBuilder::new();
        pb.move_to(dx - 4.0, dy - 4.0);
        pb.line_to(dx + 4.0, dy + 4.0);
        pb.move_to(dx + 4.0, dy - 4.0);
        pb.line_to(dx - 4.0, dy + 4.0);
        if let Some(p) = pb.finish() {
            cx.stroke(&p, color(GRAY, 0.8 * a), 1.6);
        }
    }
    let field = rect(x0 + 36.0, 352.0, 300.0, 26.0);
    match notch.folder_input() {
        Some(input) => {
            cx.rrect(field, 13.0, color(WHITE, 0.14 * a));
            let (txt, ink) = if input.is_empty() { ("Folder path, then Enter", GRAY) } else { (input, WHITE) };
            let mut shown = txt;
            while shown.chars().count() > 1 && cx.text_width(shown, 14.0) > field.w - 30.0 {
                shown = &shown[shown.chars().next().map_or(1, char::len_utf8)..];
            }
            cx.t(shown, field.x + 14.0, field.y + 18.0, 14.0, color(ink, a), Align::Left);
        }
        None => {
            cx.rrect(field, 13.0, color(WHITE, 0.07 * a));
            cx.t("+  Add folder", field.x + 14.0, field.y + 18.0, 14.0, color(GRAY, a), Align::Left);
        }
    }
}

fn kind_tint(kind: AppKind, name: &str) -> [u8; 3] {
    match kind {
        AppKind::Game => [136, 84, 255],
        AppKind::Browser => [48, 130, 246],
        AppKind::Terminal => [58, 62, 74],
        AppKind::Music => [236, 72, 100],
        AppKind::Video => [246, 130, 40],
        AppKind::Code => [20, 160, 150],
        AppKind::Office => [50, 110, 210],
        AppKind::Files => [228, 160, 30],
        AppKind::Settings => [110, 116, 130],
        AppKind::Chat => [48, 190, 100],
        AppKind::Graphics => [220, 80, 170],
        AppKind::Generic => hue_rgb((hash(name) % 360) as f32 / 360.0),
    }
}

/// Vector glyph for apps without an icon; returns false for Generic so the caller draws a letter.
fn draw_kind_glyph(cx: &mut Ctx, t: Rect, kind: AppKind, a: f32) -> bool {
    let (x, y) = (t.x + t.w / 2.0, t.y + t.h / 2.0);
    let w = color(WHITE, a);
    let dark = color([0, 0, 0], 0.55 * a);
    let line = |cx: &mut Ctx, pts: &[(f32, f32)], width: f32| {
        let mut pb = PathBuilder::new();
        for (i, (px, py)) in pts.iter().enumerate() {
            if i == 0 { pb.move_to(*px, *py) } else { pb.line_to(*px, *py) }
        }
        if let Some(p) = pb.finish() {
            cx.stroke(&p, w, width);
        }
    };
    match kind {
        AppKind::Generic => return false,
        AppKind::Game => {
            cx.rrect(rect(x - 10.0, y - 5.5, 20.0, 12.0), 6.0, w);
            cx.rrect(rect(x - 6.5, y - 0.8, 6.0, 1.8), 0.9, dark);
            cx.rrect(rect(x - 4.5, y - 2.8, 1.8, 6.0), 0.9, dark);
            cx.circle(x + 4.0, y - 1.5, 1.5, dark);
            cx.circle(x + 7.0, y + 1.5, 1.5, dark);
        }
        AppKind::Browser => {
            if let Some(p) = PathBuilder::from_circle(x, y, 8.0) {
                cx.stroke(&p, w, 1.7);
            }
            if let Some(p) = tiny_skia::Rect::from_xywh(x - 3.5, y - 8.0, 7.0, 16.0).and_then(PathBuilder::from_oval) {
                cx.stroke(&p, w, 1.4);
            }
            line(cx, &[(x - 8.0, y), (x + 8.0, y)], 1.4);
        }
        AppKind::Terminal => {
            line(cx, &[(x - 7.0, y - 4.5), (x - 1.5, y), (x - 7.0, y + 4.5)], 2.0);
            cx.rrect(rect(x + 1.0, y + 3.5, 7.0, 2.0), 1.0, w);
        }
        AppKind::Music => {
            cx.circle(x - 3.0, y + 5.0, 3.4, w);
            cx.rrect(rect(x - 0.5, y - 8.0, 2.0, 13.0), 1.0, w);
            cx.rrect(rect(x - 0.5, y - 8.0, 7.0, 3.0), 1.5, w);
        }
        AppKind::Video => cx.triangle([(x - 4.5, y - 7.0), (x - 4.5, y + 7.0), (x + 7.5, y)], w),
        AppKind::Code => cx.t("</>", x, y + 4.5, 13.0, w, Align::Center),
        AppKind::Office => {
            if let Some(p) = rrect(x - 6.0, y - 8.0, 12.0, 16.0, 2.5) {
                cx.stroke(&p, w, 1.6);
            }
            for dy in [-3.0, 0.5, 4.0] {
                cx.rrect(rect(x - 3.0, y + dy, 6.0, 1.4), 0.7, w);
            }
        }
        AppKind::Files => {
            cx.rrect(rect(x - 9.0, y - 7.0, 8.0, 5.0), 2.0, w);
            cx.rrect(rect(x - 9.0, y - 4.5, 18.0, 12.0), 3.0, w);
        }
        AppKind::Settings => {
            if let Some(p) = PathBuilder::from_circle(x, y, 5.5) {
                cx.stroke(&p, w, 2.2);
            }
            for (dx, dy) in [(0.0, -8.5), (0.0, 8.5), (-8.5, 0.0), (8.5, 0.0)] {
                cx.circle(x + dx, y + dy, 1.5, w);
            }
        }
        AppKind::Chat => {
            cx.rrect(rect(x - 9.0, y - 7.5, 18.0, 12.5), 4.5, w);
            cx.triangle([(x - 5.0, y + 4.0), (x - 1.0, y + 4.0), (x - 6.0, y + 9.0)], w);
        }
        AppKind::Graphics => {
            cx.triangle([(x - 9.0, y + 7.0), (x - 2.0, y - 5.0), (x + 4.0, y + 7.0)], w);
            cx.circle(x + 5.0, y - 4.5, 2.6, w);
        }
    }
    true
}

/// City row shared by the Weather and Settings pages; becomes a text field while typing.
fn draw_location_row(cx: &mut Ctx, notch: &Notch, rc: Rect, a: f32) {
    let h = rc.h;
    match notch.location_input() {
        Some(input) => {
            cx.rrect(rc, h / 2.0, color(WHITE, 0.14 * a));
            if input.is_empty() {
                cx.t("City name, then Enter", rc.x + 16.0, rc.y + h * 0.68, 14.0, color(GRAY, 0.9 * a), Align::Left);
                cx.rrect(rect(rc.x + 14.0, rc.y + 6.0, 1.6, h - 12.0), 0.8, color(WHITE, a));
            } else {
                let mut shown = input;
                while shown.chars().count() > 1 && cx.text_width(shown, 15.0) > rc.w - 36.0 {
                    shown = &shown[shown.chars().next().map_or(1, char::len_utf8)..];
                }
                cx.t(shown, rc.x + 16.0, rc.y + h * 0.7, 15.0, color(WHITE, a), Align::Left);
                let cxp = rc.x + 16.0 + cx.text_width(shown, 15.0) + 1.0;
                cx.rrect(rect(cxp, rc.y + 6.0, 1.6, h - 12.0), 0.8, color(WHITE, a));
            }
        }
        None => {
            cx.rrect(rc, h / 2.0, color(WHITE, 0.07 * a));
            // Map pin: ring with a point below.
            let (px, py) = (rc.x + 17.0, rc.y + h / 2.0 - 1.5);
            if let Some(p) = PathBuilder::from_circle(px, py, 4.5) {
                cx.stroke(&p, color(GRAY, a), 1.6);
            }
            cx.triangle([(px - 3.6, py + 3.0), (px + 3.6, py + 3.0), (px, py + 9.0)], color(GRAY, a));
            let (label, tint) = match (notch.location_note(), notch.settings().weather_city.as_deref()) {
                (Some(note), _) => (note.to_string(), ORANGE),
                (None, Some(city)) => (city.to_string(), WHITE),
                (None, None) => ("Set location".to_string(), GRAY),
            };
            let label = cx.fit(&label, 15.0, rc.w - 90.0);
            cx.t(&label, rc.x + 34.0, rc.y + h * 0.7, 15.0, color(tint, a), Align::Left);
            cx.t("Change", rc.x + rc.w - 14.0, rc.y + h * 0.7, 12.0, color(GRAY, 0.8 * a), Align::Right);
        }
    }
}
