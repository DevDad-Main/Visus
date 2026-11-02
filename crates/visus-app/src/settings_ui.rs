use super::*;
use visus_core::Settings;

pub const WIDTH: f32 = 860.0;
pub const HEIGHT: f32 = 720.0;
pub const CLOSE: Rect = Rect { x: 808.0, y: 20.0, w: 36.0, h: 36.0 };
const LEFT: f32 = 224.0;
const RIGHT: f32 = 824.0;

#[derive(Clone, Copy)]
pub enum Control {
    Switch,
    Number(f64, f64, f64, &'static str),
    Choice(&'static [&'static str]),
    Color,
    Text,
    Folders,
    Action(&'static str),
}

pub struct Field {
    pub key: &'static str,
    pub label: &'static str,
    pub control: Control,
}

impl Field {
    fn hint(&self) -> Option<&'static str> {
        match self.key {
            "scale" => Some("Closed notch and its contents"),
            "pill_style" => Some("Attached, floating capsule, or soft notch"),
            "open_scale" => Some("Expanded pages and tab controls"),
            "visualizer" => Some("Only visible while media is playing"),
            "minimal" => Some("Closed notch becomes a thin line"),
            "show_battery" => Some("Requires a battery; hidden in Minimal mode"),
            "clock_size" => Some("Text size inside the closed pill"),
            "header_clock_size" => Some("Clock text in the expanded header"),
            "clock_dimmed" => Some("Softer clock in both closed and open views"),
            "battery_ring_size" => Some("Ring and icon diameter; clock size unchanged"),
            "battery_match_clock" => Some("Align ring height with the clock face"),
            "ring_clock_gap" => Some("Space between the ring and clock text"),
            "hide_in_fullscreen" => Some("Game HUD can remain visible during games"),
            "clipboard_history" => Some("Recent text stays in memory, not on disk"),
            "app_folders" => Some("Separate folder paths with semicolons"),
            "frame_time_csv" => Some("Windows: a live PresentMon capture CSV"),
            "outer_padding" => Some("Space outside the closed ring and clock"),
            "accent_mode" => Some("Media artwork, mascot iris, or fixed color"),
            "accent_color" => Some("Used when Accent mode is Fixed"),
            "animation_intensity" => Some("Lower is gentler; higher is more expressive"),
            "reduced_motion" => Some("Static mascots and immediate size transitions"),
            "hover_open_ms" => Some("Wait before opening when the pointer enters"),
            "hover_close_ms" => Some("Wait before closing when the pointer leaves"),
            "quiet" => Some("Suppresses most alerts; timer/tasks still follow toggles"),
            _ => None,
        }
    }
}

pub struct Section {
    pub title: &'static str,
    pub fields: &'static [Field],
}

pub static SECTIONS: &[Section] = &[
    Section { title: "Appearance", fields: &[
        Field { key: "scale", label: "Closed pill size", control: Control::Number(0.6, 1.25, 0.05, "x") },
        Field { key: "open_scale", label: "Open UI size", control: Control::Number(0.6, 1.25, 0.05, "x") },
        Field { key: "show_battery", label: "Battery ring", control: Control::Switch },
        Field { key: "visualizer", label: "Music visualizer", control: Control::Switch },
        Field { key: "minimal", label: "Minimal pill", control: Control::Switch },
        Field { key: "net_bits", label: "Network speed in bits", control: Control::Switch },
        Field { key: "outer_padding", label: "Outer pill padding", control: Control::Number(8.0, 32.0, 1.0, "px") },
        Field { key: "accent_mode", label: "Accent mode", control: Control::Choice(&["Media", "Mascot", "Fixed"]) },
        Field { key: "accent_color", label: "Fixed accent color", control: Control::Color },
        Field { key: "pill_style", label: "Pill style", control: Control::Choice(&["Attached", "Floating", "Soft"]) },
    ] },
    Section { title: "Clock", fields: &[
        Field { key: "clock_style", label: "Style", control: Control::Choice(&["Classic", "Flip", "Led"]) },
        Field { key: "clock_size", label: "Closed clock size", control: Control::Number(12.0, 32.0, 1.0, "px") },
        Field { key: "header_clock_size", label: "Header clock size", control: Control::Number(16.0, 30.0, 1.0, "px") },
        Field { key: "clock_24h", label: "24-hour time", control: Control::Switch },
        Field { key: "clock_dimmed", label: "Dim clock", control: Control::Switch },
        Field { key: "battery_ring_size", label: "Battery ring size", control: Control::Number(12.0, 32.0, 1.0, "px") },
        Field { key: "ring_clock_gap", label: "Ring-clock gap", control: Control::Number(4.0, 32.0, 1.0, "px") },
        Field { key: "battery_match_clock", label: "Match ring to clock", control: Control::Switch },
    ] },
    Section { title: "Mascot", fields: &[
        Field { key: "mascot", label: "Idle mascot", control: Control::Switch },
        Field { key: "eye_style", label: "Style", control: Control::Choice(&["Classic", "Sauron", "Cat", "Robot"]) },
        Field { key: "idle_secs", label: "Idle delay", control: Control::Number(5.0, 300.0, 5.0, "s") },
        Field { key: "eye_moods", label: "Expressions", control: Control::Switch },
        Field { key: "eye_iris", label: "Iris color", control: Control::Color },
        Field { key: "eye_sclera", label: "Eye color", control: Control::Color },
        Field { key: "animation_intensity", label: "Animation intensity", control: Control::Number(0.3, 1.5, 0.1, "x") },
        Field { key: "reduced_motion", label: "Reduced motion", control: Control::Switch },
    ] },
    Section { title: "Behavior", fields: &[
        Field { key: "hide_in_fullscreen", label: "Hide in fullscreen", control: Control::Switch },
        Field { key: "quiet", label: "Quiet alerts", control: Control::Switch },
        Field { key: "clipboard_history", label: "Clipboard history", control: Control::Switch },
        Field { key: "timer_minutes", label: "Focus timer", control: Control::Number(1.0, 180.0, 1.0, "min") },
        Field { key: "autostart", label: "Start at login", control: Control::Action("autostart") },
        Field { key: "hover_open_ms", label: "Hover open delay", control: Control::Number(0.0, 1000.0, 50.0, "ms") },
        Field { key: "hover_close_ms", label: "Hover close delay", control: Control::Number(50.0, 2000.0, 50.0, "ms") },
    ] },
    Section { title: "Gaming", fields: &[
        Field { key: "game_hud", label: "Game HUD", control: Control::Switch },
        Field { key: "game_style", label: "Layout", control: Control::Choice(&["Pills", "Bars", "Card", "Compact"]) },
        Field { key: "frame_time_csv", label: "Frame-time CSV", control: Control::Text },
    ] },
    Section { title: "Integrations", fields: &[
        Field { key: "city", label: "Weather city", control: Control::Text },
        Field { key: "calendar_url", label: "Calendar URL or file", control: Control::Text },
        Field { key: "file_manager", label: "File manager command", control: Control::Text },
        Field { key: "app_folders", label: "Launcher folders", control: Control::Folders },
    ] },
    Section { title: "Updates", fields: &[
        Field { key: "auto_update", label: "Check automatically", control: Control::Switch },
        Field { key: "check", label: "Available updates", control: Control::Action("updates") },
    ] },
    Section { title: "Alerts", fields: &[
        Field { key: "alert_power", label: "Charger connected / removed", control: Control::Switch },
        Field { key: "alert_network", label: "Connection lost / restored", control: Control::Switch },
        Field { key: "alert_headphones", label: "Headphones", control: Control::Switch },
        Field { key: "alert_controller", label: "Controllers", control: Control::Switch },
        Field { key: "alert_downloads", label: "Download completed", control: Control::Switch },
        Field { key: "alert_timer", label: "Focus timer finished", control: Control::Switch },
        Field { key: "alert_tasks", label: "Task reminders", control: Control::Switch },
        Field { key: "alert_meetings", label: "Meeting reminders", control: Control::Switch },
        Field { key: "alert_updates", label: "Update available", control: Control::Switch },
        Field { key: "alert_controls", label: "Volume / Caps Lock / language", control: Control::Switch },
    ] },
];

#[derive(Clone, Default, PartialEq)]
pub struct Panel {
    pub usage: crate::app_usage::Usage,
    pub frame_source: String,
    pub section: usize,
    pub scroll: f32,
    pub sidebar_scroll: f32,
    pub pointer: Option<(f32, f32)>,
    pub editing: Option<(usize, String)>,
    pub dragging: Option<usize>,
    pub status: String,
    pub preview_header: bool,
    pub update_available: bool,
    pub update_status: UpdateStatus,
    pub battery: Option<(bool, u8)>,
    pub reaction_style: EyeStyle,
    pub media_accent: Option<[u8; 3]>,
    pub close_hovered: bool,
}

pub enum Command {
    Changed(Settings),
    Action(&'static str),
    City(String),
}

struct Reaction {
    title: &'static str,
    meaning: &'static str,
    mood: EyeMood,
    asleep: bool,
    blink: bool,
}

fn idle_reactions(style: EyeStyle) -> Vec<Reaction> {
    let mut reactions = vec![
        Reaction { title: "Awake", meaning: "Idle companion; follows the pointer where supported", mood: EyeMood::None, asleep: false, blink: false },
        Reaction { title: "Blink", meaning: "A normal blink, not a notification", mood: EyeMood::None, asleep: false, blink: true },
        Reaction { title: "Scan", meaning: "A playful glance around; no system status attached", mood: EyeMood::Scan, asleep: false, blink: false },
        Reaction { title: "Surprised", meaning: "A brief wide-eyed expression during idle time", mood: EyeMood::Surprised, asleep: false, blink: false },
        Reaction { title: "Squint", meaning: "A relaxed idle expression, not a warning", mood: EyeMood::Squint, asleep: false, blink: false },
    ];
    if style == EyeStyle::Cat {
        reactions.push(Reaction { title: "Purr / slow blink", meaning: "Gentle breathing, soft blinks and whisker trembles; silent", mood: EyeMood::Squint, asleep: false, blink: false });
    }
    if style == EyeStyle::Robot {
        reactions.push(Reaction { title: "Reply lights", meaning: "Alternating eyes and tiny signal bars; a silent beep-boop", mood: EyeMood::None, asleep: false, blink: false });
    }
    if style == EyeStyle::Sauron {
        reactions.push(Reaction { title: "Molten embers", meaning: "Lid flames, falling molten trails and a glowing lower edge", mood: EyeMood::None, asleep: false, blink: false });
    }
    if style != EyeStyle::Sauron {
        reactions.push(Reaction { title: if style == EyeStyle::Robot { "Pink pulse" } else { "Heart" }, meaning: "A decorative affection mood; not heat or battery status", mood: EyeMood::Heart, asleep: false, blink: false });
    }
    if style == EyeStyle::Robot {
        reactions.push(Reaction { title: "Glitch", meaning: "Intentional visor flicker and pink jitter; not an app error", mood: EyeMood::Glitch, asleep: false, blink: false });
    }
    reactions.push(Reaction { title: "Asleep", meaning: "Appears after a longer period without activity", mood: EyeMood::None, asleep: true, blink: false });
    reactions
}

const EVENT_REACTIONS: &[(Expr, &str, &str)] = &[
    (Expr::Happy, "Happy", "Charger connected, back online, or download completed"),
    (Expr::Sad, "Sad", "Charger removed or controller disconnected"),
    (Expr::Music, "Music", "Headphones connected"),
    (Expr::Dizzy, "Dizzy", "Internet connection lost"),
    (Expr::Game, "Game", "Controller connected"),
    (Expr::Party, "Celebrate", "Focus timer finished or an update is available"),
    (Expr::Alert, "Reminder", "A task is due or a meeting is about to start"),
];

fn reaction_guide_height(style: EyeStyle) -> f32 {
    96.0 + (idle_reactions(style).len() + EVENT_REACTIONS.len()) as f32 * 60.0
}

impl Panel {
    fn content_bottom(&self) -> f32 {
        let fields = 274.0 + SECTIONS[self.section].fields.len() as f32 * 60.0;
        if SECTIONS[self.section].title == "Mascot" { fields + reaction_guide_height(self.reaction_style) } else { fields }
    }

    fn preview_for(&mut self, key: &str) {
        match key {
            "open_scale" | "header_clock_size" => self.preview_header = true,
            "scale" | "pill_style" | "clock_size" | "show_battery" | "battery_ring_size" | "battery_match_clock" | "ring_clock_gap" | "outer_padding" | "visualizer" | "minimal"
                | "mascot" | "eye_style" | "idle_secs" | "eye_moods" | "eye_iris"
                | "eye_sclera" | "game_hud" | "game_style" => self.preview_header = false,
            _ => {}
        }
    }

    fn row(&self, index: usize) -> Rect {
        rect(LEFT, 274.0 + index as f32 * 60.0 - self.scroll, RIGHT - LEFT, 60.0)
    }

    pub fn scroll(&mut self, amount: f32, height: f32) {
        if self.pointer.is_some_and(|(x, _)| x < 188.0) {
            let content = 96.0 + SECTIONS.len() as f32 * 44.0;
            self.sidebar_scroll = (self.sidebar_scroll + amount).clamp(0.0, (content - height + 98.0).max(0.0));
            return;
        }
        let content = self.content_bottom();
        self.scroll = (self.scroll + amount).clamp(0.0, (content - (height - 64.0)).max(0.0));
    }

    fn assign(settings: &Settings, field: &Field, value: serde_json::Value) -> Option<Command> {
        let mut values = serde_json::to_value(settings).ok()?;
        values[field.key] = value;
        Some(Command::Changed(serde_json::from_value::<Settings>(values).ok()?.clamped()))
    }

    pub fn drag(&self, settings: &Settings, x: f32) -> Option<Command> {
        let field = SECTIONS[self.section].fields.get(self.dragging?)?;
        let Control::Number(min, max, step, _) = field.control else { return None };
        let fraction = ((x - 506.0) / 216.0).clamp(0.0, 1.0) as f64;
        let number = ((min + fraction * (max - min)) / step).round() * step;
        let value = if matches!(field.key, "idle_secs" | "timer_minutes" | "hover_open_ms" | "hover_close_ms") {
            serde_json::json!(number.round() as u32)
        } else { serde_json::json!(number) };
        Self::assign(settings, field, value)
    }

    pub fn click(&mut self, settings: &Settings, x: f32, y: f32, height: f32) -> Option<Command> {
        self.reaction_style = settings.eye_style;
        self.editing = None;
        if rect(LEFT, height - 58.0, 140.0, 38.0).contains(x, y) && self.content_bottom() - self.scroll > height - 64.0 {
            self.pointer = Some((LEFT, y));
            self.scroll(180.0, height);
            return None;
        }
        if rect(638.0, 22.0, 150.0, 32.0).contains(x, y) {
            self.preview_header = x >= 713.0;
            return None;
        }
        if x < 188.0 {
            for index in 0..SECTIONS.len() {
                let area = rect(16.0, 96.0 + index as f32 * 44.0 - self.sidebar_scroll, 156.0, 38.0);
                if area.y >= 88.0 && area.y + area.h <= height - 98.0 && area.contains(x, y) {
                    self.section = index;
                    self.scroll = 0.0;
                }
            }
            return None;
        }
        if rect(RIGHT - 120.0, height - 46.0, 120.0, 28.0).contains(x, y) {
            let mut values = serde_json::to_value(settings).ok()?;
            let defaults = serde_json::to_value(Settings::default()).ok()?;
            for field in SECTIONS[self.section].fields {
                if defaults.get(field.key).is_some() { values[field.key] = defaults[field.key].clone(); }
            }
            if SECTIONS[self.section].title == "Integrations" {
                for key in ["weather_city", "weather_lat", "weather_lon"] { values[key] = serde_json::Value::Null; }
            }
            return Some(Command::Changed(serde_json::from_value(values).ok()?));
        }
        if y < 258.0 || y > height - 64.0 { return None; }
        let values = serde_json::to_value(settings).ok()?;
        for (index, field) in SECTIONS[self.section].fields.iter().enumerate() {
            let row = self.row(index);
            if row.y < 258.0 || row.y + row.h > height - 64.0 { continue; }
            if !row.contains(x, y) || x < 500.0 { continue; }
            self.preview_for(field.key);
            match field.control {
                Control::Switch => return Self::assign(settings, field, serde_json::json!(!values[field.key].as_bool()?)),
                Control::Number(..) => {
                    if field.key == "battery_ring_size" && settings.battery_match_clock {
                        self.status = "Turn off Match ring to clock to adjust the ring manually".into();
                        return None;
                    }
                    if x >= 744.0 {
                        self.editing = Some((index, values[field.key].to_string()));
                        return None;
                    }
                    self.dragging = Some(index);
                    return self.drag(settings, x);
                }
                Control::Choice(options) => {
                    let width = (RIGHT - 500.0) / options.len() as f32;
                    let selected = ((x - 500.0) / width) as usize;
                    return Self::assign(settings, field, serde_json::json!(options.get(selected)?.to_lowercase()));
                }
                Control::Color => {
                    let colors = [[40, 110, 230], [48, 209, 88], [255, 159, 10], [255, 69, 58], [255, 255, 255], [160, 160, 168]];
                    let selected = ((x - 500.0) / 48.0) as usize;
                    return Self::assign(settings, field, serde_json::json!(Rgb(*colors.get(selected)?).hex()));
                }
                Control::Text | Control::Folders => {
                    let value = if field.key == "city" { settings.weather_city.clone().unwrap_or_default() }
                        else if matches!(field.control, Control::Folders) { settings.app_folders.join("; ") }
                        else { values[field.key].as_str().unwrap_or("").to_owned() };
                    self.editing = Some((index, value));
                }
                Control::Action(action) => return Some(Command::Action(action)),
            }
        }
        None
    }

    pub fn commit(&mut self, settings: &Settings) -> Option<Command> {
        let (index, input) = self.editing.take()?;
        let field = &SECTIONS[self.section].fields[index];
        self.preview_for(field.key);
        let input = input.trim();
        if field.key == "city" {
            if input.is_empty() {
                let mut settings = settings.clone();
                settings.weather_city = None;
                settings.weather_lat = None;
                settings.weather_lon = None;
                return Some(Command::Changed(settings));
            }
            return Some(Command::City(input.to_owned()));
        }
        if let Control::Number(min, max, step, _) = field.control {
            let Ok(value) = input.parse::<f64>() else { self.status = "Enter a valid number".into(); return None; };
            if !value.is_finite() { self.status = "Enter a finite number".into(); return None; }
            let value = ((value.clamp(min, max) / step).round() * step).clamp(min, max);
            return Self::assign(settings, field, if matches!(field.key, "idle_secs" | "timer_minutes" | "hover_open_ms" | "hover_close_ms") {
                serde_json::json!(value as u32)
            } else { serde_json::json!(value) });
        }
        let value = if matches!(field.control, Control::Folders) {
            serde_json::json!(input.split(';').map(str::trim).filter(|path| !path.is_empty()).collect::<Vec<_>>())
        } else if input.is_empty() { serde_json::Value::Null } else { serde_json::json!(input) };
        Self::assign(settings, field, value)
    }
}

fn draw_reaction_guide(cx: &mut Ctx, panel: &Panel, settings: &Settings, height: f32) {
    let mut y = 274.0 + SECTIONS[panel.section].fields.len() as f32 * 60.0 - panel.scroll;
    let mut preview = Notch::new(120.0);
    preview.set_settings(Settings { mascot: true, minimal: false, ..settings.clone() });
    let idle = idle_reactions(settings.eye_style);
    for (heading, count, event) in [("Idle expressions", idle.len(), false), ("Event reactions", EVENT_REACTIONS.len(), true)] {
        if y >= 258.0 && y + 48.0 <= height - 64.0 {
            cx.t(heading, LEFT, y + 31.0, 17.0, color(WHITE, 1.0), Align::Left);
        }
        y += 48.0;
        for index in 0..count {
            if y >= 258.0 && y + 60.0 <= height - 64.0 {
                let (title, meaning) = if event {
                    let (expression, title, meaning) = EVENT_REACTIONS[index];
                    draw_face(cx, LEFT + 42.0, y + 30.0, settings.eye_style, expression, 1.0, 1.0);
                    (title, meaning)
                } else {
                    let reaction = &idle[index];
                    let previous = (cx.ox, cx.oy, cx.scale);
                    let bounds = preview.shape().rect;
                    let scale = (28.0 / bounds.h).min(0.8);
                    cx.scale *= scale;
                    cx.ox += (LEFT + 42.0 - (bounds.x + bounds.w / 2.0) * scale) * previous.2;
                    cx.oy += (y + 30.0 - bounds.h * scale / 2.0) * previous.2;
                    draw_eye_sample(cx, &preview, 1.0, Some((reaction.mood, reaction.asleep, reaction.blink)));
                    (cx.ox, cx.oy, cx.scale) = previous;
                    (reaction.title, reaction.meaning)
                };
                cx.t(title, LEFT + 96.0, y + 24.0, 14.0, color(WHITE, 0.95), Align::Left);
                let meaning = cx.fit(meaning, 11.0, RIGHT - LEFT - 100.0);
                cx.t(&meaning, LEFT + 96.0, y + 43.0, 11.0, color(GRAY, 1.0), Align::Left);
                cx.rrect(rect(LEFT, y + 59.0, RIGHT - LEFT, 1.0), 0.0, color(WHITE, 0.07));
            }
            y += 60.0;
        }
    }
}

pub fn draw(pm: &mut Pixmap, text: &Text, panel: &Panel, settings: &Settings, scale: f32) {
    let height = pm.height() as f32 / scale;
    pm.fill(Color::from_rgba8(0, 0, 0, 255));
    let mut cx = Ctx { accent: panel.media_accent.unwrap_or(settings.eye_iris.0), ink: WHITE, pm, text, scale, ox: 0.0, oy: 0.0 };
    cx.rrect(rect(187.0, 76.0, 1.0, height - 108.0), 0.0, color(WHITE, 0.08));
    cx.t("VISUS", 24.0, 40.0, 20.0, color(WHITE, 1.0), Align::Left);
    cx.t("Settings", 24.0, 64.0, 13.0, color(GRAY, 1.0), Align::Left);
    if panel.close_hovered {
        cx.rrect(CLOSE, 6.0, color(WHITE, 0.13));
    }
    let mut close = PathBuilder::new();
    let (close_x, close_y) = (CLOSE.x + CLOSE.w / 2.0, CLOSE.y + CLOSE.h / 2.0);
    close.move_to(close_x - 6.0, close_y - 6.0);
    close.line_to(close_x + 6.0, close_y + 6.0);
    close.move_to(close_x + 6.0, close_y - 6.0);
    close.line_to(close_x - 6.0, close_y + 6.0);
    if let Some(path) = close.finish() { cx.stroke(&path, color(if panel.close_hovered { WHITE } else { GRAY }, 1.0), 1.6); }
    for (index, section) in SECTIONS.iter().enumerate() {
        let row = rect(16.0, 96.0 + index as f32 * 44.0 - panel.sidebar_scroll, 156.0, 38.0);
        if row.y < 88.0 || row.y + row.h > height - 98.0 { continue; }
        if index == panel.section {
            cx.rrect(row, 6.0, color(WHITE, 0.08));
            cx.rrect(rect(16.0, row.y + 10.0, 3.0, 18.0), 1.5, color(cx.accent, 1.0));
        }
        let title = cx.fit(section.title, 15.0, 132.0);
        cx.t(&title, 32.0, row.y + 25.0, 15.0, color(if index == panel.section { WHITE } else { GRAY }, 1.0), Align::Left);
    }
    cx.t(SECTIONS[panel.section].title, LEFT, 48.0, 25.0, color(WHITE, 1.0), Align::Left);
    for (index, label) in ["Closed", "Header"].iter().enumerate() {
        let x = 638.0 + index as f32 * 75.0;
        let selected = panel.preview_header == (index == 1);
        cx.rrect(rect(x, 22.0, 72.0, 32.0), 5.0, color(WHITE, if selected { 0.12 } else { 0.04 }));
        cx.t(label, x + 36.0, 43.0, 12.0, color(if selected { WHITE } else { GRAY }, 1.0), Align::Center);
    }
    cx.rrect(rect(LEFT, 76.0, RIGHT - LEFT, 168.0), 0.0, color([12, 12, 12], 1.0));
    let mut preview = Notch::new(RIGHT - LEFT);
    let mascot_preview = SECTIONS[panel.section].title == "Mascot" && settings.mascot && !panel.preview_header;
    preview.set_settings(Settings { mascot: mascot_preview, scale: 1.0, ..settings.clone() });
    if let Some((on_ac, percent)) = panel.battery {
        preview.handle(visus_core::Event::Power { on_ac, percent });
    }
    if SECTIONS[panel.section].title == "Appearance" && settings.visualizer && !settings.minimal && !panel.preview_header {
        preview.handle(visus_core::Event::Media(visus_core::MediaInfo { has_session: true, playing: true, ..Default::default() }));
        preview.set_audio_level(Some(0.55));
    }
    if mascot_preview { preview.rewind_activity(std::time::Duration::from_secs(settings.idle_secs as u64 + 2)); }
    let gaming_preview = SECTIONS[panel.section].title == "Gaming" && settings.game_hud && !panel.preview_header;
    if gaming_preview {
        preview.handle(visus_core::Event::Game(true));
        for index in 0..GAME_HIST {
            let fps = 144.0 + (index as f32 * 0.8).sin() * 8.0;
            preview.handle(visus_core::Event::GameStats(GameStats {
                fps: Some(fps), frame_ms: Some(1000.0 / fps), cpu: 0.42, gpu: Some(0.76),
                cpu_temp: Some(62.0), gpu_temp: Some(68.0), ram: 0.38, ram_used_gb: 12.1,
                ..Default::default()
            }));
        }
    }
    if panel.preview_header { preview.hover(true); }
    sync_closed_layout(&mut preview, text);
    for _ in 0..300 { preview.tick(std::time::Duration::from_millis(16)); }
    let shape = preview.shape().rect;
    let old_x = cx.ox;
    let old_y = cx.oy;
    let desired_scale = if panel.preview_header { settings.open_scale } else { settings.scale };
    let preview_scale = ((RIGHT - LEFT - 32.0) / shape.w).min(140.0 / shape.h).min(desired_scale);
    cx.scale = scale * preview_scale;
    cx.ox = (LEFT + (RIGHT - LEFT) / 2.0 - (shape.x + shape.w / 2.0) * preview_scale) * scale;
    cx.oy = (156.0 - shape.h * preview_scale / 2.0) * scale;
    if let Some(path) = pill_path(&preview.shape()) {
        cx.fill(&path, color([0, 0, 0], 1.0));
    }
    if panel.preview_header { draw_open(&mut cx, &preview, 1.0); }
    else if gaming_preview { draw_game(&mut cx, &preview, 1.0); }
    else if mascot_preview { draw_eye(&mut cx, &preview, 1.0); }
    else if !settings.minimal { draw_closed(&mut cx, &preview, 1.0); }
    cx.scale = scale;
    cx.ox = old_x;
    cx.oy = old_y;
    let values = serde_json::to_value(settings).unwrap_or_default();
    for (index, field) in SECTIONS[panel.section].fields.iter().enumerate() {
        let row = panel.row(index);
        if row.y < 258.0 || row.y + row.h > height - 64.0 { continue; }
        cx.rrect(rect(LEFT, row.y + row.h - 1.0, RIGHT - LEFT, 1.0), 0.0, color(WHITE, 0.07));
        let label = cx.fit(field.label, 14.0, 254.0);
        let label_y = if field.hint().is_some() { row.y + 25.0 } else { row.y + 36.0 };
        cx.t(&label, LEFT, label_y, 14.0, color(WHITE, 0.92), Align::Left);
        if let Some(hint) = field.hint() {
            let hint = cx.fit(hint, 11.0, 254.0);
            cx.t(&hint, LEFT, row.y + 43.0, 11.0, color(GRAY, 0.9), Align::Left);
        }
        let mid = row.y + 30.0;
        match field.control {
            Control::Switch => {
                let on = values[field.key].as_bool().unwrap_or(false);
                cx.rrect(rect(RIGHT - 42.0, mid - 12.0, 42.0, 24.0), 12.0, color(if on { cx.accent } else { [75, 76, 83] }, 1.0));
                cx.circle(RIGHT - if on { 12.0 } else { 30.0 }, mid, 8.5, color(WHITE, 1.0));
            }
            Control::Number(min, max, _, unit) => {
                let value = values[field.key].as_f64().unwrap_or(min);
                let fraction = ((value - min) / (max - min)).clamp(0.0, 1.0) as f32;
                cx.rrect(rect(506.0, mid - 2.0, 216.0, 4.0), 2.0, color(WHITE, 0.15));
                if fraction > 0.0 { cx.rrect(rect(506.0, mid - 2.0, 216.0 * fraction, 4.0), 2.0, color(cx.accent, 1.0)); }
                cx.circle(506.0 + 216.0 * fraction, mid, 6.0, color(WHITE, 1.0));
                let value = if let Some((_, input)) = panel.editing.as_ref().filter(|(selected, _)| *selected == index) {
                    format!("{input}|")
                } else if unit == "x" { format!("{value:.2} {unit}") } else { format!("{value:.0} {unit}") };
                let value = cx.fit(&value, 14.0, 80.0);
                cx.t(&value, RIGHT, mid + 5.0, 14.0, color(GRAY, 1.0), Align::Right);
            }
            Control::Choice(options) => {
                let width = (RIGHT - 500.0) / options.len() as f32;
                for (choice, label) in options.iter().enumerate() {
                    let x = 500.0 + choice as f32 * width;
                    let active = values[field.key].as_str().is_some_and(|value| value.eq_ignore_ascii_case(label));
                    cx.rrect(rect(x + 2.0, mid - 16.0, width - 4.0, 32.0), 5.0, color(WHITE, if active { 0.16 } else { 0.04 }));
                    cx.t(label, x + width / 2.0, mid + 4.5, 12.0, color(if active { WHITE } else { GRAY }, 1.0), Align::Center);
                }
            }
            Control::Color => {
                for (choice, rgb) in [[40, 110, 230], [48, 209, 88], [255, 159, 10], [255, 69, 58], WHITE, GRAY].iter().enumerate() {
                    let x = 514.0 + choice as f32 * 48.0;
                    if values[field.key].as_str() == Some(&Rgb(*rgb).hex()) { cx.circle(x, mid, 14.0, color(WHITE, 0.8)); }
                    cx.circle(x, mid, 10.0, color(*rgb, 1.0));
                }
            }
            Control::Text | Control::Folders => {
                let editing = panel.editing.as_ref().filter(|(selected, _)| *selected == index);
                let value = editing.map(|(_, input)| input.clone()).unwrap_or_else(|| {
                    if field.key == "city" { settings.weather_city.clone().unwrap_or_default() }
                    else if matches!(field.control, Control::Folders) { settings.app_folders.join("; ") }
                    else { values[field.key].as_str().unwrap_or("").to_owned() }
                });
                cx.rrect(rect(500.0, mid - 17.0, RIGHT - 500.0, 34.0), 5.0, color(WHITE, if editing.is_some() { 0.12 } else { 0.05 }));
                let shown = cx.fit(&format!("{value}{}", if editing.is_some() { "|" } else { "" }), 13.0, RIGHT - 522.0);
                cx.t(&shown, 510.0, mid + 4.5, 13.0, color(WHITE, 0.9), Align::Left);
            }
            Control::Action(action) => {
                let label = if action == "autostart" {
                    if visus_platform::autostart_enabled() { "Disable" } else { "Enable" }
                } else { match panel.update_status {
                    UpdateStatus::Checking => "Checking...",
                    UpdateStatus::UpToDate => "Check again",
                    UpdateStatus::Available { .. } => "Install update",
                    UpdateStatus::Installing => "Installing...",
                    UpdateStatus::Failed(_) => "Try again",
                    UpdateStatus::Idle => "Check now",
                } };
                cx.rrect(rect(RIGHT - 110.0, mid - 16.0, 110.0, 32.0), 5.0, color(WHITE, 0.08));
                cx.t(label, RIGHT - 55.0, mid + 4.5, 13.0, color(WHITE, 1.0), Align::Center);
            }
        }
    }
    if SECTIONS[panel.section].title == "Mascot" { draw_reaction_guide(&mut cx, panel, settings, height); }
    if SECTIONS[panel.section].title == "Gaming" {
        let y = 274.0 + SECTIONS[panel.section].fields.len() as f32 * 60.0 - panel.scroll + 24.0;
        if y >= 258.0 && y <= height - 76.0 {
            let note = cx.fit(&panel.frame_source, 12.0, RIGHT - LEFT);
            cx.t(&note, LEFT, y, 12.0, color(GRAY, 1.0), Align::Left);
        }
    }
    cx.rrect(rect(16.0, height - 86.0, 156.0, 86.0), 0.0, color([0, 0, 0], 1.0));
    cx.t(&format!("Visus v{}", env!("CARGO_PKG_VERSION")), 24.0, height - 62.0, 12.0, color(WHITE, 0.9), Align::Left);
    let cpu = panel.usage.cpu.map_or_else(|| "CPU --".into(), |value| format!("CPU {value:.1}%"));
    let memory = panel.usage.memory_mb.map_or_else(|| "RAM -- MB".into(), |value| format!("RAM {value:.1} MB"));
    cx.t(&cpu, 24.0, height - 42.0, 11.0, color(GRAY, 1.0), Align::Left);
    cx.t(&memory, 24.0, height - 24.0, 11.0, color(GRAY, 1.0), Align::Left);
    cx.rrect(rect(LEFT, height - 64.0, RIGHT - LEFT, 64.0), 0.0, color([0, 0, 0], 1.0));
    let remaining = panel.content_bottom() - panel.scroll - (height - 64.0);
    if remaining > 1.0 {
        cx.t("More below", LEFT + 18.0, height - 38.0, 12.0, color(WHITE, 0.9), Align::Left);
        let mut arrow = PathBuilder::new();
        arrow.move_to(LEFT, height - 44.0);
        arrow.line_to(LEFT + 5.0, height - 39.0);
        arrow.line_to(LEFT + 10.0, height - 44.0);
        if let Some(path) = arrow.finish() { cx.stroke(&path, color(WHITE, 0.9), 1.5); }
    }
    let view_height = height - 64.0 - 258.0;
    let content_height = panel.content_bottom() - 258.0;
    if content_height > view_height {
        let thumb_height = (view_height * view_height / content_height).max(24.0);
        let offset = panel.scroll / (content_height - view_height) * (view_height - thumb_height);
        cx.rrect(rect(RIGHT + 12.0, 258.0, 3.0, view_height), 1.5, color(WHITE, 0.08));
        cx.rrect(rect(RIGHT + 12.0, 258.0 + offset, 3.0, thumb_height), 1.5, color(WHITE, 0.45));
    }
    let status = cx.fit(&panel.status, 12.0, if remaining > 1.0 { 315.0 } else { 440.0 });
    cx.t(&status, LEFT + if remaining > 1.0 { 125.0 } else { 0.0 }, height - 27.0, 12.0, color(GRAY, 1.0), Align::Left);
    cx.t("Reset section", RIGHT, height - 27.0, 13.0, color(GRAY, 1.0), Align::Right);
    if panel.close_hovered {
        cx.rrect(rect(RIGHT - 112.0, 62.0, 112.0, 28.0), 5.0, color([38, 38, 38], 1.0));
        cx.t("Close settings", RIGHT - 56.0, 81.0, 12.0, color(WHITE, 1.0), Align::Center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn more_below_scrolls_and_disappears_at_the_end() {
        let settings = Settings::default();
        let mut panel = Panel { section: 1, ..Default::default() };
        assert!(panel.content_bottom() > HEIGHT - 64.0);
        panel.click(&settings, LEFT + 40.0, HEIGHT - 40.0, HEIGHT);
        assert!(panel.scroll > 0.0);
        panel.scroll(10000.0, HEIGHT);
        assert!((panel.content_bottom() - panel.scroll - (HEIGHT - 64.0)).abs() < 0.001);
        let previous = panel.scroll;
        panel.click(&settings, LEFT + 40.0, HEIGHT - 40.0, HEIGHT);
        assert_eq!(panel.scroll, previous);
    }

    #[test]
    fn reaction_guide_matches_supported_moods_and_events() {
        let text = Text::new();
        for style in [EyeStyle::Classic, EyeStyle::Sauron, EyeStyle::Cat, EyeStyle::Robot] {
            let reactions = idle_reactions(style);
            assert_eq!(reactions.iter().any(|reaction| reaction.mood == EyeMood::Glitch), style == EyeStyle::Robot);
            assert_eq!(reactions.iter().any(|reaction| reaction.mood == EyeMood::Heart), style != EyeStyle::Sauron);
            for reaction in reactions {
                assert!(text.width(reaction.meaning, 11.0) <= RIGHT - LEFT - 100.0);
            }
            let mut panel = Panel { section: 2, reaction_style: style, ..Default::default() };
            panel.scroll(10000.0, HEIGHT);
            assert!((panel.content_bottom() - panel.scroll - (HEIGHT - 64.0)).abs() < 0.001);
            let mut pixels = Pixmap::new(WIDTH as u32, HEIGHT as u32).unwrap();
            draw(&mut pixels, &text, &panel, &Settings { eye_style: style, ..Default::default() }, 1.0);
            assert!(pixels.pixels().iter().any(|pixel| pixel.red() > 200));
        }
        for (alert, expected) in [(Alert::Charging { percent: 80 }, Expr::Happy), (Alert::Unplugged { percent: 40 }, Expr::Sad),
            (Alert::Headphones, Expr::Music), (Alert::Offline, Expr::Dizzy), (Alert::Controller(true), Expr::Game),
            (Alert::TimerDone, Expr::Party), (Alert::TodoDue, Expr::Alert)] {
            assert!(expr_for(alert) == Some(expected));
        }
    }

    #[test]
    fn gap_control_selects_closed_preview_and_persists() {
        let settings = Settings::default();
        let mut panel = Panel { section: 1, preview_header: true, ..Default::default() };
        panel.scroll(1000.0, HEIGHT);
        let index = SECTIONS[1].fields.iter().position(|field| field.key == "ring_clock_gap").unwrap();
        let row = panel.row(index);
        let Some(Command::Changed(changed)) = panel.click(&settings, 722.0, row.y + 30.0, HEIGHT) else { panic!("gap slider"); };
        assert!(!panel.preview_header);
        assert_eq!(changed.ring_clock_gap, 32.0);
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.ring_clock_gap, 32.0);
    }

    #[test]
    fn edits_select_the_matching_preview_view() {
        let settings = Settings::default();
        let mut panel = Panel { section: 1, ..Default::default() };
        panel.click(&settings, 800.0, 424.0, HEIGHT);
        assert!(panel.preview_header);
        panel.editing = Some((2, "26".into()));
        assert!(matches!(panel.commit(&settings), Some(Command::Changed(changed)) if changed.header_clock_size == 26.0));
        assert!(panel.preview_header);
        panel.click(&settings, 800.0, 364.0, HEIGHT);
        assert!(!panel.preview_header);
        panel.section = 0;
        panel.click(&settings, 620.0, 364.0, HEIGHT);
        assert!(panel.preview_header);
        panel.click(&settings, 800.0, 424.0, HEIGHT);
        assert!(!panel.preview_header);
    }

    #[test]
    fn minimal_preview_has_no_clock_or_battery_content() {
        let text = Text::new();
        let panel = Panel::default();
        let mut pixels = Pixmap::new(WIDTH as u32, HEIGHT as u32).unwrap();
        draw(&mut pixels, &text, &panel, &Settings { minimal: true, ..Default::default() }, 1.0);
        for y in 154..158 {
            for x in 450..598 {
                let pixel = pixels.pixel(x, y).unwrap();
                assert_eq!((pixel.red(), pixel.green(), pixel.blue()), (0, 0, 0));
            }
        }
    }

    #[test]
    fn scale_sliders_are_independent() {
        let settings = Settings { scale: 1.2, open_scale: 0.75, ..Default::default() };
        let panel = Panel { dragging: Some(1), ..Default::default() };
        let Some(Command::Changed(changed)) = panel.drag(&settings, 722.0) else { panic!("open scale"); };
        assert_eq!(changed.open_scale, 1.25);
        assert_eq!(changed.scale, 1.2);
        assert!(Text::new().width(SECTIONS[0].fields[3].hint().unwrap(), 11.0) <= 254.0);
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(restored.open_scale, 0.75);
    }

    #[test]
    fn every_registered_value_control_changes_its_setting() {
        let settings = Settings::default();
        let initial = serde_json::to_value(&settings).unwrap();
        for (section, registered) in SECTIONS.iter().enumerate() {
            for (index, field) in registered.fields.iter().enumerate() {
                let mut panel = Panel { section, ..Default::default() };
                panel.scroll((index as f32 * 60.0 - 300.0).max(0.0), HEIGHT);
                let y = panel.row(index).y + 30.0;
                match field.control {
                    Control::Switch => {
                        let Some(Command::Changed(changed)) = panel.click(&settings, 800.0, y, HEIGHT) else { panic!("{}", field.key); };
                        let values = serde_json::to_value(changed).unwrap();
                        assert_eq!(values[field.key].as_bool(), initial[field.key].as_bool().map(|value| !value));
                    }
                    Control::Choice(options) => {
                        for (choice, option) in options.iter().enumerate() {
                            let x = 500.0 + (choice as f32 + 0.5) * 324.0 / options.len() as f32;
                            let Some(Command::Changed(changed)) = panel.click(&settings, x, y, HEIGHT) else { panic!("{}", field.key); };
                            assert_eq!(serde_json::to_value(changed).unwrap()[field.key], option.to_lowercase());
                        }
                    }
                    Control::Number(min, max, _, _) => {
                        panel.dragging = Some(index);
                        for (x, expected) in [(506.0, min), (722.0, max)] {
                            let Some(Command::Changed(changed)) = panel.drag(&settings, x) else { panic!("{}", field.key); };
                            let value = serde_json::to_value(changed).unwrap()[field.key].as_f64().unwrap();
                            assert!((value - expected).abs() < 0.001, "{}", field.key);
                        }
                    }
                    Control::Color => {
                        for (choice, rgb) in [[40, 110, 230], [48, 209, 88], [255, 159, 10], [255, 69, 58], WHITE, GRAY].into_iter().enumerate() {
                            let Some(Command::Changed(changed)) = panel.click(&settings, 514.0 + choice as f32 * 48.0, y, HEIGHT) else { panic!("{}", field.key); };
                            assert_eq!(serde_json::to_value(changed).unwrap()[field.key], Rgb(rgb).hex());
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn integration_fields_commit_and_clear_without_losing_other_settings() {
        let settings = Settings { clock_size: 28.0, ..Default::default() };
        for (index, input) in [(1, "https://example.com/calendar.ics"), (2, "thunar {path}"), (3, "/opt/apps; /home/example/Games")] {
            let mut panel = Panel { section: 5, editing: Some((index, input.into())), ..Default::default() };
            let Some(Command::Changed(changed)) = panel.commit(&settings) else { panic!("integration edit"); };
            assert_eq!(changed.clock_size, 28.0);
            if index == 3 { assert_eq!(changed.app_folders, ["/opt/apps", "/home/example/Games"]); }
            else { assert_eq!(serde_json::to_value(&changed).unwrap()[SECTIONS[5].fields[index].key], input); }
            panel.editing = Some((index, String::new()));
            let Some(Command::Changed(cleared)) = panel.commit(&changed) else { panic!("clear integration"); };
            if index == 3 { assert!(cleared.app_folders.is_empty()); }
            else { assert!(serde_json::to_value(cleared).unwrap()[SECTIONS[5].fields[index].key].is_null()); }
        }
        let mut panel = Panel { section: 5, editing: Some((0, "London".into())), ..Default::default() };
        assert!(matches!(panel.commit(&settings), Some(Command::City(city)) if city == "London"));
    }

    #[test]
    fn registered_controls_have_serialized_settings() {
        let settings = serde_json::to_value(Settings::default()).unwrap();
        for section in SECTIONS {
            for field in section.fields {
                if !matches!(field.control, Control::Action(_)) && field.key != "city" {
                    assert!(settings.get(field.key).is_some(), "{}", field.key);
                }
            }
        }
    }

    #[test]
    fn clock_slider_does_not_change_pill_scale() {
        let panel = Panel { section: 1, dragging: Some(1), ..Default::default() };
        let Some(Command::Changed(settings)) = panel.drag(&Settings::default(), 722.0) else { panic!("clock slider"); };
        assert_eq!(settings.clock_size, 32.0);
        assert_eq!(settings.scale, 1.0);
    }

    #[test]
    fn numeric_edits_and_reset_are_scoped_to_the_section() {
        let settings = Settings { scale: 1.2, game_style: GameStyle::Compact, ..Default::default() };
        let mut panel = Panel { section: 1, editing: Some((1, "28".into())), ..Default::default() };
        let Some(Command::Changed(changed)) = panel.commit(&settings) else { panic!("numeric edit"); };
        assert_eq!(changed.clock_size, 28.0);
        assert_eq!(changed.scale, 1.2);
        let Some(Command::Changed(reset)) = panel.click(&changed, 800.0, HEIGHT - 30.0, HEIGHT) else { panic!("reset"); };
        assert_eq!(reset.clock_size, 20.0);
        assert_eq!(reset.scale, 1.2);
        assert_eq!(reset.game_style, GameStyle::Compact);
    }

    #[test]
    fn older_settings_load_and_clock_sizes_round_trip() {
        let old: Settings = serde_json::from_str(r#"{"clock_24h":false,"scale":0.9}"#).unwrap();
        assert!(!old.clock_24h);
        assert_eq!(old.clock_size, 20.0);
        assert_eq!(old.battery_ring_size, 16.0);
        let changed = Settings { clock_size: 29.0, header_clock_size: 26.0, battery_ring_size: 30.0, ..old };
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.clock_size, 29.0);
        assert_eq!(restored.header_clock_size, 26.0);
        assert_eq!(restored.battery_ring_size, 30.0);
        assert_eq!(restored.scale, 0.9);
    }

    #[test]
    fn scrolled_rows_stay_inside_the_content_area() {
        let mut panel = Panel::default();
        panel.scroll(1000.0, 560.0);
        assert_eq!(panel.scroll, panel.content_bottom() - 496.0);
        assert_eq!(panel.row(SECTIONS[panel.section].fields.len() - 1).y + 60.0, 496.0);
    }
}