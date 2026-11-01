//! Local persistence: `settings.json` in the platform config dir.

use std::fs;
use std::path::PathBuf;

use visus_core::{Notch, Settings};

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("visus").join("settings.json"))
}

fn save(n: &Notch) {
    let (Some(path), Ok(json)) = (settings_path(), serde_json::to_string_pretty(n.settings())) else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Write-then-rename so a crash never leaves a half-written file.
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

/// Loads saved settings; an unreadable file falls back to defaults. Writes the file so it can be hand-edited.
pub fn load_into(n: &mut Notch) {
    // Earlier builds kept a water tally here; it is no longer used.
    if let Some(old) = dirs::data_dir().map(|d| d.join("visus").join("data.json")) {
        let _ = fs::remove_file(old);
    }
    let settings = settings_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default();
    n.set_settings(settings);
    n.take_dirty();
    save(n);
}

/// Saves settings if they changed.
pub fn sync(n: &mut Notch) {
    if n.take_dirty() {
        save(n);
    }
}
