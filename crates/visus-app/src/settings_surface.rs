use std::cell::RefCell;
use std::rc::Rc;

use tiny_skia::Pixmap;
use visus_core::{Notch, Outcome, Settings, UpdateStatus};

use crate::ui::settings::{self, Command, Panel};

thread_local! {
    static PANEL: RefCell<Panel> = RefCell::new(Panel::default());
    static SELECTED_ALL: RefCell<bool> = const { RefCell::new(false) };
    static CACHE: RefCell<Option<RenderedPanel>> = const { RefCell::new(None) };
}

struct RenderedPanel {
    settings: Settings,
    panel: Panel,
    scale: f32,
    minute: i64,
    pixels: Rc<Pixmap>,
}

pub enum KeyAction {
    Text(String),
    Backspace,
    Submit,
    Escape,
    SelectAll,
    Paste,
}

pub fn render(text: &crate::text::Text, notch: &Notch, scale: f32) -> Option<Rc<Pixmap>> {
    PANEL.with_borrow_mut(|panel| {
        panel.usage = crate::app_usage::sample();
        panel.frame_source = crate::game::frame_source_note();
        panel.close_hovered = ready(notch) && notch.pointer().is_some_and(|(x, y)| {
            let (x, y) = point(notch, x, y);
            settings::CLOSE.contains(x, y)
        });
        panel.media_accent = Some(notch.theme());
        panel.reaction_style = notch.settings().eye_style;
        panel.battery = notch.battery();
        panel.scroll(0.0, settings::HEIGHT);
        panel.update_available = matches!(notch.update_status(), UpdateStatus::Available { .. });
        if panel.update_status != *notch.update_status() || settings::SECTIONS[panel.section].title == "Updates" {
            panel.update_status = notch.update_status().clone();
            panel.status = match notch.update_status() {
                UpdateStatus::Idle => String::new(),
                UpdateStatus::Checking => "Checking for updates...".into(),
                UpdateStatus::UpToDate => format!("Visus {} is up to date", env!("CARGO_PKG_VERSION")),
                UpdateStatus::Available { version } => format!("Update available: {version}"),
                UpdateStatus::Installing => "Installing update...".into(),
                UpdateStatus::Failed(error) => format!("Update failed: {error}"),
            };
        }
        let minute = chrono::Local::now().timestamp().div_euclid(60);
        let mut visual = panel.clone();
        visual.pointer = None;
        visual.dragging = None;
        CACHE.with_borrow_mut(|cached| {
            if let Some(previous) = cached.as_ref() {
                if previous.settings == *notch.settings() && previous.panel == visual
                    && previous.scale == scale && previous.minute == minute {
                    return Some(previous.pixels.clone());
                }
            }
            let mut pixmap = Pixmap::new((settings::WIDTH * scale).ceil() as u32, (settings::HEIGHT * scale).ceil() as u32)?;
            settings::draw(&mut pixmap, text, panel, notch.settings(), scale);
            let pixels = Rc::new(pixmap);
            *cached = Some(RenderedPanel { settings: notch.settings().clone(), panel: visual, scale, minute, pixels: pixels.clone() });
            Some(pixels)
        })
    })
}

fn ready(notch: &Notch) -> bool {
    notch.settings_is_open() && notch.settings_progress() >= 0.98 && notch.shape().rect.h > settings::HEIGHT - 8.0
}

fn point(notch: &Notch, x: f32, y: f32) -> (f32, f32) {
    let bounds = notch.shape().rect;
    ((x - bounds.x) * settings::WIDTH / bounds.w, (y - bounds.y) * settings::HEIGHT / bounds.h)
}

fn apply(notch: &mut Notch, command: Option<Command>) -> Outcome {
    match command {
        Some(Command::Changed(settings)) => {
            notch.set_settings(settings);
            Outcome::None
        }
        Some(Command::City(city)) => {
            PANEL.with_borrow_mut(|panel| panel.status = "Searching...".into());
            Outcome::Geocode(city)
        }
        Some(Command::Action("autostart")) => {
            if let Err(error) = visus_platform::set_autostart(!visus_platform::autostart_enabled()) {
                PANEL.with_borrow_mut(|panel| panel.status = format!("Startup setting: {error}"));
            }
            Outcome::None
        }
        Some(Command::Action("updates")) => {
            if matches!(notch.update_status(), UpdateStatus::Checking | UpdateStatus::Installing) { return Outcome::None; }
            if matches!(notch.update_status(), UpdateStatus::Available { .. }) {
                notch.set_update_status(UpdateStatus::Installing);
                Outcome::InstallUpdate
            } else {
                notch.set_update_status(UpdateStatus::Checking);
                Outcome::CheckUpdates
            }
        }
        _ => Outcome::None,
    }
}

pub fn click(notch: &mut Notch, x: f32, y: f32) -> Outcome {
    if !ready(notch) { return Outcome::None; }
    let (x, y) = point(notch, x, y);
    SELECTED_ALL.with_borrow_mut(|selected| *selected = false);
    let pending = PANEL.with_borrow_mut(|panel| panel.commit(notch.settings()));
    let pending = apply(notch, pending);
    if settings::CLOSE.contains(x, y) {
        release(notch);
        notch.close_settings();
        return pending;
    }
    let command = PANEL.with_borrow_mut(|panel| panel.click(notch.settings(), x, y, settings::HEIGHT));
    let outcome = apply(notch, command);
    if matches!(outcome, Outcome::None) { pending } else { outcome }
}

pub fn drag(notch: &mut Notch, x: f32) {
    if !ready(notch) { return; }
    let (x, _) = point(notch, x, 0.0);
    let command = PANEL.with_borrow(|panel| panel.drag(notch.settings(), x));
    apply(notch, command);
}

pub fn release(notch: &mut Notch) {
    PANEL.with_borrow_mut(|panel| panel.dragging = None);
    notch.end_drag();
}

pub fn scroll(notch: &Notch, amount: f32) {
    if !ready(notch) { return; }
    let pointer = notch.pointer().map(|(x, y)| point(notch, x, y));
    PANEL.with_borrow_mut(|panel| {
        panel.pointer = pointer;
        panel.scroll(amount, settings::HEIGHT);
    });
}

pub fn over_clickable(notch: &Notch) -> bool {
    if !ready(notch) { return false; }
    let Some((x, y)) = notch.pointer() else { return false };
    let (x, y) = point(notch, x, y);
    (x < 188.0 && y >= 88.0) || settings::CLOSE.contains(x, y)
        || ((638.0..788.0).contains(&x) && (22.0..54.0).contains(&y))
        || ((224.0..364.0).contains(&x) && (settings::HEIGHT - 58.0..settings::HEIGHT - 20.0).contains(&y))
        || ((500.0..824.0).contains(&x) && y >= 258.0)
}

pub fn paste(text: String) {
    PANEL.with_borrow_mut(|panel| {
        if let Some((_, input)) = &mut panel.editing {
            SELECTED_ALL.with_borrow_mut(|selected| {
                if *selected { input.clear(); *selected = false; }
            });
            input.push_str(&text.replace(['\n', '\r'], " "));
        }
    });
}

pub fn key(notch: &mut Notch, action: KeyAction, emit: Option<&visus_platform::Emit>) -> Outcome {
    match action {
        KeyAction::Escape => {
            let editing = PANEL.with_borrow_mut(|panel| panel.editing.take().is_some());
            if !editing { release(notch); notch.close_settings(); }
        }
        KeyAction::Submit => {
            let command = PANEL.with_borrow_mut(|panel| panel.commit(notch.settings()));
            return apply(notch, command);
        }
        KeyAction::SelectAll => SELECTED_ALL.with_borrow_mut(|selected| *selected = true),
        KeyAction::Paste => {
            if let Some(emit) = emit.cloned() {
                std::thread::spawn(move || {
                    if let Some(text) = visus_platform::clipboard_text() { emit(visus_core::Event::SettingsPaste(text)); }
                });
            }
        }
        KeyAction::Text(text) => paste(text),
        KeyAction::Backspace => {
            PANEL.with_borrow_mut(|panel| {
                if let Some((_, input)) = &mut panel.editing {
                    SELECTED_ALL.with_borrow_mut(|selected| {
                        if *selected { input.clear(); *selected = false; } else { input.pop(); }
                    });
                }
            });
        }
    }
    Outcome::None
}

pub fn location_result(city: Option<&str>) {
    PANEL.with_borrow_mut(|panel| panel.status = city.map(|city| format!("Weather location: {city}")).unwrap_or_else(|| "City not found".into()));
}

pub fn preview(out: &str, section: Option<&str>) {
    let (name, mode) = section.unwrap_or("Clock").split_once(':').unwrap_or((section.unwrap_or("Clock"), "closed"));
    let section = settings::SECTIONS.iter().position(|section| section.title.eq_ignore_ascii_case(name)).unwrap_or(1);
    let style = match mode { "robot" => visus_core::EyeStyle::Robot, "cat" => visus_core::EyeStyle::Cat, "sauron" => visus_core::EyeStyle::Sauron, _ => visus_core::EyeStyle::Classic };
    let pill_style = match mode { "floating" => visus_core::PillStyle::Floating, "soft" => visus_core::PillStyle::Soft, _ => visus_core::PillStyle::Attached };
    let panel = Panel { section, preview_header: mode == "header", reaction_style: style,
        scroll: if matches!(mode, "robot" | "cat" | "sauron") { 420.0 } else { 0.0 }, ..Default::default() };
    let mut pixmap = Pixmap::new(settings::WIDTH as u32, settings::HEIGHT as u32).expect("settings preview");
    settings::draw(&mut pixmap, &crate::text::Text::new(), &panel, &Settings { eye_style: style, pill_style, ..Default::default() }, 1.0);
    pixmap.save_png(out).expect("settings preview png");
}

pub fn snapshot(out: &str, section: Option<&str>) {
    let section = section.and_then(|name| settings::SECTIONS.iter().position(|section| section.title.eq_ignore_ascii_case(name))).unwrap_or(1);
    PANEL.with_borrow_mut(|panel| *panel = Panel { section, ..Default::default() });
    let mut notch = Notch::new(960.0);
    notch.open_settings();
    for _ in 0..600 { notch.tick(std::time::Duration::from_millis(16)); }
    let mut pixmap = Pixmap::new(1920, 1600).expect("settings snapshot");
    crate::ui::draw(&mut pixmap, &crate::text::Text::new(), &notch, 2.0);
    pixmap.save_png(out).expect("settings snapshot png");
}

pub fn morph_preview(prefix: &str) {
    let text = crate::text::Text::new();
    let mut notch = Notch::new(960.0);
    notch.update_settings(|settings| settings.mascot = false);
    for _ in 0..300 { notch.tick(std::time::Duration::from_millis(16)); }
    PANEL.with_borrow_mut(|panel| panel.section = 1);
    notch.open_settings();
    for frame in 0..180 {
        if frame == 90 { notch.close_settings(); }
        notch.tick(std::time::Duration::from_millis(16));
        let mut pixmap = Pixmap::new(960, 800).expect("morph preview");
        pixmap.fill(tiny_skia::Color::from_rgba8(47, 49, 54, 255));
        crate::ui::draw(&mut pixmap, &text, &notch, 1.0);
        pixmap.save_png(format!("{prefix}-{frame:03}.png")).expect("morph preview png");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn benchmark_settings_animation_frames() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        let start = std::time::Instant::now();
        for _ in 0..30 {
            let mut pixmap = Pixmap::new(960, 800).unwrap();
            crate::ui::draw(&mut pixmap, &text, &notch, 1.0);
            notch.tick(std::time::Duration::from_millis(16));
        }
        eprintln!("Settings draw: {:.2} ms/frame", start.elapsed().as_secs_f64() * 1000.0 / 30.0);
    }

    fn opened() -> Notch {
        PANEL.with_borrow_mut(|panel| *panel = Panel::default());
        let mut notch = Notch::new(960.0);
        notch.open_settings();
        for _ in 0..600 { notch.tick(std::time::Duration::from_millis(16)); }
        notch
    }

    #[test]
    fn settings_surface_is_black_and_preserves_top_flares() {
        let notch = opened();
        let mut pixels = Pixmap::new(960, 800).unwrap();
        crate::ui::draw(&mut pixels, &crate::text::Text::new(), &notch, 1.0);
        let black = pixels.pixel(480, 680).unwrap();
        assert_eq!((black.red(), black.green(), black.blue(), black.alpha()), (0, 0, 0, 255));
        let left = notch.shape().rect.x as u32;
        let flare = pixels.pixel(left - 4, 1).unwrap();
        assert_eq!((flare.red(), flare.green(), flare.blue(), flare.alpha()), (0, 0, 0, 255));
        assert_eq!(pixels.pixel(left - 4, 20).unwrap().alpha(), 0);
    }

    #[test]
    fn settings_highlights_follow_media_theme_on_black() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        let before = render(&text, &notch, 1.0).unwrap();
        notch.handle(visus_core::Event::Media(visus_core::MediaInfo {
            has_session: true, playing: true, title: "Theme test".into(),
            art: Some(std::sync::Arc::new(visus_core::Art { size: 2, rgba: [230, 60, 40, 255].repeat(4), id: 987 })),
            ..Default::default()
        }));
        for _ in 0..120 { notch.tick(std::time::Duration::from_millis(16)); }
        let after = render(&text, &notch, 1.0).unwrap();
        PANEL.with_borrow(|panel| assert_eq!(panel.media_accent, Some(notch.theme())));
        assert_ne!(before.data(), after.data());
        let background = after.pixel(480, 680).unwrap();
        assert_eq!((background.red(), background.green(), background.blue()), (0, 0, 0));
    }

    #[test]
    fn preview_battery_tracks_real_state_and_invalidates_cached_pixels() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        notch.handle(visus_core::Event::Power { on_ac: false, percent: 45 });
        let unplugged = render(&text, &notch, 1.0).unwrap();
        PANEL.with_borrow(|panel| assert_eq!(panel.battery, Some((false, 45))));
        notch.handle(visus_core::Event::Power { on_ac: true, percent: 46 });
        let charging = render(&text, &notch, 1.0).unwrap();
        PANEL.with_borrow(|panel| assert_eq!(panel.battery, Some((true, 46))));
        assert!(!Rc::ptr_eq(&unplugged, &charging));
        assert_ne!(unplugged.data(), charging.data());
    }

    #[test]
    fn settings_render_cache_reuses_pixels_and_invalidates_on_changes() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        let initial = render(&text, &notch, 1.0).unwrap();
        assert!(Rc::ptr_eq(&initial, &render(&text, &notch, 1.0).unwrap()));
        notch.update_settings(|settings| settings.clock_size = 24.0);
        let changed = render(&text, &notch, 1.0).unwrap();
        assert!(!Rc::ptr_eq(&initial, &changed));
        PANEL.with_borrow_mut(|panel| panel.section = 1);
        assert!(!Rc::ptr_eq(&changed, &render(&text, &notch, 1.0).unwrap()));
    }

    #[test]
    fn updater_feedback_and_resource_footer_survive_tab_changes() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        notch.set_update_status(UpdateStatus::UpToDate);
        for section in 0..settings::SECTIONS.len() {
            PANEL.with_borrow_mut(|panel| { panel.section = section; panel.scroll = 500.0; });
            let pixels = render(&text, &notch, 1.0).unwrap();
            PANEL.with_borrow(|panel| {
                assert!(panel.status.contains(env!("CARGO_PKG_VERSION")));
                assert!(panel.usage.memory_mb.is_some());
            });
            assert!((630..710).any(|y| (24..176).any(|x| pixels.pixel(x, y).unwrap().red() > 150)));
        }
    }

    #[test]
    fn update_controls_request_check_and_install_actions() {
        let mut notch = opened();
        assert!(matches!(apply(&mut notch, Some(Command::Action("updates"))), Outcome::CheckUpdates));
        assert_eq!(notch.update_status(), &UpdateStatus::Checking);
        notch.set_update_status(UpdateStatus::Available { version: "v99.0.0".into() });
        assert!(matches!(apply(&mut notch, Some(Command::Action("updates"))), Outcome::InstallUpdate));
        assert_eq!(notch.update_status(), &UpdateStatus::Installing);
    }

    #[test]
    fn close_hover_updates_pixels_without_overlapping_preview_tabs() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        let normal = render(&text, &notch, 1.0).unwrap();
        let bounds = notch.shape().rect;
        notch.set_pointer(Some((bounds.x + settings::CLOSE.x + 18.0, settings::CLOSE.y + 18.0)));
        assert!(over_clickable(&notch));
        let hovered = render(&text, &notch, 1.0).unwrap();
        assert_ne!(normal.data(), hovered.data());
        PANEL.with_borrow(|panel| assert!(panel.close_hovered));
        notch.set_pointer(None);
        render(&text, &notch, 1.0);
        PANEL.with_borrow(|panel| assert!(!panel.close_hovered));
        assert!(settings::CLOSE.x > 788.0);
    }

    #[test]
    fn all_styles_map_the_settings_close_target_correctly() {
        for style in [visus_core::PillStyle::Attached, visus_core::PillStyle::Floating, visus_core::PillStyle::Soft] {
            let mut notch = opened();
            notch.update_settings(|settings| settings.pill_style = style);
            let bounds = notch.shape().rect;
            let screen = (bounds.x + settings::CLOSE.x + 18.0, crate::ui::top_offset(&notch) + settings::CLOSE.y + 18.0);
            let logical = crate::ui::to_logical(&notch, screen.0, screen.1);
            click(&mut notch, logical.0, logical.1);
            assert!(!notch.settings_is_open());
        }
    }

    #[test]
    fn embedded_close_button_reverses_the_morph() {
        let mut notch = opened();
        let bounds = notch.shape().rect;
        click(&mut notch, bounds.x + settings::CLOSE.x + 18.0, settings::CLOSE.y + 18.0);
        assert!(!notch.settings_is_open());
        assert!(notch.is_animating());
    }

    #[test]
    fn escape_cancels_edit_then_collapses_settings() {
        let mut notch = opened();
        PANEL.with_borrow_mut(|panel| panel.editing = Some((0, "test".into())));
        key(&mut notch, KeyAction::Escape, None);
        assert!(notch.settings_is_open());
        key(&mut notch, KeyAction::Escape, None);
        assert!(!notch.settings_is_open());
    }

    #[test]
    fn morph_pixels_remain_inside_the_animated_surface() {
        let mut notch = opened();
        let text = crate::text::Text::new();
        for closing in [false, true] {
            if closing { notch.close_settings(); }
            for _ in 0..24 {
                notch.tick(std::time::Duration::from_millis(16));
                let mut pixmap = Pixmap::new(960, 800).unwrap();
                crate::ui::draw(&mut pixmap, &text, &notch, 1.0);
                let bottom = (notch.shape().rect.h.ceil() as usize + 1).min(800);
                assert!(pixmap.pixels()[bottom * 960..].iter().all(|pixel| pixel.alpha() == 0));
                assert!(pixmap.pixels().iter().any(|pixel| pixel.alpha() > 0));
            }
        }
    }
}