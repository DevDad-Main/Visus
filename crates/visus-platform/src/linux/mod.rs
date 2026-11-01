//! Linux integration over D-Bus: MPRIS for media, UPower for the charger.

mod autostart;
mod mpris;
mod power;

pub use autostart::{autostart_enabled, set_autostart};
pub use mpris::media_command;

use crate::Emit;

/// Starts all background event sources. Call once.
pub fn start(emit: Emit) {
    mpris::start(emit.clone());
    power::start(emit);
}

pub fn prepare_window(_hwnd: isize) {}

pub fn set_window_region(_hwnd: isize, _x: i32, _y: i32, _w: i32, _h: i32, _radius: i32) {}

/// The compositor hides the global cursor; hosts that can query it do so themselves.
pub fn cursor_pos() -> Option<(i32, i32)> {
    None
}
