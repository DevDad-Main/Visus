//! Current conditions from Open-Meteo (no account or key); only fetched while the Weather page is open.

use std::time::{Duration, Instant};

use visus_core::{Event, Notch, Weather};
use visus_platform::Emit;

const REFRESH: Duration = Duration::from_secs(15 * 60);

#[derive(Default)]
pub struct Poller {
    last: Option<Instant>,
}

impl Poller {
    pub fn tick(&mut self, notch: &Notch, emit: Option<&Emit>) {
        let (Some(emit), true) = (emit, notch.wants_weather()) else { return };
        if self.last.is_some_and(|t| t.elapsed() < REFRESH) {
            return;
        }
        self.last = Some(Instant::now());
        let s = notch.settings();
        let (Some(lat), Some(lon)) = (s.weather_lat, s.weather_lon) else { return };
        let emit = emit.clone();
        std::thread::spawn(move || emit(Event::Weather(fetch(lat, lon))));
    }
}

fn fetch(lat: f32, lon: f32) -> Option<Weather> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}&current=temperature_2m,weather_code,wind_speed_10m"
    );
    let body = ureq::get(&url).call().ok()?.body_mut().read_to_string().ok()?;
    let v: serde_json::Value = serde_json::from_str(&body).ok()?;
    let c = &v["current"];
    Some(Weather { temp_c: c["temperature_2m"].as_f64()? as f32, code: c["weather_code"].as_u64()? as u16, wind_kph: c["wind_speed_10m"].as_f64()? as f32 })
}
