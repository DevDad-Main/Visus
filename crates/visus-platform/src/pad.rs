//! Reports game controllers being connected or removed.

use std::time::Duration;

use visus_core::Event;

use crate::Emit;

#[cfg(windows)]
fn connected() -> bool {
    use windows::Win32::UI::Input::XboxController::{XINPUT_STATE, XInputGetState};
    // XInput covers Xbox pads and anything exposed as one (Steam Input, DS4Windows).
    (0..4).any(|i| {
        let mut state = XINPUT_STATE::default();
        unsafe { XInputGetState(i, &mut state) == 0 }
    })
}

#[cfg(not(windows))]
fn connected() -> bool {
    (0..4).any(|i| std::path::Path::new(&format!("/dev/input/js{i}")).exists())
}

pub fn watch(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-pad".into())
        .spawn(move || {
            let mut last: Option<bool> = None;
            loop {
                let now = connected();
                if last != Some(now) {
                    last = Some(now);
                    emit(Event::Controller(now));
                }
                std::thread::sleep(Duration::from_millis(1500));
            }
        })
        .ok();
}
