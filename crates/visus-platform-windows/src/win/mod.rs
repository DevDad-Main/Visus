//! Windows implementation. Everything is event-driven: one hidden-window thread receives
//! power broadcasts, a low-level key hook and foreground-change hooks; media runs on WinRT events.

mod autostart;
mod events;
mod media;
mod window;

pub use autostart::{autostart_enabled, set_autostart};
pub use media::media_command;
pub use window::{cursor_pos, prepare_window, set_window_region};

use crate::Emit;

/// Starts all background event sources. Call once.
pub fn start(emit: Emit) {
    media::start(emit.clone());
    events::start(emit);
}
