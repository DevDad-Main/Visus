//! Finds and starts the per-user Inno Setup uninstaller.

use std::path::PathBuf;
use std::process::Command;

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

fn from_registry(root: RegKey, uninstall_key: &str) -> Option<PathBuf> {
    let key = root.open_subkey(uninstall_key).ok()?;
    for name in key.enum_keys().flatten() {
        let Ok(entry) = key.open_subkey(&name) else { continue };
        let Ok(display) = entry.get_value::<String, _>("DisplayName") else { continue };
        if display != "Visus" {
            continue;
        }
        let Ok(command) = entry.get_value::<String, _>("UninstallString") else { continue };
        let command = command.trim();
        let path = match command.strip_prefix('"') {
            Some(rest) => rest.split('"').next().unwrap_or(rest),
            None => command.split_whitespace().next().unwrap_or(command),
        };
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn uninstaller() -> Option<PathBuf> {
    // Prefer the uninstaller next to the running copy, then fall back to the registry so an
    // update that moved the app (for example out of Program Files) still uninstalls cleanly.
    if let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) {
        let local = dir.join("unins000.exe");
        if local.is_file() {
            return Some(local);
        }
    }
    const UNINSTALL: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
    from_registry(RegKey::predef(HKEY_CURRENT_USER), UNINSTALL)
        .or_else(|| from_registry(RegKey::predef(HKEY_LOCAL_MACHINE), UNINSTALL))
}

/// Starts the uninstaller. Returns false when Visus was not installed with the installer.
pub fn uninstall() -> bool {
    let Some(path) = uninstaller() else { return false };
    Command::new(path).spawn().is_ok()
}
