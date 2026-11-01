use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MediaInfo {
    pub has_session: bool,
    pub title: String,
    pub artist: String,
    pub playing: bool,
    pub position: Duration,
    pub duration: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hud {
    CapsLock(bool),
    Volume { level: f32, muted: bool },
}

/// Everything the OS layer can tell the notch about.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Power { on_ac: bool, percent: u8 },
    Media(MediaInfo),
    Hud(Hud),
    Fullscreen(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Previous,
}
