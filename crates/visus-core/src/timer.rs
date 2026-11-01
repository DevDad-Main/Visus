use std::time::{Duration, Instant};

pub const FOCUS: Duration = Duration::from_secs(25 * 60);

/// Pomodoro countdown based on wall-clock time, so it stays correct while the app sleeps.
#[derive(Clone, Copy, Debug)]
pub struct Timer {
    total: Duration,
    banked: Duration,
    started: Option<Instant>,
}

impl Timer {
    pub fn new(total: Duration) -> Self {
        Self { total, banked: Duration::ZERO, started: None }
    }

    pub fn total(&self) -> Duration {
        self.total
    }

    pub fn is_running(&self) -> bool {
        self.started.is_some()
    }

    fn elapsed(&self) -> Duration {
        self.banked + self.started.map_or(Duration::ZERO, |s| s.elapsed())
    }

    pub fn remaining(&self) -> Duration {
        self.total.saturating_sub(self.elapsed())
    }

    /// 0.0 = just started, 1.0 = done.
    pub fn progress(&self) -> f32 {
        (self.elapsed().as_secs_f32() / self.total.as_secs_f32()).clamp(0.0, 1.0)
    }

    pub fn toggle(&mut self) {
        match self.started.take() {
            Some(s) => self.banked += s.elapsed(),
            None => {
                if self.remaining().is_zero() {
                    self.banked = Duration::ZERO;
                }
                self.started = Some(Instant::now());
            }
        }
    }

    pub fn reset(&mut self) {
        self.banked = Duration::ZERO;
        self.started = None;
    }

    /// Returns true exactly once, when a running timer reaches zero.
    pub fn poll_finished(&mut self) -> bool {
        if self.is_running() && self.remaining().is_zero() {
            self.banked = self.total;
            self.started = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_and_finish() {
        let mut t = Timer::new(Duration::from_millis(10));
        assert!(!t.is_running());
        t.toggle();
        assert!(t.is_running());
        std::thread::sleep(Duration::from_millis(20));
        assert!(t.poll_finished());
        assert!(!t.poll_finished());
        assert_eq!(t.progress(), 1.0);
        t.reset();
        assert_eq!(t.remaining(), Duration::from_millis(10));
    }
}
