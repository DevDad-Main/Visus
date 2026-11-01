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

    request_icons(notch, emit);
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
        Outcome::Quit => return true,
    }
    false
}
