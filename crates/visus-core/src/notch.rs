use crate::{CalEvent, Controls, Action, Command, Event, FOCUS, Hud, Lyrics, MediaInfo, Outcome, Palette, Settings, Spring, Stats, Timer, Todo, UpdateStatus, Weather, parse_task};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Music,
    Control,
    Timer,
    Stats,
    Shelf,
    Todo,
    Notes,
    Clipboard,
    Windows,
    Calendar,
    Weather,
}

impl Page {
    pub fn title(self) -> &'static str {
        match self {
            Page::Music => "Now playing",
            Page::Control => "Controls",
            Page::Timer => "Focus timer",
            Page::Stats => "System",
            Page::Shelf => "File shelf",
            Page::Todo => "To-do",
            Page::Notes => "Notes",
            Page::Clipboard => "Clipboard",
            Page::Windows => "Windows",
            Page::Calendar => "Calendar",
            Page::Weather => "Weather",
        }
    }

    pub const ALL: [Page; 11] = [Page::Music, Page::Control, Page::Windows, Page::Timer, Page::Stats, Page::Shelf, Page::Todo, Page::Notes, Page::Clipboard, Page::Calendar, Page::Weather];
}

/// Windows shown per page of the Windows grid.
pub const WIN_GRID: usize = 6;

/// Short-lived expressions the awake eye cycles through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeMood {
    None,
    Scan,
    Surprised,
    Squint,
    Heart,
    /// Robot only: the visor flickers and jitters.
    Glitch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeState {
    Hidden,
    Awake,
    Asleep,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alert {
    Charging { percent: u8 },
    Unplugged { percent: u8 },
    Headphones,
    CapsLock(bool),
    Volume { level: f32, muted: bool },
    TimerDone,
    TodoDue,
    Online,
    Offline,
    Language([u8; 2]),
    Meeting { mins: u16 },
    DownloadDone,
    Controller(bool),
    UpdateAvailable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// Current animated geometry, relative to the window.
#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub rect: Rect,
    pub bottom_radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Closed,
    Peek(Alert),
    Open,
}

const MINIMAL: (f32, f32) = (190.0, 6.0);
/// Extra closed-pill width while the music visualizer is shown.
const VISUALIZER_EXTRA: f32 = 76.0;
/// Layout of the in-game HUD: side padding and width of each reading.
pub const GAME_PAD: f32 = 12.0;
pub const GAME_HIST: usize = 40;
pub const GAME_ITEM_W: f32 = 124.0;
/// The card layout: width, header (graph) height and height of each row.
pub const GAME_CARD_W: f32 = 270.0;
pub const GAME_CARD_HEAD: f32 = 106.0;
pub const GAME_CARD_ROW: f32 = 38.0;
/// The idle pill drops the empty visualizer slot and tightens around battery and clock.
const IDLE_TRIM: f32 = 32.0;
const MOOD_PERIOD: f32 = 16.0;
const MOOD_START: f32 = 11.0;
const MOOD_LEN: f32 = 3.2;
pub const VIZ_BARS: usize = 9;
const DOWNLOAD_EXTRA: f32 = 84.0;
const EYE_PILL_W: f32 = 104.0;
const CONTROLS_REFRESH: Duration = Duration::from_millis(400);
const VISUALIZER_REFRESH: Duration = Duration::from_millis(33);
const PEEK: (f32, f32) = (500.0, 92.0);
const OPEN: (f32, f32) = (744.0, 248.0);
const PEEK_TIME: Duration = Duration::from_millis(2800);
const HOVER_CLOSE_DELAY: Duration = Duration::from_millis(250);
const VINYL_DEG_PER_SEC: f32 = 110.0;
const WAVE_REFRESH: Duration = Duration::from_millis(66);
const PROGRESS_REFRESH: Duration = Duration::from_millis(200);
const STATS_REFRESH: Duration = Duration::from_secs(1);
const LYRIC_REFRESH: Duration = Duration::from_millis(250);
const EYE_FOLLOW: Duration = Duration::from_millis(50);
const EYE_FADE_IN: f32 = 0.35;
const WANDER_EVERY: f32 = 2.4;

pub struct Notch {
    window_w: f32,
    mode: Mode,
    pub page: Page,
    w: Spring,
    h: Spring,
    r: Spring,
    peek_left: Duration,
    leave_left: Option<Duration>,
    media: MediaInfo,
    media_at: Instant,
    on_ac: Option<bool>,
    battery: Option<(bool, u8)>,
    hidden: bool,
    fullscreen: bool,
    game: bool,
    game_stats: crate::GameStats,
    game_hist: Vec<crate::GameStats>,
    timer: Timer,
    settings: Settings,
    stats: Stats,
    lyrics: Lyrics,
    lyrics_open: bool,
    todos: Vec<Todo>,
    shelf: Vec<String>,
    location_input: Option<String>,
    location_note: Option<String>,
    shelf_dirty: bool,
    weather: Option<Weather>,
    update: UpdateStatus,
    online: Option<bool>,
    todo_input: Option<String>,
    todos_dirty: bool,
    palette: Palette,
    palette_open: bool,
    palette_anim: Spring,
    settings_open: bool,
    settings_surface: bool,
    settings_anim: Spring,
    folder_input: Option<String>,
    dirty: bool,
    last_activity: Instant,
    look_x: Spring,
    look_y: Spring,
    tracking: bool,
    last_motion: Instant,
    controls: Controls,
    /// 0 = none, 1 = volume, 2 = brightness.
    slider_drag: u8,
    viz_level: f32,
    viz_live: bool,
    viz_bars: [f32; VIZ_BARS],
    viz_phase: f32,
    alert_at: Option<Instant>,
    notes: Vec<String>,
    notes_dirty: bool,
    clips: Vec<String>,
    events: Vec<CalEvent>,
    alerted: Vec<i64>,
    alert_event: Option<CalEvent>,
    last_meeting_check: Instant,
    window_request: bool,
    hour: u8,
    art_fallback: Option<(String, std::sync::Arc<crate::Art>)>,
    art_request: Option<(String, String)>,
    art_asked: String,
    art_pending: Option<(String, Instant)>,
    theme: [f32; 3],
    vinyl_angle: f32,
    vinyl_speed: f32,
    theme_cache: std::cell::Cell<Option<(u64, [u8; 3])>>,
    download: Option<u64>,
    alert_text: Option<String>,
    windows: Vec<(isize, String)>,
    windows_asked: Instant,
    win_page: usize,
    pointer: Option<(f32, f32)>,
    toast: Option<(String, Instant)>,
    drop_hover: bool,
    controller: Option<bool>,
}

impl Notch {
    pub fn new(window_w: f32) -> Self {
        let mut n = Self {
            window_w,
            mode: Mode::Closed,
            page: Page::Music,
            w: Spring::new(Settings::default().closed_width),
            h: Spring::new(Settings::default().closed_height),
            r: Spring::new(16.0),
            peek_left: Duration::ZERO,
            leave_left: None,
            media: MediaInfo::default(),
            media_at: Instant::now(),
            on_ac: None,
            battery: None,
            hidden: false,
            fullscreen: false,
            game: false,
            game_stats: crate::GameStats::default(),
            game_hist: Vec::new(),
            timer: Timer::new(FOCUS),
            settings: Settings::default(),
            stats: Stats::default(),
            lyrics: Lyrics::None,
            lyrics_open: false,
            todos: Vec::new(),
            shelf: Vec::new(),
            location_input: None,
            location_note: None,
            shelf_dirty: false,
            weather: None,
            update: UpdateStatus::Idle,
            online: None,
            todo_input: None,
            todos_dirty: false,
            palette: Palette::default(),
            palette_open: false,
            palette_anim: Spring::snappy(0.0),
            settings_open: false,
            settings_surface: false,
            settings_anim: Spring::new(0.0),
            folder_input: None,
            dirty: false,
            last_activity: Instant::now(),
            look_x: Spring::new(0.0),
            look_y: Spring::new(0.0),
            tracking: false,
            last_motion: Instant::now(),
            controls: Controls::default(),
            slider_drag: 0,
            viz_level: 0.0,
            viz_live: false,
            viz_bars: [0.08; VIZ_BARS],
            viz_phase: 0.0,
            alert_at: None,
            notes: Vec::new(),
            notes_dirty: false,
            clips: Vec::new(),
            events: Vec::new(),
            alerted: Vec::new(),
            alert_event: None,
            last_meeting_check: Instant::now(),
            window_request: false,
            hour: 12,
            art_fallback: None,
            art_request: None,
            art_asked: String::new(),
            art_pending: None,
            theme: Settings::default().eye_iris.0.map(f32::from),
            vinyl_angle: 0.0,
            vinyl_speed: 0.0,
            theme_cache: std::cell::Cell::new(None),
            download: None,
            alert_text: None,
            windows: Vec::new(),
            windows_asked: Instant::now() - Duration::from_secs(60),
            win_page: 0,
            pointer: None,
            toast: None,
            drop_hover: false,
            controller: None,
        };
        n.retarget();
        n
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
            Event::SettingsChanged(settings) => self.set_settings(*settings),
            Event::SettingsPaste(_) => {}
            Event::Power { on_ac, percent } => {
                // The first report is the initial state, not a plug/unplug.
                let changed = self.on_ac.is_some_and(|prev| prev != on_ac);
                self.on_ac = Some(on_ac);
                self.battery = Some((on_ac, percent));
                if changed {
                    self.alert(if on_ac {
                        Alert::Charging { percent }
                    } else {
                        Alert::Unplugged { percent }
                    });
                }
            }
            Event::Media(m) => {
                let changed = (&m.title, &m.artist) != (&self.media.title, &self.media.artist);
                self.media = m;
                self.media_at = Instant::now();
                self.fill_missing_art();
                self.retarget();
                if changed {
                    self.lyrics = if self.track_key().is_some() { Lyrics::Loading } else { Lyrics::None };
                }
            }
            Event::Lyrics { title, artist, lines } => {
                if title == self.media.title && artist == self.media.artist {
                    self.lyrics = match lines {
                        Some(l) if !l.is_empty() => Lyrics::Ready(l),
                        _ => Lyrics::NotFound,
                    };
                }
            }
            Event::Hud(Hud::CapsLock(on)) => self.alert(Alert::CapsLock(on)),
            Event::Hud(Hud::Language(l)) => self.alert(Alert::Language(l)),
            Event::Weather(w) => self.weather = w,
            Event::AppIcon { target, art } => self.palette.set_icon(target, art),
            Event::Geocode(found) => match found {
                Some((city, lat, lon)) => {
                    self.location_note = None;
                    self.weather = None;
                    self.update_settings(|s| {
                        s.weather_city = Some(city);
                        s.weather_lat = Some(lat);
                        s.weather_lon = Some(lon);
                    });
                }
                None => self.location_note = Some("City not found".into()),
            },
            Event::Update(u) => {
                let was = matches!(self.update, UpdateStatus::Available { .. });
                let now = matches!(u, UpdateStatus::Available { .. });
                self.update = u;
                if now && !was {
                    self.alert(Alert::UpdateAvailable);
                }
            }
            Event::Network { online } => {
                // The first report is the starting state, not a change.
                let changed = self.online.is_some_and(|p| p != online);
                self.online = Some(online);
                if changed {
                    self.alert(if online { Alert::Online } else { Alert::Offline });
                }
            }
            Event::Hud(Hud::Volume { level, muted }) => self.alert(Alert::Volume { level, muted }),
            Event::Fullscreen(f) => {
                self.fullscreen = f;
                self.refresh_hidden();
            }
            Event::Game(g) => {
                if g != self.game {
                    self.game_hist.clear();
                    self.game_stats = crate::GameStats::default();
                }
                self.game = g;
                self.refresh_hidden();
                if self.game_active() {
                    self.mode = Mode::Closed;
                }
                self.retarget();
            }
            Event::GameStats(s) => {
                self.game_stats = s;
                self.game_hist.push(s);
                if self.game_hist.len() > GAME_HIST {
                    self.game_hist.remove(0);
                }
                self.retarget();
            }
            Event::Apps(apps) => self.palette.set_apps(apps),
            Event::Windows(w) => {
                self.windows = (*w).clone();
                self.palette.set_windows((*w).clone());
            }
            Event::Controller(on) => {
                // The first report is the starting state, not a change.
                let changed = self.controller.is_some_and(|p| p != on);
                self.controller = Some(on);
                if changed {
                    self.alert(Alert::Controller(on));
                }
            }
            Event::Calendar(e) => self.events = (*e).clone(),
            Event::Clipboard(t) => self.clip_add(t),
            Event::Download(d) => {
                self.download = d;
                self.retarget();
            }
            Event::DownloadDone(name) => {
                self.alert_text = Some(name);
                self.alert(Alert::DownloadDone);
            }
            Event::ArtFallback { key, art } => {
                if let Some(art) = art {
                    self.art_fallback = Some((key, art));
                    self.fill_missing_art();
                }
            }
            Event::TogglePalette => self.toggle_palette(),
        }
    }

    /// The pill shows live game stats instead of hiding while a game is in the foreground.
    pub fn game_active(&self) -> bool {
        self.game && self.settings.game_hud && !self.settings.minimal
    }

    fn refresh_hidden(&mut self) {
        self.hidden = self.fullscreen && self.settings.hide_in_fullscreen && !self.game_active();
    }

    /// Recent readings, oldest first, for the card's graph.
    pub fn game_history(&self) -> &[crate::GameStats] {
        &self.game_hist
    }

    /// Rows in the card layout: CPU, GPU when available, RAM.
    pub fn game_rows(&self) -> usize {
        2 + self.game_stats.gpu.is_some() as usize
    }

    pub fn game_stats(&self) -> crate::GameStats {
        self.game_stats
    }

    /// HUD readings the machine provides, in display order.
    pub fn game_items(&self) -> Vec<crate::GameItem> {
        use crate::GameItem::*;
        let g = &self.game_stats;
        let mut v = vec![Cpu];
        if g.cpu_temp.is_some() {
            v.push(CpuTemp);
        }
        if g.gpu.is_some() {
            v.push(Gpu);
        }
        if g.gpu_temp.is_some() {
            v.push(GpuTemp);
        }
        v.push(Ram);
        v
    }

    pub fn alert(&mut self, a: Alert) {
        if self.game_active() || self.hidden || self.mode == Mode::Open || self.palette_open || self.settings_open {
            return;
        }
        if self.settings.quiet && !matches!(a, Alert::TimerDone | Alert::TodoDue) {
            return;
        }
        self.touch();
        self.alert_at = Some(Instant::now());
        self.mode = Mode::Peek(a);
        self.peek_left = PEEK_TIME;
        self.retarget();
    }

    /// Seconds since the current alert landed while its ripple still plays.
    pub fn alert_age(&self) -> Option<f32> {
        self.alert_at.map(|t| t.elapsed().as_secs_f32()).filter(|a| *a < 0.9)
    }

    /// Uses online cover art for tracks whose player sent none, and asks the host to look one up.
    fn fill_missing_art(&mut self) {
        if self.media.art.is_some() || !self.media.has_session || self.media.title.is_empty() {
            return;
        }
        let key = format!("{}\u{1}{}", self.media.title, self.media.artist);
        if let Some((k, art)) = &self.art_fallback {
            if *k == key {
                self.media.art = Some(art.clone());
                return;
            }
        }
        if self.art_asked != key {
            // Give the player a moment to deliver its own cover before looking one up online.
            self.art_asked = key.clone();
            self.art_pending = Some((key, Instant::now()));
        }
    }

    /// (title, artist) of a track that needs cover art looked up; returned once.
    pub fn take_art_request(&mut self) -> Option<(String, String)> {
        self.art_request.take()
    }

    // ---- notes, clipboard history, calendar

    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    pub fn set_notes(&mut self, n: Vec<String>) {
        self.notes = n;
    }

    pub fn take_notes_dirty(&mut self) -> bool {
        std::mem::take(&mut self.notes_dirty)
    }

    pub fn note_add(&mut self, text: String) {
        self.notes.insert(0, text.trim().chars().take(400).collect());
        self.notes.truncate(30);
        self.notes_dirty = true;
    }

    pub fn note_remove(&mut self, i: usize) {
        if i < self.notes.len() {
            self.notes.remove(i);
            self.notes_dirty = true;
        }
    }

    pub fn clips(&self) -> &[String] {
        &self.clips
    }

    fn clip_add(&mut self, text: String) {
        let t: String = text.trim().chars().take(400).collect();
        if !self.settings.clipboard_history || t.is_empty() {
            return;
        }
        self.clips.retain(|c| *c != t);
        self.clips.insert(0, t);
        self.clips.truncate(30);
    }

    pub fn clip_remove(&mut self, i: usize) {
        if i < self.clips.len() {
            self.clips.remove(i);
        }
    }

    /// True once after the launcher opened, so the host can list open windows.
    pub fn take_window_request(&mut self) -> bool {
        std::mem::take(&mut self.window_request)
    }

    /// Hour of day 0..=23, used for the eye's mood.
    pub fn set_hour(&mut self, h: u8) {
        self.hour = h.min(23);
    }

    pub fn events(&self) -> &[CalEvent] {
        &self.events
    }

    /// The event a meeting alert is about.
    pub fn windows(&self) -> &[(isize, String)] {
        &self.windows
    }

    /// Current page of the window grid, clamped to the number of windows.
    pub fn win_page(&self) -> usize {
        self.win_page.min(self.windows.len().saturating_sub(1) / WIN_GRID)
    }

    pub fn win_step(&mut self, delta: i32) {
        let pages = self.windows.len().div_ceil(WIN_GRID).max(1) as i32;
        self.win_page = (self.win_page() as i32 + delta).rem_euclid(pages) as usize;
    }

    /// True every couple of seconds while the Windows page is open, so the host refreshes the list.
    pub fn wants_windows(&mut self) -> bool {
        if self.is_open() && self.page == Page::Windows && self.windows_asked.elapsed() > Duration::from_secs(2) {
            self.windows_asked = Instant::now();
            return true;
        }
        false
    }

    /// Pointer position in pill coordinates while it is over the window; drives hover highlights.
    pub fn set_pointer(&mut self, p: Option<(f32, f32)>) {
        self.pointer = p;
    }

    pub fn pointer(&self) -> Option<(f32, f32)> {
        self.pointer
    }

    pub fn toast(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }

    /// Short confirmation text (e.g. "Copied") for about a second and a half.
    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().filter(|(_, t)| t.elapsed() < Duration::from_millis(1500)).map(|(m, _)| m.as_str())
    }

    pub fn set_drop_hover(&mut self, on: bool) {
        self.drop_hover = on;
    }

    pub fn drop_hover(&self) -> bool {
        self.drop_hover
    }

    pub fn open_palette_with(&mut self, text: &str) {
        self.open_palette();
        self.palette.type_text(text);
    }

    pub fn download(&self) -> Option<u64> {
        self.download
    }

    pub fn alert_text(&self) -> Option<&str> {
        self.alert_text.as_deref()
    }

    pub fn alert_event(&self) -> Option<&CalEvent> {
        self.alert_event.as_ref()
    }

    fn check_meetings(&mut self) {
        if self.last_meeting_check.elapsed() < Duration::from_secs(15) {
            return;
        }
        self.last_meeting_check = Instant::now();
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let due = self.events.iter().find(|e| !e.all_day && e.start > now && e.start - now <= 600 && !self.alerted.contains(&e.start)).cloned();
        if let Some(e) = due {
            self.alerted.push(e.start);
            let mins = ((e.start - now) as u16).div_ceil(60).max(1);
            self.alert_event = Some(e);
            self.alert(Alert::Meeting { mins });
        }
    }

    pub fn alert_kind(&self) -> Option<Alert> {
        match self.mode {
            Mode::Peek(a) => Some(a),
            _ => None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.mode == Mode::Open
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Flips play/pause on screen right away; the player's own report corrects it a moment later.
    pub fn toggle_play_local(&mut self) {
        let pos = self.media_position();
        self.media.playing = !self.media.playing;
        self.media.position = pos;
        self.media_at = Instant::now();
    }

    pub fn media(&self) -> &MediaInfo {
        &self.media
    }

    /// Position extrapolated since the last report while playing.
    pub fn media_position(&self) -> Duration {
        let m = &self.media;
        let pos = if m.playing { m.position + self.media_at.elapsed() } else { m.position };
        if m.duration.is_zero() { pos } else { pos.min(m.duration) }
    }

    /// Redraw cadence needed while otherwise static, e.g. the progress bar.
    pub fn refresh_interval(&self) -> Option<Duration> {
        let playing = self.media.has_session && self.media.playing;
        let media = self.is_open() && playing;
        let closed_progress = playing && self.mode == Mode::Closed && !self.settings.minimal;
        let synced = media && self.lyrics_open && matches!(self.lyrics, Lyrics::Ready(_));
        let viz = self.visualizer_active() && self.mode == Mode::Closed;
        let base = if viz {
            Some(VISUALIZER_REFRESH)
        } else if self.download.is_some() && self.mode == Mode::Closed {
            Some(Duration::from_millis(100))
        } else if synced {
            Some(LYRIC_REFRESH)
        } else if self.is_open() && self.page == Page::Music && (playing || self.vinyl_speed > 2.0) {
            // The record spins, so the Music page redraws at about 30 fps.
            Some(Duration::from_millis(33))
        } else if (media || closed_progress) && self.media.art.is_some() {
            // The album-art backdrop drifts slowly, so keep drawing while music plays.
            Some(WAVE_REFRESH)
        } else if media || closed_progress || self.timer.is_running() {
            Some(PROGRESS_REFRESH)
        } else if self.is_open() && self.page == Page::Control {
            Some(CONTROLS_REFRESH)
        } else if self.wants_stats() {
            Some(STATS_REFRESH)
        } else {
            None
        };
        // Flip cards animate for the first half second of each minute; LED colons and analog hands tick.
        let clock = if self.hidden || self.game_active() || self.settings.minimal {
            None
        } else {
            match self.settings.clock_style {
                crate::ClockStyle::Flip => {
                    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1000, |d| d.as_millis() % 60_000);
                    (ms < 500).then_some(Duration::from_millis(16))
                }
                crate::ClockStyle::Led => Some(Duration::from_millis(500)),
                crate::ClockStyle::Classic => None,
            }
        };
        let base = match (base, clock) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let base = match (base, self.toast_text().map(|_| Duration::from_millis(100)).or(self.art_pending.as_ref().map(|_| Duration::from_millis(300)))) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        match (base, self.eye_wake()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }

    /// The closed pill shows an equalizer (and widens) only while music plays.
    pub fn visualizer_active(&self) -> bool {
        !self.hidden && !self.game_active() && !self.settings.minimal && self.settings.visualizer && self.media.has_session && self.media.playing
    }

    /// True while the closed pill shows the visualizer, i.e. the host should sample audio.
    pub fn wants_audio(&self) -> bool {
        self.visualizer_active() && self.mode == Mode::Closed && self.download.is_none()
    }

    /// Feeds the latest output peak (0..1); `None` when the OS cannot provide one.
    pub fn set_audio_level(&mut self, raw: Option<f32>) {
        let Some(r) = raw else {
            self.viz_live = false;
            return;
        };
        self.viz_live = true;
        let t = (r.clamp(0.0, 1.0).sqrt() * 1.25).min(1.0);
        self.viz_level = if t > self.viz_level { t } else { self.viz_level * 0.8 + t * 0.2 };
    }

    /// Smoothed loudness when real audio is available; `None` means animate without it.
    pub fn visualizer_level(&self) -> Option<f32> {
        self.viz_live.then_some(self.viz_level)
    }

    fn step_visualizer(&mut self, s: f32) {
        if !self.wants_audio() {
            return;
        }
        self.viz_phase += s;
        let t = self.viz_phase;
        let level = if self.viz_live { self.viz_level } else { 0.55 + 0.25 * (t * 1.3).sin() };
        let mid = (VIZ_BARS - 1) as f32 / 2.0;
        for (i, bar) in self.viz_bars.iter_mut().enumerate() {
            let f = i as f32;
            let wave = 0.5 + 0.5 * (t * (4.0 + f * 1.3) + f * 1.7).sin() * (0.6 + 0.4 * (t * (1.7 + f * 0.6) + f).sin());
            let profile = 0.55 + 0.45 * (1.0 - (f - mid).abs() / mid);
            let target = (0.08 + level * (0.25 + 0.75 * wave) * 0.92) * profile;
            let rate = if target > *bar { 18.0 } else { 7.0 };
            *bar += (target - *bar) * (1.0 - (-rate * s).exp());
        }
    }

    /// Smoothed bar heights 0..1 for the closed-pill visualizer.
    pub fn visualizer_bars(&self) -> [f32; VIZ_BARS] {
        self.viz_bars
    }

    /// Bright tint taken from the album art; white when there is none.
    pub fn accent(&self) -> [u8; 3] {
        let Some(art) = &self.media.art else { return [255, 255, 255] };
        let (mut sum, mut n) = ([0u32; 3], 0u32);
        for px in art.rgba.chunks_exact(4).step_by(7) {
            if px[3] > 200 {
                for c in 0..3 {
                    sum[c] += px[c] as u32;
                }
                n += 1;
            }
        }
        if n == 0 {
            return [255, 255, 255];
        }
        let avg = [sum[0] as f32 / n as f32, sum[1] as f32 / n as f32, sum[2] as f32 / n as f32];
        let m = avg.iter().cloned().fold(1.0, f32::max);
        // Normalise brightness, then lift towards white so it reads on black.
        avg.map(|c| ((c / m) * 255.0 * 0.8 + 255.0 * 0.2) as u8)
    }

    /// Saturated colour of the cover art, cached per image.
    fn art_theme(&self) -> Option<[u8; 3]> {
        let art = self.media.art.as_ref()?;
        if let Some((id, c)) = self.theme_cache.get() {
            if id == art.id {
                return Some(c);
            }
        }
        let (mut sum, mut n) = ([0f32; 3], 0f32);
        for px in art.rgba.chunks_exact(4).step_by(5) {
            if px[3] > 200 {
                // Weight vivid pixels so a grey border does not wash the colour out.
                let (mx, mn) = (px[..3].iter().max().copied().unwrap_or(0) as f32, px[..3].iter().min().copied().unwrap_or(0) as f32);
                let w = 0.2 + (mx - mn) / 255.0 * 2.0;
                for c in 0..3 {
                    sum[c] += px[c] as f32 * w;
                }
                n += w;
            }
        }
        if n <= 0.0 {
            return None;
        }
        let avg = sum.map(|s| s / n);
        let m = avg.iter().cloned().fold(1.0, f32::max);
        let v = avg.map(|c| c / m);
        let grey = (v[0] + v[1] + v[2]) / 3.0;
        let out = v.map(|c| ((grey + (c - grey) * 1.3).clamp(0.0, 1.0) * 235.0).max(30.0) as u8);
        self.theme_cache.set(Some((art.id, out)));
        Some(out)
    }

    fn theme_target(&self) -> [f32; 3] {
        match (self.media.has_session && self.media.playing, self.art_theme()) {
            (true, Some(c)) => c.map(f32::from),
            _ => self.settings.eye_iris.0.map(f32::from),
        }
    }

    /// Accent colour for buttons and highlights: the cover's colour while music plays, otherwise the eye blue.
    pub fn theme(&self) -> [u8; 3] {
        self.theme.map(|c| c.round().clamp(0.0, 255.0) as u8)
    }

    fn theme_settled(&self) -> bool {
        let t = self.theme_target();
        (0..3).all(|i| (t[i] - self.theme[i]).abs() < 1.5)
    }

    /// Rotation of the album record in degrees.
    pub fn vinyl_angle(&self) -> f32 {
        self.vinyl_angle
    }

    pub fn wants_controls(&self) -> bool {
        self.is_open() && self.page == Page::Control
    }

    pub fn controls(&self) -> Controls {
        self.controls
    }

    pub fn set_controls(&mut self, c: Controls) {
        let volume = if self.slider_drag == 1 { self.controls.volume } else { c.volume };
        let brightness = if self.slider_drag == 2 { self.controls.brightness } else { c.brightness };
        self.controls = Controls { volume, brightness, ..c };
    }

    pub fn set_volume_local(&mut self, v: f32) {
        self.touch();
        self.controls.volume = v.clamp(0.0, 1.0);
        self.controls.muted = false;
        self.controls.has_volume = true;
    }

    pub fn set_muted_local(&mut self, m: bool) {
        self.controls.muted = m;
    }

    pub fn set_radio_local(&mut self, r: crate::Radio, on: bool) {
        match r {
            crate::Radio::Wifi => self.controls.wifi = Some(on),
            crate::Radio::Bluetooth => self.controls.bluetooth = Some(on),
        }
    }

    pub fn begin_drag(&mut self, kind: u8) {
        self.slider_drag = kind;
    }

    pub fn end_drag(&mut self) {
        self.slider_drag = 0;
    }

    /// Which slider is being dragged: 1 volume, 2 brightness.
    pub fn slider_dragging(&self) -> u8 {
        self.slider_drag
    }

    pub fn set_brightness_local(&mut self, v: f32) {
        self.touch();
        self.controls.brightness = Some(v.clamp(0.0, 1.0));
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    /// (charging, percent) once the OS has reported a battery.
    pub fn battery(&self) -> Option<(bool, u8)> {
        self.battery
    }

    /// Size the pill is animating towards, in window-logical pixels.
    pub fn target_size(&self) -> (f32, f32) {
        (self.w.target, self.h.target)
    }

    pub fn set_timer_minutes(&mut self, minutes: u32) {
        self.touch();
        self.update_settings(|s| s.timer_minutes = minutes);
        // Also restarts a timer that was already running with the new length.
        self.timer.set_total(Duration::from_secs(self.settings.timer_minutes as u64 * 60));
    }

    /// Steps the session length: by 1 minute up to 10, then by 5.
    pub fn adjust_timer(&mut self, dir: i32) {
        let m = self.settings.timer_minutes as i32;
        let step = if m < 10 || (m == 10 && dir < 0) { 1 } else { 5 };
        let next = if dir > 0 && m < 10 && m + step > 10 { 10 } else { m + dir * step };
        self.set_timer_minutes(next.max(1) as u32);
    }

    pub fn toggle_timer(&mut self) {
        self.touch();
        self.timer.toggle();
    }

    pub fn reset_timer(&mut self) {
        self.timer.reset();
    }

    /// Marks user activity; resets the idle timer that brings up the eye.
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn set_settings(&mut self, s: Settings) {
        self.settings = s.clamped();
        if !(self.media.has_session && self.media.playing) {
            self.theme = self.settings.eye_iris.0.map(f32::from);
        }
        let total = Duration::from_secs(self.settings.timer_minutes as u64 * 60);
        if !self.timer.is_running() && self.timer.total() != total {
            self.timer.set_total(total);
        }
        self.refresh_hidden();
        self.dirty = true;
        self.retarget();
    }

    pub fn update_settings(&mut self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.settings.clone();
        f(&mut s);
        self.set_settings(s);
    }

    /// Identifies the current track for lyric lookups; None when nothing identifiable plays.
    pub fn track_key(&self) -> Option<(String, String, Duration)> {
        let m = &self.media;
        (m.has_session && !m.title.is_empty()).then(|| (m.title.clone(), m.artist.clone(), m.duration))
    }

    pub fn lyrics(&self) -> &Lyrics {
        &self.lyrics
    }

    pub fn lyrics_open(&self) -> bool {
        self.lyrics_open
    }

    pub fn toggle_lyrics(&mut self) {
        self.touch();
        self.lyrics_open = !self.lyrics_open;
    }

    pub fn settings_is_open(&self) -> bool {
        self.settings_open
    }

    pub fn settings_is_visible(&self) -> bool {
        self.settings_surface
    }

    /// 0.0 (collapsed) ..= 1.0 (expanded) for the settings morph.
    pub fn settings_progress(&self) -> f32 {
        self.settings_anim.value
    }

    pub fn open_settings(&mut self) {
        if self.hidden {
            return;
        }
        self.touch();
        self.close_palette();
        self.settings_open = true;
        self.settings_surface = true;
        self.settings_anim.target = 1.0;
        self.leave_left = None;
        self.mode = Mode::Closed;
        self.retarget();
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
        self.settings_anim.target = 0.0;
        self.location_input = None;
        self.folder_input = None;
        self.retarget();
    }

    pub fn folder_input(&self) -> Option<&str> {
        self.folder_input.as_deref()
    }

    pub fn begin_folder_input(&mut self) {
        self.touch();
        self.folder_input = Some(String::new());
    }

    pub fn toggle_app_folder(&mut self, path: &str) {
        self.update_settings(|s| match s.app_folders.iter().position(|f| f == path) {
            Some(i) => {
                s.app_folders.remove(i);
            }
            None => s.app_folders.push(path.to_string()),
        });
    }

    pub fn remove_app_folder(&mut self, i: usize) {
        if i < self.settings.app_folders.len() {
            self.update_settings(|s| {
                s.app_folders.remove(i);
            });
        }
    }

    /// Steps how long the eye waits before appearing.
    pub fn adjust_eye_delay(&mut self, dir: i32) {
        const STEPS: [u32; 10] = [5, 10, 15, 30, 45, 60, 90, 120, 300, 600];
        let cur = self.settings.idle_secs;
        let next = if dir > 0 {
            STEPS.iter().copied().find(|s| *s > cur).unwrap_or(STEPS[9])
        } else {
            STEPS.iter().rev().copied().find(|s| *s < cur).unwrap_or(STEPS[0])
        };
        self.update_settings(|s| s.idle_secs = next);
    }

    pub fn location_input(&self) -> Option<&str> {
        self.location_input.as_deref()
    }

    pub fn location_note(&self) -> Option<&str> {
        self.location_note.as_deref()
    }

    pub fn begin_location_input(&mut self) {
        self.touch();
        self.location_note = None;
        self.location_input = Some(String::new());
    }

    /// Apps whose icons should be loaded for the visible launcher results.
    pub fn take_icon_requests(&mut self) -> Vec<crate::App> {
        self.palette.take_icon_requests()
    }

    pub fn shelf(&self) -> &[String] {
        &self.shelf
    }

    pub fn set_shelf(&mut self, s: Vec<String>) {
        self.shelf = s;
    }

    pub fn take_shelf_dirty(&mut self) -> bool {
        std::mem::take(&mut self.shelf_dirty)
    }

    /// Adds a dropped file and shows the Shelf page briefly.
    pub fn shelf_add(&mut self, path: String) {
        if !self.shelf.contains(&path) {
            self.shelf.push(path);
            self.shelf_dirty = true;
        }
        self.show_page(Page::Shelf);
    }

    pub fn shelf_remove(&mut self, i: usize) {
        if i < self.shelf.len() {
            self.shelf.remove(i);
            self.shelf_dirty = true;
        }
        self.touch();
    }

    pub fn weather(&self) -> Option<Weather> {
        self.weather
    }

    /// Weather is fetched only while its page is open and a location is configured.
    pub fn wants_weather(&self) -> bool {
        self.is_open()
            && self.page == Page::Weather
            && self.settings.weather_lat.is_some()
            && self.settings.weather_lon.is_some()
    }

    pub fn update_status(&self) -> &UpdateStatus {
        &self.update
    }

    pub fn set_update_status(&mut self, s: UpdateStatus) {
        self.update = s;
    }

    /// Opens the pill on `page`; it closes by itself unless the pointer takes over.
    pub fn show_page(&mut self, page: Page) {
        self.touch();
        self.page = page;
        self.mode = Mode::Open;
        self.leave_left = Some(Duration::from_secs(4));
        self.retarget();
    }

    /// A file drag left the window without dropping: close the shelf shortly after, unless the pointer takes over.
    pub fn drag_cancelled(&mut self) {
        if self.mode == Mode::Open && self.page == Page::Shelf && self.leave_left.is_some() {
            // The short delay absorbs flicker when a drag skims the window edge.
            self.leave_left = Some(Duration::from_millis(500));
        }
    }

    pub fn todos(&self) -> &[Todo] {
        &self.todos
    }

    pub fn set_todos(&mut self, t: Vec<Todo>) {
        self.todos = t;
    }

    pub fn take_todos_dirty(&mut self) -> bool {
        std::mem::take(&mut self.todos_dirty)
    }

    pub fn todo_input(&self) -> Option<&str> {
        self.todo_input.as_deref()
    }

    pub fn begin_todo_input(&mut self) {
        self.touch();
        self.todo_input = Some(String::new());
    }

    pub fn todo_toggle(&mut self, i: usize) {
        if let Some(t) = self.todos.get_mut(i) {
            t.done = !t.done;
            self.todos_dirty = true;
        }
        self.touch();
    }

    pub fn todo_delete(&mut self, i: usize) {
        if i < self.todos.len() {
            self.todos.remove(i);
            self.todos_dirty = true;
        }
        self.touch();
    }

    /// Fires a peek for tasks whose reminder time has arrived; `minutes` is local time since midnight.
    pub fn check_due(&mut self, minutes: u16) {
        let mut fire = false;
        for t in self.todos.iter_mut().filter(|t| !t.done) {
            if t.due.is_some_and(|d| minutes >= d && minutes - d <= 1) {
                t.due = None;
                fire = true;
                self.todos_dirty = true;
            }
        }
        if fire {
            self.alert(Alert::TodoDue);
        }
    }

    /// True while typing goes to the launcher or the to-do field.
    pub fn keyboard_active(&self) -> bool {
        self.settings_open || self.palette_open || self.todo_input.is_some() || self.location_input.is_some() || self.folder_input.is_some()
    }

    pub fn key_text(&mut self, s: &str) {
        if self.palette_open {
            self.palette.type_text(s);
        } else if let Some(i) = self.todo_input.as_mut().or(self.location_input.as_mut()).or(self.folder_input.as_mut()) {
            i.extend(s.chars().filter(|c| !c.is_control()));
        }
    }

    pub fn key_backspace(&mut self) {
        if self.palette_open {
            self.palette.backspace();
        } else if let Some(i) = self.todo_input.as_mut().or(self.location_input.as_mut()).or(self.folder_input.as_mut()) {
            i.pop();
        }
    }

    pub fn key_move(&mut self, delta: i32) {
        if self.palette_open {
            self.palette.move_selection(delta);
        }
    }

    pub fn key_escape(&mut self) {
        if self.palette_open {
            self.close_palette();
        } else if self.todo_input.take().is_some() || self.location_input.take().is_some() || self.folder_input.take().is_some() {
            self.leave_left = Some(HOVER_CLOSE_DELAY);
        } else if self.settings_open {
            self.close_settings();
        }
    }

    pub fn key_enter(&mut self) -> Outcome {
        if self.palette_open {
            return self.palette_activate();
        }
        if let Some(path) = self.folder_input.take() {
            let path = path.trim().trim_matches('"').to_string();
            if !path.is_empty() && !self.settings.app_folders.contains(&path) {
                self.update_settings(|s| s.app_folders.push(path));
                return Outcome::Rescan;
            }
            return Outcome::None;
        }
        if let Some(q) = self.location_input.take() {
            self.leave_left = Some(HOVER_CLOSE_DELAY);
            if !q.trim().is_empty() {
                return Outcome::Geocode(q.trim().to_string());
            }
            return Outcome::None;
        }
        if let Some(input) = self.todo_input.take() {
            if let Some(task) = parse_task(&input) {
                self.todos.push(task);
                self.todos_dirty = true;
            }
            self.leave_left = Some(HOVER_CLOSE_DELAY);
        }
        Outcome::None
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn palette_is_open(&self) -> bool {
        self.palette_open
    }

    /// 0.0 (hidden) ..= 1.0 (fully shown); keeps drawing while it animates closed.
    pub fn palette_progress(&self) -> f32 {
        self.palette_anim.value
    }

    pub fn toggle_palette(&mut self) {
        if self.palette_open {
            self.close_palette();
        } else {
            self.open_palette();
        }
    }

    pub fn open_palette(&mut self) {
        if self.hidden {
            return;
        }
        self.close_settings();
        self.touch();
        self.palette.clear();
        self.window_request = true;
        self.palette_open = true;
        self.palette_anim.tune(700.0, 46.0);
        self.palette_anim.target = 1.0;
        // The pill returns to its small shape so the panel reads as a drop-down beneath it.
        self.leave_left = None;
        if self.mode != Mode::Closed {
            self.mode = Mode::Closed;
            self.retarget();
        }
    }

    pub fn close_palette(&mut self) {
        self.palette_open = false;
        // Slow start, then accelerating: reads as being sucked into the pill.
        self.palette_anim.tune(130.0, 23.0);
        self.palette_anim.target = 0.0;
    }

    pub fn palette_type(&mut self, s: &str) {
        self.palette.type_text(s);
    }

    pub fn palette_backspace(&mut self) {
        self.palette.backspace();
    }

    pub fn palette_select(&mut self, i: usize) {
        if i < self.palette.results.len() {
            self.palette.selected = i;
        }
    }

    pub fn palette_move(&mut self, delta: i32) {
        self.palette.move_selection(delta);
    }

    /// Runs the selected result. Commands that affect the notch apply here; the host handles the rest.
    pub fn palette_activate(&mut self) -> Outcome {
        let Some(action) = self.palette.selected_item().and_then(|i| i.action.clone()) else {
            return Outcome::None;
        };
        self.close_palette();
        match action {
            Action::Launch(t) => Outcome::Launch(t),
            Action::OpenUrl(u) => Outcome::OpenUrl(u),
            Action::FocusWindow(h) => Outcome::FocusWindow(h),
            Action::SetFileManager(cmd) => {
                self.update_settings(|s| s.file_manager = if cmd.eq_ignore_ascii_case("default") { None } else { Some(cmd) });
                self.toast("File manager updated");
                // Open the pill so the confirmation is visible.
                let page = self.page;
                self.show_page(page);
                Outcome::None
            }
            Action::AddNote(t) => {
                self.note_add(t);
                self.show_page(Page::Notes);
                Outcome::None
            }
            Action::SetCalendar(u) => {
                self.update_settings(|s| s.calendar_url = Some(u));
                self.show_page(Page::Calendar);
                Outcome::None
            }
            Action::Command(c) => match c {
                Command::Quit => Outcome::Quit,
                Command::ShowPage(p) => {
                    self.show_page(p);
                    Outcome::None
                }
                Command::OpenSettings => {
                    self.open_settings();
                    Outcome::None
                }
                Command::Screenshot => Outcome::Screenshot,
                Command::CheckUpdates => {
                    self.open_settings();
                    self.update = UpdateStatus::Checking;
                    Outcome::CheckUpdates
                }
                Command::ToggleTimer => {
                    self.toggle_timer();
                    Outcome::None
                }
                Command::ResetTimer => {
                    self.reset_timer();
                    Outcome::None
                }
                Command::ToggleMinimal => {
                    self.update_settings(|s| s.minimal = !s.minimal);
                    Outcome::None
                }
                Command::ToggleEye => {
                    self.update_settings(|s| s.mascot = !s.mascot);
                    Outcome::None
                }
            },
        }
    }

    pub fn stats(&self) -> &Stats {
        &self.stats
    }

    pub fn set_stats(&mut self, s: Stats) {
        self.stats = s;
    }

    /// The host should sample system stats only while the page that shows them is open.
    pub fn wants_stats(&self) -> bool {
        self.is_open() && self.page == Page::Stats
    }

    /// True once after state worth saving changed.
    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    fn idle_for(&self) -> Duration {
        self.last_activity.elapsed()
    }

    pub fn eye_state(&self) -> EyeState {
        if !self.settings.mascot || self.game_active() || self.hidden || self.palette_open || self.settings_open || self.mode != Mode::Closed || self.settings.minimal {
            return EyeState::Hidden;
        }
        let idle = self.idle_for();
        let wake = Duration::from_secs(self.settings.idle_secs as u64);
        if idle < wake {
            EyeState::Hidden
        } else if idle < wake * 4 || self.last_motion.elapsed() < wake * 2 {
            // Someone is still moving the mouse, so the eye keeps watching instead of dozing off.
            EyeState::Awake
        } else {
            EyeState::Asleep
        }
    }

    /// Direction the eye looks, each axis in -1..=1.
    pub fn eye_look(&self) -> (f32, f32) {
        if let (EyeMood::Scan, ph) = self.eye_mood() {
            // Quick left-right glance, like checking both sides.
            return ((ph * std::f32::consts::TAU * 1.5).sin() * 0.95 * (ph * std::f32::consts::PI).sin().sqrt(), 0.0);
        }
        (self.look_x.value, self.look_y.value)
    }

    /// Current expression and how far through it the eye is (0..1).
    pub fn eye_mood(&self) -> (EyeMood, f32) {
        if !self.settings.eye_moods || self.eye_state() != EyeState::Awake {
            return (EyeMood::None, 0.0);
        }
        let t = self.awake_secs();
        let (slot, within) = ((t / MOOD_PERIOD) as u32, t % MOOD_PERIOD);
        let phase = (within - MOOD_START) / MOOD_LEN;
        if !(0.0..1.0).contains(&phase) {
            return (EyeMood::None, 0.0);
        }
        let sauron = self.settings.eye_style == crate::EyeStyle::Sauron;
        let mood = match hash(slot + 77) % 6 {
            1 => EyeMood::Scan,
            2 => EyeMood::Surprised,
            3 => EyeMood::Squint,
            4 if sauron => EyeMood::Surprised,
            4 => EyeMood::Heart,
            5 if self.settings.eye_style == crate::EyeStyle::Robot => EyeMood::Glitch,
            _ => EyeMood::None,
        };
        (mood, phase)
    }

    /// 0..1 drowsiness over the last seconds before the eye falls asleep.
    pub fn eye_drowsy(&self) -> f32 {
        if self.eye_state() != EyeState::Awake {
            return 0.0;
        }
        let wake = Duration::from_secs(self.settings.idle_secs as u64);
        let to_sleep = (wake * 4).saturating_sub(self.idle_for()).max((wake * 2).saturating_sub(self.last_motion.elapsed()));
        1.0 - (to_sleep.as_secs_f32() / 25.0).min(1.0)
    }

    /// Pupil pulse 0..1: follows the music while something plays.
    pub fn eye_pulse(&self) -> f32 {
        if !self.visualizer_active() {
            return 0.0;
        }
        if self.viz_live { self.viz_level } else { 0.5 + 0.5 * (self.viz_phase * 9.0).sin() }
    }

    pub fn set_eye_target(&mut self, dx: f32, dy: f32) {
        self.look_x.target = dx.clamp(-1.0, 1.0);
        self.look_y.target = dy.clamp(-1.0, 1.0);
    }

    /// 1.0 = fully open. Awake eyes blink at irregular intervals (sometimes twice); sleeping eyes are a slit.
    pub fn eye_openness(&self) -> f32 {
        match self.eye_state() {
            EyeState::Hidden => 0.0,
            EyeState::Asleep => 0.12,
            EyeState::Awake => {
                let mut o = blink_at(self.awake_secs()).0;
                let (mood, ph) = self.eye_mood();
                let e = (ph * std::f32::consts::PI).sin();
                match mood {
                    EyeMood::Squint => o = o.min(1.0 - 0.5 * e),
                    EyeMood::Surprised => o *= 1.0 + 0.22 * e,
                    _ => {}
                }
                // Late at night the eye stays sleepy; in the morning it is wide awake.
                if self.hour >= 23 || self.hour < 5 {
                    o = o.min(0.72);
                } else if (6..10).contains(&self.hour) {
                    o *= 1.08;
                }
                o.min(1.0 - 0.55 * self.eye_drowsy()).max(0.0)
            }
        }
    }

    /// 0..=1 fade used when the eye first appears.
    pub fn eye_fade(&self) -> f32 {
        match self.eye_state() {
            EyeState::Hidden => 0.0,
            EyeState::Asleep => 1.0,
            EyeState::Awake => (self.awake_secs() / EYE_FADE_IN).clamp(0.0, 1.0),
        }
    }

    /// Hosts that can read the global cursor set this and feed `set_eye_target`; otherwise the eye wanders.
    /// The global cursor moved. Keeps the eye awake, and wakes a sleeping one (the clock returns).
    pub fn note_motion(&mut self) {
        if self.eye_state() == EyeState::Asleep {
            self.touch();
        }
        self.last_motion = Instant::now();
    }

    /// Pretends the last activity happened `by` earlier (previews and tests of the idle eye).
    pub fn rewind_activity(&mut self, by: Duration) {
        self.last_activity -= by;
        self.last_motion -= by;
    }

    pub fn set_cursor_tracking(&mut self, on: bool) {
        self.tracking = on;
    }

    pub fn tracks_cursor(&self) -> bool {
        self.tracking
    }

    /// Seconds the eye has been awake; drives its flicker and sweeping gaze.
    pub fn awake_time(&self) -> f32 {
        self.awake_secs()
    }

    fn awake_secs(&self) -> f32 {
        let wake = self.settings.idle_secs as f32;
        (self.idle_for().as_secs_f32() - wake).max(0.0)
    }

    /// Picks a new glance direction every couple of seconds when the cursor is unknown.
    fn wander(&mut self) {
        if self.tracking || self.eye_state() != EyeState::Awake {
            return;
        }
        let h = hash((self.awake_secs() / WANDER_EVERY) as u32 + 1000);
        let dx = ((h % 200) as f32 / 100.0 - 1.0) * 0.9;
        let dy = (((h >> 8) % 200) as f32 / 100.0 - 1.0) * 0.5;
        self.set_eye_target(dx, dy);
    }

    /// When the eye next needs a redraw (blink, glance, cursor follow or state change).
    fn eye_wake(&self) -> Option<Duration> {
        if !self.settings.mascot || self.game_active() || self.hidden || self.palette_open || self.settings_open || self.mode != Mode::Closed || self.settings.minimal {
            return None;
        }
        let idle = self.idle_for();
        let wake = Duration::from_secs(self.settings.idle_secs as u64);
        match self.eye_state() {
            EyeState::Hidden => Some(wake.saturating_sub(idle) + Duration::from_millis(20)),
            EyeState::Awake => {
                let t = self.awake_secs();
                let mut next = blink_at(t).1;
                if self.settings.eye_style == crate::EyeStyle::Sauron {
                    // Flames flicker and the gaze sweeps, so keep animating while it watches.
                    next = next.min(0.05);
                }
                if self.tracking {
                    next = next.min(EYE_FOLLOW.as_secs_f32());
                } else {
                    next = next.min(WANDER_EVERY - t % WANDER_EVERY);
                }
                // Fade-in needs frames too.
                if t < EYE_FADE_IN {
                    next = next.min(0.016);
                }
                // Expressions animate, and the next one starts at a known time.
                let within = t % MOOD_PERIOD;
                if self.eye_mood().0 != EyeMood::None {
                    next = next.min(0.033);
                } else if within < MOOD_START {
                    next = next.min(MOOD_START - within + 0.02);
                }
                if self.eye_drowsy() > 0.0 {
                    next = next.min(0.2);
                }
                let to_sleep = (wake * 4).saturating_sub(idle).max((wake * 2).saturating_sub(self.last_motion.elapsed()));
                let mut d = Duration::from_secs_f32(next.max(0.005));
                if !to_sleep.is_zero() {
                    d = d.min(to_sleep + Duration::from_millis(20));
                }
                Some(d)
            }
            // A sleeping eye only needs a slow look at the cursor to notice you coming back.
            // Floating z's drift continuously while asleep.
            EyeState::Asleep => Some(Duration::from_millis(66)),
        }
    }

    pub fn hover(&mut self, inside: bool) {
        if self.settings_surface { return; }
        if self.game_active() || self.hidden || self.palette_open || self.settings_open || self.todo_input.is_some() || self.location_input.is_some() || self.folder_input.is_some() {
            return;
        }
        if inside {
            self.touch();
            self.leave_left = None;
            if self.mode != Mode::Open {
                self.mode = Mode::Open;
                self.retarget();
            }
        } else if self.mode == Mode::Open && self.leave_left.is_none() {
            self.leave_left = Some(HOVER_CLOSE_DELAY);
        }
    }

    pub fn next_page(&mut self, delta: i32) {
        self.touch();
        let n = Page::ALL.len() as i32;
        let i = Page::ALL.iter().position(|p| *p == self.page).unwrap_or(0) as i32;
        self.page = Page::ALL[(i + delta).rem_euclid(n) as usize];
    }

    pub fn shape(&self) -> Shape {
        // Squash and stretch: the pill gets slightly shorter while it widens quickly, and taller while it narrows.
        let squash = (self.w.velocity() * 0.000012).clamp(-0.04, 0.04);
        Shape {
            rect: Rect {
                x: (self.window_w - self.w.value) / 2.0,
                y: 0.0,
                w: self.w.value,
                h: self.h.value * (1.0 - squash),
            },
            bottom_radius: self.r.value,
        }
    }

    /// True while anything needs another frame; false lets the app sleep.
    pub fn is_animating(&self) -> bool {
        !(self.w.is_settled() && self.h.is_settled() && self.r.is_settled())
            || !(self.look_x.is_settled() && self.look_y.is_settled())
            || !self.theme_settled()
            || !self.palette_anim.is_settled()
            || !self.settings_anim.is_settled()
            || self.peek_left > Duration::ZERO
            || self.leave_left.is_some()
    }

    pub fn tick(&mut self, dt: Duration) {
        self.wander();
        {
            // The record spins up and down smoothly with playback.
            let target = if self.media.has_session && self.media.playing { VINYL_DEG_PER_SEC } else { 0.0 };
            let s = dt.as_secs_f32();
            self.vinyl_speed += (target - self.vinyl_speed) * (1.0 - (-2.5 * s).exp());
            self.vinyl_angle = (self.vinyl_angle + self.vinyl_speed * s) % 360.0;
        }
        {
            let (t, k) = (self.theme_target(), 1.0 - (-5.0 * dt.as_secs_f32()).exp());
            for i in 0..3 {
                self.theme[i] += (t[i] - self.theme[i]) * k;
            }
        }
        if let Some((k, at)) = &self.art_pending {
            if at.elapsed() > Duration::from_millis(1200) {
                let current = format!("{}\u{1}{}", self.media.title, self.media.artist);
                if self.media.art.is_none() && *k == current {
                    self.art_request = Some((self.media.title.clone(), self.media.artist.clone()));
                }
                self.art_pending = None;
            }
        }
        self.check_meetings();
        if self.mode == Mode::Closed {
            self.retarget();
        }
        self.step_visualizer(dt.as_secs_f32());
        if self.timer.poll_finished() {
            self.alert(Alert::TimerDone);
        }
        if self.peek_left > Duration::ZERO {
            self.peek_left = self.peek_left.saturating_sub(dt);
            if self.peek_left.is_zero() && matches!(self.mode, Mode::Peek(_)) {
                self.mode = Mode::Closed;
                self.retarget();
            }
        }
        if let Some(left) = self.leave_left {
            let left = left.saturating_sub(dt);
            if left.is_zero() {
                self.leave_left = None;
                self.mode = Mode::Closed;
                self.retarget();
            } else {
                self.leave_left = Some(left);
            }
        }
        let s = dt.as_secs_f32();
        self.w.step(s);
        self.h.step(s);
        self.r.step(s);
        self.look_x.step(s);
        self.look_y.step(s);
        self.palette_anim.step(s);
        self.settings_anim.step(s);
        if !self.settings_open && self.settings_anim.is_settled() && self.w.is_settled() && self.h.is_settled() && self.r.is_settled() {
            self.settings_surface = false;
        }
    }

    fn retarget(&mut self) {
        if self.settings_open || self.settings_anim.value > 0.001 {
            let progress = self.settings_anim.value.clamp(0.0, 1.0);
            let width = (progress / 0.55).clamp(0.0, 1.0);
            let height = ((progress - 0.15) / 0.85).clamp(0.0, 1.0);
            let closed_width = (self.settings.closed_width - IDLE_TRIM).max(140.0);
            self.w.target = closed_width + (860.0 - closed_width) * width;
            self.h.target = self.settings.closed_height + (720.0 - self.settings.closed_height) * height;
            self.r.target = 14.0 + 14.0 * height;
            return;
        }
        let ((w, h), r) = match self.mode {
            Mode::Closed if self.game_active() => match self.settings.game_style {
                crate::GameStyle::Card => ((GAME_CARD_W, GAME_CARD_HEAD + GAME_CARD_ROW * self.game_rows() as f32 + 8.0), 22.0),
                crate::GameStyle::Compact => ((350.0, 40.0), 20.0),
                _ => ((GAME_PAD * 2.0 + GAME_ITEM_W * self.game_items().len() as f32, self.settings.closed_height), self.settings.closed_height * 0.5),
            },
            Mode::Closed if self.settings.minimal => (MINIMAL, 3.0),
            Mode::Closed => {
                let clock_extra = (self.settings.clock_size - 20.0).max(0.0)
                    * if self.settings.clock_style == crate::ClockStyle::Flip { 3.8 } else { 4.5 };
                let clock_height = self.settings.clock_size
                    * if self.settings.clock_style == crate::ClockStyle::Flip { 1.3 } else { 1.0 } + 8.0;
                // While the idle eye shows, the pill tightens around it; hovering grows it again.
                let (w, h) = if self.eye_state() != EyeState::Hidden {
                    (EYE_PILL_W, self.settings.closed_height - 4.0)
                } else {
                    ((self.settings.closed_width + if self.download.is_some() { DOWNLOAD_EXTRA } else if self.visualizer_active() { VISUALIZER_EXTRA } else { -IDLE_TRIM }).max(140.0) + clock_extra, self.settings.closed_height.max(clock_height))
                };
                ((w, h), h * 0.42)
            }
            Mode::Peek(_) => (PEEK, 26.0),
            Mode::Open => (OPEN, 44.0),
        };
        self.w.target = w;
        self.h.target = h;
        self.r.target = r;
    }
}

fn hash(k: u32) -> u32 {
    let mut x = k.wrapping_mul(0x9E37_79B1) ^ 0x85EB_CA6B;
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^ (x >> 12)
}

/// Eyelid openness `t` seconds into the awake period, and seconds until it next needs a redraw.
/// Intervals between blinks vary from 2.6 to 5.5 s; one in four is a double blink.
fn blink_at(t: f32) -> (f32, f32) {
    let mut start = 0.0;
    for k in 0u32.. {
        let h = hash(k);
        let period = 2.6 + (h % 30) as f32 / 10.0;
        if t < start + period {
            let local = t - start;
            let windows: &[(f32, f32)] = if h % 4 == 0 {
                &[(period - 0.5, 0.17), (period - 0.2, 0.2)]
            } else {
                &[(period - 0.2, 0.2)]
            };
            for &(begin, len) in windows {
                if local < begin {
                    return (1.0, begin - local);
                }
                if local < begin + len {
                    let b = (local - begin) / len;
                    return ((1.0 - (std::f32::consts::PI * b).sin()).max(0.06), 0.016);
                }
            }
            return (1.0, (period - local).max(0.016));
        }
        start += period;
    }
    (1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settle(n: &mut Notch) {
        for _ in 0..600 {
            n.tick(Duration::from_millis(16));
        }
    }

    #[test]
    fn hover_opens_and_leave_closes() {
        let mut n = Notch::new(800.0);
        n.hover(true);
        settle(&mut n);
        assert_eq!(n.shape().rect.w, OPEN.0);
        n.hover(false);
        settle(&mut n);
        assert_eq!(n.shape().rect.w, n.settings().closed_width - IDLE_TRIM);
        assert!(!n.is_animating());
    }

    #[test]
    fn alert_expires() {
        let mut n = Notch::new(800.0);
        n.alert(Alert::Headphones);
        assert!(n.alert_kind().is_some());
        settle(&mut n);
        assert!(n.alert_kind().is_none());
    }

    #[test]
    fn pages_wrap() {
        let mut n = Notch::new(800.0);
        n.next_page(-1);
        assert_eq!(n.page, Page::Weather);
    }

    #[test]
    fn power_percentage_updates_without_charger_changes() {
        for on_ac in [false, true] {
            let mut notch = Notch::new(800.0);
            for percent in [70, 69, 20, 0, 100] {
                notch.handle(Event::Power { on_ac, percent });
                assert_eq!(notch.battery(), Some((on_ac, percent)));
                assert!(notch.alert_kind().is_none());
            }
        }
    }

    #[test]
    fn first_power_report_is_silent_then_plug_alerts() {
        let mut n = Notch::new(800.0);
        n.handle(Event::Power { on_ac: false, percent: 70 });
        assert!(n.alert_kind().is_none());
        n.handle(Event::Power { on_ac: true, percent: 70 });
        assert_eq!(n.alert_kind(), Some(Alert::Charging { percent: 70 }));
    }

    #[test]
    fn update_available_pops_up_once() {
        let mut n = Notch::new(800.0);
        n.handle(Event::Update(crate::UpdateStatus::Available { version: "v9.0.0".into() }));
        assert_eq!(n.alert_kind(), Some(Alert::UpdateAvailable));
        n.handle(Event::Update(crate::UpdateStatus::Available { version: "v9.0.0".into() }));
        assert!(matches!(n.update_status(), crate::UpdateStatus::Available { .. }));
    }

    #[test]
    fn settings_can_reverse_mid_animation_without_snapping() {
        let mut notch = Notch::new(960.0);
        settle(&mut notch);
        notch.open_settings();
        for _ in 0..10 { notch.tick(Duration::from_millis(16)); }
        let opening = notch.shape().rect;
        notch.close_settings();
        assert_eq!(notch.shape().rect, opening);
        notch.hover(true);
        assert!(!notch.is_open());
        for _ in 0..5 { notch.tick(Duration::from_millis(16)); }
        let closing = notch.shape().rect;
        notch.open_settings();
        assert_eq!(notch.shape().rect, closing);
        settle(&mut notch);
        assert_eq!(notch.shape().rect.h, 720.0);
    }

    #[test]
    fn settings_inflate_from_and_collapse_into_the_pill() {
        let mut notch = Notch::new(960.0);
        settle(&mut notch);
        let closed = notch.shape().rect;
        notch.open_settings();
        notch.tick(Duration::from_millis(32));
        let opening = notch.shape().rect;
        assert!(opening.w >= closed.w);
        assert!(opening.h < 720.0);
        settle(&mut notch);
        assert_eq!(notch.shape().rect.w, 860.0);
        assert_eq!(notch.shape().rect.h, 720.0);
        notch.close_settings();
        assert!(notch.is_animating());
        settle(&mut notch);
        assert_eq!(notch.shape().rect.w, closed.w);
        assert_eq!(notch.shape().rect.h, closed.h);
        assert!(!notch.is_animating());
        assert!(!notch.settings_is_visible());
    }

    #[test]
    fn larger_clocks_keep_the_closed_pill_content_inside() {
        let mut notch = Notch::new(800.0);
        settle(&mut notch);
        let original = notch.shape().rect;
        notch.handle(Event::SettingsChanged(Box::new(Settings { clock_size: 32.0, clock_style: crate::ClockStyle::Flip, ..Default::default() })));
        settle(&mut notch);
        let enlarged = notch.shape().rect;
        assert!(enlarged.w > original.w);
        assert!(enlarged.h >= 32.0 * 1.3 + 8.0);
        assert_eq!(notch.settings().scale, 1.0);
    }

    #[test]
    fn compact_game_hud_has_fixed_slim_dimensions() {
        let mut notch = Notch::new(800.0);
        notch.update_settings(|settings| settings.game_style = crate::GameStyle::Compact);
        notch.handle(Event::Game(true));
        settle(&mut notch);
        assert_eq!(notch.shape().rect.w, 350.0);
        assert_eq!(notch.shape().rect.h, 40.0);
        assert_eq!(crate::GameStyle::Card.next(), crate::GameStyle::Compact);
        assert_eq!(crate::GameStyle::Compact.next(), crate::GameStyle::Pills);
    }

    #[test]
    fn game_shows_hud_instead_of_hiding() {
        let mut n = Notch::new(800.0);
        n.handle(Event::Fullscreen(true));
        n.handle(Event::Game(true));
        assert!(n.game_active() && !n.is_hidden());
        n.handle(Event::GameStats(crate::GameStats { cpu: 0.5, gpu: Some(0.7), gpu_temp: Some(60.0), ..Default::default() }));
        assert_eq!(n.game_items().len(), 4);
        n.update_settings(|s| s.game_hud = false);
        assert!(n.is_hidden());
        n.handle(Event::Game(false));
        n.handle(Event::Fullscreen(false));
        assert!(!n.is_hidden() && !n.game_active());
    }

    #[test]
    fn fullscreen_suppresses_alerts() {
        let mut n = Notch::new(800.0);
        n.handle(Event::Fullscreen(true));
        n.alert(Alert::Headphones);
        assert!(n.alert_kind().is_none());
        assert!(n.is_hidden());
    }

    #[test]
    fn blink_closes_then_reopens() {
        // Sample a long stretch: the lid must close at some point and always end open between blinks.
        let mut min = 1.0f32;
        let mut t = 0.0;
        while t < 30.0 {
            min = min.min(blink_at(t).0);
            t += 0.01;
        }
        assert!(min < 0.1);
        assert_eq!(blink_at(0.5).0, 1.0);
    }

    #[test]
    fn quiet_mode_hides_alerts_but_not_timers() {
        let mut n = Notch::new(800.0);
        n.update_settings(|s| s.quiet = true);
        n.alert(Alert::Headphones);
        assert!(n.alert_kind().is_none());
        n.alert(Alert::TimerDone);
        assert_eq!(n.alert_kind(), Some(Alert::TimerDone));
    }

    #[test]
    fn dragged_slider_keeps_local_value() {
        let mut n = Notch::new(800.0);
        n.begin_drag(1);
        n.set_volume_local(0.8);
        n.set_controls(Controls { volume: 0.2, has_volume: true, ..Controls::default() });
        assert!((n.controls().volume - 0.8).abs() < 1e-6);
        n.end_drag();
        n.set_controls(Controls { volume: 0.2, has_volume: true, ..Controls::default() });
        assert!((n.controls().volume - 0.2).abs() < 1e-6);
    }

    #[test]
    fn closed_pill_grows_for_playback_and_shrinks_when_paused() {
        let mut notch = Notch::new(800.0);
        settle(&mut notch);
        let idle_width = notch.settings().closed_width - IDLE_TRIM;
        assert_eq!(notch.shape().rect.w, idle_width);
        notch.handle(Event::Media(MediaInfo { has_session: true, playing: true, ..MediaInfo::default() }));
        settle(&mut notch);
        assert!(notch.wants_audio());
        assert_eq!(notch.shape().rect.w, notch.settings().closed_width + VISUALIZER_EXTRA);
        notch.handle(Event::Media(MediaInfo { has_session: true, playing: false, ..MediaInfo::default() }));
        settle(&mut notch);
        assert!(!notch.wants_audio());
        assert_eq!(notch.shape().rect.w, idle_width);
        notch.handle(Event::Media(MediaInfo::default()));
        settle(&mut notch);
        assert_eq!(notch.shape().rect.w, idle_width);
        notch.update_settings(|settings| settings.closed_width = 140.0);
        settle(&mut notch);
        assert_eq!(notch.shape().rect.w, 140.0);
    }

    #[test]
    fn visualizer_needs_playing_music() {
        let mut n = Notch::new(800.0);
        assert!(!n.visualizer_active());
        n.handle(Event::Media(MediaInfo { has_session: true, playing: true, ..MediaInfo::default() }));
        assert!(n.visualizer_active());
        n.update_settings(|s| s.visualizer = false);
        assert!(!n.visualizer_active());
    }

    #[test]
    fn eye_cycles_through_moods() {
        let mut seen = std::collections::HashSet::new();
        for slot in 0..30u32 {
            let mut n = Notch::new(800.0);
            let wake = n.settings().idle_secs as f32;
            n.rewind_activity(Duration::from_secs_f32(wake + slot as f32 * MOOD_PERIOD + MOOD_START + MOOD_LEN / 2.0));
            let (m, _) = n.eye_mood();
            seen.insert(format!("{m:?}"));
        }
        assert!(seen.len() >= 4, "{seen:?}");
    }
}
