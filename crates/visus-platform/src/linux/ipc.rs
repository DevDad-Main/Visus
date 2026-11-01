//! Tiny local socket so a window-manager keybinding can open the launcher:
//! `bind = SUPER, space, exec, visus-app --toggle-palette`.

use std::io::Read;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

use visus_core::Event;

use crate::Emit;

fn socket_path() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("visus.sock"))
}

pub fn listen(emit: Emit) {
    let Some(path) = socket_path() else { return };
    // A live instance answers; only a stale file is removed.
    if UnixStream::connect(&path).is_ok() {
        return;
    }
    let _ = std::fs::remove_file(&path);
    let Ok(listener) = UnixListener::bind(&path) else { return };
    std::thread::Builder::new()
        .name("visus-ipc".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut buf = [0u8; 16];
                let _ = (&stream).read(&mut buf);
                emit(Event::TogglePalette);
            }
        })
        .expect("spawn ipc thread");
}

/// Asks the running instance to toggle the launcher; false if none is running.
pub fn send_toggle() -> bool {
    use std::io::Write;
    let Some(path) = socket_path() else { return false };
    UnixStream::connect(path).and_then(|mut s| s.write_all(b"toggle")).is_ok()
}
