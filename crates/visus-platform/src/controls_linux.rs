use std::process::Command;

use visus_core::{Controls, Radio};

pub fn init_thread() {}

fn run(prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn read() -> Controls {
    let mut c = Controls::default();
    // "Volume: 0.45 [MUTED]"
    if let Some(s) = run("wpctl", &["get-volume", "@DEFAULT_AUDIO_SINK@"]) {
        if let Some(v) = s.split_whitespace().nth(1).and_then(|v| v.parse::<f32>().ok()) {
            c.volume = v.clamp(0.0, 1.0);
            c.muted = s.contains("MUTED");
            c.has_volume = true;
        }
    }
    c.wifi = run("nmcli", &["radio", "wifi"]).map(|s| s.trim() == "enabled");
    c.bluetooth = run("bluetoothctl", &["show"]).map(|s| s.contains("Powered: yes"));
    c
}

pub fn set_volume(v: f32) {
    let _ = run("wpctl", &["set-volume", "@DEFAULT_AUDIO_SINK@", &format!("{v:.2}")]);
}

pub fn set_mute(m: bool) {
    let _ = run("wpctl", &["set-mute", "@DEFAULT_AUDIO_SINK@", if m { "1" } else { "0" }]);
}

pub fn set_radio(r: Radio, on: bool) {
    let state = if on { "on" } else { "off" };
    match r {
        Radio::Wifi => drop(run("nmcli", &["radio", "wifi", state])),
        Radio::Bluetooth => drop(run("bluetoothctl", &["power", state])),
    }
}

pub struct Meter;
impl Meter {
    pub fn open() -> Option<Meter> {
        None
    }
    pub fn peak(&self) -> Option<f32> {
        None
    }
}
