//! Local persistence: `settings.json` in the config dir, `data.json` (water tally) in the data dir.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use visus_core::{Notch, Settings, Water};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct Data {
    water: Water,
}

fn dir(base: Option<PathBuf>) -> Option<PathBuf> {
    base.map(|d| d.join("visus"))
}

pub fn settings_path() -> Option<PathBuf> {
    dir(dirs::config_dir()).map(|d| d.join("settings.json"))
}

fn data_path() -> Option<PathBuf> {
    dir(dirs::data_dir()).map(|d| d.join("data.json"))
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

fn write(path: Option<PathBuf>, json: String) {
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Write-then-rename so a crash never leaves a half-written file.
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn save(n: &Notch) {
    if let Ok(j) = serde_json::to_string_pretty(n.settings()) {
        write(settings_path(), j);
    }
    if let Ok(j) = serde_json::to_string_pretty(&Data { water: n.water().clone() }) {
        write(data_path(), j);
    }
}

/// Loads saved state; unreadable files fall back to defaults. Writes the files so they can be hand-edited.
pub fn load_into(n: &mut Notch) {
    let settings = settings_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default();
    let data = data_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Data>(&s).ok())
        .unwrap_or_default();
    n.set_settings(settings);
    n.set_water(data.water);
    n.set_today(&today());
    n.take_dirty();
    save(n);
}

/// Rolls the water tally over at midnight and saves anything that changed.
pub fn sync(n: &mut Notch) {
    n.set_today(&today());
    if n.take_dirty() {
        save(n);
    }
}
