use std::collections::HashSet;
use std::path::{Path, PathBuf};

use visus_core::App;
use windows::core::{HSTRING, PCWSTR, w};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn start_menu_dirs() -> Vec<PathBuf> {
    let rel = r"Microsoft\Windows\Start Menu\Programs";
    ["ProgramData", "APPDATA"]
        .iter()
        .filter_map(|v| std::env::var_os(v))
        .map(|base| PathBuf::from(base).join(rel))
        .collect()
}

fn collect(dir: &Path, depth: u32, out: &mut Vec<App>, seen: &mut HashSet<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < 4 {
                collect(&path, depth + 1, out, seen);
            }
            continue;
        }
        if path.extension().is_none_or(|e| !e.eq_ignore_ascii_case("lnk")) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
        let lower = name.to_lowercase();
        if lower.contains("uninstall") || !seen.insert(lower) {
            continue;
        }
        out.push(App { name, target: path.to_string_lossy().into_owned() });
    }
}

/// Start Menu shortcuts (classic desktop apps; Store apps are not listed yet).
pub fn list() -> Vec<App> {
    let (mut apps, mut seen) = (Vec::new(), HashSet::new());
    for dir in start_menu_dirs() {
        collect(&dir, 0, &mut apps, &mut seen);
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn shell_open(target: &str) {
    let file = HSTRING::from(target);
    unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(file.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

pub fn launch(target: &str) {
    shell_open(target);
}

pub fn open_path(path: &str) {
    shell_open(path);
}

/// Opens the Windows snipping overlay.
pub fn screenshot() {
    shell_open("ms-screenclip:");
}

pub fn open_url(url: &str) {
    // Only web addresses are handed to the shell.
    if url.starts_with("https://") || url.starts_with("http://") {
        shell_open(url);
    }
}
