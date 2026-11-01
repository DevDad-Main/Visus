use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Closed pill collapses to a thin line (for thick bezels).
    pub minimal: bool,
    pub hide_in_fullscreen: bool,
    /// Show the idle eye when nothing has happened for a while.
    pub mascot: bool,
    pub idle_secs: u32,
    pub water_goal_ml: u32,
    pub water_cup_ml: u32,
    /// Overall size multiplier for the pill and its content.
    pub scale: f32,
    /// Closed pill size in logical pixels, before `scale`.
    pub closed_width: f32,
    pub closed_height: f32,
}

impl Settings {
    pub const SCALE_RANGE: (f32, f32) = (0.6, 1.25);

    /// Keeps hand-edited values inside what the layout can render.
    pub fn clamped(mut self) -> Self {
        self.scale = self.scale.clamp(Self::SCALE_RANGE.0, Self::SCALE_RANGE.1);
        self.closed_width = self.closed_width.clamp(140.0, 400.0);
        self.closed_height = self.closed_height.clamp(24.0, 80.0);
        self.idle_secs = self.idle_secs.clamp(5, 3600);
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            minimal: false,
            hide_in_fullscreen: true,
            mascot: true,
            idle_secs: 45,
            water_goal_ml: 1500,
            water_cup_ml: 250,
            scale: 1.0,
            closed_width: 228.0,
            closed_height: 38.0,
        }
    }
}

/// Water drunk on one calendar day; resets when the day changes.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Water {
    pub day: String,
    pub ml: u32,
}

impl Water {
    /// Starts a fresh day if `today` differs; returns true when it reset.
    pub fn roll_over(&mut self, today: &str) -> bool {
        if self.day == today {
            return false;
        }
        self.day = today.to_string();
        self.ml = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_resets_on_new_day() {
        let mut w = Water { day: "2026-11-01".into(), ml: 500 };
        assert!(!w.roll_over("2026-11-01"));
        assert_eq!(w.ml, 500);
        assert!(w.roll_over("2026-11-02"));
        assert_eq!(w.ml, 0);
    }
}
