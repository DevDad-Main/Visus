//! Deciding whether the foreground app is a game.

/// Programs that go fullscreen without being games.
const NOT_GAMES: &[&str] = &[
    "chrome", "firefox", "msedge", "msedgewebview2", "brave", "opera", "vivaldi", "iexplore", "vlc", "mpv", "potplayer", "wmplayer", "mpc-hc",
    "powerpnt", "explorer", "code", "slack", "teams", "zoom", "obs64", "obs", "mstsc", "vmware", "virtualbox", "citrix", "visus", "visus-app",
    "applicationframehost", "lockapp", "searchhost", "startmenuexperiencehost", "shellexperiencehost", "taskmgr",
];

/// `path` is the foreground executable while something covers the whole screen.
pub fn is_game_path(path: &str) -> bool {
    let p = path.to_lowercase().replace('\\', "/");
    let stem = p.rsplit('/').next().unwrap_or("").trim_end_matches(".exe");
    !NOT_GAMES.contains(&stem)
}

/// Window classes (Hyprland) that belong to games or game runtimes.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn is_game_class(class: &str) -> bool {
    let c = class.to_lowercase();
    c.starts_with("steam_app_") || c.starts_with("gamescope") || c.ends_with(".exe") || ["lutris", "heroic", "minecraft", "retroarch", "cs2", "dota2", "wine", "cemu", "yuzu", "ryujinx", "rpcs3", "dolphin", "pcsx2", "duckstation", "ppsspp", "citra", "xemu", "mgba", "melonds", "flycast"].iter().any(|k| c.contains(k))
}

/// Polls Hyprland for a fullscreen game window.
#[cfg(target_os = "linux")]
pub fn watch(emit: crate::Emit) {
    std::thread::spawn(move || {
        let mut last = false;
        loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            let Ok(out) = std::process::Command::new("hyprctl").args(["activewindow", "-j"]).output() else { return };
            let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
            let full = v["fullscreen"].as_i64().map_or_else(|| v["fullscreen"].as_bool().unwrap_or(false), |n| n > 0);
            let game = full && v["class"].as_str().is_some_and(is_game_class);
            if game != last {
                last = game;
                emit(visus_core::Event::Game(game));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_games_not_browsers() {
        assert!(is_game_path(r"C:\Program Files (x86)\Steam\steamapps\common\Hades\Hades.exe"));
        assert!(is_game_path(r"D:\Games\thing.exe"));
        assert!(!is_game_path(r"C:\Program Files\Google\Chrome\chrome.exe"));
        assert!(!is_game_path(r"C:\Windows\explorer.exe"));
        assert!(is_game_class("steam_app_1245620"));
        assert!(is_game_class("Cemu"));
        assert!(!is_game_class("firefox"));
    }
}
