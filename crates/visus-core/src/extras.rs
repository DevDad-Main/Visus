/// Current conditions for the Weather page (Open-Meteo, WMO weather codes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Weather {
    pub temp_c: f32,
    pub code: u16,
    pub wind_kph: f32,
}

pub fn weather_label(code: u16) -> &'static str {
    match code {
        0 => "Clear sky",
        1 | 2 => "Mostly clear",
        3 => "Overcast",
        45 | 48 => "Fog",
        51..=57 => "Drizzle",
        61..=67 | 80..=82 => "Rain",
        71..=77 | 85 | 86 => "Snow",
        95..=99 => "Thunderstorm",
        _ => "Unknown",
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum UpdateStatus {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available { version: String },
    Installing,
    Failed(String),
}

fn split_version(v: &str) -> (Vec<u64>, Option<String>) {
    let v = v.trim().trim_start_matches('v');
    let (core, pre) = match v.split_once('-') {
        Some((c, p)) => (c, Some(p.to_string())),
        None => (v, None),
    };
    (core.split('.').map(|n| n.parse().unwrap_or(0)).collect(), pre)
}

/// True when `latest` is a newer release than `current` (a final release beats its own prereleases).
pub fn is_newer(latest: &str, current: &str) -> bool {
    let ((l, lp), (c, cp)) = (split_version(latest), split_version(current));
    if l != c {
        return l > c;
    }
    match (lp, cp) {
        (None, Some(_)) => true,
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(is_newer("0.1.0", "0.1.0-alpha.1"));
        assert!(is_newer("0.1.0-alpha.2", "0.1.0-alpha.1"));
        assert!(!is_newer("0.1.0-alpha.1", "0.1.0-alpha.1"));
        assert!(!is_newer("0.1.0-alpha.1", "0.1.0"));
    }
}
