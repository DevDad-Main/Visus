use std::sync::Mutex;
use std::time::Duration;

use visus_core::{Event, MediaCommand, MediaInfo};
use windows::Foundation::TypedEventHandler;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Session,
    GlobalSystemMediaTransportControlsSessionManager as Manager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::core::IInspectable;

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

fn read_session(s: &Session) -> windows::core::Result<MediaInfo> {
    let props = s.TryGetMediaPropertiesAsync()?.get()?;
    let playing = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    let tl = s.GetTimelineProperties()?;
    let start = ticks(tl.StartTime()?);
    let end = ticks(tl.EndTime()?);
    Ok(MediaInfo {
        has_session: true,
        title: props.Title()?.to_string(),
        artist: props.Artist()?.to_string(),
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
            let Ok(manager) = op.get() else { return };
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
                TypedEventHandler::<Manager, IInspectable>::new(move |_, _| {
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
        let Ok(manager) = Manager::RequestAsync().and_then(|o| o.get()) else { return };
        let Ok(session) = manager.GetCurrentSession() else { return };
        let _ = match cmd {
            MediaCommand::PlayPause => session.TryTogglePlayPauseAsync().and_then(|o| o.get()),
            MediaCommand::Next => session.TrySkipNextAsync().and_then(|o| o.get()),
            MediaCommand::Previous => session.TrySkipPreviousAsync().and_then(|o| o.get()),
        };
    });
}
