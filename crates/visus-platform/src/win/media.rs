use std::sync::{Arc, Mutex};
use std::time::Duration;

use visus_core::{Art, Event, MediaCommand, MediaInfo};
use windows::Storage::Streams::DataReader;
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};


use crate::Emit;

struct Subscription {
    session: Session,
    tokens: [i64; 3],
}

fn ticks(t: windows::Foundation::TimeSpan) -> Duration {
    // TimeSpan counts 100 ns units.
    Duration::from_nanos(t.Duration.max(0) as u64 * 100)
}

fn snapshot(manager: &Manager) -> MediaInfo {
    let Ok(session) = manager.GetCurrentSession() else { return MediaInfo::default() };
    read_session(&session).unwrap_or_default()
}

/// Last decoded cover keyed by track, so state updates do not re-read the thumbnail stream.
static ART_CACHE: Mutex<Option<((String, String), Option<Arc<Art>>)>> = Mutex::new(None);

fn thumbnail(props: &windows::Media::Control::GlobalSystemMediaTransportControlsSessionMediaProperties) -> Option<Arc<Art>> {
    let stream = props.Thumbnail().ok()?.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 8_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size).ok()?.join().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    crate::art::decode(&buf)
}

fn read_session(s: &Session) -> windows::core::Result<MediaInfo> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    let tl = s.GetTimelineProperties()?;
    let start = ticks(tl.StartTime()?);
    let end = ticks(tl.EndTime()?);
    let (title, artist) = (props.Title()?.to_string(), props.Artist()?.to_string());
    let key = (title.clone(), artist.clone());
    let cached = ART_CACHE.lock().unwrap().as_ref().filter(|(k, _)| *k == key).map(|(_, a)| a.clone());
    let art = match cached {
        Some(a) => a,
        None => {
            let a = thumbnail(&props);
            *ART_CACHE.lock().unwrap() = Some((key, a.clone()));
            a
        }
    };
    Ok(MediaInfo {
        has_session: true,
        title,
        artist,
        art,
        playing,
        position: ticks(tl.Position()?).saturating_sub(start),
        duration: end.saturating_sub(start),
    })
}

pub fn start(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-media".into())
        .spawn(move || {
            let Ok(op) = Manager::RequestAsync() else { return };
            let Ok(manager) = op.join() else { return };
            let sub: std::sync::Arc<Mutex<Option<Subscription>>> = Default::default();

            let attach = {
                let manager = manager.clone();
                let emit = emit.clone();
                let sub = sub.clone();
                move || {
                    let mut slot = sub.lock().unwrap();
                    if let Some(old) = slot.take() {
                        let _ = old.session.RemoveMediaPropertiesChanged(old.tokens[0]);
                        let _ = old.session.RemovePlaybackInfoChanged(old.tokens[1]);
                        let _ = old.session.RemoveTimelinePropertiesChanged(old.tokens[2]);
                    }
                    if let Ok(session) = manager.GetCurrentSession() {
                        let mk = |session: &Session| {
                            let emit = emit.clone();
                            let session = session.clone();
                            move || emit(Event::Media(read_session(&session).unwrap_or_default()))
                        };
                        let (a, b, c) = (mk(&session), mk(&session), mk(&session));
                        let t0 = session.MediaPropertiesChanged(&TypedEventHandler::new(
                            move |_, _| {
                                a();
                                Ok(())
                            },
                        ));
                        let t1 = session.PlaybackInfoChanged(&TypedEventHandler::new(
                            move |_, _| {
                                b();
                                Ok(())
                            },
                        ));
                        let t2 = session.TimelinePropertiesChanged(&TypedEventHandler::new(
                            move |_, _| {
                                c();
                                Ok(())
                            },
                        ));
                        if let (Ok(t0), Ok(t1), Ok(t2)) = (t0, t1, t2) {
                            *slot = Some(Subscription { session, tokens: [t0, t1, t2] });
                        }
                    }
                    emit(Event::Media(snapshot(&manager)));
                }
            };

            attach();
            let handler = {
                let attach = Mutex::new(attach);
                TypedEventHandler::<Manager, CurrentSessionChangedEventArgs>::new(move |_, _| {
                    (attach.lock().unwrap())();
                    Ok(())
                })
            };
            let _keep = manager.CurrentSessionChanged(&handler);
            loop {
                std::thread::park();
            }
        })
        .expect("spawn media thread");
}

pub fn media_command(cmd: MediaCommand) {
    std::thread::spawn(move || {
        let Ok(manager) = Manager::RequestAsync().and_then(|o| o.join()) else { return };
        let Ok(session) = manager.GetCurrentSession() else { return };
        let _ = match cmd {
            MediaCommand::PlayPause => session.TryTogglePlayPauseAsync().and_then(|o| o.join()),
            MediaCommand::Next => session.TrySkipNextAsync().and_then(|o| o.join()),
            MediaCommand::Previous => session.TrySkipPreviousAsync().and_then(|o| o.join()),
        };
    });
}
