use std::sync::Arc;
use std::time::Duration;

use crate::{App, LyricLine};

/// Square album art, straight-alpha RGBA, already scaled down for display.
#[derive(Clone, Debug)]
pub struct Art {
    pub size: u32,
    pub rgba: Vec<u8>,
    /// Content hash; lets callers detect a changed cover without comparing pixels.
    pub id: u64,
}

impl PartialEq for Art {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaInfo {
    pub has_session: bool,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position: Duration,
    pub duration: Duration,
    pub art: Option<Arc<Art>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hud {
    CapsLock(bool),
    Volume { level: f32, muted: bool },
    /// Two-letter keyboard layout code, e.g. `EN`.
    Language([u8; 2]),
}

/// Everything the OS layer can tell the notch about.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Power { on_ac: bool, percent: u8 },
    Media(MediaInfo),
    Hud(Hud),
    Fullscreen(bool),
    /// Installed applications the launcher can start.
    Apps(Arc<Vec<App>>),
    /// Global hotkey pressed.
    TogglePalette,
    Weather(Option<crate::Weather>),
    Update(crate::UpdateStatus),
    Network { online: bool },
    /// Result of a lyrics lookup for a track; `None` means nothing was found.
    Lyrics { title: String, artist: String, lines: Option<Arc<Vec<LyricLine>>> },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
}
