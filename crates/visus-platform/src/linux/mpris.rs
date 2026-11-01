use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use visus_core::{Event, MediaCommand, MediaInfo};
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
    Some(MediaInfo {
        has_session: true,
        title,
        artist,
        playing: status == "Playing",
        position: micros(position),
        duration: micros(length.unwrap_or(0)),
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
        };
        let _ = p.call_method(method, &());
    });
}
