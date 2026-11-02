use super::*;
use visus_core::Settings;

pub const WIDTH: f32 = 860.0;
pub const HEIGHT: f32 = 720.0;
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

pub struct Section {
    pub title: &'static str,
    pub fields: &'static [Field],
}

pub static SECTIONS: &[Section] = &[
    Section { title: "Appearance", fields: &[
        Field { key: "scale", label: "Pill scale", control: Control::Number(0.6, 1.25, 0.05, "x") },
        Field { key: "show_battery", label: "Battery ring", control: Control::Switch },
        Field { key: "visualizer", label: "Music visualizer", control: Control::Switch },
        Field { key: "minimal", label: "Minimal pill", control: Control::Switch },
        Field { key: "net_bits", label: "Network speed in bits", control: Control::Switch },
    ] },
    Section { title: "Clock", fields: &[
        Field { key: "clock_style", label: "Style", control: Control::Choice(&["Classic", "Flip", "Led"]) },
        Field { key: "clock_size", label: "Closed clock size", control: Control::Number(12.0, 32.0, 1.0, "px") },
        Field { key: "header_clock_size", label: "Header clock size", control: Control::Number(16.0, 30.0, 1.0, "px") },
        Field { key: "clock_24h", label: "24-hour time", control: Control::Switch },
    ] },
    Section { title: "Mascot", fields: &[
        Field { key: "mascot", label: "Idle mascot", control: Control::Switch },
        Field { key: "eye_style", label: "Style", control: Control::Choice(&["Classic", "Sauron", "Cat", "Robot"]) },
        Field { key: "idle_secs", label: "Idle delay", control: Control::Number(5.0, 300.0, 5.0, "s") },
        Field { key: "eye_moods", label: "Expressions", control: Control::Switch },
        Field { key: "eye_iris", label: "Iris color", control: Control::Color },
        Field { key: "eye_sclera", label: "Eye color", control: Control::Color },
    ] },
    Section { title: "Behavior", fields: &[
        Field { key: "hide_in_fullscreen", label: "Hide in fullscreen", control: Control::Switch },
        Field { key: "quiet", label: "Quiet alerts", control: Control::Switch },
        Field { key: "clipboard_history", label: "Clipboard history", control: Control::Switch },
        Field { key: "timer_minutes", label: "Focus timer", control: Control::Number(1.0, 180.0, 1.0, "min") },
        Field { key: "autostart", label: "Start at login", control: Control::Action("autostart") },
    ] },
    Section { title: "Gaming", fields: &[
        Field { key: "game_hud", label: "Game HUD", control: Control::Switch },
        Field { key: "game_style", label: "Layout", control: Control::Choice(&["Pills", "Bars", "Card", "Compact"]) },
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
];

#[derive(Default)]
pub struct Panel {
    pub section: usize,
    pub scroll: f32,
    pub sidebar_scroll: f32,
    pub pointer: Option<(f32, f32)>,
    pub editing: Option<(usize, String)>,
    pub dragging: Option<usize>,
    pub status: String,
    pub preview_header: bool,
    pub update_available: bool,
}

pub enum Command {
    Changed(Settings),
    Action(&'static str),
    City(String),
}

impl Panel {
    fn row(&self, index: usize) -> Rect {
        rect(LEFT, 274.0 + index as f32 * 60.0 - self.scroll, RIGHT - LEFT, 60.0)
    }

    pub fn scroll(&mut self, amount: f32, height: f32) {
        if self.pointer.is_some_and(|(x, _)| x < 188.0) {
            let content = 96.0 + SECTIONS.len() as f32 * 44.0;
            self.sidebar_scroll = (self.sidebar_scroll + amount).clamp(0.0, (content - height + 16.0).max(0.0));
            return;
        }
        let content = 274.0 + SECTIONS[self.section].fields.len() as f32 * 60.0;
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
        let value = if matches!(field.key, "idle_secs" | "timer_minutes") {
            serde_json::json!(number.round() as u32)
        } else { serde_json::json!(number) };
        Self::assign(settings, field, value)
    }

    pub fn click(&mut self, settings: &Settings, x: f32, y: f32, height: f32) -> Option<Command> {
        self.editing = None;
        if rect(674.0, 22.0, 150.0, 32.0).contains(x, y) {
            self.preview_header = x >= 749.0;
            return None;
        }
        if x < 188.0 {
            for index in 0..SECTIONS.len() {
                let area = rect(16.0, 96.0 + index as f32 * 44.0 - self.sidebar_scroll, 156.0, 38.0);
                if area.y >= 88.0 && area.y + area.h <= height - 12.0 && area.contains(x, y) {
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
            match field.control {
                Control::Switch => return Self::assign(settings, field, serde_json::json!(!values[field.key].as_bool()?)),
                Control::Number(..) => {
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
            return Self::assign(settings, field, if matches!(field.key, "idle_secs" | "timer_minutes") {
                serde_json::json!(value as u32)
            } else { serde_json::json!(value) });
        }
        let value = if matches!(field.control, Control::Folders) {
            serde_json::json!(input.split(';').map(str::trim).filter(|path| !path.is_empty()).collect::<Vec<_>>())
        } else if input.is_empty() { serde_json::Value::Null } else { serde_json::json!(input) };
        Self::assign(settings, field, value)
    }
}

pub fn draw(pm: &mut Pixmap, text: &Text, panel: &Panel, settings: &Settings, scale: f32) {
    let height = pm.height() as f32 / scale;
    pm.fill(Color::from_rgba8(23, 24, 27, 255));
    let mut cx = Ctx { accent: settings.eye_iris.0, ink: WHITE, pm, text, scale, ox: 0.0, oy: 0.0 };
    cx.rrect(rect(0.0, 0.0, 188.0, height), 0.0, color([29, 30, 34], 1.0));
    cx.t("VISUS", 24.0, 40.0, 20.0, color(WHITE, 1.0), Align::Left);
    cx.t("Settings", 24.0, 64.0, 13.0, color(GRAY, 1.0), Align::Left);
    let mut close = PathBuilder::new();
    close.move_to(151.0, 31.0);
    close.line_to(163.0, 43.0);
    close.move_to(163.0, 31.0);
    close.line_to(151.0, 43.0);
    if let Some(path) = close.finish() { cx.stroke(&path, color(GRAY, 1.0), 1.6); }
    for (index, section) in SECTIONS.iter().enumerate() {
        let row = rect(16.0, 96.0 + index as f32 * 44.0 - panel.sidebar_scroll, 156.0, 38.0);
        if row.y < 88.0 || row.y + row.h > height - 12.0 { continue; }
        if index == panel.section {
            cx.rrect(row, 6.0, color(WHITE, 0.08));
            cx.rrect(rect(16.0, row.y + 10.0, 3.0, 18.0), 1.5, color(cx.accent, 1.0));
        }
        let title = cx.fit(section.title, 15.0, 132.0);
        cx.t(&title, 32.0, row.y + 25.0, 15.0, color(if index == panel.section { WHITE } else { GRAY }, 1.0), Align::Left);
    }
    cx.t(SECTIONS[panel.section].title, LEFT, 48.0, 25.0, color(WHITE, 1.0), Align::Left);
    for (index, label) in ["Closed", "Header"].iter().enumerate() {
        let x = 674.0 + index as f32 * 75.0;
        let selected = panel.preview_header == (index == 1);
        cx.rrect(rect(x, 22.0, 72.0, 32.0), 5.0, color(WHITE, if selected { 0.12 } else { 0.04 }));
        cx.t(label, x + 36.0, 43.0, 12.0, color(if selected { WHITE } else { GRAY }, 1.0), Align::Center);
    }
    cx.rrect(rect(LEFT, 76.0, RIGHT - LEFT, 168.0), 8.0, color([19, 20, 23], 1.0));
    let mut preview = Notch::new(RIGHT - LEFT);
    let mascot_preview = SECTIONS[panel.section].title == "Mascot" && settings.mascot && !panel.preview_header;
    preview.set_settings(Settings { mascot: mascot_preview, scale: 1.0, ..settings.clone() });
    preview.handle(visus_core::Event::Power { on_ac: false, percent: 72 });
    if mascot_preview { preview.rewind_activity(std::time::Duration::from_secs(settings.idle_secs as u64 + 2)); }
    let gaming_preview = SECTIONS[panel.section].title == "Gaming" && settings.game_hud && !panel.preview_header;
    if gaming_preview {
        preview.handle(visus_core::Event::Game(true));
        for index in 0..GAME_HIST {
            let fps = 144.0 + (index as f32 * 0.8).sin() * 8.0;
            preview.handle(visus_core::Event::GameStats(GameStats {
                fps: Some(fps), frame_ms: Some(1000.0 / fps), cpu: 0.42, gpu: Some(0.76),
                cpu_temp: Some(62.0), gpu_temp: Some(68.0), ram: 0.38, ram_used_gb: 12.1,
            }));
        }
    }
    if panel.preview_header { preview.hover(true); }
    for _ in 0..300 { preview.tick(std::time::Duration::from_millis(16)); }
    let shape = preview.shape().rect;
    let old_x = cx.ox;
    let old_y = cx.oy;
    let preview_scale = ((RIGHT - LEFT - 32.0) / shape.w).min(140.0 / shape.h).min(settings.scale);
    cx.scale = scale * preview_scale;
    cx.ox = (LEFT + (RIGHT - LEFT) / 2.0 - (shape.x + shape.w / 2.0) * preview_scale) * scale;
    cx.oy = (156.0 - shape.h * preview_scale / 2.0) * scale;
    cx.rrect(shape, preview.shape().bottom_radius, color([0, 0, 0], 1.0));
    if panel.preview_header { draw_open(&mut cx, &preview, 1.0); }
    else if gaming_preview { draw_game(&mut cx, &preview, 1.0); }
    else if mascot_preview { draw_eye(&mut cx, &preview, 1.0); }
    else { draw_closed(&mut cx, &preview, 1.0); }
    cx.scale = scale;
    cx.ox = old_x;
    cx.oy = old_y;
    let values = serde_json::to_value(settings).unwrap_or_default();
    for (index, field) in SECTIONS[panel.section].fields.iter().enumerate() {
        let row = panel.row(index);
        if row.y < 258.0 || row.y + row.h > height - 64.0 { continue; }
        cx.rrect(rect(LEFT, row.y + row.h - 1.0, RIGHT - LEFT, 1.0), 0.0, color(WHITE, 0.07));
        let label = cx.fit(field.label, 14.0, 254.0);
        cx.t(&label, LEFT, row.y + 36.0, 14.0, color(WHITE, 0.92), Align::Left);
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
                } else if panel.update_available { "Install" } else { "Check now" };
                cx.rrect(rect(RIGHT - 110.0, mid - 16.0, 110.0, 32.0), 5.0, color(WHITE, 0.08));
                cx.t(label, RIGHT - 55.0, mid + 4.5, 13.0, color(WHITE, 1.0), Align::Center);
            }
        }
    }
    cx.rrect(rect(LEFT, height - 64.0, RIGHT - LEFT, 64.0), 0.0, color([23, 24, 27], 1.0));
    let status = cx.fit(&panel.status, 12.0, 440.0);
    cx.t(&status, LEFT, height - 27.0, 12.0, color(GRAY, 1.0), Align::Left);
    cx.t("Reset section", RIGHT, height - 27.0, 13.0, color(GRAY, 1.0), Align::Right);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registered_value_control_changes_its_setting() {
        let settings = Settings::default();
        let initial = serde_json::to_value(&settings).unwrap();
        for (section, registered) in SECTIONS.iter().enumerate() {
            for (index, field) in registered.fields.iter().enumerate() {
                let mut panel = Panel { section, ..Default::default() };
                let y = 304.0 + index as f32 * 60.0;
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
        let changed = Settings { clock_size: 29.0, header_clock_size: 26.0, ..old };
        let restored: Settings = serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(restored.clock_size, 29.0);
        assert_eq!(restored.header_clock_size, 26.0);
        assert_eq!(restored.scale, 0.9);
    }

    #[test]
    fn scrolled_rows_stay_inside_the_content_area() {
        let mut panel = Panel { section: 2, ..Default::default() };
        panel.scroll(1000.0, 560.0);
        assert_eq!(panel.scroll, 138.0);
        assert_eq!(panel.row(5).y + 60.0, 496.0);
    }
}