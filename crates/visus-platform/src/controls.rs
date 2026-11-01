//! Volume, Wi-Fi/Bluetooth and the output level. Slow OS calls run on one worker thread.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

use visus_core::{Controls, Radio};

#[cfg(windows)]
#[path = "controls_win.rs"]
mod sys;
#[cfg(target_os = "linux")]
#[path = "controls_linux.rs"]
mod sys;
#[cfg(not(any(windows, target_os = "linux")))]
#[path = "controls_other.rs"]
mod sys;

static STATE: Mutex<Controls> = Mutex::new(Controls { volume: 0.0, muted: false, has_volume: false, wifi: None, bluetooth: None });
static SAMPLING: AtomicBool = AtomicBool::new(false);
static LIVE: AtomicBool = AtomicBool::new(false);
static LEVEL: AtomicU32 = AtomicU32::new(0);
static SAMPLER: Once = Once::new();

enum Cmd {
    Refresh,
    Volume(f32),
    Mute(bool),
    Radio(Radio, bool),
}

fn send(cmd: Cmd) {
    static TX: OnceLock<Sender<Cmd>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = channel::<Cmd>();
        std::thread::Builder::new()
            .name("visus-controls".into())
            .spawn(move || {
                sys::init_thread();
                while let Ok(first) = rx.recv() {
                    let mut batch = vec![first];
                    while let Ok(c) = rx.try_recv() {
                        batch.push(c);
                    }
                    let (mut volume, mut refresh) = (None, false);
                    for c in batch {
                        match c {
                            Cmd::Volume(v) => volume = Some(v),
                            Cmd::Mute(m) => {
                                sys::set_mute(m);
                                refresh = true;
                            }
                            Cmd::Radio(r, on) => {
                                sys::set_radio(r, on);
                                refresh = true;
                            }
                            Cmd::Refresh => refresh = true,
                        }
                    }
                    if let Some(v) = volume {
                        sys::set_volume(v);
                    }
                    if refresh {
                        *STATE.lock().unwrap() = sys::read();
                    }
                }
            })
            .expect("spawn controls thread");
        tx
    });
    let _ = tx.send(cmd);
}

/// Last state read from the OS.
pub fn controls() -> Controls {
    *STATE.lock().unwrap()
}

pub fn refresh_controls() {
    send(Cmd::Refresh);
}

pub fn set_volume(v: f32) {
    send(Cmd::Volume(v));
}

pub fn set_muted(m: bool) {
    send(Cmd::Mute(m));
}

pub fn set_radio(r: Radio, on: bool) {
    send(Cmd::Radio(r, on));
}

/// Turns output-level sampling on or off; it only runs while the visualizer is visible.
pub fn set_audio_sampling(on: bool) {
    SAMPLING.store(on, Ordering::Relaxed);
    if on {
        SAMPLER.call_once(|| {
            let _ = std::thread::Builder::new().name("visus-audio".into()).spawn(sampler);
        });
    }
}

/// Current output peak 0..1, or `None` when the system cannot report it.
pub fn audio_level() -> Option<f32> {
    LIVE.load(Ordering::Relaxed).then(|| f32::from_bits(LEVEL.load(Ordering::Relaxed)))
}

fn sampler() {
    sys::init_thread();
    let mut meter: Option<sys::Meter> = None;
    let mut opened = Instant::now();
    loop {
        if !SAMPLING.load(Ordering::Relaxed) {
            LIVE.store(false, Ordering::Relaxed);
            meter = None;
            std::thread::sleep(Duration::from_millis(250));
            continue;
        }
        // Reopen now and then so a changed default output device is followed.
        if meter.is_none() || opened.elapsed() > Duration::from_secs(5) {
            meter = sys::Meter::open();
            opened = Instant::now();
        }
        match meter.as_ref().and_then(|m| m.peak()) {
            Some(p) => {
                LEVEL.store(p.to_bits(), Ordering::Relaxed);
                LIVE.store(true, Ordering::Relaxed);
            }
            None => {
                LIVE.store(false, Ordering::Relaxed);
                meter = None;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}
