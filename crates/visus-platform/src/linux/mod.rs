//! Linux integration over D-Bus: MPRIS for media, UPower for the charger.

mod apps;
mod clip;
mod autostart;
mod ipc;
mod mpris;
mod power;

pub use autostart::{autostart_enabled, set_autostart};
pub use apps::{launch, open_path, open_url, screenshot};

/// Loads an app's icon off-thread and reports it as an event (None when it has no usable PNG).
pub fn request_icon(app: visus_core::App, emit: crate::Emit) {
    std::thread::spawn(move || {
        let art = apps::load_icon(&app);
        emit(visus_core::Event::AppIcon { target: app.target, art });
    });
}
pub use ipc::{already_running, send_toggle};
pub use mpris::media_command;

use crate::Emit;

/// Starts all background event sources. Call once.
pub fn start(emit: Emit) {
    mpris::start(emit.clone());
    power::start(emit.clone());
    ipc::listen(emit.clone());
    clip::watch(emit.clone());
    crate::game::watch(emit.clone());
    std::thread::spawn(move || emit(visus_core::Event::Apps(std::sync::Arc::new(apps::list()))));
}

pub use clip::{focus_window, list_windows};

pub fn prepare_window(_hwnd: isize) {}

pub fn set_window_region(_hwnd: isize, _x: i32, _y: i32, _w: i32, _h: i32, _radius: i32) {}

/// The compositor hides the global cursor; hosts that can query it do so themselves.
pub fn cursor_pos() -> Option<(i32, i32)> {
    None
}

/// Rebuilds the app list (after the user changes search folders) and reports it.
pub fn rescan_apps(emit: crate::Emit) {
    std::thread::spawn(move || emit(visus_core::Event::Apps(std::sync::Arc::new(apps::list()))));
}

/// Drive roots offered as search locations (Windows only).
pub fn list_drives() -> Vec<String> {
    Vec::new()
}

/// Windows-only: lets the notch take keyboard focus while the launcher is open.
pub fn set_focusable(_hwnd: isize, _on: bool) {}
