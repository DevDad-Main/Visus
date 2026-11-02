//! OS integration for Visus: real implementations on Windows and Linux, inert stubs elsewhere
//! so the rest of the workspace builds and runs on any platform.

use std::sync::Arc;
use visus_core::Event;

mod art;
mod game;
pub use game::is_game_path;
pub use art::decode as decode_art;
mod clipboard;
mod pad;
mod controls;
pub use clipboard::{clipboard_text, set_clipboard};

/// Starts the game-controller watcher (Windows and Linux only).
#[cfg(any(windows, target_os = "linux"))]
pub fn start_pad(emit: Emit) {
    pad::watch(emit);
}
pub use controls::*;

/// Extra folders the launcher scans for apps; set before `start`.
pub static APP_FOLDERS: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

pub fn set_app_folders(v: Vec<String>) {
    *APP_FOLDERS.lock().unwrap() = v;
}

pub type Emit = Arc<dyn Fn(Event) + Send + Sync + 'static>;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(any(windows, target_os = "linux")))]
mod stub;
#[cfg(not(any(windows, target_os = "linux")))]
pub use stub::*;
