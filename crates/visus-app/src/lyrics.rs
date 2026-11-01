//! Looks up synced lyrics on LRCLIB (https://lrclib.net) off the UI thread.

use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use visus_core::{Event, LyricLine, parse_lrc};
use visus_platform::Emit;

const USER_AGENT: &str = concat!("Visus/", env!("CARGO_PKG_VERSION"), " (https://github.com/DevDad-Main/Visus)");
const CACHE_LIMIT: usize = 64;

type Request = (String, String, Duration);
type Lines = Option<Arc<Vec<LyricLine>>>;

pub struct Fetcher {
    tx: Sender<Request>,
}

impl Fetcher {
    pub fn new(emit: Emit) -> Self {
        let (tx, rx) = channel();
        std::thread::Builder::new()
            .name("visus-lyrics".into())
            .spawn(move || worker(rx, emit))
            .expect("spawn lyrics thread");
        Self { tx }
    }

    pub fn request(&self, title: String, artist: String, duration: Duration) {
        let _ = self.tx.send((title, artist, duration));
    }
}

fn worker(rx: Receiver<Request>, emit: Emit) {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .user_agent(USER_AGENT)
        .build()
        .into();
    let mut cache: HashMap<(String, String), Lines> = HashMap::new();

    while let Ok(mut req) = rx.recv() {
        // Skip tracks the user already skipped past.
        while let Ok(newer) = rx.try_recv() {
            req = newer;
        }
        let (title, artist, duration) = req;
        let key = (title.clone(), artist.clone());
        let lines = match cache.get(&key) {
            Some(hit) => hit.clone(),
            None => {
                let found = lookup(&agent, &title, &artist, duration);
                if cache.len() >= CACHE_LIMIT {
                    cache.clear();
                }
                cache.insert(key, found.clone());
                found
            }
        };
        emit(Event::Lyrics { title, artist, lines });
    }
}

fn get_json(agent: &ureq::Agent, path: &str, params: &[(&str, String)]) -> Option<Value> {
    let mut req = agent.get(format!("https://lrclib.net/api/{path}"));
    for (k, v) in params {
        req = req.query(k, v);
    }
    let mut resp = req.call().ok()?;
    let body = resp.body_mut().read_to_string().ok()?;
    serde_json::from_str(&body).ok()
}

fn synced(v: &Value) -> Lines {
    let lines = parse_lrc(v["syncedLyrics"].as_str()?);
    (!lines.is_empty()).then(|| Arc::new(lines))
}

fn lookup(agent: &ureq::Agent, title: &str, artist: &str, duration: Duration) -> Lines {
    let mut params = vec![("track_name", title.to_string()), ("artist_name", artist.to_string())];
    if !duration.is_zero() {
        params.push(("duration", duration.as_secs().to_string()));
    }
    // Exact match first, then a looser search when durations disagree.
    if let Some(lines) = get_json(agent, "get", &params).and_then(|v| synced(&v)) {
        return Some(lines);
    }
    params.retain(|(k, _)| *k != "duration");
    get_json(agent, "search", &params)?.as_array()?.iter().find_map(synced)
}
