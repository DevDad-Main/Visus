use std::process::Command;

use visus_core::Event;

use crate::Emit;

/// Polls the Wayland clipboard once a second; stops quietly when `wl-paste` is not installed.
pub fn watch(emit: Emit) {
    std::thread::spawn(move || {
        let mut last = String::new();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            let Ok(out) = Command::new("wl-paste").arg("--no-newline").output() else { return };
            if !out.status.success() {
                continue;
            }
            if let Ok(text) = String::from_utf8(out.stdout) {
                if !text.is_empty() && text != last {
                    last = text.clone();
                    emit(Event::Clipboard(text));
                }
            }
        }
    });
}

/// Hyprland clients (address, title); empty elsewhere.
pub fn list_windows() -> Vec<(isize, String)> {
    let Ok(out) = Command::new("hyprctl").args(["clients", "-j"]).output() else { return Vec::new() };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&out.stdout) else { return Vec::new() };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let title = c["title"].as_str().filter(|t| !t.is_empty())?;
                    let addr = isize::from_str_radix(c["address"].as_str()?.trim_start_matches("0x"), 16).ok()?;
                    Some((addr, title.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn focus_window(h: isize) {
    let _ = Command::new("hyprctl").args(["dispatch", "focuswindow", &format!("address:0x{h:x}")]).output();
}
