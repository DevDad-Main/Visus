use windows::Win32::Foundation::{HWND, LPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GWL_EXSTYLE, GetWindowLongW, GetWindowTextLengthW, GetWindowTextW, IsIconic, IsWindowVisible, SW_RESTORE,
    SetForegroundWindow, ShowWindow, WS_EX_TOOLWINDOW,
};
use windows::core::BOOL;

unsafe extern "system" fn collect(hwnd: HWND, lp: LPARAM) -> BOOL {
    unsafe {
        let out = &mut *(lp.0 as *mut Vec<(isize, String)>);
        if IsWindowVisible(hwnd).as_bool() && GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 == 0 {
            let len = GetWindowTextLengthW(hwnd);
            if len > 0 {
                let mut buf = vec![0u16; len as usize + 1];
                let n = GetWindowTextW(hwnd, &mut buf).max(0) as usize;
                let title = String::from_utf16_lossy(&buf[..n]);
                if !title.is_empty() && title != "Program Manager" && title != "Visus" {
                    out.push((hwnd.0 as isize, title));
                }
            }
        }
        BOOL(1)
    }
}

/// Visible top-level windows with titles, in z-order.
pub fn list_windows() -> Vec<(isize, String)> {
    let mut out: Vec<(isize, String)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

pub fn focus_window(h: isize) {
    unsafe {
        let hwnd = HWND(h as *mut _);
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);
    }
}
