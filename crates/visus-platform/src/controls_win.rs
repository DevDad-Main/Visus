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
    c.brightness = brightness();
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

// ---- Display brightness: DDC/CI for external monitors, WMI for laptop panels.

use std::sync::Mutex;

#[derive(Clone, Copy, PartialEq)]
enum Backend {
    Ddc,
    Wmi,
    None,
}

static BRIGHT: Mutex<Option<(Backend, f32)>> = Mutex::new(None);

fn with_primary_monitor<T>(f: impl Fn(windows::Win32::Foundation::HANDLE) -> Option<T>) -> Option<T> {
    use windows::Win32::Devices::Display::{DestroyPhysicalMonitors, GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR};
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};
    unsafe {
        let hmon = MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY);
        let mut n = 0u32;
        GetNumberOfPhysicalMonitorsFromHMONITOR(hmon, &mut n).ok()?;
        if n == 0 {
            return None;
        }
        let mut mons = vec![PHYSICAL_MONITOR::default(); n as usize];
        GetPhysicalMonitorsFromHMONITOR(hmon, &mut mons).ok()?;
        let out = f(mons[0].hPhysicalMonitor);
        let _ = DestroyPhysicalMonitors(&mons);
        out
    }
}

fn ddc_get() -> Option<f32> {
    use windows::Win32::Devices::Display::GetMonitorBrightness;
    with_primary_monitor(|h| unsafe {
        let (mut lo, mut cur, mut hi) = (0u32, 0u32, 0u32);
        (GetMonitorBrightness(h, &mut lo, &mut cur, &mut hi) != 0 && hi > lo).then(|| (cur - lo) as f32 / (hi - lo) as f32)
    })
}

fn ddc_set(v: f32) -> bool {
    use windows::Win32::Devices::Display::{GetMonitorBrightness, SetMonitorBrightness};
    with_primary_monitor(|h| unsafe {
        let (mut lo, mut cur, mut hi) = (0u32, 0u32, 0u32);
        if GetMonitorBrightness(h, &mut lo, &mut cur, &mut hi) == 0 || hi <= lo {
            return None;
        }
        let target = lo + ((hi - lo) as f32 * v.clamp(0.0, 1.0)).round() as u32;
        (SetMonitorBrightness(h, target) != 0).then_some(())
    })
    .is_some()
}

fn powershell(script: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn wmi_get() -> Option<f32> {
    let s = powershell("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction Stop | Select-Object -First 1).CurrentBrightness")?;
    s.parse::<f32>().ok().map(|p| p / 100.0)
}

fn wmi_set(v: f32) -> bool {
    let pct = (v.clamp(0.0, 1.0) * 100.0).round() as u32;
    powershell(&format!(
        "(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods -ErrorAction Stop | Select-Object -First 1) | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=1; Brightness={pct}}} | Out-Null"
    ))
    .is_some()
}

/// Detects a working backend once, then serves the cached value.
pub fn brightness() -> Option<f32> {
    let mut cache = BRIGHT.lock().unwrap();
    if cache.is_none() {
        *cache = Some(if let Some(v) = ddc_get() {
            (Backend::Ddc, v)
        } else if let Some(v) = wmi_get() {
            (Backend::Wmi, v)
        } else {
            (Backend::None, 0.0)
        });
    }
    cache.filter(|(b, _)| *b != Backend::None).map(|(_, v)| v)
}

pub fn set_brightness(v: f32) {
    brightness();
    let mut cache = BRIGHT.lock().unwrap();
    let Some((backend, _)) = *cache else { return };
    let ok = match backend {
        Backend::Ddc => ddc_set(v),
        Backend::Wmi => wmi_set(v),
        Backend::None => false,
    };
    if ok {
        *cache = Some((backend, v));
    }
}
