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
    Quit,
}

/// White pill on transparent, readable on light and dark taskbars.
fn icon() -> Option<Icon> {
    const N: u32 = 32;
    let mut px = vec![0u8; (N * N * 4) as usize];
    for y in 6..18 {
        for x in 2..30 {
            // Round the two bottom corners only, like the notch.
            let dx = (x as i32 - 15).abs() - 9;
            let dy = y as i32 - 11;
            if y >= 11 && dx > 0 && dx * dx + dy * dy > 49 {
                continue;
            }
            let i = ((y * N + x) * 4) as usize;
            px[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
        }
    }
    Icon::from_rgba(px, N, N).ok()
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

    let quit = MenuItem::new("Exit Visus", true, None);
    menu.append(&quit).ok()?;
    actions.push((quit.id().clone(), Action::Quit));

    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        let Some((_, action)) = actions.iter().find(|(id, _)| *id == e.id) else { return };
        let ev = match action {
            Action::Setting(c) => UserEvent::Setting(*c),
            Action::Autostart => UserEvent::ToggleAutostart,
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
