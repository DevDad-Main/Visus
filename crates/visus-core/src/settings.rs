use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EyeStyle {
    #[default]
    Classic,
    Sauron,
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
    /// Closed pill collapses to a thin line (for thick bezels).
    pub minimal: bool,
    pub hide_in_fullscreen: bool,
    /// Show the idle eye when nothing has happened for a while.
    pub mascot: bool,
    pub idle_secs: u32,
    /// Overall size multiplier for the pill and its content.
    pub scale: f32,
    /// Closed pill size in logical pixels, before `scale`.
    pub closed_width: f32,
    pub closed_height: f32,
    pub timer_minutes: u32,
    pub eye_style: EyeStyle,
    /// Weather is only fetched once a location is set (latitude/longitude in degrees).
    pub weather_lat: Option<f32>,
    pub weather_lon: Option<f32>,
    pub weather_city: Option<String>,
    pub eye_iris: Rgb,
    pub eye_sclera: Rgb,
}

impl Settings {
    pub const SCALE_RANGE: (f32, f32) = (0.6, 1.25);

    /// Keeps hand-edited values inside what the layout can render.
    pub fn clamped(mut self) -> Self {
        self.scale = self.scale.clamp(Self::SCALE_RANGE.0, Self::SCALE_RANGE.1);
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
            minimal: false,
            hide_in_fullscreen: true,
            mascot: true,
            idle_secs: 45,
            scale: 1.0,
            closed_width: 228.0,
            closed_height: 38.0,
            timer_minutes: 25,
            eye_style: EyeStyle::Classic,
            weather_lat: None,
            weather_lon: None,
            weather_city: None,
            eye_iris: Rgb([40, 110, 230]),
            eye_sclera: Rgb([255, 255, 255]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_round_trips_and_rejects_garbage() {
        let c = Rgb::parse("#2870e6").unwrap();
        assert_eq!(c, Rgb([40, 112, 230]));
        assert_eq!(c.hex(), "#2870e6");
        assert!(Rgb::parse("nope").is_none());
        assert!(Rgb::parse("#12345").is_none());
    }
}
