use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use visus_core::App;

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/applications"));
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    dirs
}

/// Parses the `[Desktop Entry]` group of a .desktop file into a launchable app.
fn parse(text: &str) -> Option<App> {
    let (mut name, mut exec) = (None, None);
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        match line.split_once('=') {
            Some(("Name", v)) if name.is_none() => name = Some(v.trim().to_string()),
            Some(("Exec", v)) => exec = Some(v.trim().to_string()),
            Some(("Type", v)) if v.trim() != "Application" => return None,
            Some(("NoDisplay" | "Hidden", v)) if v.trim() == "true" => return None,
            _ => {}
        }
    }
    // %f, %U and friends are placeholders for files/URLs we never pass.
    let exec = exec?.split_whitespace().filter(|t| !(t.len() == 2 && t.starts_with('%'))).collect::<Vec<_>>().join(" ");
    let name = name?;
    (!exec.is_empty()).then_some(App { name, target: exec })
}

pub fn list() -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for dir in search_dirs() {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "desktop") {
                continue;
            }
            let Some(app) = std::fs::read_to_string(&path).ok().and_then(|t| parse(&t)) else { continue };
            if seen.insert(app.name.to_lowercase()) {
                apps.push(app);
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn spawn_detached(mut cmd: Command) {
    use std::os::unix::process::CommandExt;
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    if let Ok(mut child) = cmd.spawn() {
        // Reap the child so it does not linger as a zombie.
        std::thread::spawn(move || {
            let _ = child.wait();
        });
    }
}

pub fn launch(target: &str) {
    let mut c = Command::new("sh");
    c.arg("-c").arg(target);
    spawn_detached(c);
}

pub fn open_path(path: &str) {
    open_url(path);
}

/// Region screenshot into ~/Pictures via grim + slurp.
pub fn screenshot() {
    let mut c = Command::new("sh");
    c.arg("-c").arg("grim -g \"$(slurp)\" \"$HOME/Pictures/visus-$(date +%Y%m%d-%H%M%S).png\"");
    spawn_detached(c);
}

pub fn open_url(url: &str) {
    let mut c = Command::new("xdg-open");
    c.arg(url);
    spawn_detached(c);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_desktop_entries() {
        let a = parse("[Desktop Entry]\nType=Application\nName=Firefox\nExec=firefox %u\n[Desktop Action x]\nName=Other\n").unwrap();
        assert_eq!(a.name, "Firefox");
        assert_eq!(a.target, "firefox");
        assert!(parse("[Desktop Entry]\nName=Hidden\nExec=x\nNoDisplay=true\n").is_none());
    }
}
