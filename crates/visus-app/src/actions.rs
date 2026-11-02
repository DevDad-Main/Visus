use visus_core::Outcome;
use visus_platform::Emit;

use crate::updater;

/// Asks the platform for icons of launcher results that do not have one yet.
pub fn request_icons(notch: &mut visus_core::Notch, emit: Option<&Emit>) {
    let Some(emit) = emit else { return };
    for app in notch.take_icon_requests() {
        visus_platform::request_icon(app, emit.clone());
    }
}

/// Per-frame housekeeping: icon requests, audio level for the visualizer, quick-control state.
pub fn pump(notch: &mut visus_core::Notch, emit: Option<&Emit>) {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    static LAST_REFRESH: Mutex<Option<Instant>> = Mutex::new(None);
    static LAST_FOLDERS: Mutex<Option<Vec<String>>> = Mutex::new(None);
    if let Some(emit) = emit {
        let mut previous = LAST_FOLDERS.lock().unwrap();
        let folders = &notch.settings().app_folders;
        if previous.as_ref().is_some_and(|previous| previous != folders) {
            visus_platform::set_app_folders(folders.clone());
            visus_platform::rescan_apps(emit.clone());
        }
        *previous = Some(folders.clone());
    }

    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(visus_platform::refresh_controls);
    static DL: std::sync::Once = std::sync::Once::new();
    if let Some(e) = emit {
        DL.call_once(|| {
            crate::downloads::start(e.clone());
            #[cfg(any(windows, target_os = "linux"))]
            visus_platform::start_pad(e.clone());
        });
    }
    request_icons(notch, emit);
    if let Some(e) = emit {
        static STARTED: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        static LAST_CHECK: Mutex<Option<Instant>> = Mutex::new(None);
        let started = *STARTED.get_or_init(Instant::now);
        let mut last_check = LAST_CHECK.lock().unwrap();
        if notch.settings().auto_update && started.elapsed() > Duration::from_secs(25) && last_check.is_none_or(|t| t.elapsed() > Duration::from_secs(12 * 3600)) {
            *last_check = Some(Instant::now());
            updater::check_silent(e.clone());
        }
    }
    if let Some(e) = emit {
        crate::game::set_active(notch.game_active(), e);
    }
    notch.set_hour(chrono::Timelike::hour(&chrono::Local::now()) as u8);
    if let (true, Some(e)) = (notch.take_window_request() || notch.wants_windows(), emit) {
        let e = e.clone();
        std::thread::spawn(move || e(visus_core::Event::Windows(std::sync::Arc::new(visus_platform::list_windows()))));
    }
    if let (Some((t, a)), Some(e)) = (notch.take_art_request(), emit) {
        crate::artfetch::fetch(t, a, e.clone());
    }
    crate::calendar::tick(notch, emit);
    let audio = notch.wants_audio();
    visus_platform::set_audio_sampling(audio);
    if audio {
        notch.set_audio_level(visus_platform::audio_level());
    }
    let mut last = LAST_REFRESH.lock().unwrap();
    if notch.wants_controls() {
        if last.is_none_or(|t| t.elapsed() > Duration::from_millis(1500)) {
            visus_platform::refresh_controls();
            *last = Some(Instant::now());
        }
        notch.set_controls(visus_platform::controls());
    } else {
        *last = None;
    }
}

/// Performs what a confirmed result asks of the OS; returns true when the app should quit.
pub fn run(outcome: Outcome, emit: Option<&Emit>, folders: &[String]) -> bool {
    match outcome {
        Outcome::None => {}
        Outcome::Launch(t) => visus_platform::launch(&t),
        Outcome::OpenUrl(u) => visus_platform::open_url(&u),
        Outcome::OpenPath(p) => visus_platform::open_path(&p),
        Outcome::Screenshot => visus_platform::screenshot(),
        Outcome::CheckUpdates => {
            if let Some(e) = emit {
                updater::check(e.clone());
            }
        }
        Outcome::InstallUpdate => {
            if let Some(e) = emit {
                updater::install(e.clone());
            }
        }
        Outcome::Geocode(q) => {
            if let Some(e) = emit {
                crate::weather::geocode(q, e.clone());
            }
        }
        Outcome::Rescan => {
            visus_platform::set_app_folders(folders.to_vec());
            if let Some(e) = emit {
                visus_platform::rescan_apps(e.clone());
            }
        }
        Outcome::FocusWindow(h) => visus_platform::focus_window(h),
        Outcome::OpenFolder { path, manager } => open_folder(&path, manager.as_deref()),
        Outcome::Quit => return true,
    }
    false
}

/// Splits a command line into words, keeping "quoted phrases" together.
fn split_command(cmd: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in cmd.chars() {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Opens a folder in the configured file manager, or the system default when none is set.
fn open_folder(path: &str, manager: Option<&str>) {
    let Some(tpl) = manager.filter(|m| !m.trim().is_empty()) else {
        visus_platform::open_path(path);
        return;
    };
    let mut words = split_command(tpl);
    if words.is_empty() {
        return;
    }
    let has_slot = words.iter().any(|w| w.contains("{path}"));
    for w in &mut words {
        *w = w.replace("{path}", path);
    }
    if !has_slot {
        words.push(path.to_string());
    }
    let prog = words.remove(0);
    let _ = std::process::Command::new(prog).args(words).spawn();
}

#[cfg(test)]
mod tests {
    use super::split_command;

    #[test]
    fn splits_quoted_commands() {
        assert_eq!(split_command("\"C:\\Program Files\\Explorer++\\Explorer++.exe\" {path}"), vec!["C:\\Program Files\\Explorer++\\Explorer++.exe", "{path}"]);
        assert_eq!(split_command("foot -e yazi {path}"), vec!["foot", "-e", "yazi", "{path}"]);
    }
}
