use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use visus_core::{Art, Event, MediaCommand, MediaInfo};
use zbus::blocking::fdo::DBusProxy;
use zbus::blocking::{Connection, MessageIterator, Proxy};
use zbus::message::Type;
use zbus::zvariant::OwnedValue;
use zbus::MatchRule;

use crate::Emit;

const PREFIX: &str = "org.mpris.MediaPlayer2.";
const PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER: &str = "org.mpris.MediaPlayer2.Player";

/// Bus name of the player the notch currently controls.
static CURRENT: Mutex<Option<String>> = Mutex::new(None);
/// Decoded covers by URL (None = failed), so each cover is fetched once.
static ART: Mutex<Option<HashMap<String, Option<Arc<Art>>>>> = Mutex::new(None);
static LOADING: Mutex<Option<HashSet<String>>> = Mutex::new(None);
/// Re-publishes media state; set once the watcher is running so art loads can trigger an update.
static REFRESH: OnceLock<Arc<dyn Fn() + Send + Sync>> = OnceLock::new();

fn percent_decode(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = hex {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

fn fetch_art(url: &str) -> Option<Arc<Art>> {
    let bytes = if let Some(path) = url.strip_prefix("file://") {
        let decoded = String::from_utf8(percent_decode(path)).ok()?;
        std::fs::read(decoded).ok()?
    } else if url.starts_with("http://") || url.starts_with("https://") {
        let mut resp = ureq::get(url).call().ok()?;
        resp.body_mut().read_to_vec().ok()?
    } else {
        return None;
    };
    crate::art::decode(&bytes)
}

/// Returns a cached cover, or starts loading it in the background and returns None for now.
fn art_for(url: &str) -> Option<Arc<Art>> {
    if let Some(hit) = ART.lock().unwrap().get_or_insert_with(HashMap::new).get(url) {
        return hit.clone();
    }
    if !LOADING.lock().unwrap().get_or_insert_with(HashSet::new).insert(url.to_string()) {
        return None;
    }
    let url = url.to_string();
    std::thread::spawn(move || {
        let art = fetch_art(&url);
        ART.lock().unwrap().get_or_insert_with(HashMap::new).insert(url.clone(), art);
        LOADING.lock().unwrap().get_or_insert_with(HashSet::new).remove(&url);
        if let Some(refresh) = REFRESH.get() {
            refresh();
        }
    });
    None
}

fn players(conn: &Connection) -> Vec<String> {
    DBusProxy::new(conn)
        .ok()
        .and_then(|p| p.list_names().ok())
        .map(|names| names.into_iter().map(|n| n.to_string()).filter(|n| n.starts_with(PREFIX)).collect())
        .unwrap_or_default()
}

fn micros(v: i64) -> Duration {
    Duration::from_micros(v.max(0) as u64)
}

fn read(conn: &Connection, name: &str) -> Option<MediaInfo> {
    let p = Proxy::new(conn, name.to_string(), PATH, PLAYER).ok()?;
    let status: String = p.get_property("PlaybackStatus").ok()?;
    let meta: HashMap<String, OwnedValue> = p.get_property("Metadata").unwrap_or_default();
    let title = meta.get("xesam:title").and_then(|v| String::try_from(v.clone()).ok()).unwrap_or_default();
    let artist = meta
        .get("xesam:artist")
        .and_then(|v| Vec::<String>::try_from(v.clone()).ok())
        .map(|a| a.join(", "))
        .unwrap_or_default();
    let length = meta.get("mpris:length").and_then(|v| {
        i64::try_from(v.clone()).ok().or_else(|| u64::try_from(v.clone()).ok().map(|x| x as i64))
    });
    let position: i64 = p.get_property("Position").unwrap_or(0);
    let art = meta.get("mpris:artUrl").and_then(|v| String::try_from(v.clone()).ok()).and_then(|u| art_for(&u));
    Some(MediaInfo {
        has_session: true,
        title,
        artist,
        playing: status == "Playing",
        position: micros(position),
        duration: micros(length.unwrap_or(0)),
        art,
    })
}

/// Prefers a player that is playing; otherwise the first one that answers.
fn snapshot(conn: &Connection) -> MediaInfo {
    let mut best: Option<(String, MediaInfo)> = None;
    for name in players(conn) {
        let Some(info) = read(conn, &name) else { continue };
        let better = match &best {
            None => true,
            Some((_, b)) => info.playing && !b.playing,
        };
        if better {
            best = Some((name, info));
        }
    }
    match best {
        Some((name, info)) => {
            *CURRENT.lock().unwrap() = Some(name);
            info
        }
        None => {
            *CURRENT.lock().unwrap() = None;
            MediaInfo::default()
        }
    }
}

fn watch(conn: &Connection, rule: MatchRule<'static>, on_event: Arc<dyn Fn() + Send + Sync>) {
    let Ok(iter) = MessageIterator::for_match_rule(rule, conn, None) else { return };
    let on_event = on_event.clone();
    std::thread::spawn(move || {
        for _ in iter {
            on_event();
        }
    });
}

pub fn start(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-mpris".into())
        .spawn(move || {
            let Ok(conn) = Connection::session() else { return };
            let refresh: Arc<dyn Fn() + Send + Sync> = {
                let conn = conn.clone();
                Arc::new(move || emit(Event::Media(snapshot(&conn))))
            };
            let _ = REFRESH.set(refresh.clone());
            refresh();

            // Property changes of any player, and players appearing or disappearing.
            let props = MatchRule::builder()
                .msg_type(Type::Signal)
                .interface("org.freedesktop.DBus.Properties")
                .and_then(|b| b.member("PropertiesChanged"))
                .and_then(|b| b.path(PATH))
                .map(|b| b.build());
            let seeked = MatchRule::builder()
                .msg_type(Type::Signal)
                .interface(PLAYER)
                .and_then(|b| b.member("Seeked"))
                .map(|b| b.build());
            let owners = MatchRule::builder()
                .msg_type(Type::Signal)
                .interface("org.freedesktop.DBus")
                .and_then(|b| b.member("NameOwnerChanged"))
                .map(|b| b.build());
            for rule in [props, seeked, owners].into_iter().flatten() {
                watch(&conn, rule, refresh.clone());
            }
            loop {
                std::thread::park();
            }
        })
        .expect("spawn mpris thread");
}

pub fn media_command(cmd: MediaCommand) {
    std::thread::spawn(move || {
        let Some(name) = CURRENT.lock().unwrap().clone() else { return };
        let Ok(conn) = Connection::session() else { return };
        let Ok(p) = Proxy::new(&conn, name, PATH, PLAYER) else { return };
        let method = match cmd {
            MediaCommand::PlayPause => "PlayPause",
            MediaCommand::Next => "Next",
            MediaCommand::Previous => "Previous",
            MediaCommand::Seek { target, current } => {
                let offset = target.as_micros() as i64 - current.as_micros() as i64;
                let _ = p.call_method("Seek", &(offset,));
                return;
            }
        };
        let _ = p.call_method(method, &());
    });
}
