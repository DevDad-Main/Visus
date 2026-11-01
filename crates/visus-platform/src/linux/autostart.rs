use std::io;
use std::path::PathBuf;

fn desktop_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("autostart").join("visus.desktop"))
}

pub fn autostart_enabled() -> bool {
    desktop_file().is_some_and(|p| p.exists())
}

pub fn set_autostart(enabled: bool) -> io::Result<()> {
    let path = desktop_file().ok_or_else(|| io::Error::other("no config directory"))?;
    if !enabled {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            r => r,
        };
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let exe = std::env::current_exe()?;
    std::fs::write(
        path,
        format!("[Desktop Entry]\nType=Application\nName=Visus\nExec=\"{}\"\nX-GNOME-Autostart-enabled=true\n", exe.display()),
    )
}
