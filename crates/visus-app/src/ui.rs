use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, Path, PathBuilder, Pixmap, Point, RadialGradient,
    SpreadMode, FilterQuality, Pattern,
    Stroke, Transform, LineCap,
};
use visus_core::{Alert, AppKind, ClockStyle, Radio, EyeMood, EyeState, EyeStyle, Kind, Lyrics, Notch, Outcome, Page, Rect, Rgb, Shape, UpdateStatus, WIN_GRID, current_index, weather_label};

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
    Volume,
    Brightness,
    Tile(u8),
    NoteCopy(usize),
    NoteDelete(usize),
    ClipCopy(usize),
    ClipDelete(usize),
    Event(usize),
    NetUnit,
    WindowFocus(usize),
    NewNote,
    ClipEnable,
    ShelfReveal(usize),
    Downloads,
    PagePrev,
    PageNext,
    WinPrev,
    WinNext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggle {
    Minimal,
    Eye,
    Fullscreen,
    Autostart,
    Quiet,
    Clock24,
    Visualizer,
    Battery,
    Clipboard,
    EyeMoods,
    ClockStyle,
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
    btns.push((Button::Palette, rect(x0 + s.rect.w - 80.0, 8.0, 36.0, 36.0)));
    btns.push((Button::SettingsOpen, rect(x0 + s.rect.w - 120.0, 8.0, 36.0, 36.0)));
    if notch.download().is_some() {
        btns.push((Button::Downloads, rect(x0 + 36.0, 12.0, 160.0, 28.0)));
    }
    let (ay, off) = (s.rect.h - 26.0, Page::ALL.len() as f32 * 7.0 + 14.0);
    btns.push((Button::PagePrev, rect(cx - off - 28.0, ay, 32.0, 24.0)));
    btns.push((Button::PageNext, rect(cx + off - 4.0, ay, 32.0, 24.0)));
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
            cx.circle(bx, by, 9.0, color(cx.accent, a));
            let mut pb = PathBuilder::new();
            pb.move_to(bx - 4.0, by);
            pb.line_to(bx - 1.0, by + 3.5);
            pb.line_to(bx + 4.5, by - 3.5);
            if let Some(p) = pb.finish() {
                cx.stroke(&p, color(cx.ink, a), 2.0);
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

const LIST_ROWS: usize = 4;

fn list_buttons(x0: f32, n: usize, copy: fn(usize) -> Button, del: fn(usize) -> Button) -> Vec<(Button, Rect)> {
    let mut v = Vec::new();
    for i in 0..n.min(LIST_ROWS) {
        let y = 62.0 + i as f32 * 38.0;
        v.push((copy(i), rect(x0 + 36.0, y, 590.0, 34.0)));
        v.push((del(i), rect(x0 + 630.0, y, 40.0, 34.0)));
    }
    v
}

fn tile_letter(cx: &mut Ctx, rc: Rect, text: &str, a: f32) {
    let tint = hue_rgb((hash(text) % 360) as f32 / 360.0);
    cx.rrect(rc, 9.0, color(tint, 0.95 * a));
    let first = text.chars().find(|c| c.is_alphanumeric()).map_or("?".to_string(), |c| c.to_uppercase().to_string());
    cx.t(&first, rc.x + rc.w / 2.0, rc.y + rc.h * 0.68, rc.h * 0.5, color(WHITE, a), Align::Center);
}

fn pill_button(cx: &mut Ctx, rc: Rect, label: &str, a: f32) {
    cx.rrect(rc, rc.h / 2.0, color(WHITE, 0.16 * a));
    cx.t(label, rc.x + rc.w / 2.0, rc.y + rc.h * 0.66, 14.0, color(WHITE, a), Align::Center);
}

fn x_mark(cx: &mut Ctx, dx: f32, dy: f32, a: f32) {
    let mut pb = PathBuilder::new();
    pb.move_to(dx - 4.0, dy - 4.0);
    pb.line_to(dx + 4.0, dy + 4.0);
    pb.move_to(dx + 4.0, dy - 4.0);
    pb.line_to(dx - 4.0, dy + 4.0);
    if let Some(p) = pb.finish() {
        cx.stroke(&p, color(GRAY, 0.8 * a), 1.6);
    }
}

fn copy_icon(cx: &mut Ctx, x: f32, y: f32, a: f32) {
    for (dx, dy) in [(-2.0, -2.0), (1.5, 1.5)] {
        if let Some(p) = rrect(x + dx - 4.0, y + dy - 4.5, 8.0, 9.0, 2.0) {
            cx.stroke(&p, color(GRAY, 0.85 * a), 1.3);
        }
    }
}

/// Rows of text that copy back to the clipboard when clicked; shared by Notes and Clipboard.
fn draw_list(cx: &mut Ctx, notch: &Notch, items: &[String], empty: (&str, &str, Option<&str>), a: f32) {
    let r = notch.shape().rect;
    let (x0, mid) = (r.x, r.x + r.w / 2.0);
    if items.is_empty() {
        cx.circle(mid, 96.0, 24.0, color(WHITE, 0.08 * a));
        for k in 0..3 {
            cx.rrect(rect(mid - 9.0, 88.0 + k as f32 * 6.0, if k == 2 { 11.0 } else { 18.0 }, 2.4), 1.2, color(WHITE, 0.8 * a));
        }
        cx.t(empty.0, mid, 148.0, 22.0, color(WHITE, a), Align::Center);
        cx.t(empty.1, mid, 170.0, 14.0, color(GRAY, a), Align::Center);
        if let Some(label) = empty.2 {
            pill_button(cx, rect(mid - 70.0, 178.0, 140.0, 32.0), label, a);
        }
        return;
    }
    for (i, text) in items.iter().take(LIST_ROWS).enumerate() {
        let y = 62.0 + i as f32 * 38.0;
        cx.rrect(rect(x0 + 36.0, y, 634.0, 34.0), 12.0, color(WHITE, 0.07 * a));
        let one_line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let shown = cx.fit(&one_line, 15.0, 540.0);
        cx.t(&shown, x0 + 50.0, y + 22.0, 15.0, color(WHITE, a), Align::Left);
        copy_icon(cx, x0 + 610.0, y + 17.0, a);
        x_mark(cx, x0 + 650.0, y + 17.0, a);
    }
    if items.len() > LIST_ROWS {
        cx.t(&format!("+{} more", items.len() - LIST_ROWS), x0 + 670.0, 222.0, 12.0, color(GRAY, 0.8 * a), Align::Right);
    }
}

/// Card slot `i` (0..6) of the 3x2 window grid.
fn win_card(x0: f32, i: usize) -> Rect {
    let (col, row) = ((i % 3) as f32, (i / 3) as f32);
    rect(x0 + 36.0 + col * 228.0, 62.0 + row * 78.0, 216.0, 70.0)
}

/// Splits "document - Application" so the card can show both.
fn split_title(title: &str) -> (&str, &str) {
    title.rsplit_once(" - ").or_else(|| title.rsplit_once(" \u{2014} ")).map_or((title, ""), |(doc, app)| (doc, app))
}

fn draw_windows(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let (x0, mid) = (r.x, r.x + r.w / 2.0);
    let wins = notch.windows();
    if wins.is_empty() {
        cx.t("Looking for open windows\u{2026}", mid, 140.0, 20.0, color(GRAY, a), Align::Center);
        return;
    }
    let first = notch.win_page() * WIN_GRID;
    for (slot, (_, title)) in wins.iter().skip(first).take(WIN_GRID).enumerate() {
        let rc = win_card(x0, slot);
        let (doc, app) = split_title(title);
        cx.rrect(rc, 16.0, color(WHITE, 0.07 * a));
        tile_letter(cx, rect(rc.x + 12.0, rc.y + 12.0, 46.0, 46.0), if app.is_empty() { doc } else { app }, a);
        let tw = rc.w - 76.0;
        let top = cx.fit(doc, 14.0, tw);
        cx.t(&top, rc.x + 68.0, rc.y + if app.is_empty() { 40.0 } else { 33.0 }, 14.0, color(WHITE, a), Align::Left);
        if !app.is_empty() {
            let sub = cx.fit(app, 12.0, tw);
            cx.t(&sub, rc.x + 68.0, rc.y + 52.0, 12.0, color(GRAY, a), Align::Left);
        }
    }
    if wins.len() > WIN_GRID {
        let pages = wins.len().div_ceil(WIN_GRID);
        for (dir, x) in [(-1.0f32, x0 + 640.0), (1.0, x0 + 706.0)] {
            let mut pb = PathBuilder::new();
            pb.move_to(x - 2.5 * dir, 224.0 - 4.5);
            pb.line_to(x + 2.5 * dir, 224.0);
            pb.line_to(x - 2.5 * dir, 224.0 + 4.5);
            if let Some(p) = pb.finish() {
                cx.stroke(&p, color(WHITE, 0.8 * a), 1.8);
            }
        }
        cx.t(&format!("{} / {}", notch.win_page() + 1, pages), x0 + 673.0, 229.0, 12.0, color(GRAY, a), Align::Center);
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Upcoming events (not yet finished), soonest first.
fn upcoming(notch: &Notch, n: usize) -> Vec<&visus_core::CalEvent> {
    let now = now_unix();
    notch.events().iter().filter(|e| e.end >= now).take(n).collect()
}

fn event_buttons(notch: &Notch, x0: f32) -> Vec<(Button, Rect)> {
    upcoming(notch, 2).iter().enumerate().filter(|(_, e)| e.url.is_some()).map(|(i, _)| (Button::Event(i), rect(x0 + 36.0, 204.0 + i as f32 * 20.0, 230.0, 20.0))).collect()
}

fn tile_rect(x0: f32, i: usize) -> Rect {
    rect(x0 + 36.0 + (i % 2) as f32 * 126.0, 66.0 + (i / 2) as f32 * 84.0, 116.0, 74.0)
}

/// Slider track geometry: x, width, and the centre y of the volume and brightness tracks.
const TRACK_X: f32 = 340.0;
const TRACK_W: f32 = 320.0;
const VOLUME_Y: f32 = 116.0;
const BRIGHT_Y: f32 = 184.0;

/// Pointer areas (taller than the thin tracks so they are easy to grab).
fn slider_rect(x0: f32, y: f32) -> Rect {
    rect(x0 + TRACK_X - 10.0, y - 20.0, TRACK_W + 20.0, 40.0)
}

fn control_buttons(notch: &Notch, x0: f32) -> Vec<(Button, Rect)> {
    let mut v: Vec<(Button, Rect)> = (0..4).map(|i| (Button::Tile(i as u8), tile_rect(x0, i))).collect();
    v.push((Button::Volume, slider_rect(x0, VOLUME_Y)));
    if notch.controls().brightness.is_some() {
        v.push((Button::Brightness, slider_rect(x0, BRIGHT_Y)));
    }
    v
}

/// Continues a slider drag; call on pointer motion with pill-logical coordinates.
pub fn drag(notch: &mut Notch, x: f32) {
    let kind = notch.slider_dragging();
    if kind == 0 {
        return;
    }
    let v = ((x - (notch.shape().rect.x + TRACK_X)) / TRACK_W).clamp(0.0, 1.0);
    if kind == 1 {
        notch.set_volume_local(v);
        visus_platform::set_volume(v);
    } else {
        notch.set_brightness_local(v);
        visus_platform::set_brightness(v);
    }
}

fn draw_slider(cx: &mut Ctx, x0: f32, y: f32, label: &str, v: f32, a: f32) {
    let tx = x0 + TRACK_X;
    cx.t(label, tx, y - 30.0, 14.0, color(GRAY, a), Align::Left);
    cx.t(&format!("{:.0}%", v * 100.0), tx + TRACK_W, y - 30.0, 14.0, color(WHITE, a), Align::Right);
    cx.rrect(rect(tx, y - 3.0, TRACK_W, 6.0), 3.0, color(WHITE, 0.16 * a));
    if v > 0.0 {
        cx.rrect(rect(tx, y - 3.0, (TRACK_W * v).max(6.0), 6.0), 3.0, color(WHITE, 0.95 * a));
    }
    let kx = tx + TRACK_W * v;
    cx.circle(kx, y + 1.0, 11.0, color([0, 0, 0], 0.35 * a));
    cx.circle(kx, y, 9.0, color(WHITE, a));
}

fn quad_arc(pb: &mut PathBuilder, cx: f32, cy: f32, r: f32) {
    pb.move_to(cx - r * 0.85, cy - r * 0.45);
    pb.quad_to(cx, cy - r * 1.45, cx + r * 0.85, cy - r * 0.45);
}

fn control_icon(cx: &mut Ctx, i: usize, (x, y): (f32, f32), c: [u8; 4], muted: bool) {
    let mut pb = PathBuilder::new();
    match i {
        0 => {
            quad_arc(&mut pb, x, y + 5.0, 5.0);
            quad_arc(&mut pb, x, y + 5.0, 9.0);
            if let Some(p) = pb.finish() {
                cx.stroke(&p, c, 1.8);
            }
            cx.circle(x, y + 5.0, 1.6, c);
        }
        1 => {
            let pts = [(-3.6, -3.8), (3.4, 3.2), (0.0, 6.8), (0.0, -6.8), (3.4, -3.2), (-3.6, 3.8)];
            pb.move_to(x + pts[0].0, y + pts[0].1);
            for p in &pts[1..] {
                pb.line_to(x + p.0, y + p.1);
            }
            if let Some(p) = pb.finish() {
                cx.stroke(&p, c, 1.6);
            }
        }
        2 => {
            pb.move_to(x - 7.0, y - 3.0);
            pb.line_to(x - 3.5, y - 3.0);
            pb.line_to(x + 1.0, y - 7.0);
            pb.line_to(x + 1.0, y + 7.0);
            pb.line_to(x - 3.5, y + 3.0);
            pb.line_to(x - 7.0, y + 3.0);
            pb.close();
            if let Some(p) = pb.finish() {
                cx.fill(&p, c);
            }
            let mut w = PathBuilder::new();
            if muted {
                w.move_to(x + 4.0, y - 3.5);
                w.line_to(x + 10.0, y + 3.5);
                w.move_to(x + 10.0, y - 3.5);
                w.line_to(x + 4.0, y + 3.5);
            } else {
                w.move_to(x + 4.5, y - 4.0);
                w.quad_to(x + 9.0, y, x + 4.5, y + 4.0);
            }
            if let Some(p) = w.finish() {
                cx.stroke(&p, c, 1.7);
            }
        }
        _ => {
            pb.move_to(x + 2.5, y - 7.5);
            pb.cubic_to(x - 9.0, y - 6.0, x - 9.0, y + 6.0, x + 2.5, y + 7.5);
            pb.cubic_to(x - 3.0, y + 4.0, x - 3.0, y - 4.0, x + 2.5, y - 7.5);
            pb.close();
            if let Some(p) = pb.finish() {
                cx.fill(&p, c);
            }
        }
    }
}

fn draw_control(cx: &mut Ctx, notch: &Notch, a: f32) {
    let x0 = notch.shape().rect.x;
    let c = notch.controls();
    let tiles: [(&str, Option<bool>, [u8; 3]); 4] = [
        ("Wi-Fi", c.wifi, [10, 132, 255]),
        ("Bluetooth", c.bluetooth, [10, 132, 255]),
        ("Mute", c.has_volume.then_some(c.muted), ORANGE),
        ("Quiet", Some(notch.settings().quiet), [191, 90, 242]),
    ];
    for (i, (label, state, tint)) in tiles.iter().enumerate() {
        let rc = tile_rect(x0, i);
        let on = *state == Some(true);
        let (bg, badge, ink) = match state {
            Some(true) => (color(*tint, 0.96 * a), color([255, 255, 255], 0.28 * a), color([255, 255, 255], a)),
            Some(false) => (color(WHITE, 0.10 * a), color(WHITE, 0.12 * a), color(WHITE, a)),
            None => (color(WHITE, 0.04 * a), color(WHITE, 0.06 * a), color(GRAY, 0.7 * a)),
        };
        cx.rrect(rc, 22.0, bg);
        if let Some(edge) = rrect(rc.x + 0.5, rc.y + 0.5, rc.w - 1.0, rc.h - 1.0, 22.0) {
            cx.stroke(&edge, color(WHITE, 0.07 * a), 1.0);
        }
        let ic = (rc.x + 28.0, rc.y + 27.0);
        cx.circle(ic.0, ic.1, 15.0, badge);
        control_icon(cx, i, ic, ink, c.muted);
        cx.t(label, rc.x + 16.0, rc.y + 62.0, 15.0, ink, Align::Left);
        let sub = match state {
            Some(true) => "On",
            Some(false) => "Off",
            None => "N/A",
        };
        cx.t(sub, rc.x + rc.w - 14.0, rc.y + 31.0, 12.0, color(if on { [255, 255, 255] } else { GRAY }, 0.85 * a), Align::Right);
    }

    draw_slider(cx, x0, VOLUME_Y, "Volume", if c.muted { 0.0 } else { c.volume }, a);
    match c.brightness {
        Some(b) => draw_slider(cx, x0, BRIGHT_Y, "Brightness", b, a),
        None => {
            cx.t("Brightness", x0 + TRACK_X, BRIGHT_Y - 30.0, 14.0, color(GRAY, 0.6 * a), Align::Left);
            cx.t("Not available on this display", x0 + TRACK_X, BRIGHT_Y, 13.0, color(GRAY, 0.6 * a), Align::Left);
        }
    }
}

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
        Page::Control => control_buttons(notch, x0),
        Page::Stats => vec![(Button::NetUnit, rect(x0 + 470.0, 80.0, 230.0, 140.0))],
        Page::Notes => {
            let mut v = list_buttons(x0, notch.notes().len(), Button::NoteCopy, Button::NoteDelete);
            if notch.notes().is_empty() {
                v.push((Button::NewNote, rect(cx - 70.0, 178.0, 140.0, 32.0)));
            }
            v
        }
        Page::Clipboard => {
            let mut v = list_buttons(x0, notch.clips().len(), Button::ClipCopy, Button::ClipDelete);
            if notch.clips().is_empty() && !notch.settings().clipboard_history {
                v.push((Button::ClipEnable, rect(cx - 70.0, 178.0, 140.0, 32.0)));
            }
            v
        }
        Page::Windows => {
            let first = notch.win_page() * WIN_GRID;
            let mut v: Vec<_> = (first..notch.windows().len().min(first + WIN_GRID)).map(|i| (Button::WindowFocus(i), win_card(x0, i - first))).collect();
            if notch.windows().len() > WIN_GRID {
                v.push((Button::WinPrev, rect(x0 + 624.0, 212.0, 32.0, 24.0)));
                v.push((Button::WinNext, rect(x0 + 690.0, 212.0, 32.0, 24.0)));
            }
            v
        }
        Page::Calendar => event_buttons(notch, x0),
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
    /// Theme accent and the text colour that reads on top of it.
    accent: [u8; 3],
    ink: [u8; 3],
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
    let accent = notch.theme();
    let lum = 0.2126 * accent[0] as f32 + 0.7152 * accent[1] as f32 + 0.0722 * accent[2] as f32;
    let ink = if lum > 150.0 { [0, 0, 0] } else { [255, 255, 255] };
    let mut cx = Ctx { accent, ink, pm, text, scale: scale * user, ox: centre * (1.0 - user) * scale, oy: 0.0 };

    let r = shape.rect;
    if let Some(path) = pill_path(&shape) {
        cx.fill(&path, [0, 0, 0, 255]);
        // Album-art ambience: the pill's lower edge picks up the cover colour and breathes with the music.
        let m = notch.media();
        if m.has_session && m.playing && m.art.is_some() {
            let tint = notch.accent();
            let level = notch.visualizer_level().unwrap_or(0.35);
            let amt = if notch.is_open() && notch.page == Page::Music { 0.32 } else { 0.2 } + 0.12 * level;
            let stop = |at: f32, al: f32| GradientStop::new(at, Color::from_rgba8(tint[0], tint[1], tint[2], (al.clamp(0.0, 1.0) * 255.0) as u8));
            if let Some(shader) = LinearGradient::new(
                Point::from_xy(centre, 0.0),
                Point::from_xy(centre, r.h),
                vec![stop(0.0, 0.0), stop(0.55, amt * 0.25), stop(1.0, amt)],
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                let mut p = Paint::default();
                p.shader = shader;
                p.anti_alias = true;
                let xf = cx.xf();
                cx.pm.fill_path(&path, &p, FillRule::Winding, xf, None);
            }
            // Two slow swells roll horizontally along the lower edge, clipped to the pill.
            if let Some(mut mask) = tiny_skia::Mask::new(cx.pm.width(), cx.pm.height()) {
                let xf = cx.xf();
                mask.fill_path(&path, FillRule::Winding, true, xf);
                let t = anim_secs();
                let open = notch.is_open();
                let (base, amp) = if open { (r.h * 0.80, 11.0) } else { (r.h * 0.64, 3.0) };
                for (phase, speed, alpha, drop) in [(0.0f32, 0.96f32, 0.55f32, 0.0f32), (2.1, -0.96, 0.35, r.h * 0.06)] {
                    let (x_l, x_r, n) = (r.x - 24.0, r.x + r.w + 24.0, 56);
                    let mut pb = PathBuilder::new();
                    for i in 0..=n {
                        let u = i as f32 / n as f32;
                        let x = x_l + (x_r - x_l) * u;
                        let y = base + drop + amp * ((u * std::f32::consts::TAU * 1.6 + t * speed + phase).sin() + 0.3 * (u * std::f32::consts::TAU * 2.4 - t * speed).sin());
                        if i == 0 {
                            pb.move_to(x, y);
                        } else {
                            pb.line_to(x, y);
                        }
                    }
                    pb.line_to(x_r, r.h + 12.0);
                    pb.line_to(x_l, r.h + 12.0);
                    pb.close();
                    let Some(wave) = pb.finish() else { continue };
                    let stop = |at: f32, al: f32| GradientStop::new(at, Color::from_rgba8(tint[0], tint[1], tint[2], (al.clamp(0.0, 1.0) * 255.0) as u8));
                    if let Some(shader) = LinearGradient::new(
                        Point::from_xy(centre, base - amp),
                        Point::from_xy(centre, r.h),
                        vec![stop(0.0, amt * alpha * 0.8), stop(1.0, amt * alpha * 0.25)],
                        SpreadMode::Pad,
                        Transform::identity(),
                    ) {
                        let mut p = Paint::default();
                        p.shader = shader;
                        p.anti_alias = true;
                        cx.pm.fill_path(&wave, &p, FillRule::Winding, xf, Some(&mask));
                    }
                }
            }
        }
        // Ripple: the rim flashes and fades when an alert lands.
        if let Some(age) = notch.alert_age() {
            let k = 1.0 - age / 0.9;
            cx.stroke(&path, color(WHITE, 0.55 * k * k), 1.5 + age * 5.0);
        }
    }

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
        let text = match alert {
            Alert::DownloadDone => notch.alert_text(),
            _ => notch.alert_event().map(|e| e.title.as_str()),
        };
        draw_alert(&mut cx, &shape, alert, text, (notch.settings().eye_style, notch.settings().eye_moods, notch.alert_age().unwrap_or(1.0)), peek_a);
    }
    if open_a > 0.0 {
        draw_open(&mut cx, notch, open_a);
    }
    draw_palette(&mut cx, notch);
    draw_settings_panel(&mut cx, notch);
    draw_hover(&mut cx, notch);
}

/// The button under the pointer, if any (same hit list the click handler uses).
fn hovered(notch: &Notch) -> Option<(Button, Rect)> {
    let (px, py) = notch.pointer()?;
    if palette_layout(notch).is_some() {
        return None;
    }
    let list = if notch.settings_is_open() { settings_buttons(notch) } else { buttons(notch) };
    list.into_iter().find(|(_, r)| r.contains(px, py))
}

/// True when the pointer is over something clickable; hosts use it to show a hand cursor.
pub fn over_clickable(notch: &Notch) -> bool {
    if hovered(notch).is_some() {
        return true;
    }
    match (notch.pointer(), palette_layout(notch)) {
        (Some((px, py)), Some(l)) => l.rows.iter().any(|r| r.contains(px, py)),
        _ => false,
    }
}

/// Moves the launcher selection to the row under the pointer.
pub fn hover_palette(notch: &mut Notch) {
    let (Some((px, py)), Some(l)) = (notch.pointer(), palette_layout(notch)) else { return };
    if let Some(i) = l.rows.iter().position(|r| r.contains(px, py)) {
        notch.palette_select(i);
    }
}

fn draw_hover(cx: &mut Ctx, notch: &Notch) {
    let Some((b, r)) = hovered(notch) else { return };
    match b {
        Button::NoteDelete(_) | Button::ClipDelete(_) | Button::ShelfRemove(_) | Button::TodoDelete(_) | Button::FolderRemove(_) | Button::SettingsClose => {
            cx.circle(r.x + r.w / 2.0, r.y + r.h / 2.0, 13.0, color([255, 69, 58], 0.28));
        }
        Button::Volume | Button::Brightness => {}
        _ => {
            let radius = match b {
                Button::Palette | Button::SettingsOpen => 18.0,
                Button::Tile(_) => 22.0,
                Button::Toggle(_) => 12.0,
                _ => (r.h / 2.0).min(14.0),
            };
            cx.rrect(r, radius, color(WHITE, 0.08));
            if let Some(p) = rrect(r.x + 0.5, r.y + 0.5, r.w - 1.0, r.h - 1.0, radius) {
                cx.stroke(&p, color(WHITE, 0.16), 1.0);
            }
        }
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
    let style = notch.settings().eye_style;
    let (cat, robot) = (style == EyeStyle::Cat, style == EyeStyle::Robot);
    let iris_rgb = if cat { Rgb([170, 210, 60]) } else { notch.settings().eye_iris };
    let sclera = notch.settings().eye_sclera.0;
    let sauron = notch.settings().eye_style == EyeStyle::Sauron;
    let [gr, gg, gb] = if sauron { [255, 95, 20] } else { iris_rgb.0 };
    let t = notch.awake_time();

    let (mood, mood_ph) = notch.eye_mood();
    let mood_e = (mood_ph * std::f32::consts::PI).sin();
    let pulse = notch.eye_pulse();
    if robot {
        draw_robot_eye(cx, x, y, ex, ey, open, t, fade, asleep, r.h, (mood, mood_e), pulse);
        return;
    }
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

    if sauron && !asleep {
        sauron_fire(cx, x, y, ex, ey, open, t, fade, mood == EyeMood::Surprised);
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
            draw_zs(cx, x, y, ex, r.h, t, fade);
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
        let (px, ry) = (x + lx * ex * 0.7, h * 0.62);
        let py = y + ly * h * 0.2;
        let slit = ey * 0.06 * (1.0 + 1.6 * pulse + 1.2 * if mood == EyeMood::Surprised { mood_e } else { 0.0 });
        // A hot rim around the slit makes it read as a burning gap rather than a hole.
        if let Some(rim) = tiny_skia::Rect::from_xywh(px - slit * 2.4, py - ry * 1.05, slit * 4.8, ry * 2.1).and_then(PathBuilder::from_oval) {
            let mut glow = Paint::default();
            glow.anti_alias = true;
            glow.set_color(Color::from_rgba8(255, 200, 70, (fade * 200.0) as u8));
            let t = cx.xf();
            cx.pm.fill_path(&rim, &glow, FillRule::Winding, t, Some(&mask));
        }
        if let Some(oval) = tiny_skia::Rect::from_xywh(px - slit, py - ry, slit * 2.0, ry * 2.0).and_then(PathBuilder::from_oval) {
            let mut ink = Paint::default();
            ink.anti_alias = true;
            ink.set_color(Color::from_rgba8(0, 0, 0, (fade * 255.0) as u8));
            let t = cx.xf();
            cx.pm.fill_path(&oval, &ink, FillRule::Winding, t, Some(&mask));
        }
        return;
    }
    let light = iris_rgb.lighten(0.55);
    let ir = ey * 0.95 * if mood == EyeMood::Surprised { 1.0 - 0.18 * mood_e } else { 1.0 };
    let (ix, iy) = (x + lx * (ex - ir) * 0.9, y + ly * ey * 0.25);
    if mood == EyeMood::Heart {
        // A beating heart takes the place of the iris.
        let s = ir * 1.25 * (1.0 + 0.12 * (mood_ph * std::f32::consts::TAU * 4.0).sin()) * (0.4 + 0.6 * mood_e.sqrt());
        let mut pb = PathBuilder::new();
        pb.move_to(ix, iy + s * 0.9);
        pb.cubic_to(ix - s * 1.3, iy - s * 0.1, ix - s * 0.7, iy - s * 0.95, ix, iy - s * 0.35);
        pb.cubic_to(ix + s * 0.7, iy - s * 0.95, ix + s * 1.3, iy - s * 0.1, ix, iy + s * 0.9);
        pb.close();
        if let Some(heart) = pb.finish() {
            let mut p = Paint::default();
            p.anti_alias = true;
            p.set_color(Color::from_rgba8(255, 70, 110, (fade * 255.0) as u8));
            let t = cx.xf();
            cx.pm.fill_path(&heart, &p, FillRule::Winding, t, Some(&mask));
        }
        return;
    }
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
    let pupil = if cat {
        let pw = ir * 0.17 * (1.0 + 0.9 * pulse);
        tiny_skia::Rect::from_xywh(ix - pw, iy - ir * 0.95, pw * 2.0, ir * 1.9).and_then(PathBuilder::from_oval)
    } else {
        PathBuilder::from_circle(ix, iy, ir * 0.45 * (1.0 + 0.5 * pulse))
    };
    if let Some(c) = pupil {
        cx.pm.fill_path(&c, &ink, FillRule::Winding, t, Some(&mask));
    }
    ink.set_color(Color::from_rgba8(255, 255, 255, (fade * 230.0) as u8));
    if let Some(c) = PathBuilder::from_circle(ix - ir * 0.3, iy - ir * 0.35, ir * 0.17) {
        cx.pm.fill_path(&c, &ink, FillRule::Winding, t, Some(&mask));
    }
}

/// Three z's drift up and fade out in turn.
fn draw_zs(cx: &mut Ctx, x: f32, y: f32, ex: f32, pill_h: f32, t: f32, fade: f32) {
    for i in 0..3 {
        let ph = (t / 3.6 + i as f32 / 3.0) % 1.0;
        let zx = x + ex + 3.0 + ph * 14.0 + (ph * 9.0 + i as f32).sin() * 1.5;
        let zy = y + 1.0 - ph * pill_h * 0.38;
        cx.t("z", zx, zy, 6.0 + ph * 6.0, color(WHITE, fade * (ph * std::f32::consts::PI).sin() * 0.85), Align::Left);
    }
}

/// A little robot: a glowing visor with a scanning bar; blinks by squashing, sleeps as a dim line.
#[allow(clippy::too_many_arguments)]
fn draw_robot_eye(cx: &mut Ctx, x: f32, y: f32, ex: f32, ey: f32, open: f32, t: f32, fade: f32, asleep: bool, pill_h: f32, mood: (EyeMood, f32), pulse: f32) {
    let w = ex * 1.9;
    let (m, e) = mood;
    let glitch = m == EyeMood::Glitch;
    let x = if glitch { x + (t * 41.0).sin() * 2.0 } else { x };
    let tint = if m == EyeMood::Heart || (glitch && (t * 13.0) as i32 % 2 == 0) { [255, 80, 160] } else { [60, 220, 255] };
    if asleep || open < 0.12 {
        cx.rrect(rect(x - w / 2.0, y - 1.0, w, 2.0), 1.0, color(tint, fade * if asleep { 0.35 } else { 0.9 }));
        if asleep {
            draw_zs(cx, x, y, w / 2.0, pill_h, t, fade);
        }
        return;
    }
    let h = (ey * 1.7 * open.min(1.25)).max(2.0);
    cx.rrect(rect(x - w / 2.0 - 3.0, y - h / 2.0 - 3.0, w + 6.0, h + 6.0), 9.0, color(tint, fade * (0.16 + 0.12 * pulse)));
    cx.rrect(rect(x - w / 2.0, y - h / 2.0, w, h), 6.0, color([8, 18, 24], fade));
    let inner = rect(x - w / 2.0 + 2.0, y - h / 2.0 + 2.0, w - 4.0, (h - 4.0).max(1.0));
    cx.rrect(inner, 4.5, color(tint, fade * 0.85));
    // Scanning bar sweeps back and forth; the surprise mood makes it dart faster.
    let speed = if m == EyeMood::Surprised { 2.6 + 2.0 * e } else { 0.8 };
    let sweep = ((t * speed).sin() * 0.5 + 0.5) * (inner.w - 3.0);
    cx.rrect(rect(inner.x + sweep, inner.y, 3.0, inner.h), 1.5, color([255, 255, 255], fade * 0.95));
    if glitch {
        for k in 0..2 {
            let gy = inner.y + inner.h * (0.25 + 0.4 * k as f32) + (t * 23.0 + k as f32).sin() * 1.5;
            cx.rrect(rect(inner.x - 2.0, gy, inner.w + 4.0, 1.6), 0.5, color([8, 18, 24], fade * 0.85));
        }
    }
    for dx in [-1.0f32, 1.0] {
        cx.circle(x + dx * (w / 2.0 + 6.0), y, 1.4, color(tint, fade * 0.7));
    }
}

/// Licking flames along both lids, a pulsing red halo and rising embers around the Eye of Sauron.
fn sauron_fire(cx: &mut Ctx, x: f32, y: f32, ex: f32, ey: f32, open: f32, t: f32, fade: f32, rage: bool) {
    // Slow breathing halo.
    let halo = ex * (1.5 + 0.12 * (t * 2.2).sin()) * if rage { 1.25 } else { 1.0 };
    if let (Some(shader), Some(c)) = (
        RadialGradient::new(
            Point::from_xy(x, y),
            Point::from_xy(x, y),
            halo,
            vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 90, 10, (fade * 110.0) as u8)),
                GradientStop::new(0.6, Color::from_rgba8(200, 30, 0, (fade * 50.0) as u8)),
                GradientStop::new(1.0, Color::from_rgba8(120, 10, 0, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        ),
        PathBuilder::from_circle(x, y, halo),
    ) {
        let mut p = Paint::default();
        p.shader = shader;
        p.anti_alias = true;
        let xf = cx.xf();
        cx.pm.fill_path(&c, &p, FillRule::Winding, xf, None);
    }

    // Flame tongues rooted on the lids; each sways and flares on its own rhythm.
    let lid = ey * (0.35 + 0.65 * open.min(1.0));
    let n = 11;
    for i in 0..n {
        let f = i as f32;
        let u = f / (n - 1) as f32 * 2.0 - 1.0;
        let bx = x + u * ex * 0.92;
        // Lid height follows the almond: tallest in the middle.
        let base = lid * (1.0 - u * u) * 0.78;
        for dir in [-1.0f32, 1.0] {
            let seed = f * 1.7 + if dir > 0.0 { 40.0 } else { 0.0 };
            let flare = 0.5 + 0.5 * (t * 7.0 + seed).sin() * (t * 3.1 + seed * 0.7).cos().abs();
            let len = (2.5 + 6.5 * flare) * (0.55 + 0.45 * (1.0 - u * u)) * if rage { 1.4 } else { 1.0 };
            let sway = (t * 4.0 + seed).sin() * 1.6;
            let by = y + dir * base;
            let tip = (bx + sway + u * 1.2, by + dir * len);
            for (scale, col, al) in [(1.0, [200, 25, 0], 0.55), (0.72, [255, 110, 10], 0.75), (0.42, [255, 215, 90], 0.9)] {
                let w = 2.2 * scale + 0.6;
                let mut pb = PathBuilder::new();
                pb.move_to(bx - w, by);
                pb.quad_to(bx - w * 0.6 + sway * 0.3, by + dir * len * 0.55 * scale, tip.0, by + dir * len * scale);
                pb.quad_to(bx + w * 0.6 + sway * 0.3, by + dir * len * 0.55 * scale, bx + w, by);
                pb.close();
                if let Some(path) = pb.finish() {
                    cx.fill(&path, color(col, fade * al));
                }
            }
        }
    }

    // Embers drifting up and fading.
    for i in 0..7 {
        let f = i as f32;
        let ph = (t * 0.55 + f / 7.0) % 1.0;
        let ex_off = (f * 2.37).sin() * ex * 0.8 + (ph * 6.0 + f).sin() * 2.0;
        let ey_off = -ey * 0.8 - ph * ey * 2.6;
        let al = (ph * std::f32::consts::PI).sin() * 0.9;
        cx.circle(x + ex_off, y + ey_off, 0.9 + (1.0 - ph) * 0.6, color([255, 190, 60], fade * al));
    }
}

const SEGMENTS: [u8; 10] = [0b0111111, 0b0000110, 0b1011011, 0b1001111, 0b1100110, 0b1101101, 0b1111101, 0b0000111, 0b1111111, 0b1101111];

/// Hour and minute digits as shown (hour tens blank in 12-hour mode below 10).
fn clock_digits(notch: &Notch, h: u32, m: u32) -> [Option<u8>; 4] {
    let hh = if notch.settings().clock_24h { h } else { [12, h % 12][(h % 12 != 0) as usize] };
    let tens = if !notch.settings().clock_24h && hh < 10 { None } else { Some((hh / 10) as u8) };
    [tens, Some((hh % 10) as u8), Some((m / 10) as u8), Some((m % 10) as u8)]
}

/// One split-flap card; mid-flip it is composed from two cached faces.
#[allow(clippy::too_many_arguments)]
fn flip_card(cx: &mut Ctx, x: f32, y: f32, w: f32, h: f32, old: Option<u8>, new: Option<u8>, p: f32, a: f32) {
    fn face(cx: &mut Ctx, x: f32, y: f32, w: f32, h: f32, d: Option<u8>, a: f32) {
        cx.rrect(rect(x, y, w, h), 4.0, color([34, 34, 40], a));
        if let Some(d) = d {
            cx.t(&d.to_string(), x + w / 2.0, y + h * 0.76, h * 0.74, color(WHITE, a), Align::Center);
        }
        cx.rrect(rect(x, y + h / 2.0 - 0.6, w, 1.2), 0.5, color([0, 0, 0], 0.75 * a));
    }
    if old == new || p >= 1.0 {
        face(cx, x, y, w, h, new, a);
        return;
    }
    let k = cx.scale;
    let (pw, ph) = ((w * k).ceil() as u32 + 2, (h * k).ceil() as u32 + 2);
    let (Some(mut t_old), Some(mut t_new)) = (Pixmap::new(pw, ph), Pixmap::new(pw, ph)) else {
        face(cx, x, y, w, h, new, a);
        return;
    };
    for (pm, d) in [(&mut t_old, old), (&mut t_new, new)] {
        let mut sub = Ctx { accent: cx.accent, ink: cx.ink, pm, text: cx.text, scale: k, ox: -x * k, oy: -y * k };
        face(&mut sub, x, y, w, h, d, a);
    }
    let (px, py, pwf, hh) = (x * k + cx.ox, y * k + cx.oy, w * k, h * k / 2.0);
    let ym = py + hh;
    let mut blit = |pm: &Pixmap, dest: (f32, f32, f32, f32), xf: Transform| {
        let Some(r) = tiny_skia::Rect::from_xywh(dest.0, dest.1, dest.2, dest.3) else { return };
        let mut paint = Paint::default();
        paint.shader = tiny_skia::Pattern::new(pm.as_ref(), SpreadMode::Pad, tiny_skia::FilterQuality::Bilinear, 1.0, xf);
        cx.pm.fill_rect(r, &paint, Transform::identity(), None);
    };
    let place = Transform::from_translate(px, py);
    blit(&t_new, (px, py, pwf, hh), place);
    blit(&t_old, (px, ym, pwf, hh), place);
    if p < 0.5 {
        let s = 1.0 - 2.0 * p;
        blit(&t_old, (px, ym - s * hh, pwf, s * hh), Transform::from_row(1.0, 0.0, 0.0, s, px, ym - s * hh));
    } else {
        let s = 2.0 * p - 1.0;
        blit(&t_new, (px, ym, pwf, s * hh), Transform::from_row(1.0, 0.0, 0.0, s, px, ym - s * hh));
    }
}

/// The pill's clock in the chosen style, centred on (xm, ym); `size` is the font size or digit height.
fn draw_clock(cx: &mut Ctx, notch: &Notch, xm: f32, ym: f32, size: f32, a: f32) {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let (h, m, s) = (now.hour(), now.minute(), now.second());
    let frac = now.timestamp_subsec_millis() as f32 / 1000.0;
    match notch.settings().clock_style {
        ClockStyle::Classic => {
            cx.t(&now.format(clock_fmt(notch)).to_string(), xm, ym + size * 0.35, size, color(WHITE, a), Align::Center);
        }
        ClockStyle::Flip => {
            let (cw, ch, gap, pair) = (size * 0.76, size * 1.3, 2.0, 9.0);
            let total = 4.0 * cw + 2.0 * gap + pair;
            let (x0, y0) = (xm - total / 2.0, ym - ch / 2.0);
            let prev = now - chrono::Duration::minutes(1);
            let (cur, old) = (clock_digits(notch, h, m), clock_digits(notch, prev.hour(), prev.minute()));
            let p = if s == 0 { ((frac * 1000.0) / 380.0).min(1.0) } else { 1.0 };
            let xs = [x0, x0 + cw + gap, x0 + 2.0 * cw + gap + pair, x0 + 3.0 * cw + 2.0 * gap + pair];
            for i in 0..4 {
                flip_card(cx, xs[i], y0, cw, ch, old[i], cur[i], p, a);
            }
            let cxm = x0 + 2.0 * cw + gap + pair / 2.0;
            for dy in [-ch * 0.17, ch * 0.17] {
                cx.circle(cxm, ym + dy, 1.5, color(cx.accent, a));
            }
        }
        ClockStyle::Led => {
            let (dw, dh) = (size * 0.52, size);
            let t = size * 0.14;
            let (gap, pair) = (size * 0.26, size * 0.5);
            let total = 4.0 * dw + 2.0 * gap + pair;
            let (x0, y0) = (xm - total / 2.0, ym - dh / 2.0);
            let xs = [x0, x0 + dw + gap, x0 + 2.0 * dw + gap + pair, x0 + 3.0 * dw + 2.0 * gap + pair];
            for (i, d) in clock_digits(notch, h, m).iter().enumerate() {
                let half = dh / 2.0;
                let segs = [
                    (t * 0.8, 0.0, dw - 1.6 * t, t),
                    (dw - t, t * 0.8, t, half - 1.2 * t),
                    (dw - t, half + t * 0.4, t, half - 1.2 * t),
                    (t * 0.8, dh - t, dw - 1.6 * t, t),
                    (0.0, half + t * 0.4, t, half - 1.2 * t),
                    (0.0, t * 0.8, t, half - 1.2 * t),
                    (t * 0.8, half - t / 2.0, dw - 1.6 * t, t),
                ];
                let mask = d.map_or(0, |d| SEGMENTS[d as usize % 10]);
                for (k, (sx, sy, sw, sh)) in segs.iter().enumerate() {
                    let on = mask & (1 << k) != 0;
                    let rc = rect(xs[i] + sx, y0 + sy, *sw, *sh);
                    if on {
                        cx.rrect(rect(rc.x - 1.2, rc.y - 1.2, rc.w + 2.4, rc.h + 2.4), t * 0.7, color(cx.accent, 0.18 * a));
                        cx.rrect(rc, t * 0.5, color(cx.accent, a));
                    } else if d.is_some() {
                        cx.rrect(rc, t * 0.5, color(WHITE, 0.06 * a));
                    }
                }
            }
            if frac < 0.55 {
                let cxm = x0 + 2.0 * dw + gap + pair / 2.0;
                for dy in [-dh * 0.2, dh * 0.2] {
                    cx.circle(cxm, ym + dy, t * 0.6, color(cx.accent, a));
                }
            }
        }
    }
}

/// Seconds on a wrapping clock; small enough for f32 to keep sub-millisecond phase.
fn anim_secs() -> f32 {
    // Snapshots pin the clock so captured frames line up.
    static FIXED: std::sync::OnceLock<Option<f32>> = std::sync::OnceLock::new();
    if let Some(t) = FIXED.get_or_init(|| std::env::var("VISUS_ANIM_T").ok().and_then(|v| v.parse().ok())) {
        return *t;
    }
    (chrono::Local::now().timestamp_millis().rem_euclid(3_600_000)) as f32 / 1000.0
}

fn clock_fmt(notch: &Notch) -> &'static str {
    if notch.settings().clock_24h { "%H:%M" } else { "%-I:%M %p" }
}

fn draw_closed(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let mid = r.y + r.h / 2.0;
    let timer = notch.timer();
    let timer_active = timer.is_running() || timer.progress() > 0.0;
    // Left slot: the focus timer while one is active, otherwise the battery level.
    if timer_active {
        let left = timer.remaining().as_secs_f32() / timer.total().as_secs_f32();
        cx.ring(r.x + 28.0, mid, 8.0, 2.5, left, cx.accent, a);
    } else if let Some((charging, pct)) = notch.battery().filter(|_| notch.settings().show_battery) {
        let tint = if charging || pct > 20 { GREEN } else { ORANGE };
        cx.ring(r.x + 28.0, mid, 8.0, 2.5, pct as f32 / 100.0, tint, a);
        if charging {
            cx.triangle([(r.x + 29.5, mid - 4.5), (r.x + 25.0, mid + 0.5), (r.x + 28.5, mid + 0.5)], color(WHITE, a));
            cx.triangle([(r.x + 26.5, mid + 4.5), (r.x + 31.0, mid - 0.5), (r.x + 27.5, mid - 0.5)], color(WHITE, a));
        }
    }
    // Thin progress line along the bottom edge: the focus timer, otherwise the playing track.
    let rad = (r.h * 0.4).min(r.w / 4.0);
    let m = notch.media();
    let progress = if timer_active {
        Some((1.0 - timer.remaining().as_secs_f32() / timer.total().as_secs_f32(), cx.accent))
    } else if m.has_session && m.playing && !m.duration.is_zero() {
        Some((notch.media_position().as_secs_f32() / m.duration.as_secs_f32(), notch.accent()))
    } else {
        None
    };
    if let Some((frac, tint)) = progress {
        let w = ((r.w - 2.0 * rad) * frac.clamp(0.0, 1.0)).max(2.0);
        cx.rrect(rect(r.x + rad, r.h - 4.0, r.w - 2.0 * rad, 1.5), 0.75, color(WHITE, 0.10 * a));
        cx.rrect(rect(r.x + rad, r.h - 4.0, w, 1.5), 0.75, color(tint, 0.9 * a));
    }
    draw_clock(cx, notch, r.x + r.w / 2.0, mid, 20.0, a);
    if let Some(rate) = notch.download() {
        let txt = rate_fmt(rate, notch.settings().net_bits);
        let tw = cx.text_width(&txt, 12.0);
        let rx = r.x + r.w - 22.0;
        cx.t(&txt, rx, mid + 4.5, 12.0, color(WHITE, a), Align::Right);
        let ax = rx - tw - 11.0;
        let bob = (anim_secs() * 3.33).sin() * 1.6;
        cx.rrect(rect(ax - 1.0, mid - 7.0 + bob, 2.0, 7.0), 1.0, color(cx.accent, a));
        cx.triangle([(ax - 4.5, mid - 1.0 + bob), (ax + 4.5, mid - 1.0 + bob), (ax, mid + 5.0 + bob)], color(cx.accent, a));
    }
    if notch.wants_audio() {
        let bars = notch.visualizer_bars();
        let tint = notch.accent();
        let (bw, gap) = (2.4, 2.6);
        let x0 = r.x + r.w - 24.0 - (bars.len() as f32 * bw + (bars.len() - 1) as f32 * gap);
        for (i, v) in bars.iter().enumerate() {
            let h = 3.5 + 19.0 * v.clamp(0.0, 1.0);
            let x = x0 + i as f32 * (bw + gap);
            cx.rrect(rect(x - 1.5, mid - h / 2.0 - 2.0, bw + 3.0, h + 4.0), (bw + 3.0) / 2.0, color(tint, 0.14 * a));
            cx.rrect(rect(x, mid - h / 2.0, bw, h), bw / 2.0, color(tint, a));
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Expr {
    Happy,
    Sad,
    Dizzy,
    Music,
    Party,
    Alert,
    Game,
}

fn expr_for(a: Alert) -> Option<Expr> {
    match a {
        Alert::Charging { .. } | Alert::Online | Alert::DownloadDone => Some(Expr::Happy),
        Alert::Unplugged { .. } => Some(Expr::Sad),
        Alert::Headphones => Some(Expr::Music),
        Alert::Offline => Some(Expr::Dizzy),
        Alert::Controller(true) => Some(Expr::Game),
        Alert::Controller(false) => Some(Expr::Sad),
        Alert::TimerDone => Some(Expr::Party),
        Alert::TodoDue | Alert::Meeting { .. } => Some(Expr::Alert),
        _ => None,
    }
}

/// The mascot's face for an alert: two expressive eyes in the current skin's colour, bouncing as the alert lands.
fn draw_face(cx: &mut Ctx, x: f32, y: f32, style: EyeStyle, expr: Expr, age: f32, a: f32) {
    let (tint, visor) = match style {
        EyeStyle::Classic => ([235, 240, 255], false),
        EyeStyle::Sauron => ([255, 130, 30], false),
        EyeStyle::Cat => ([190, 225, 70], false),
        EyeStyle::Robot => ([60, 220, 255], true),
    };
    let y = y - (1.0 - (age / 0.9).min(1.0)) * 4.0 * (age * 16.0).sin().abs();
    let ink = if visor { [8, 18, 24] } else { tint };
    if visor {
        cx.rrect(rect(x - 18.0, y - 13.0, 36.0, 26.0), 9.0, color(tint, 0.92 * a));
    } else {
        cx.circle(x, y, 18.0, color(WHITE, 0.07 * a));
    }
    let mut pb = PathBuilder::new();
    let mut fill_dots: Vec<(f32, f32, f32, [u8; 3])> = Vec::new();
    for dx in [-7.0f32, 7.0] {
        let (ex, ey) = (x + dx, y - 2.0);
        match expr {
            Expr::Happy | Expr::Music => {
                pb.move_to(ex - 4.5, ey + 2.5);
                pb.quad_to(ex, ey - 6.0, ex + 4.5, ey + 2.5);
            }
            Expr::Sad => {
                pb.move_to(ex - 4.5, ey - 1.0);
                pb.quad_to(ex, ey + 7.0, ex + 4.5, ey - 1.0);
            }
            Expr::Dizzy => {
                pb.move_to(ex - 3.5, ey - 3.5);
                pb.line_to(ex + 3.5, ey + 3.5);
                pb.move_to(ex + 3.5, ey - 3.5);
                pb.line_to(ex - 3.5, ey + 3.5);
            }
            Expr::Party => {
                pb.move_to(ex - 4.5, ey);
                pb.line_to(ex + 4.5, ey);
                pb.move_to(ex, ey - 4.5);
                pb.line_to(ex, ey + 4.5);
            }
            Expr::Game => {
                // Excited "> <" eyes.
                let s = if dx < 0.0 { 1.0 } else { -1.0 };
                pb.move_to(ex - 3.0 * s, ey - 4.0);
                pb.line_to(ex + 2.5 * s, ey);
                pb.line_to(ex - 3.0 * s, ey + 4.0);
            }
            Expr::Alert => {
                if let Some(c) = PathBuilder::from_circle(ex, ey, 4.2) {
                    cx.stroke(&c, color(ink, a), 2.0);
                }
                fill_dots.push((ex, ey, 1.6, ink));
            }
        }
    }
    // Mouth for the cheerful ones.
    if matches!(expr, Expr::Happy | Expr::Music | Expr::Party | Expr::Game) {
        pb.move_to(x - 4.5, y + 6.0);
        pb.quad_to(x, y + 11.0, x + 4.5, y + 6.0);
    }
    if let Some(p) = pb.finish() {
        cx.stroke(&p, color(ink, a), 2.2);
    }
    for (dx, dy, r, c) in fill_dots {
        cx.circle(dx, dy, r, color(c, a));
    }
    match expr {
        Expr::Sad => cx.circle(x + 14.0, y + 5.0, 1.8, color([120, 190, 255], a)),
        Expr::Game => {
            // A little gamepad in the corner.
            cx.rrect(rect(x + 8.0, y + 6.0, 13.0, 7.0), 3.5, color(tint, a));
            cx.circle(x + 12.0, y + 9.5, 1.1, color(ink, a));
            cx.circle(x + 17.0, y + 9.5, 1.1, color(ink, a));
        }
        Expr::Music => {
            cx.circle(x + 15.0, y + 8.0, 2.2, color(tint, a));
            cx.rrect(rect(x + 16.2, y - 3.0, 1.4, 11.0), 0.7, color(tint, a));
        }
        Expr::Party => {
            for (i, col) in [[255, 90, 120], [255, 210, 60], [90, 220, 130], [110, 160, 255]].iter().enumerate() {
                let ang = i as f32 * 1.6 + age * 3.0;
                cx.circle(x + ang.cos() * 21.0, y + ang.sin() * 15.0, 1.5, color(*col, a));
            }
        }
        _ => {}
    }
}

fn draw_alert(cx: &mut Ctx, shape: &Shape, alert: Alert, event_title: Option<&str>, look: (EyeStyle, bool, f32), a: f32) {
    let r = shape.rect;
    let meeting = match alert {
        Alert::Meeting { mins } => format!("Meeting in {mins} min"),
        _ => String::new(),
    };
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
        Alert::TimerDone => ("Focus complete", "Time for a break".into(), cx.accent, 1.0, None),
        Alert::Online => ("Back online", "Connected to the internet".into(), GREEN, 1.0, None),
        Alert::Offline => ("Offline", "Connection lost".into(), ORANGE, 0.0, None),
        Alert::Language(l) => ("Keyboard", String::from_utf8_lossy(&l).to_uppercase(), BLUE, 1.0, None),
        Alert::TodoDue => ("Time's up!", "A task is due".into(), ORANGE, 1.0, None),
        Alert::Controller(on) => ("Controller", if on { "Connected" } else { "Disconnected" }.into(), if on { BLUE } else { GRAY }, 1.0, None),
        Alert::DownloadDone => ("Download complete", event_title.unwrap_or("").to_string(), GREEN, 1.0, None),
        Alert::Meeting { .. } => (meeting.as_str(), event_title.unwrap_or("Click to join").to_string(), BLUE, 1.0, None),
    };
    match (look.1, expr_for(alert)) {
        (true, Some(expr)) => draw_face(cx, ix, mid, look.0, expr, look.2, a),
        _ => cx.ring(ix, mid, 16.0, 3.5, ring, accent, a),
    }
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
    if notch.settings().clock_style == ClockStyle::Classic {
        let header = format!("{}  {}", now.format("%a %-d %b %Y"), now.format(clock_fmt(notch)));
        cx.t(&header, mid, 44.0, 22.0, color(WHITE, a), Align::Center);
    } else {
        cx.t(&now.format("%A %-d %B").to_string(), mid, 20.0, 12.0, color(GRAY, a), Align::Center);
        draw_clock(cx, notch, mid, 40.0, 22.0, a);
    }
    if notch.download().is_none() {
        cx.t(&notch.page.title().to_uppercase(), r.x + 36.0, 44.0, 12.0, color(GRAY, a), Align::Left);
    }
    if let Some(rate) = notch.download() {
        let chip = rect(r.x + 36.0, 12.0, 160.0, 28.0);
        cx.rrect(chip, 14.0, color(WHITE, 0.10 * a));
        cx.rrect(rect(chip.x + 14.0, chip.y + 7.0, 2.0, 8.0), 1.0, color(cx.accent, a));
        cx.triangle([(chip.x + 10.5, chip.y + 14.0), (chip.x + 19.5, chip.y + 14.0), (chip.x + 15.0, chip.y + 20.0)], color(cx.accent, a));
        cx.t(&rate_fmt(rate, notch.settings().net_bits), chip.x + 30.0, chip.y + 19.0, 13.0, color(WHITE, a), Align::Left);
    }
    magnifier(cx, r.x + r.w - 62.0, 26.0, color(WHITE, 0.85 * a));
    gear(cx, r.x + r.w - 102.0, 26.0, color(WHITE, 0.85 * a));

    match notch.page {
        Page::Music => draw_music(cx, notch, a),
        Page::Control => draw_control(cx, notch, a),
        Page::Windows => draw_windows(cx, notch, a),
        Page::Notes => draw_list(cx, notch, notch.notes(), ("No notes yet", "Jot something down and it is saved here", Some("+  New note")), a),
        Page::Clipboard => draw_list(cx, notch, notch.clips(), if notch.settings().clipboard_history { ("Nothing copied yet", "Copy some text (Ctrl+C) and it appears here", None) } else { ("Clipboard history is off", "Nothing is being remembered", Some("Turn on")) }, a),
        Page::Timer => draw_timer(cx, notch, a),
        Page::Stats => draw_stats(cx, notch, a),
        Page::Todo => draw_todo(cx, notch, a),
        Page::Shelf => draw_shelf(cx, notch, a),
        Page::Calendar => draw_calendar(cx, notch, a),
        Page::Weather => draw_weather(cx, notch, a),
    }

    if let Some(msg) = notch.toast_text() {
        let w = cx.text_width(msg, 13.0) + 32.0;
        cx.rrect(rect(mid - w / 2.0, r.h - 46.0, w, 26.0), 13.0, color([66, 66, 72], a));
        cx.t(msg, mid, r.h - 28.0, 13.0, color(WHITE, a), Align::Center);
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
    let off = n * 7.0 + 14.0;
    for (dir, x) in [(-1.0f32, mid - off - 12.0), (1.0, mid + off + 12.0)] {
        let mut pb = PathBuilder::new();
        pb.move_to(x - 2.5 * dir, y - 4.5);
        pb.line_to(x + 2.5 * dir, y);
        pb.line_to(x - 2.5 * dir, y + 4.5);
        if let Some(p) = pb.finish() {
            cx.stroke(&p, color(WHITE, 0.7 * a), 1.8);
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

    draw_art(cx, rect(x0 + 39.0, 62.0, 96.0, 96.0), 18.0, m, notch.vinyl_angle(), a);

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
    draw_art(cx, rect(x0 + 36.0, 62.0, 72.0, 72.0), 14.0, m, notch.vinyl_angle(), a);
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
    cx.ring(x0 + 170.0, 118.0, 52.0, 9.0, t.remaining().as_secs_f32() / t.total().as_secs_f32(), cx.accent, a);
    cx.t(&mmss(t.remaining()), x0 + 170.0, 128.0, 30.0, color(WHITE, a), Align::Center);

    let mins = t.total().as_secs() / 60;
    for (b, rc) in timer_buttons(x0) {
        match b {
            Button::TimerPreset(m) => {
                let active = u64::from(m) == mins;
                cx.rrect(rc, 15.0, if active { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.12 * a) });
                let ink = if active { color(cx.ink, a) } else { color(WHITE, a) };
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
                cx.rrect(rc, 22.0, if go { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.14 * a) });
                let ink = if go { color(cx.ink, a) } else { color(WHITE, a) };
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
    if notch.alert_kind() == Some(Alert::DownloadDone) {
        if let Some(d) = dirs::download_dir() {
            return Outcome::OpenFolder { path: d.to_string_lossy().into_owned(), manager: notch.settings().file_manager.clone() };
        }
    }
    if let Some(Alert::Meeting { .. }) = notch.alert_kind() {
        if let Some(url) = notch.alert_event().and_then(|e| e.url.clone()) {
            return Outcome::OpenUrl(url);
        }
    }
    // Clicking a lyric line jumps the track to that line.
    if notch.is_open() && notch.page == Page::Music && notch.lyrics_open() {
        if let Lyrics::Ready(lines) = notch.lyrics() {
            let s = notch.shape().rect;
            let (lx, lw) = (s.x + 392.0, s.w - 392.0 - 28.0);
            if x >= lx && x <= lx + lw {
                let pos = notch.media_position();
                let focus = current_index(lines, pos).map_or(-1, |i| i as i32);
                for off in -1..=3i32 {
                    let base = [96.0, 130.0, 164.0, 194.0, 222.0][(off + 1) as usize];
                    if y >= base - 22.0 && y <= base + 8.0 {
                        if let Some(line) = usize::try_from(focus + off).ok().and_then(|i| lines.get(i)) {
                            visus_platform::media_command(MediaCommand::Seek { target: line.at, current: pos });
                            return Outcome::None;
                        }
                    }
                }
            }
        }
    }
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
            Button::PlayPause => {
                notch.toggle_play_local();
                visus_platform::media_command(MediaCommand::PlayPause);
            }
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
                Toggle::Quiet => notch.update_settings(|s| s.quiet = !s.quiet),
                Toggle::Clock24 => notch.update_settings(|s| s.clock_24h = !s.clock_24h),
                Toggle::Visualizer => notch.update_settings(|s| s.visualizer = !s.visualizer),
                Toggle::Battery => notch.update_settings(|s| s.show_battery = !s.show_battery),
                Toggle::Clipboard => notch.update_settings(|s| s.clipboard_history = !s.clipboard_history),
                Toggle::EyeMoods => notch.update_settings(|s| s.eye_moods = !s.eye_moods),
                Toggle::ClockStyle => notch.update_settings(|s| s.clock_style = s.clock_style.next()),
                Toggle::Autostart => {
                    let _ = visus_platform::set_autostart(!visus_platform::autostart_enabled());
                }
            },
            Button::Size(i) => notch.update_settings(|s| s.scale = SIZES[i as usize % SIZES.len()].1),
            Button::EyeColour(i) => notch.update_settings(|s| s.eye_iris = Rgb(EYE_COLOURS[i as usize % EYE_COLOURS.len()])),
            Button::Style(i) => notch.update_settings(|s| s.eye_style = [EyeStyle::Classic, EyeStyle::Sauron, EyeStyle::Cat, EyeStyle::Robot][i as usize % 4]),
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
            Button::NoteCopy(i) => {
                if let Some(t) = notch.notes().get(i).cloned() {
                    visus_platform::set_clipboard(t);
                    notch.toast("Copied to clipboard");
                }
            }
            Button::NoteDelete(i) => notch.note_remove(i),
            Button::ClipCopy(i) => {
                if let Some(t) = notch.clips().get(i).cloned() {
                    visus_platform::set_clipboard(t);
                    notch.toast("Copied to clipboard");
                }
            }
            Button::WindowFocus(i) => {
                if let Some((h, _)) = notch.windows().get(i) {
                    visus_platform::focus_window(*h);
                }
            }
            Button::NewNote => notch.open_palette_with("note "),
            Button::ClipEnable => notch.update_settings(|s| s.clipboard_history = true),
            Button::Downloads => {
                if let Some(d) = dirs::download_dir() {
                    return Outcome::OpenFolder { path: d.to_string_lossy().into_owned(), manager: notch.settings().file_manager.clone() };
                }
            }
            Button::ShelfReveal(i) => {
                if let Some(p) = notch.shelf().get(i) {
                    let dir = std::path::Path::new(p).parent().map_or(p.clone(), |d| d.to_string_lossy().into_owned());
                    return Outcome::OpenFolder { path: dir, manager: notch.settings().file_manager.clone() };
                }
            }
            Button::ClipDelete(i) => notch.clip_remove(i),
            Button::Event(i) => {
                if let Some(url) = upcoming(notch, 2).get(i).and_then(|e| e.url.clone()) {
                    return Outcome::OpenUrl(url);
                }
            }
            Button::WinPrev => notch.win_step(-1),
            Button::WinNext => notch.win_step(1),
            Button::PagePrev => notch.next_page(-1),
            Button::PageNext => notch.next_page(1),
            Button::NetUnit => notch.update_settings(|s| s.net_bits = !s.net_bits),
            Button::Volume => {
                notch.begin_drag(1);
                drag(notch, x);
            }
            Button::Brightness => {
                notch.begin_drag(2);
                drag(notch, x);
            }
            Button::Tile(i) => {
                let c = notch.controls();
                match i {
                    0 | 1 => {
                        let (radio, state) = if i == 0 { (Radio::Wifi, c.wifi) } else { (Radio::Bluetooth, c.bluetooth) };
                        if let Some(on) = state {
                            notch.set_radio_local(radio, !on);
                            visus_platform::set_radio(radio, !on);
                        }
                    }
                    2 => {
                        if c.has_volume {
                            notch.set_muted_local(!c.muted);
                            visus_platform::set_muted(!c.muted);
                        }
                    }
                    _ => notch.update_settings(|s| s.quiet = !s.quiet),
                }
            }
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
    // While closing the panel keeps its full geometry; the genie warp is applied at draw time.
    let p = if notch.palette_is_open() { p.clamp(0.0, 1.04) } else { 1.0 };
    let w = PANEL_W * (0.72 + 0.28 * p.min(1.0));
    let panel = Rect { x: centre - w / 2.0, y: s.h + 10.0, w, h: full_h * p };
    let search = Rect { x: panel.x + 16.0, y: panel.y + 14.0, w: w - 32.0, h: 40.0 };
    let rows = (0..notch.palette().results.len())
        .map(|i| Rect { x: panel.x + 10.0, y: panel.y + PANEL_TOP_PAD + i as f32 * ROW_H, w: w - 20.0, h: ROW_H - 4.0 })
        .collect();
    Some(PaletteLayout { panel, search, rows, full_h })
}

fn magnifier(cx: &mut Ctx, x: f32, y: f32, c: [u8; 4]) {
    // Drawn so the whole glyph (lens plus handle) is centred on (x, y).
    if let Some(p) = PathBuilder::from_circle(x - 2.4, y - 2.4, 6.5) {
        cx.stroke(&p, c, 2.0);
    }
    let mut pb = PathBuilder::new();
    pb.move_to(x + 2.4, y + 2.4);
    pb.line_to(x + 7.6, y + 7.6);
    if let Some(p) = pb.finish() {
        cx.stroke(&p, c, 2.2);
    }
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Closing launcher: the fully drawn panel is sliced by row and each slice is narrowed and pulled up
/// into the pill, rows nearest the pill first, like a minimising macOS window.
fn draw_palette(cx: &mut Ctx, notch: &Notch) {
    let Some(l) = palette_layout(notch) else { return };
    if notch.palette_is_open() {
        draw_panel(cx, notch, &l, notch.palette_progress().clamp(0.0, 1.0));
        return;
    }
    let g = (1.0 - notch.palette_progress()).clamp(0.0, 1.0);
    let Some(mut tmp) = Pixmap::new(cx.pm.width(), cx.pm.height()) else { return };
    {
        let mut sub = Ctx { accent: cx.accent, ink: cx.ink, pm: &mut tmp, text: cx.text, scale: cx.scale, ox: cx.ox, oy: cx.oy };
        draw_panel(&mut sub, notch, &l, 1.0);
    }
    let (k, ox, oy) = (cx.scale, cx.ox, cx.oy);
    let centre = (l.panel.x + l.panel.w / 2.0) * k + ox;
    let src_top = (l.panel.y - 16.0) * k + oy;
    let src_h = (l.panel.h + 32.0) * k;
    // A small swell at the start makes the pull-in feel springy.
    let swell = 1.0 + 0.05 * (std::f32::consts::PI * (g / 0.3).min(1.0)).sin();
    let y_shrink = (1.0 - smooth((g - 0.05) / 0.95)) * swell;
    // The funnel ends as a point just under the pill instead of inside it.
    let pill_bottom = (notch.shape().rect.h + 3.0) * k + oy;
    let dst_top = src_top + (pill_bottom - src_top) * smooth(g);
    let alpha = 1.0 - smooth((g - 0.84) / 0.15);
    let half = (PANEL_W / 2.0 + 16.0) * k;
    const N: usize = 90;
    for i in 0..N {
        let (u0, u1) = (i as f32 / N as f32, (i + 1) as f32 / N as f32);
        let um = (u0 + u1) / 2.0;
        let sx = ((1.0 - smooth(g * 1.25 - 0.25 * um)) * swell).max(0.004);
        let (sy0, dy0, dy1) = (src_top + u0 * src_h, dst_top + u0 * src_h * y_shrink, dst_top + u1 * src_h * y_shrink);
        let dh = dy1 - dy0;
        if dh < 0.02 {
            continue;
        }
        let kk = dh / (src_h / N as f32);
        let xf = Transform::from_row(sx, 0.0, 0.0, kk, centre - centre * sx, dy0 - sy0 * kk);
        let mut paint = Paint::default();
        paint.shader = Pattern::new(tmp.as_ref(), SpreadMode::Pad, FilterQuality::Bilinear, alpha, xf);
        if let Some(r) = tiny_skia::Rect::from_xywh(centre - half * sx, dy0, 2.0 * half * sx, dh + 0.7) {
            cx.pm.fill_rect(r, &paint, Transform::identity(), None);
        }
    }
}

fn draw_panel(cx: &mut Ctx, notch: &Notch, l: &PaletteLayout, p: f32) {
    let ca = ((p - 0.5) / 0.5).clamp(0.0, 1.0);
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
                Kind::Calc => (cx.accent, "=".to_string()),
                Kind::Web => (BLUE, "\u{2197}".to_string()),
                Kind::Window => ([90, 130, 255], "W".to_string()),
                Kind::Note => ([255, 204, 0], "N".to_string()),
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

/// Speeds in bytes (KB/s, MB/s, GB/s) or in bits as ISPs quote them (Kbps, Mbps, Gbps).
fn rate_fmt(bytes_per_sec: u64, bits: bool) -> String {
    if bits {
        let b = bytes_per_sec as f64 * 8.0;
        if b >= 1e9 {
            format!("{:.2} Gbps", b / 1e9)
        } else if b >= 1e6 {
            format!("{:.1} Mbps", b / 1e6)
        } else {
            format!("{:.0} Kbps", b / 1e3)
        }
    } else {
        let b = bytes_per_sec as f64;
        if b >= 1024.0 * 1024.0 * 1024.0 {
            format!("{:.2} GB/s", b / (1024.0 * 1024.0 * 1024.0))
        } else if b >= 1024.0 * 1024.0 {
            format!("{:.1} MB/s", b / (1024.0 * 1024.0))
        } else {
            format!("{:.0} KB/s", b / 1024.0)
        }
    }
}

fn draw_stats(cx: &mut Ctx, notch: &Notch, a: f32) {
    let x0 = notch.shape().rect.x;
    let st = notch.stats();
    let gauges = [
        ("CPU", st.cpu, format!("{:.0}%", st.cpu * 100.0), cx.accent, x0 + 130.0),
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
    let bits = notch.settings().net_bits;
    cx.t(&format!("\u{2193} {}", rate_fmt(st.rx_bytes_per_sec, bits)), x0 + 480.0, 138.0, 26.0, color(WHITE, a), Align::Left);
    cx.t(&format!("\u{2191} {}", rate_fmt(st.tx_bytes_per_sec, bits)), x0 + 480.0, 174.0, 26.0, color(WHITE, 0.75 * a), Align::Left);
    cx.t(if bits { "Live traffic  \u{b7}  tap for MB/s" } else { "Live traffic  \u{b7}  tap for Mbps" }, x0 + 480.0, 204.0, 12.0, color(GRAY, 0.8 * a), Align::Left);
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
/// A cached premultiplied copy of the cover (shared with `draw_image`).
fn cached_art(art: &visus_core::Art) -> Option<Pixmap> {
    ART_PIXMAPS.with(|cell| {
        let mut map = cell.borrow_mut();
        if !map.contains_key(&art.id) {
            let mut pm = Pixmap::new(art.size, art.size)?;
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
        map.get(&art.id).cloned()
    })
}

/// The cover as a spinning vinyl record: grooves, a rotating label, a spindle hole and a fixed sheen.
fn draw_vinyl(cx: &mut Ctx, r: Rect, m: &visus_core::MediaInfo, angle: f32, a: f32) {
    let (c, rad) = ((r.x + r.w / 2.0, r.y + r.h / 2.0), r.w.min(r.h) / 2.0);
    cx.circle(c.0, c.1 + 1.5, rad + 1.5, color([0, 0, 0], 0.4 * a));
    if let (Some(shader), Some(disc)) = (
        RadialGradient::new(
            Point::from_xy(c.0, c.1),
            Point::from_xy(c.0, c.1),
            rad,
            vec![GradientStop::new(0.0, Color::from_rgba8(44, 44, 50, (a * 255.0) as u8)), GradientStop::new(1.0, Color::from_rgba8(12, 12, 15, (a * 255.0) as u8))],
            SpreadMode::Pad,
            Transform::identity(),
        ),
        PathBuilder::from_circle(c.0, c.1, rad),
    ) {
        let mut p = Paint::default();
        p.shader = shader;
        p.anti_alias = true;
        let xf = cx.xf();
        cx.pm.fill_path(&disc, &p, FillRule::Winding, xf, None);
    }
    for i in 0..8 {
        if let Some(g) = PathBuilder::from_circle(c.0, c.1, rad * (0.76 + 0.03 * i as f32)) {
            cx.stroke(&g, color(WHITE, (0.05 + 0.04 * (i % 2) as f32) * a), 0.6);
        }
    }
    // Label: the cover, rotating with the record.
    let lr = rad * 0.72;
    if let Some(label) = PathBuilder::from_circle(c.0, c.1, lr) {
        let xf = cx.xf();
        let art_pm = m.art.as_deref().and_then(cached_art);
        let mut paint = Paint::default();
        paint.anti_alias = true;
        match &art_pm {
            Some(pm) => {
                let k = lr * 2.0 / pm.width() as f32;
                let t = Transform::from_translate(-(pm.width() as f32) / 2.0, -(pm.height() as f32) / 2.0).post_scale(k, k).post_rotate(angle).post_translate(c.0, c.1);
                paint.shader = tiny_skia::Pattern::new(pm.as_ref(), SpreadMode::Pad, tiny_skia::FilterQuality::Bilinear, a, t);
                cx.pm.fill_path(&label, &paint, FillRule::Winding, xf, None);
            }
            None => {
                let h = if m.has_session { (hash(&m.title) % 360) as f32 / 360.0 } else { 0.62 };
                let (c1, c2) = (hue_rgb(h), hue_rgb((h + 0.12) % 1.0));
                let (dx, dy) = ((angle.to_radians()).cos() * lr, (angle.to_radians()).sin() * lr);
                if let Some(shader) = LinearGradient::new(
                    Point::from_xy(c.0 - dx, c.1 - dy),
                    Point::from_xy(c.0 + dx, c.1 + dy),
                    vec![GradientStop::new(0.0, Color::from_rgba8(c1[0], c1[1], c1[2], (a * 255.0) as u8)), GradientStop::new(1.0, Color::from_rgba8(c2[0], c2[1], c2[2], (a * 255.0) as u8))],
                    SpreadMode::Pad,
                    Transform::identity(),
                ) {
                    paint.shader = shader;
                    cx.pm.fill_path(&label, &paint, FillRule::Winding, xf, None);
                }
            }
        }
        cx.stroke(&label, color(WHITE, 0.22 * a), 0.8);
    }
    cx.circle(c.0, c.1, rad * 0.075, color([6, 6, 8], a));
    // Sheen: two soft wedges that stay put while the record turns.
    for start in [-62.0f32, 118.0] {
        let mut pb = PathBuilder::new();
        pb.move_to(c.0, c.1);
        for k in 0..=6 {
            let ang = (start + 22.0 * k as f32 / 6.0).to_radians();
            pb.line_to(c.0 + ang.cos() * rad, c.1 + ang.sin() * rad);
        }
        pb.close();
        if let Some(w) = pb.finish() {
            cx.fill(&w, color(WHITE, 0.07 * a));
        }
    }
}

fn draw_art(cx: &mut Ctx, r: Rect, radius: f32, m: &visus_core::MediaInfo, angle: f32, a: f32) {
    let _ = radius;
    draw_vinyl(cx, r, m, angle, a);
    if true {
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
        let y = 62.0 + i as f32 * 42.0;
        v.push((Button::ShelfOpen(i), rect(x0 + 36.0, y, w - 94.0, 38.0)));
        v.push((Button::ShelfReveal(i), rect(x0 + 36.0 + w - 92.0, y, 44.0, 38.0)));
        v.push((Button::ShelfRemove(i), rect(x0 + 36.0 + w - 46.0, y, 46.0, 38.0)));
    }
    v
}

fn human_size(b: u64) -> String {
    let f = b as f64;
    if f >= 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} GB", f / (1024.0 * 1024.0 * 1024.0))
    } else if f >= 1024.0 * 1024.0 {
        format!("{:.1} MB", f / (1024.0 * 1024.0))
    } else {
        format!("{:.0} KB", (f / 1024.0).max(1.0))
    }
}

thread_local! {
    static FILE_SIZES: std::cell::RefCell<std::collections::HashMap<String, String>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn file_size_label(path: &str) -> String {
    FILE_SIZES.with(|c| {
        c.borrow_mut()
            .entry(path.to_string())
            .or_insert_with(|| std::fs::metadata(path).map_or("missing".to_string(), |m| if m.is_dir() { "folder".to_string() } else { human_size(m.len()) }))
            .clone()
    })
}

fn dashed_rrect(cx: &mut Ctx, rc: Rect, radius: f32, c: [u8; 4]) {
    let Some(path) = rrect(rc.x, rc.y, rc.w, rc.h, radius) else { return };
    let stroke = Stroke { width: 1.5, dash: tiny_skia::StrokeDash::new(vec![6.0, 5.0], 0.0), ..Default::default() };
    let xf = cx.xf();
    cx.pm.stroke_path(&path, &Ctx::paint(c), &stroke, xf, None);
}

fn folder_icon(cx: &mut Ctx, x: f32, y: f32, c: [u8; 4]) {
    if let Some(p) = rrect(x - 8.0, y - 4.0, 16.0, 11.0, 2.5) {
        cx.stroke(&p, c, 1.4);
    }
    cx.rrect(rect(x - 8.0, y - 7.0, 7.0, 3.5), 1.5, c);
}

fn draw_shelf(cx: &mut Ctx, notch: &Notch, a: f32) {
    let r = notch.shape().rect;
    let (x0, w) = (r.x, r.w - 72.0);
    let mid = r.x + r.w / 2.0;
    let n = notch.shelf().len();
    if n == 0 {
        let zone = rect(x0 + 36.0, 62.0, w, 150.0);
        let hot = notch.drop_hover();
        cx.rrect(zone, 22.0, color(WHITE, if hot { 0.10 } else { 0.04 } * a));
        dashed_rrect(cx, zone, 22.0, if hot { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.28 * a) });
        // Arrow dropping into a tray.
        let ay = if hot { 106.0 } else { 100.0 };
        cx.rrect(rect(mid - 1.5, ay - 18.0, 3.0, 22.0), 1.5, color(WHITE, 0.9 * a));
        cx.triangle([(mid - 8.0, ay + 1.0), (mid + 8.0, ay + 1.0), (mid, ay + 10.0)], color(WHITE, 0.9 * a));
        if let Some(p) = rrect(mid - 18.0, 118.0, 36.0, 10.0, 4.0) {
            cx.stroke(&p, color(WHITE, 0.7 * a), 1.8);
        }
        cx.t(if hot { "Release to add" } else { "Drop files here" }, mid, 158.0, 22.0, color(WHITE, a), Align::Center);
        cx.t("Keep them close until you need them", mid, 182.0, 14.0, color(GRAY, a), Align::Center);
        return;
    }
    for (i, path) in notch.shelf().iter().take(4).enumerate() {
        let y = 62.0 + i as f32 * 42.0;
        cx.rrect(rect(x0 + 36.0, y, w, 38.0), 14.0, color(WHITE, 0.06 * a));
        let name = file_name(path);
        let ext = std::path::Path::new(path).extension().map_or(String::new(), |e| e.to_string_lossy().to_uppercase());
        let tint = hue_rgb((hash(&ext) % 360) as f32 / 360.0);
        let tile = rect(x0 + 44.0, y + 4.0, 30.0, 30.0);
        cx.rrect(tile, 9.0, color(tint, a));
        cx.rrect(rect(tile.x, tile.y, 30.0, 13.0), 9.0, color(WHITE, 0.14 * a));
        cx.t(&ext.chars().take(4).collect::<String>(), tile.x + 15.0, tile.y + 20.0, 10.0, color(WHITE, a), Align::Center);
        let room = w - 200.0;
        let title = cx.fit(&name, 16.0, room);
        cx.t(&title, x0 + 86.0, y + 17.0, 16.0, color(WHITE, a), Align::Left);
        let dir = std::path::Path::new(path).parent().map_or(String::new(), |d| d.to_string_lossy().into_owned());
        let sub = cx.fit(&format!("{} \u{b7} {}", file_size_label(path), dir), 12.0, room);
        cx.t(&sub, x0 + 86.0, y + 32.0, 12.0, color(GRAY, 0.9 * a), Align::Left);
        folder_icon(cx, x0 + 36.0 + w - 70.0, y + 19.0, color(GRAY, 0.9 * a));
        x_mark(cx, x0 + 36.0 + w - 23.0, y + 19.0, a);
    }
    if n < 4 {
        let rc = rect(x0 + 36.0, 62.0 + n as f32 * 42.0, w, 38.0);
        let hot = notch.drop_hover();
        if hot {
            cx.rrect(rc, 14.0, color(cx.accent, 0.12 * a));
        }
        dashed_rrect(cx, rc, 14.0, if hot { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.2 * a) });
        cx.t(if hot { "Release to add" } else { "Drop more files here" }, rc.x + rc.w / 2.0, rc.y + 24.0, 13.0, color(GRAY, a), Align::Center);
    }
}

fn draw_calendar(cx: &mut Ctx, notch: &Notch, a: f32) {
    use chrono::{DateTime, Datelike, NaiveDate};
    let x0 = notch.shape().rect.x;
    let today = chrono::Local::now().date_naive();
    cx.t(&today.format("%A").to_string(), x0 + 130.0, 92.0, 18.0, color(GRAY, a), Align::Center);
    cx.t(&today.day().to_string(), x0 + 130.0, 160.0, 68.0, color(WHITE, a), Align::Center);
    cx.t(&today.format("%B %Y").to_string(), x0 + 130.0, 196.0, 18.0, color(GRAY, a), Align::Center);
    let due = notch.todos().iter().filter(|t| !t.done && t.due.is_some()).count();
    let next = upcoming(notch, 2);
    if !next.is_empty() {
        for (i, e) in next.iter().enumerate() {
            let when = DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.start.max(0) as u64));
            let stamp = if e.all_day { when.format("%a").to_string() } else { when.format(if notch.settings().clock_24h { "%a %H:%M" } else { "%a %-I:%M %p" }).to_string() };
            let line = cx.fit(&format!("{stamp}  {}", e.title), 13.0, 226.0);
            let tint = if e.url.is_some() { BLUE } else { WHITE };
            cx.t(&line, x0 + 36.0, 218.0 + i as f32 * 20.0, 13.0, color(tint, a), Align::Left);
        }
    } else if due > 0 {
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
            cx.circle(px, py + 6.0, 10.5, color(cx.accent, a));
        }
        let ink = if day == today.day() { color(cx.ink, a) } else { color(WHITE, a) };
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
    for i in 0..4 {
        v.push((Button::Style(i), rect(x0 + 380.0 + i as f32 * 76.0, 178.0, 70.0, 28.0)));
    }
    for (i, t) in CHIPS.iter().enumerate() {
        v.push((Button::Toggle(t.0), chip_rect(x0, i)));
    }
    v.push((Button::Location, rect(x0 + 36.0, 182.0, 300.0, 26.0)));
    v.push((Button::Update, rect(x0 + 36.0, 214.0, 130.0, 26.0)));
    v.push((Button::SettingsClose, rect(x0 + SETTINGS_W - 56.0, 40.0, 36.0, 28.0)));
    v.push((Button::EyeTime(-1), rect(x0 + 190.0, 250.0, 30.0, 26.0)));
    v.push((Button::EyeTime(1), rect(x0 + 276.0, 250.0, 30.0, 26.0)));
    for i in 0..visus_platform::list_drives().len().min(7) {
        v.push((Button::DriveToggle(i as u8), rect(x0 + 380.0 + i as f32 * 52.0, 308.0, 46.0, 26.0)));
    }
    v
}

const CHIPS: [(Toggle, &str); 7] = [
    (Toggle::Quiet, "Quiet"),
    (Toggle::Clock24, "24h clock"),
    (Toggle::Visualizer, "Visualizer"),
    (Toggle::Battery, "Battery"),
    (Toggle::Clipboard, "Clipboard"),
    (Toggle::EyeMoods, "Eye moods"),
    (Toggle::ClockStyle, "Clock"),
];

fn chip_rect(x0: f32, i: usize) -> Rect {
    rect(x0 + 380.0 + (i % 3) as f32 * 98.0, 212.0 + (i / 3) as f32 * 26.0, 92.0, 22.0)
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

/// Open Iconic "cog" (MIT), 8x8 grid, centred at (x, y) and drawn `size` logical pixels wide.
fn gear(cx: &mut Ctx, x: f32, y: f32, c: [u8; 4]) {
    let size = 20.0;
    let mut pb = PathBuilder::new();
    pb.move_to(3.5, 0.0);
    pb.line_to(3.0, 1.19);
    pb.cubic_to(2.9, 1.22, 2.81, 1.27, 2.72, 1.32);
    pb.line_to(1.53, 0.82);
    pb.line_to(0.81, 1.54);
    pb.line_to(1.31, 2.73);
    pb.cubic_to(1.26, 2.83, 1.22, 2.91, 1.18, 3.01);
    pb.line_to(-0.01, 3.51);
    pb.line_to(-0.01, 4.51);
    pb.line_to(1.18, 5.01);
    pb.cubic_to(1.22, 5.11, 1.26, 5.19, 1.31, 5.29);
    pb.line_to(0.81, 6.48);
    pb.line_to(1.53, 7.2);
    pb.line_to(2.72, 6.7);
    pb.cubic_to(2.81, 6.74, 2.9, 6.79, 3.0, 6.83);
    pb.line_to(3.5, 8.02);
    pb.line_to(4.5, 8.02);
    pb.line_to(5.0, 6.83);
    pb.cubic_to(5.09, 6.79, 5.19, 6.75, 5.28, 6.7);
    pb.line_to(6.47, 7.2);
    pb.line_to(7.19, 6.48);
    pb.line_to(6.69, 5.29);
    pb.cubic_to(6.73, 5.2, 6.78, 5.1, 6.82, 5.01);
    pb.line_to(8.01, 4.51);
    pb.line_to(8.01, 3.51);
    pb.line_to(6.82, 3.01);
    pb.cubic_to(6.79, 2.92, 6.74, 2.82, 6.69, 2.73);
    pb.line_to(7.19, 1.54);
    pb.line_to(6.47, 0.82);
    pb.line_to(5.28, 1.32);
    pb.cubic_to(5.19, 1.28, 5.09, 1.23, 5.0, 1.19);
    pb.line_to(4.5, 0.0);
    pb.close();
    pb.move_to(4.0, 2.5);
    pb.cubic_to(4.83, 2.5, 5.5, 3.17, 5.5, 4.0);
    pb.cubic_to(5.5, 4.83, 4.83, 5.5, 4.0, 5.5);
    pb.cubic_to(3.17, 5.5, 2.5, 4.83, 2.5, 4.0);
    pb.cubic_to(2.5, 3.17, 3.17, 2.5, 4.0, 2.5);
    if let Some(p) = pb.finish() {
        let k = size / 8.0;
        let t = cx.xf().pre_concat(Transform::from_row(k, 0.0, 0.0, k, x - size / 2.0, y - size / 2.0));
        cx.pm.fill_path(&p, &Ctx::paint(c), FillRule::EvenOdd, t, None);
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
        cx.rrect(sw, 12.0, if *on { color(cx.accent, a) } else { color(WHITE, 0.18 * a) });
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
    for (i, label) in ["Classic", "Sauron", "Cat", "Robot"].iter().enumerate() {
        let on = s.eye_style == [EyeStyle::Classic, EyeStyle::Sauron, EyeStyle::Cat, EyeStyle::Robot][i];
        let rc = rect(x0 + 380.0 + i as f32 * 76.0, 178.0, 70.0, 28.0);
        cx.rrect(rc, 14.0, if on { color(WHITE, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        cx.t(label, rc.x + 35.0, rc.y + 19.0, 14.0, if on { color([0, 0, 0], a) } else { color(WHITE, a) }, Align::Center);
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
    cx.rrect(rc, 13.0, color(cx.accent, 0.9 * a));
    let fitted = cx.fit(&label, 13.0, 120.0);
    cx.t(&fitted, rc.x + 65.0, rc.y + 18.0, 13.0, color(cx.ink, a), Align::Center);
    let st = cx.fit(&format!("v{} {}", env!("CARGO_PKG_VERSION"), status), 12.0, SETTINGS_W - 176.0 - 40.0);
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
    for (i, (t, label)) in CHIPS.iter().enumerate() {
        let on = match t {
            Toggle::Quiet => s.quiet,
            Toggle::Clock24 => s.clock_24h,
            Toggle::Visualizer => s.visualizer,
            Toggle::Clipboard => s.clipboard_history,
            Toggle::EyeMoods => s.eye_moods,
            Toggle::ClockStyle => false,
            _ => s.show_battery,
        };
        let rc = chip_rect(x0, i);
        cx.rrect(rc, 12.0, if on { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        let label = if *t == Toggle::ClockStyle { format!("Clock: {}", s.clock_style.name()) } else { label.to_string() };
        cx.t(&label, rc.x + rc.w / 2.0, rc.y + 16.0, 12.0, if on { color(cx.ink, a) } else { color(WHITE, a) }, Align::Center);
    }
    cx.t("Search drives for apps", x0 + 380.0, 302.0, 14.0, color(GRAY, a), Align::Left);
    for (i, d) in visus_platform::list_drives().iter().take(7).enumerate() {
        let on = s.app_folders.iter().any(|f| f == d);
        let rc = rect(x0 + 380.0 + i as f32 * 52.0, 308.0, 46.0, 26.0);
        cx.rrect(rc, 13.0, if on { color(cx.accent, 0.9 * a) } else { color(WHITE, 0.12 * a) });
        cx.t(d.trim_end_matches('\\'), rc.x + 23.0, rc.y + 18.0, 14.0, if on { color(cx.ink, a) } else { color(WHITE, a) }, Align::Center);
    }
    cx.t("Scans up to 3 folders deep", x0 + 380.0, 352.0, 12.0, color(GRAY, 0.8 * a), Align::Left);
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
