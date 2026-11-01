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
        .timeout_global(Some(Duration::from_secs(5)))
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

/// NetEase Music's public endpoints: a broad catalogue with synced lyrics, used when LRCLIB has nothing.
fn netease(agent: &ureq::Agent, title: &str, artist: &str, duration: Duration) -> Lines {
    let get = |url: &str, params: &[(&str, String)]| -> Option<Value> {
        let mut req = agent.get(url).header("Referer", "https://music.163.com");
        for (k, v) in params {
            req = req.query(k, v);
        }
        serde_json::from_str(&req.call().ok()?.body_mut().read_to_string().ok()?).ok()
    };
    let found = get(
        "https://music.163.com/api/search/get/",
        &[("s", format!("{title} {artist}")), ("type", "1".into()), ("offset", "0".into()), ("limit", "6".into())],
    )?;
    let want = duration.as_millis() as i64;
    let wanted = title.to_lowercase();
    let song = found["result"]["songs"].as_array()?.iter().find(|s| {
        let name = s["name"].as_str().unwrap_or("").to_lowercase();
        let near = want == 0 || (s["duration"].as_i64().unwrap_or(0) - want).abs() < 6000;
        near && !name.is_empty() && (name.contains(&wanted) || wanted.contains(&name))
    })?;
    let lyric = get("https://music.163.com/api/song/lyric", &[("id", song["id"].as_i64()?.to_string()), ("lv", "1".into())])?;
    let mut lines = parse_lrc(lyric["lrc"]["lyric"].as_str()?);
    // Credit lines such as "作词 : …" sit at the very start.
    lines.retain(|l| !(l.at < Duration::from_secs(15) && (l.text.contains(" : ") || l.text.contains('：'))));
    (!lines.is_empty()).then(|| Arc::new(lines))
}

pub(crate) fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Plain (unsynced) lyrics from lyrics.ovh, spread evenly over the track so the pane still scrolls.
fn lyrics_ovh(agent: &ureq::Agent, title: &str, artist: &str, duration: Duration) -> Lines {
    if duration.is_zero() {
        return None;
    }
    let url = format!("https://api.lyrics.ovh/v1/{}/{}", encode(artist), encode(title));
    let v: Value = serde_json::from_str(&agent.get(url).call().ok()?.body_mut().read_to_string().ok()?).ok()?;
    let rows: Vec<String> = v["lyrics"].as_str()?.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
    if rows.len() < 4 {
        return None;
    }
    let step = duration.as_secs_f32() * 0.9 / rows.len() as f32;
    let lines = rows.into_iter().enumerate().map(|(i, text)| LyricLine { at: Duration::from_secs_f32(duration.as_secs_f32() * 0.05 + i as f32 * step), text }).collect();
    Some(Arc::new(lines))
}

fn synced(v: &Value) -> Lines {
    let lines = parse_lrc(v["syncedLyrics"].as_str()?);
    (!lines.is_empty()).then(|| Arc::new(lines))
}

/// Strips "(Official Video)", "[HD]", "ft. X" and " - Topic" noise typical of YouTube titles.
pub(crate) fn clean(s: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in s.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = (depth - 1).max(0),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    let lower = out.to_lowercase();
    let cut = [" ft.", " feat.", " featuring", " | ", " // "].iter().filter_map(|m| lower.find(m)).min().unwrap_or(out.len());
    out.truncate(cut);
    out.trim().trim_end_matches(" - Topic").trim().to_string()
}

fn lookup(agent: &ureq::Agent, title: &str, artist: &str, duration: Duration) -> Lines {
    let (title, artist) = (title.to_string(), artist.to_string());
    let mut attempts = vec![(title.clone(), artist.clone())];
    let (ct, ca) = (clean(&title), clean(&artist));
    // YouTube titles often read "Artist - Song" with the channel as the artist.
    if let Some((a, t)) = ct.split_once(" - ") {
        attempts.push((t.trim().to_string(), a.trim().to_string()));
    }
    attempts.push((ct.clone(), ca.clone()));
    attempts.dedup();

    // The three sources run side by side so a miss on one never delays the others; LRCLIB wins when it has a match.
    let lrclib = {
        let (agent, attempts, q) = (agent.clone(), attempts.clone(), format!("{ca} {ct}"));
        std::thread::spawn(move || -> Lines {
            for (t, a) in &attempts {
                let mut params = vec![("track_name", t.clone()), ("artist_name", a.clone())];
                if !duration.is_zero() {
                    params.push(("duration", duration.as_secs().to_string()));
                }
                // Exact match first, then a looser search when durations disagree.
                if let Some(lines) = get_json(&agent, "get", &params).and_then(|v| synced(&v)) {
                    return Some(lines);
                }
                params.retain(|(k, _)| *k != "duration");
                if let Some(lines) = get_json(&agent, "search", &params).and_then(|v| v.as_array()?.iter().find_map(synced)) {
                    return Some(lines);
                }
            }
            get_json(&agent, "search", &[("q", q)]).and_then(|v| v.as_array()?.iter().find_map(synced))
        })
    };
    let ne = {
        let (agent, attempts) = (agent.clone(), attempts.clone());
        std::thread::spawn(move || -> Lines { attempts.iter().take(3).find_map(|(t, a)| netease(&agent, t, a, duration)) })
    };
    let ovh = {
        let (agent, attempts) = (agent.clone(), attempts.clone());
        std::thread::spawn(move || -> Lines { attempts.iter().take(2).find_map(|(t, a)| lyrics_ovh(&agent, t, a, duration)) })
    };
    lrclib.join().ok().flatten().or_else(|| ne.join().ok().flatten()).or_else(|| ovh.join().ok().flatten())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_strips_youtube_noise() {
        assert_eq!(clean("Blinding Lights (Official Video) [HD]"), "Blinding Lights");
        assert_eq!(clean("Song ft. Someone"), "Song");
        assert_eq!(clean("Artist - Topic"), "Artist");
    }

    #[test]
    fn encode_escapes_unsafe_bytes() {
        assert_eq!(encode("a b/c"), "a%20b%2Fc");
    }
}
