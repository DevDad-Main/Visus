use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PillStyle {
    #[default]
    Attached,
    Floating,
    Soft,
}

/// Layout of the in-game HUD.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GameStyle {
    #[default]
    Pills,
    Bars,
    Card,
    Compact,
}

impl GameStyle {
    pub fn name(self) -> &'static str {
        match self {
            GameStyle::Pills => "Pills",
            GameStyle::Bars => "Bars",
            GameStyle::Card => "Card",
            GameStyle::Compact => "Compact",
        }
    }

    pub fn next(self) -> GameStyle {
        match self {
            GameStyle::Pills => GameStyle::Bars,
            GameStyle::Bars => GameStyle::Card,
            GameStyle::Card => GameStyle::Compact,
            GameStyle::Compact => GameStyle::Pills,
        }
    }
}

/// How the clock is drawn in the pill and the open header.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClockStyle {
    #[default]
    Classic,
    Flip,
    Led,
}

impl ClockStyle {
    pub const ALL: [ClockStyle; 3] = [ClockStyle::Classic, ClockStyle::Flip, ClockStyle::Led];

    pub fn name(self) -> &'static str {
        match self {
            ClockStyle::Classic => "Classic",
            ClockStyle::Flip => "Flip",
            ClockStyle::Led => "LED",
        }
    }

    pub fn next(self) -> ClockStyle {
        let i = Self::ALL.iter().position(|s| *s == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EyeStyle {
    #[default]
    Classic,
    Sauron,
    Cat,
    Robot,
}

/// sRGB colour stored as `#rrggbb` in settings so it is easy to hand-edit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub [u8; 3]);

impl Rgb {
    pub fn parse(s: &str) -> Option<Rgb> {
        let h = s.trim().trim_start_matches('#');
        if h.len() != 6 || !h.is_ascii() {
            return None;
        }
        let v = u32::from_str_radix(h, 16).ok()?;
        Some(Rgb([(v >> 16) as u8, (v >> 8) as u8, v as u8]))
    }

    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0[0], self.0[1], self.0[2])
    }

    /// Moves `t` (0..=1) of the way towards white.
    pub fn lighten(&self, t: f32) -> [u8; 3] {
        self.0.map(|c| (c as f32 + (255.0 - c as f32) * t).round() as u8)
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    /// A malformed colour falls back to the default eye blue instead of discarding all settings.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(Rgb::parse(&s).unwrap_or(Settings::default().eye_iris))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub pill_style: PillStyle,
    /// Closed pill collapses to a thin line (for thick bezels).
    pub minimal: bool,
    /// Silences pop-up alerts (timers and reminders still show).
    pub quiet: bool,
    pub reduced_motion: bool,
    pub animation_intensity: f32,
    pub accent_mode: String,
    pub accent_color: Rgb,
    pub outer_padding: f32,
    pub hover_open_ms: u32,
    pub hover_close_ms: u32,
    pub alert_power: bool,
    pub alert_network: bool,
    pub alert_headphones: bool,
    pub alert_controller: bool,
    pub alert_downloads: bool,
    pub alert_timer: bool,
    pub alert_tasks: bool,
    pub alert_meetings: bool,
    pub alert_updates: bool,
    pub alert_controls: bool,
    /// 24-hour clock in the pill; false shows 12-hour time.
    pub clock_24h: bool,
    pub clock_style: ClockStyle,
    pub clock_size: f32,
    pub header_clock_size: f32,
    pub clock_dimmed: bool,
    /// Equalizer bars on the closed pill while music plays.
    pub visualizer: bool,
    /// Battery ring on the closed pill when the machine has a battery.
    pub show_battery: bool,
    pub battery_ring_size: f32,
    pub battery_match_clock: bool,
    pub ring_clock_gap: f32,
    /// Show network speed in bits (Kbps/Mbps) instead of bytes (KB/s, MB/s).
    pub net_bits: bool,
    /// Remember recent clipboard text (memory only, never written to disk).
    pub clipboard_history: bool,
    /// Let the idle eye change expressions now and then.
    pub eye_moods: bool,
    /// ICS link or file path whose events appear in the calendar and as meeting peeks.
    pub calendar_url: Option<String>,
    /// Command that opens a folder, with `{path}` where the folder goes (e.g. `thunar {path}`); None = the system default.
    pub file_manager: Option<String>,
    pub hide_in_fullscreen: bool,
    pub game_hud: bool,
    pub game_style: GameStyle,
    /// Look for a newer release in the background (a few seconds after start, then twice a day).
    pub auto_update: bool,
    /// Show the idle eye when nothing has happened for a while.
    pub mascot: bool,
    pub idle_secs: u32,
    /// Size multiplier for the closed pill and its content.
    pub scale: f32,
    pub open_scale: f32,
    /// Closed pill size in logical pixels, before `scale`.
    #[serde(skip)]
    pub closed_width: f32,
    #[serde(skip)]
    pub closed_height: f32,
    pub timer_minutes: u32,
    pub eye_style: EyeStyle,
    /// Weather is only fetched once a location is set (latitude/longitude in degrees).
    pub weather_lat: Option<f32>,
    pub weather_lon: Option<f32>,
    pub weather_city: Option<String>,
    /// Extra folders (any drive) searched for apps by the launcher, in addition to the Start Menu.
    pub app_folders: Vec<String>,
    pub eye_iris: Rgb,
    pub eye_sclera: Rgb,
}

impl Settings {
    pub const SCALE_RANGE: (f32, f32) = (0.6, 1.25);

    /// Keeps hand-edited values inside what the layout can render.
    pub fn clamped(mut self) -> Self {
        self.scale = self.scale.clamp(Self::SCALE_RANGE.0, Self::SCALE_RANGE.1);
        self.open_scale = if self.open_scale.is_finite() { self.open_scale.clamp(Self::SCALE_RANGE.0, Self::SCALE_RANGE.1) } else { 1.0 };
        self.clock_size = if self.clock_size.is_finite() { self.clock_size.clamp(12.0, 32.0) } else { 20.0 };
        self.header_clock_size = if self.header_clock_size.is_finite() { self.header_clock_size.clamp(16.0, 30.0) } else { 22.0 };
        self.battery_ring_size = if self.battery_ring_size.is_finite() { self.battery_ring_size.clamp(12.0, 32.0) } else { 16.0 };
        self.ring_clock_gap = if self.ring_clock_gap.is_finite() { self.ring_clock_gap.clamp(4.0, 32.0) } else { 12.0 };
        self.animation_intensity = if self.animation_intensity.is_finite() { self.animation_intensity.clamp(0.3, 1.5) } else { 1.0 };
        self.outer_padding = if self.outer_padding.is_finite() { self.outer_padding.clamp(8.0, 32.0) } else { 16.0 };
        self.hover_open_ms = self.hover_open_ms.min(1000);
        self.hover_close_ms = self.hover_close_ms.clamp(50, 2000);
        if !matches!(self.accent_mode.as_str(), "media" | "mascot" | "fixed") { self.accent_mode = "media".into(); }
        self.closed_width = self.closed_width.clamp(140.0, 400.0);
        self.closed_height = self.closed_height.clamp(24.0, 80.0);
        self.idle_secs = self.idle_secs.clamp(5, 3600);
        self.timer_minutes = self.timer_minutes.clamp(1, 180);
        self
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            pill_style: PillStyle::Attached,
            minimal: false,
            quiet: false,
            reduced_motion: false,
            animation_intensity: 1.0,
            accent_mode: "media".into(),
            accent_color: Rgb([40, 110, 230]),
            outer_padding: 16.0,
            hover_open_ms: 0,
            hover_close_ms: 250,
            alert_power: true,
            alert_network: true,
            alert_headphones: true,
            alert_controller: true,
            alert_downloads: true,
            alert_timer: true,
            alert_tasks: true,
            alert_meetings: true,
            alert_updates: true,
            alert_controls: true,
            clock_24h: true,
            clock_style: ClockStyle::Classic,
            clock_size: 20.0,
            header_clock_size: 22.0,
            clock_dimmed: false,
            visualizer: true,
            show_battery: true,
            battery_ring_size: 16.0,
            battery_match_clock: false,
            ring_clock_gap: 12.0,
            net_bits: false,
            clipboard_history: true,
            eye_moods: true,
            calendar_url: None,
            file_manager: None,
            hide_in_fullscreen: true,
            game_hud: true,
            game_style: GameStyle::Pills,
            auto_update: true,
            mascot: true,
            idle_secs: 45,
            scale: 1.0,
            open_scale: 1.0,
            closed_width: 172.0,
            closed_height: 32.0,
            timer_minutes: 25,
            eye_style: EyeStyle::Classic,
            weather_lat: None,
            weather_lon: None,
            weather_city: None,
            app_folders: Vec::new(),
            eye_iris: Rgb([40, 110, 230]),
            eye_sclera: Rgb([255, 255, 255]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_size_is_bounded_and_independent() {
        let defaults = Settings::default();
        assert_eq!(defaults.battery_ring_size, 16.0);
        for (requested, expected) in [(0.0, 12.0), (40.0, 32.0), (f32::NAN, 16.0)] {
            let settings = Settings { battery_ring_size: requested, clock_size: 30.0, ..Default::default() }.clamped();
            assert_eq!(settings.battery_ring_size, expected);
            assert_eq!(settings.clock_size, 30.0);
        }
    }

    #[test]
    fn clock_sizes_are_independent_and_bounded() {
        let settings = Settings { clock_size: 28.0, header_clock_size: 18.0, ..Settings::default() }.clamped();
        assert_eq!(settings.scale, 1.0);
        assert_eq!(settings.clock_size, 28.0);
        assert_eq!(settings.header_clock_size, 18.0);
        let settings = Settings { clock_size: f32::NAN, header_clock_size: 90.0, ..settings }.clamped();
        assert_eq!(settings.clock_size, 20.0);
        assert_eq!(settings.header_clock_size, 30.0);
    }

    #[test]
    fn rgb_round_trips_and_rejects_garbage() {
        let c = Rgb::parse("#2870e6").unwrap();
        assert_eq!(c, Rgb([40, 112, 230]));
        assert_eq!(c.hex(), "#2870e6");
        assert!(Rgb::parse("nope").is_none());
        assert!(Rgb::parse("#12345").is_none());
    }
}
