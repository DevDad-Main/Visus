//! Windows implementation. Everything is event-driven: one hidden-window thread receives
//! power broadcasts, a low-level key hook and foreground-change hooks; media runs on WinRT events.

mod apps;
mod autostart;
mod events;
mod media;
mod network;
mod window;

pub use apps::{launch, open_path, open_url, screenshot};

/// Loads an app's icon off-thread and reports it as an event (None when the shell has none).
pub fn request_icon(app: visus_core::App, emit: crate::Emit) {
    std::thread::spawn(move || {
        let art = apps::load_icon(&app);
        emit(visus_core::Event::AppIcon { target: app.target, art });
    });
}
pub use autostart::{autostart_enabled, set_autostart};
pub use media::media_command;
pub use window::{cursor_pos, prepare_window, set_focusable, set_window_region};

use crate::Emit;

/// Starts all background event sources. Call once.
pub fn start(emit: Emit) {
    media::start(emit.clone());
    network::start(emit.clone());
    events::start(emit.clone());
    std::thread::spawn(move || emit(visus_core::Event::Apps(std::sync::Arc::new(apps::list()))));
}

/// True when another Visus holds the single-instance mutex (this process takes it otherwise).
pub fn already_running() -> bool {
    use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
    use windows::Win32::System::Threading::CreateMutexW;
    unsafe {
        let handle = CreateMutexW(None, true, windows::core::w!("Local\\VisusSingleInstance"));
        let exists = GetLastError() == ERROR_ALREADY_EXISTS;
        // The mutex must outlive main; the OS releases it at exit.
        std::mem::forget(handle);
        exists
    }
}

/// Linux-only: asks a running instance to toggle the launcher (Windows uses the Alt+Space hotkey).
pub fn send_toggle() -> bool {
    false
}
