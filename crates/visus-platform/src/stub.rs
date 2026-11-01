use crate::Emit;
use visus_core::MediaCommand;

pub fn start(_emit: Emit) {}

pub fn media_command(_cmd: MediaCommand) {}

pub fn prepare_window(_hwnd: isize) {}

pub fn set_window_region(_hwnd: isize, _x: i32, _y: i32, _w: i32, _h: i32, _radius: i32) {}

pub fn autostart_enabled() -> bool {
    false
}

pub fn set_autostart(_enabled: bool) -> std::io::Result<()> {
    Ok(())
}

/// Global cursor position in screen pixels; unavailable off Windows.
pub fn cursor_pos() -> Option<(i32, i32)> {
    None
}

pub fn launch(_target: &str) {}

pub fn open_url(_url: &str) {}

pub fn set_focusable(_hwnd: isize, _on: bool) {}

/// Linux-only: asks a running instance to toggle the launcher.
pub fn send_toggle() -> bool {
    false
}

pub fn open_path(_path: &str) {}

pub fn list_windows() -> Vec<(isize, String)> {
    Vec::new()
}

pub fn focus_window(_h: isize) {}

pub fn screenshot() {}

pub fn request_icon(app: visus_core::App, emit: crate::Emit) {
    emit(visus_core::Event::AppIcon { target: app.target, art: None });
}

pub fn already_running() -> bool {
    false
}

pub fn rescan_apps(_emit: crate::Emit) {}

pub fn list_drives() -> Vec<String> {
    Vec::new()
}
