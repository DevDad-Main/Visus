use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};
use winit::event_loop::EventLoopProxy;

use crate::UserEvent;

pub struct Tray {
    _icon: TrayIcon,
    autostart: CheckMenuItem,
}

impl Tray {
    pub fn set_autostart_checked(&self, on: bool) {
        self.autostart.set_checked(on);
    }
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

pub fn create(proxy: EventLoopProxy<UserEvent>) -> Option<Tray> {
    let menu = Menu::new();
    let autostart = CheckMenuItem::new("Start with Windows", true, platform_autostart(), None);
    let quit = MenuItem::new("Exit Visus", true, None);
    menu.append_items(&[&autostart, &quit]).ok()?;

    let (auto_id, quit_id) = (autostart.id().clone(), quit.id().clone());
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
        if e.id == auto_id {
            let _ = proxy.send_event(UserEvent::ToggleAutostart);
        } else if e.id == quit_id {
            let _ = proxy.send_event(UserEvent::Quit);
        }
    }));

    let mut b = TrayIconBuilder::new().with_menu(Box::new(menu)).with_tooltip("Visus");
    if let Some(i) = icon() {
        b = b.with_icon(i);
    }
    Some(Tray { _icon: b.build().ok()?, autostart })
}

fn platform_autostart() -> bool {
    visus_platform_windows::autostart_enabled()
}
