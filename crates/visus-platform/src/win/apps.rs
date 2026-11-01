use std::collections::HashSet;
use std::path::{Path, PathBuf};

use std::sync::Arc;

use visus_core::{App, Art, guess_kind};
use windows::core::{HSTRING, PCWSTR, w};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

fn env_dir(var: &str, rel: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(|b| PathBuf::from(b).join(rel))
}

/// Executables that are not something a user launches on purpose.
fn is_noise(lower: &str) -> bool {
    ["unins", "setup", "install", "update", "crash", "helper", "redist", "service", "report", "notif", "elevat", "cef", "renderer", "host", "handler", "diagnos", "launcherhelper"]
        .iter()
        .any(|w| lower.contains(w))
}

const MAX_APPS: usize = 6000;

fn collect(dir: &Path, depth: u32, max_depth: u32, exes: bool, out: &mut Vec<App>, seen: &mut HashSet<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        if out.len() >= MAX_APPS {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            if depth < max_depth {
                collect(&path, depth + 1, max_depth, exes, out, seen);
            }
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
        let shortcut = ext == "lnk" || ext == "url";
        if !(shortcut || (exes && ext == "exe")) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
        let lower = name.to_lowercase();
        if lower.contains("uninstall") || (!shortcut && is_noise(&lower)) || !seen.insert(lower) {
            continue;
        }
        let kind = guess_kind(&name, "");
        out.push(App { name, target: path.to_string_lossy().into_owned(), icon: String::new(), kind });
    }
}

/// Start Menu and Desktop shortcuts first, then executables under Program Files and any folders
/// listed in settings (`app_folders`, any drive). Store/UWP apps are not listed yet.
pub fn list() -> Vec<App> {
    let (mut apps, mut seen) = (Vec::new(), HashSet::new());
    let shortcut_roots = [
        env_dir("ProgramData", r"Microsoft\Windows\Start Menu\Programs"),
        env_dir("APPDATA", r"Microsoft\Windows\Start Menu\Programs"),
        env_dir("USERPROFILE", "Desktop"),
        env_dir("PUBLIC", "Desktop"),
    ];
    for dir in shortcut_roots.into_iter().flatten() {
        collect(&dir, 0, 4, false, &mut apps, &mut seen);
    }
    let program_roots = [env_dir("ProgramFiles", ""), env_dir("ProgramFiles(x86)", ""), env_dir("LOCALAPPDATA", "Programs")];
    for dir in program_roots.into_iter().flatten() {
        collect(&dir, 0, 2, true, &mut apps, &mut seen);
    }
    let extra = crate::APP_FOLDERS.lock().unwrap().clone();
    for dir in extra {
        collect(Path::new(&dir), 0, 3, true, &mut apps, &mut seen);
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

/// The shortcut's icon as 48x48 RGBA, taken from the shell.
pub fn load_icon(app: &App) -> Option<Arc<Art>> {
    use std::ffi::c_void;
    use windows::Win32::Graphics::Gdi::{
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, HGDIOBJ,
        SelectObject,
    };
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
    use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx};

    const SIZE: i32 = 48;
    let path = HSTRING::from(app.target.as_str());
    unsafe {
        let mut info = SHFILEINFOW::default();
        let ok = SHGetFileInfoW(
            PCWSTR(path.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        );
        if ok == 0 || info.hIcon.is_invalid() {
            return None;
        }
        let hdc = CreateCompatibleDC(None);
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE,
                biHeight: -SIZE,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let dib = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        let mut rgba = None;
        if let Ok(dib) = dib {
            let old = SelectObject(hdc, HGDIOBJ(dib.0));
            if DrawIconEx(hdc, 0, 0, info.hIcon, SIZE, SIZE, 0, None, DI_NORMAL).is_ok() && !bits.is_null() {
                let mut px = std::slice::from_raw_parts(bits as *const u8, (SIZE * SIZE * 4) as usize).to_vec();
                // Some icons draw without alpha; treat any painted pixel as opaque then.
                let has_alpha = px.chunks_exact(4).any(|p| p[3] != 0);
                for p in px.chunks_exact_mut(4) {
                    p.swap(0, 2);
                    if !has_alpha && (p[0] | p[1] | p[2]) != 0 {
                        p[3] = 255;
                    }
                }
                rgba = Some(px);
            }
            SelectObject(hdc, old);
            let _ = DeleteObject(HGDIOBJ(dib.0));
        }
        let _ = DeleteDC(hdc);
        let _ = DestroyIcon(info.hIcon);
        rgba.map(|px| crate::art::from_rgba(SIZE as u32, px))
    }
}

fn shell_open(target: &str) {
    let file = HSTRING::from(target);
    unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(file.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

pub fn launch(target: &str) {
    shell_open(target);
}

pub fn open_path(path: &str) {
    shell_open(path);
}

/// Opens the Windows snipping overlay.
pub fn screenshot() {
    shell_open("ms-screenclip:");
}

pub fn open_url(url: &str) {
    // Only web addresses are handed to the shell.
    if url.starts_with("https://") || url.starts_with("http://") {
        shell_open(url);
    }
}
