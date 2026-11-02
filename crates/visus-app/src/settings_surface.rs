use std::cell::RefCell;

use tiny_skia::Pixmap;
use visus_core::{Notch, Outcome, Settings, UpdateStatus};

use crate::ui::settings::{self, Command, Panel};

thread_local! {
    static PANEL: RefCell<Panel> = RefCell::new(Panel::default());
    static SELECTED_ALL: RefCell<bool> = const { RefCell::new(false) };
}

pub enum KeyAction {
    Text(String),
    Backspace,
    Submit,
    Escape,
    SelectAll,
    Paste,
}

pub fn render(text: &crate::text::Text, notch: &Notch, scale: f32) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new((settings::WIDTH * scale).ceil() as u32, (settings::HEIGHT * scale).ceil() as u32)?;
    PANEL.with_borrow_mut(|panel| {
        panel.update_available = matches!(notch.update_status(), UpdateStatus::Available { .. });
        if panel.section == settings::SECTIONS.len() - 1 {
            panel.status = match notch.update_status() {
                UpdateStatus::Idle => String::new(),
                UpdateStatus::Checking => "Checking for updates...".into(),
                UpdateStatus::UpToDate => "Visus is up to date".into(),
                UpdateStatus::Available { version } => format!("Update available: {version}"),
                UpdateStatus::Installing => "Installing update...".into(),
                UpdateStatus::Failed(error) => format!("Update failed: {error}"),
            };
        }
        settings::draw(&mut pixmap, text, panel, notch.settings(), scale);
    });
    Some(pixmap)
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
    if (140.0..176.0).contains(&x) && (20.0..56.0).contains(&y) {
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
    (x < 188.0 && y >= 88.0) || ((140.0..176.0).contains(&x) && (20.0..56.0).contains(&y))
        || ((674.0..824.0).contains(&x) && (22.0..54.0).contains(&y))
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
    let panel = Panel { section, preview_header: mode == "header", ..Default::default() };
    let mut pixmap = Pixmap::new(settings::WIDTH as u32, settings::HEIGHT as u32).expect("settings preview");
    settings::draw(&mut pixmap, &crate::text::Text::new(), &panel, &Settings::default(), 1.0);
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

    fn opened() -> Notch {
        PANEL.with_borrow_mut(|panel| *panel = Panel::default());
        let mut notch = Notch::new(960.0);
        notch.open_settings();
        for _ in 0..600 { notch.tick(std::time::Duration::from_millis(16)); }
        notch
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
    fn embedded_close_button_reverses_the_morph() {
        let mut notch = opened();
        let bounds = notch.shape().rect;
        click(&mut notch, bounds.x + 158.0, 38.0);
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