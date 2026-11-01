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

/// Looks a city name up (Open-Meteo geocoding) and reports its coordinates.
pub fn geocode(query: String, emit: Emit) {
    std::thread::spawn(move || {
        let found = (|| {
            let mut resp = ureq::get("https://geocoding-api.open-meteo.com/v1/search")
                .query("name", &query)
                .query("count", "1")
                .query("language", "en")
                .call()
                .ok()?;
            let v: serde_json::Value = serde_json::from_str(&resp.body_mut().read_to_string().ok()?).ok()?;
            let r = v["results"].get(0)?;
            let name = r["name"].as_str()?;
            let city = match r["country"].as_str() {
                Some(c) => format!("{name}, {c}"),
                None => name.to_string(),
            };
            Some((city, r["latitude"].as_f64()? as f32, r["longitude"].as_f64()? as f32))
        })();
        emit(Event::Geocode(found));
    });
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
