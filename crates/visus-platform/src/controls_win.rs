use visus_core::{Controls, Radio};
use windows::Devices::Radios::{Radio as WinRadio, RadioKind, RadioState};
use windows::Win32::Media::Audio::Endpoints::{IAudioEndpointVolume, IAudioMeterInformation};
use windows::Win32::Media::Audio::{IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};

pub fn init_thread() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

fn device() -> Option<IMMDevice> {
    unsafe {
        let e: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        e.GetDefaultAudioEndpoint(eRender, eConsole).ok()
    }
}

fn endpoint() -> Option<IAudioEndpointVolume> {
    unsafe { device()?.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).ok() }
}

fn radios() -> Vec<WinRadio> {
    let Ok(op) = WinRadio::GetRadiosAsync() else { return Vec::new() };
    let Ok(list) = op.join() else { return Vec::new() };
    (0..list.Size().unwrap_or(0)).filter_map(|i| list.GetAt(i).ok()).collect()
}

fn kind(r: Radio) -> RadioKind {
    match r {
        Radio::Wifi => RadioKind::WiFi,
        Radio::Bluetooth => RadioKind::Bluetooth,
    }
}

pub fn read() -> Controls {
    let mut c = Controls::default();
    if let Some(v) = endpoint() {
        unsafe {
            if let (Ok(level), Ok(muted)) = (v.GetMasterVolumeLevelScalar(), v.GetMute()) {
                c.volume = level;
                c.muted = muted.as_bool();
                c.has_volume = true;
            }
        }
    }
    for r in radios() {
        let on = r.State().ok().map(|s| s == RadioState::On);
        match r.Kind() {
            Ok(k) if k == RadioKind::WiFi => c.wifi = c.wifi.or(on),
            Ok(k) if k == RadioKind::Bluetooth => c.bluetooth = c.bluetooth.or(on),
            _ => {}
        }
    }
    c
}

pub fn set_volume(v: f32) {
    if let Some(e) = endpoint() {
        unsafe {
            let _ = e.SetMasterVolumeLevelScalar(v.clamp(0.0, 1.0), std::ptr::null());
            if v > 0.0 {
                let _ = e.SetMute(false, std::ptr::null());
            }
        }
    }
}

pub fn set_mute(m: bool) {
    if let Some(e) = endpoint() {
        unsafe {
            let _ = e.SetMute(m, std::ptr::null());
        }
    }
}

pub fn set_radio(r: Radio, on: bool) {
    let _ = WinRadio::RequestAccessAsync().and_then(|op| op.join());
    let state = if on { RadioState::On } else { RadioState::Off };
    for radio in radios() {
        if radio.Kind().is_ok_and(|k| k == kind(r)) {
            let _ = radio.SetStateAsync(state).and_then(|op| op.join());
        }
    }
}

pub struct Meter(IAudioMeterInformation);

impl Meter {
    pub fn open() -> Option<Meter> {
        unsafe { device()?.Activate::<IAudioMeterInformation>(CLSCTX_ALL, None).ok().map(Meter) }
    }

    pub fn peak(&self) -> Option<f32> {
        unsafe { self.0.GetPeakValue().ok() }
    }
}
