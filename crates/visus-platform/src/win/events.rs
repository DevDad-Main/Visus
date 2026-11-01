use std::sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering};
use std::sync::OnceLock;

use visus_core::{Event, Hud};
use windows::core::w;
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::GUID_ACDC_POWER_SOURCE;
use windows::Win32::System::Power::{
    GetSystemPowerStatus, RegisterPowerSettingNotification,
    SYSTEM_POWER_STATUS,
};
use windows::Win32::UI::Accessibility::SetWinEventHook;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, VK_CAPITAL, VK_SPACE, VK_VOLUME_DOWN, VK_VOLUME_MUTE,
    VK_VOLUME_UP,
};
use windows::Win32::UI::Shell::{
    QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, SHQueryUserNotificationState,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, CreateWindowExW, DefWindowProcW, DispatchMessageW, EVENT_SYSTEM_FOREGROUND,
    EVENT_SYSTEM_MINIMIZEEND, GetMessageW, KBDLLHOOKSTRUCT, KillTimer, MSG, PostMessageW,
    RegisterClassW, SetTimer, SetWindowsHookExW, TranslateMessage, WH_KEYBOARD_LL, WINDOW_EX_STYLE,
    WINDOW_STYLE, WINEVENT_OUTOFCONTEXT, WM_APP, WM_HOTKEY, WM_KEYDOWN, WM_POWERBROADCAST, WM_TIMER,
    WNDCLASSW, DEVICE_NOTIFY_WINDOW_HANDLE, PBT_POWERSETTINGCHANGE,
};

use crate::Emit;

static EMIT: OnceLock<Emit> = OnceLock::new();
static HWND_RAW: AtomicIsize = AtomicIsize::new(0);
static FULLSCREEN: AtomicBool = AtomicBool::new(false);
static LAYOUT: AtomicU32 = AtomicU32::new(0);

const WM_KEY_CAPS: u32 = WM_APP + 1;
const WM_KEY_VOLUME: u32 = WM_APP + 2;
const WM_FOREGROUND: u32 = WM_APP + 3;
const T_CAPS: usize = 1;
const T_VOLUME: usize = 2;
const T_FULLSCREEN: usize = 3;
/// Some fullscreen changes (F11 in a browser) fire no foreground event, so re-check cheaply.
const FULLSCREEN_RECHECK_MS: u32 = 2000;
/// Let the OS apply the key press before reading the resulting state.
const SETTLE_MS: u32 = 50;

fn emit(ev: Event) {
    if let Some(f) = EMIT.get() {
        f(ev);
    }
}

pub fn start(e: Emit) {
    let _ = EMIT.set(e);
    std::thread::Builder::new()
        .name("visus-os-events".into())
        .spawn(|| unsafe { run() })
        .expect("spawn os event thread");
}

unsafe fn run() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok();

        let hmodule = GetModuleHandleW(None).unwrap_or_default();
        let hinstance = HINSTANCE(hmodule.0);
        let class = w!("VisusEvents");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance,
            lpszClassName: class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        // Hidden top-level window: power notifications are delivered to it.
        let hwnd = match CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            class,
            w!("Visus"),
            WINDOW_STYLE::default(),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance),
            None,
        ) {
            Ok(h) => h,
            Err(_) => return,
        };
        HWND_RAW.store(hwnd.0 as isize, Ordering::Relaxed);

        let _ = RegisterPowerSettingNotification(
            HANDLE(hwnd.0),
            &GUID_ACDC_POWER_SOURCE,
            DEVICE_NOTIFY_WINDOW_HANDLE,
        );
        let _ = SetWindowsHookExW(WH_KEYBOARD_LL, Some(key_hook), Some(hinstance), 0);
        for ev in [EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND] {
            SetWinEventHook(ev, ev, None, Some(foreground_hook), 0, 0, WINEVENT_OUTOFCONTEXT);
        }
        SetTimer(Some(hwnd), T_FULLSCREEN, FULLSCREEN_RECHECK_MS, None);
        // Alt+Space opens the launcher; quietly skipped if another app owns the combination.
        let _ = RegisterHotKey(Some(hwnd), 1, MOD_ALT | MOD_NOREPEAT, VK_SPACE.0 as u32);
        let _ = RegisterHotKey(Some(hwnd), 2, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, VK_SPACE.0 as u32);

        report_power(false);
        report_fullscreen();

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_POWERBROADCAST if wp.0 as u32 == PBT_POWERSETTINGCHANGE => {
                report_power(true);
                LRESULT(1)
            }
            WM_HOTKEY => {
                emit(Event::TogglePalette);
                LRESULT(0)
            }
            WM_KEY_CAPS => {
                SetTimer(Some(hwnd), T_CAPS, SETTLE_MS, None);
                LRESULT(0)
            }
            WM_KEY_VOLUME => {
                SetTimer(Some(hwnd), T_VOLUME, SETTLE_MS, None);
                LRESULT(0)
            }
            WM_FOREGROUND => {
                report_fullscreen();
                report_language();
                LRESULT(0)
            }
            WM_TIMER => {
                match wp.0 {
                    T_CAPS => {
                        let _ = KillTimer(Some(hwnd), T_CAPS);
                        let on = GetKeyState(VK_CAPITAL.0 as i32) & 1 != 0;
                        emit(Event::Hud(Hud::CapsLock(on)));
                    }
                    T_VOLUME => {
                        let _ = KillTimer(Some(hwnd), T_VOLUME);
                        if let Some((level, muted)) = read_volume() {
                            emit(Event::Hud(Hud::Volume { level, muted }));
                        }
                    }
                    T_FULLSCREEN => report_fullscreen(),
                    _ => {}
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

/// Only inspects the few keys we show HUDs for; nothing else is read, stored or forwarded.
unsafe extern "system" fn key_hook(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && wp.0 as u32 == WM_KEYDOWN {
            let vk = (*(lp.0 as *const KBDLLHOOKSTRUCT)).vkCode as u16;
            let target = if vk == VK_CAPITAL.0 {
                Some(WM_KEY_CAPS)
            } else if vk == VK_VOLUME_UP.0 || vk == VK_VOLUME_DOWN.0 || vk == VK_VOLUME_MUTE.0 {
                Some(WM_KEY_VOLUME)
            } else {
                None
            };
            if let Some(m) = target {
                let h = HWND_RAW.load(Ordering::Relaxed);
                if h != 0 {
                    let _ = PostMessageW(Some(HWND(h as *mut _)), m, WPARAM(0), LPARAM(0));
                }
            }
        }
        CallNextHookEx(None, code, wp, lp)
    }
}

unsafe extern "system" fn foreground_hook(
    _hook: windows::Win32::UI::Accessibility::HWINEVENTHOOK,
    _event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _thread: u32,
    _time: u32,
) {
    let h = HWND_RAW.load(Ordering::Relaxed);
    if h != 0 {
        unsafe {
            let _ = PostMessageW(Some(HWND(h as *mut _)), WM_FOREGROUND, WPARAM(0), LPARAM(0));
        }
    }
}

fn report_power(_changed: bool) {
    let mut s = SYSTEM_POWER_STATUS::default();
    if unsafe { GetSystemPowerStatus(&mut s) }.is_err() {
        return;
    }
    // BatteryFlag bit 7 = no battery (desktop): nothing to report.
    // Flag 255 / percent 255 = unknown (virtual machines, cloud rigs): also nothing to report.
    if s.BatteryFlag & 128 != 0 || s.BatteryFlag == 255 || s.BatteryLifePercent > 100 {
        return;
    }
    let percent = s.BatteryLifePercent;
    emit(Event::Power { on_ac: s.ACLineStatus == 1, percent });
}

/// Announces the keyboard layout of the focused window when it changes (e.g. Alt+Shift).
fn report_language() {
    use windows::Win32::Globalization::{GetLocaleInfoW, LOCALE_SISO639LANGNAME};
    use windows::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

    let langid = unsafe {
        let thread = GetWindowThreadProcessId(GetForegroundWindow(), None);
        (GetKeyboardLayout(thread).0 as usize & 0xFFFF) as u32
    };
    let prev = LAYOUT.swap(langid, Ordering::Relaxed);
    if prev == 0 || prev == langid || langid == 0 {
        return;
    }
    let mut buf = [0u16; 9];
    let n = unsafe { GetLocaleInfoW(langid, LOCALE_SISO639LANGNAME, Some(&mut buf)) };
    if n >= 3 {
        let code = [buf[0] as u8, buf[1] as u8];
        emit(Event::Hud(Hud::Language([code[0].to_ascii_uppercase(), code[1].to_ascii_uppercase()])));
    }
}

fn report_fullscreen() {
    let state = unsafe { SHQueryUserNotificationState() };
    let full = matches!(
        state,
        Ok(s) if s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE
    );
    if FULLSCREEN.swap(full, Ordering::Relaxed) != full {
        emit(Event::Fullscreen(full));
    }
}

fn read_volume() -> Option<(f32, bool)> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        let vol: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None).ok()?;
        let level = vol.GetMasterVolumeLevelScalar().ok()?;
        let muted = vol.GetMute().ok()?.as_bool();
        Some((level, muted))
    }
}
