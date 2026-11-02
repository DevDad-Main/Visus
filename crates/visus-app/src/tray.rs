use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use visus_core::{Rgb, Settings};
use winit::event_loop::EventLoopProxy;

use crate::{Change, UserEvent};

pub struct Tray {
    _icon: TrayIcon,
    autostart: CheckMenuItem,
}

impl Tray {
    pub fn set_autostart_checked(&self, on: bool) {
        self.autostart.set_checked(on);
    }
}

const SIZES: [(&str, f32); 3] = [("Small", 0.8), ("Default", 1.0), ("Large", 1.15)];
const EYE_COLOURS: [(&str, [u8; 3]); 6] = [
    ("Blue", [40, 110, 230]),
    ("Green", [48, 209, 88]),
    ("Amber", [255, 159, 10]),
    ("Red", [255, 69, 58]),
    ("Violet", [191, 90, 242]),
    ("Grey", [160, 160, 168]),
];

enum Action {
    Setting(Change),
    Autostart,
    Uninstall,
    Quit,
}

/// The app icon rendered at 32x32 (see tools/make_icon.py).
fn icon() -> Option<Icon> {
    Icon::from_rgba(include_bytes!("../assets/tray32.rgba").to_vec(), 32, 32).ok()
}

pub fn create(proxy: EventLoopProxy<UserEvent>, s: &Settings) -> Option<Tray> {
    let mut actions: Vec<(MenuId, Action)> = Vec::new();
    let menu = Menu::new();

    let mut check = |label: &str, on: bool, change: Change| -> Option<CheckMenuItem> {
        let item = CheckMenuItem::new(label, true, on, None);
        menu.append(&item).ok()?;
        actions.push((item.id().clone(), Action::Setting(change)));
        Some(item)
    };
    check("Minimal notch", s.minimal, Change::ToggleMinimal)?;
    check("Idle eye", s.mascot, Change::ToggleMascot)?;
    check("Hide in full screen", s.hide_in_fullscreen, Change::ToggleHideFullscreen)?;

    let autostart = CheckMenuItem::new("Start with Windows", true, visus_platform::autostart_enabled(), None);
    menu.append(&autostart).ok()?;
    actions.push((autostart.id().clone(), Action::Autostart));
    menu.append(&PredefinedMenuItem::separator()).ok()?;

    let size = Submenu::new("Size", true);
    for (label, k) in SIZES {
        let item = MenuItem::new(label, true, None);
        size.append(&item).ok()?;
        actions.push((item.id().clone(), Action::Setting(Change::Scale(k))));
    }
    menu.append(&size).ok()?;

    let eye = Submenu::new("Eye colour", true);
    for (label, rgb) in EYE_COLOURS {
        let item = MenuItem::new(label, true, None);
        eye.append(&item).ok()?;
        actions.push((item.id().clone(), Action::Setting(Change::EyeIris(Rgb(rgb)))));
    }
    menu.append(&eye).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;

    let uninstall = MenuItem::new("Uninstall Visus\u{2026}", true, None);
    menu.append(&uninstall).ok()?;
    actions.push((uninstall.id().clone(), Action::Uninstall));

    let quit = MenuItem::new("Exit Visus", true, None);
    menu.append(&quit).ok()?;
    actions.push((quit.id().clone(), Action::Quit));

    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let Some((_, action)) = actions.iter().find(|(id, _)| *id == e.id) else { return };
        let ev = match action {
            Action::Setting(c) => UserEvent::Setting(*c),
            Action::Autostart => UserEvent::ToggleAutostart,
            Action::Uninstall => UserEvent::Uninstall,
            Action::Quit => UserEvent::Quit,
        };
        let _ = proxy.send_event(ev);
    }));

    let mut b = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("Visus");
    if let Some(i) = icon() {
        b = b.with_icon(i);
    }
    Some(Tray { _icon: b.build().ok()?, autostart })
}
