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
