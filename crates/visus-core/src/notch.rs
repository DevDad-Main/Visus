use crate::{Event, FOCUS, Hud, MediaInfo, Spring, Timer};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Music,
    Timer,
    Water,
    Shelf,
    Todo,
}

impl Page {
    pub const ALL: [Page; 5] = [Page::Music, Page::Timer, Page::Water, Page::Shelf, Page::Todo];
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Alert {
    Charging { percent: u8 },
    Unplugged { percent: u8 },
    Headphones,
    CapsLock(bool),
    Volume { level: f32, muted: bool },
    TimerDone,
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

const CLOSED: (f32, f32) = (266.0, 52.0);
const PEEK: (f32, f32) = (550.0, 106.0);
const OPEN: (f32, f32) = (744.0, 248.0);
const PEEK_TIME: Duration = Duration::from_millis(2800);
const HOVER_CLOSE_DELAY: Duration = Duration::from_millis(250);
const PROGRESS_REFRESH: Duration = Duration::from_millis(500);

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
    hidden: bool,
    timer: Timer,
}

impl Notch {
    pub fn new(window_w: f32) -> Self {
        let mut n = Self {
            window_w,
            mode: Mode::Closed,
            page: Page::Music,
            w: Spring::new(CLOSED.0),
            h: Spring::new(CLOSED.1),
            r: Spring::new(22.0),
            peek_left: Duration::ZERO,
            leave_left: None,
            media: MediaInfo::default(),
            media_at: Instant::now(),
            on_ac: None,
            hidden: false,
            timer: Timer::new(FOCUS),
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
                if changed {
                    self.alert(if on_ac {
                        Alert::Charging { percent }
                    } else {
                        Alert::Unplugged { percent }
                    });
                }
            }
            Event::Media(m) => {
                self.media = m;
                self.media_at = Instant::now();
            }
            Event::Hud(Hud::CapsLock(on)) => self.alert(Alert::CapsLock(on)),
            Event::Hud(Hud::Volume { level, muted }) => self.alert(Alert::Volume { level, muted }),
            Event::Fullscreen(f) => self.hidden = f,
        }
    }

    pub fn alert(&mut self, a: Alert) {
        if self.hidden || self.mode == Mode::Open {
            return;
        }
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
        (media || self.timer.is_running()).then_some(PROGRESS_REFRESH)
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn toggle_timer(&mut self) {
        self.timer.toggle();
    }

    pub fn reset_timer(&mut self) {
        self.timer.reset();
    }

    pub fn hover(&mut self, inside: bool) {
        if self.hidden {
            return;
        }
        if inside {
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
            || self.peek_left > Duration::ZERO
            || self.leave_left.is_some()
    }

    pub fn tick(&mut self, dt: Duration) {
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
    }

    fn retarget(&mut self) {
        let ((w, h), r) = match self.mode {
            Mode::Closed => (CLOSED, 22.0),
            Mode::Peek(_) => (PEEK, 30.0),
            Mode::Open => (OPEN, 44.0),
        };
        self.w.target = w;
        self.h.target = h;
        self.r.target = r;
    }
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
        assert_eq!(n.shape().rect.w, CLOSED.0);
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
        assert_eq!(n.page, Page::Todo);
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
}
