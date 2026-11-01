use crate::{Action, Command, Event, FOCUS, Hud, Lyrics, MediaInfo, Outcome, Palette, Settings, Spring, Stats, Timer, Todo, UpdateStatus, Weather, parse_task};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Music,
    Timer,
    Stats,
    Shelf,
    Todo,
    Calendar,
    Weather,
}

impl Page {
    pub const ALL: [Page; 7] = [Page::Music, Page::Timer, Page::Stats, Page::Shelf, Page::Todo, Page::Calendar, Page::Weather];
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
const PEEK: (f32, f32) = (500.0, 92.0);
const OPEN: (f32, f32) = (744.0, 248.0);
const PEEK_TIME: Duration = Duration::from_millis(2800);
const HOVER_CLOSE_DELAY: Duration = Duration::from_millis(250);
const PROGRESS_REFRESH: Duration = Duration::from_millis(500);
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
    settings_anim: Spring,
    folder_input: Option<String>,
    dirty: bool,
    last_activity: Instant,
    look_x: Spring,
    look_y: Spring,
    tracking: bool,
    last_motion: Instant,
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
            palette_anim: Spring::new(0.0),
            settings_open: false,
            settings_anim: Spring::new(0.0),
            folder_input: None,
            dirty: false,
            last_activity: Instant::now(),
            look_x: Spring::new(0.0),
            look_y: Spring::new(0.0),
            tracking: false,
            last_motion: Instant::now(),
        };
        n.retarget();
        n
    }

    pub fn handle(&mut self, ev: Event) {
        match ev {
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
            Event::Update(u) => self.update = u,
            Event::Network { online } => {
                // The first report is the starting state, not a change.
                let changed = self.online.is_some_and(|p| p != online);
                self.online = Some(online);
                if changed {
                    self.alert(if online { Alert::Online } else { Alert::Offline });
                }
            }
            Event::Hud(Hud::Volume { level, muted }) => self.alert(Alert::Volume { level, muted }),
            Event::Fullscreen(f) => self.hidden = f && self.settings.hide_in_fullscreen,
            Event::Apps(apps) => self.palette.set_apps(apps),
            Event::TogglePalette => self.toggle_palette(),
        }
    }

    pub fn alert(&mut self, a: Alert) {
        if self.hidden || self.mode == Mode::Open || self.palette_open || self.settings_open {
            return;
        }
        self.touch();
        self.mode = Mode::Peek(a);
        self.peek_left = PEEK_TIME;
        self.retarget();
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
        let media = self.is_open() && self.media.has_session && self.media.playing;
        let synced = media && self.lyrics_open && matches!(self.lyrics, Lyrics::Ready(_));
        let base = if synced {
            Some(LYRIC_REFRESH)
        } else if media || self.timer.is_running() {
            Some(PROGRESS_REFRESH)
        } else if self.wants_stats() {
            Some(STATS_REFRESH)
        } else {
            None
        };
        match (base, self.eye_wake()) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
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
        let total = Duration::from_secs(self.settings.timer_minutes as u64 * 60);
        if !self.timer.is_running() && self.timer.total() != total {
            self.timer.set_total(total);
        }
        if self.hidden && !self.settings.hide_in_fullscreen {
            self.hidden = false;
        }
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

    /// 0.0 (hidden) ..= 1.0 (fully shown) for the settings popup.
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
        self.settings_anim.target = 1.0;
        self.leave_left = None;
        if self.mode != Mode::Closed {
            self.mode = Mode::Closed;
            self.retarget();
        }
    }

    pub fn close_settings(&mut self) {
        self.settings_open = false;
        self.settings_anim.target = 0.0;
        self.location_input = None;
        self.folder_input = None;
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
        self.palette_open || self.todo_input.is_some() || self.location_input.is_some() || self.folder_input.is_some()
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
        self.palette_open = true;
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
        if !self.settings.mascot || self.hidden || self.palette_open || self.settings_open || self.mode != Mode::Closed || self.settings.minimal {
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
        (self.look_x.value, self.look_y.value)
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
            EyeState::Awake => blink_at(self.awake_secs()).0,
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
        if !self.settings.mascot || self.hidden || self.palette_open || self.settings_open || self.mode != Mode::Closed || self.settings.minimal {
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
                let to_sleep = (wake * 4).saturating_sub(idle).max((wake * 2).saturating_sub(self.last_motion.elapsed()));
                let mut d = Duration::from_secs_f32(next.max(0.005));
                if !to_sleep.is_zero() {
                    d = d.min(to_sleep + Duration::from_millis(20));
                }
                Some(d)
            }
            // A sleeping eye only needs a slow look at the cursor to notice you coming back.
            EyeState::Asleep => self.tracking.then_some(Duration::from_millis(500)),
        }
    }

    pub fn hover(&mut self, inside: bool) {
        if self.hidden || self.palette_open || self.settings_open || self.todo_input.is_some() || self.location_input.is_some() || self.folder_input.is_some() {
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
        Shape {
            rect: Rect {
                x: (self.window_w - self.w.value) / 2.0,
                y: 0.0,
                w: self.w.value,
                h: self.h.value,
            },
            bottom_radius: self.r.value,
        }
    }

    /// True while anything needs another frame; false lets the app sleep.
    pub fn is_animating(&self) -> bool {
        !(self.w.is_settled() && self.h.is_settled() && self.r.is_settled())
            || !(self.look_x.is_settled() && self.look_y.is_settled())
            || !self.palette_anim.is_settled()
            || !self.settings_anim.is_settled()
            || self.peek_left > Duration::ZERO
            || self.leave_left.is_some()
    }

    pub fn tick(&mut self, dt: Duration) {
        self.wander();
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
    }

    fn retarget(&mut self) {
        let ((w, h), r) = match self.mode {
            Mode::Closed if self.settings.minimal => (MINIMAL, 3.0),
            Mode::Closed => ((self.settings.closed_width, self.settings.closed_height), self.settings.closed_height * 0.42),
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
        assert_eq!(n.shape().rect.w, n.settings().closed_width);
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
    fn first_power_report_is_silent_then_plug_alerts() {
        let mut n = Notch::new(800.0);
        n.handle(Event::Power { on_ac: false, percent: 70 });
        assert!(n.alert_kind().is_none());
        n.handle(Event::Power { on_ac: true, percent: 70 });
        assert_eq!(n.alert_kind(), Some(Alert::Charging { percent: 70 }));
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
}
