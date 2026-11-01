//! Local persistence: `settings.json` in the platform config dir.

use std::fs;
use std::path::PathBuf;

use visus_core::{Notch, Settings, Todo};

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("visus").join("settings.json"))
}

fn todos_path() -> Option<PathBuf> {
    settings_path().map(|p| p.with_file_name("todos.json"))
}

fn write_atomic(path: PathBuf, json: String) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    // Write-then-rename so a crash never leaves a half-written file.
    let tmp = path.with_extension("tmp");
    if fs::write(&tmp, json).is_ok() {
        let _ = fs::rename(&tmp, &path);
    }
}

fn save_todos(n: &Notch) {
    if let (Some(path), Ok(json)) = (todos_path(), serde_json::to_string_pretty(n.todos())) {
        write_atomic(path, json);
    }
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
    let todos = todos_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<Vec<Todo>>(&s).ok())
        .unwrap_or_default();
    n.set_todos(todos);
    n.take_dirty();
    save(n);
}

/// Fires due reminders and saves whatever changed.
pub fn sync(n: &mut Notch) {
    use chrono::Timelike;
    let now = chrono::Local::now();
    n.check_due((now.hour() * 60 + now.minute()) as u16);
    if n.take_dirty() {
        save(n);
    }
    if n.take_todos_dirty() {
        save_todos(n);
    }
}
