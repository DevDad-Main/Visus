use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetCursorPos, GetWindowLongPtrW, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW,
};

fn hwnd(h: isize) -> HWND {
    HWND(h as *mut core::ffi::c_void)
}

/// Keeps the notch out of the taskbar/Alt-Tab and stops it stealing focus.
pub fn prepare_window(h: isize) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd(h), GWL_EXSTYLE);
        let ex = ex | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0) as isize;
        SetWindowLongPtrW(hwnd(h), GWL_EXSTYLE, ex);
    }
}

/// Clips the window to the pill so everything around it is click-through.
/// Coordinates are physical pixels relative to the window.
pub fn set_window_region(h: isize, x: i32, y: i32, w: i32, height: i32, radius: i32) {
    unsafe {
        // Start above the window so only the bottom corners are rounded.
        let rgn = CreateRoundRectRgn(x, y - radius * 2, x + w, y + height, radius * 2, radius * 2);
        // The system owns the region after a successful call.
        SetWindowRgn(hwnd(h), Some(rgn), true);
    }
}

/// Puts the notch back on top of the topmost band, e.g. above a game that took the screen.
pub fn raise_window(h: isize) {
    unsafe {
        let _ = SetWindowPos(hwnd(h), Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
}

/// Global cursor position in screen pixels.
pub fn cursor_pos() -> Option<(i32, i32)> {
    let mut p = POINT::default();
    unsafe { GetCursorPos(&mut p) }.ok()?;
    Some((p.x, p.y))
}

/// Lets the notch take keyboard focus while the launcher is open; restores click-through focus rules after.
pub fn set_focusable(h: isize, on: bool) {
    unsafe {
        let ex = GetWindowLongPtrW(hwnd(h), GWL_EXSTYLE);
        let noactivate = WS_EX_NOACTIVATE.0 as isize;
        SetWindowLongPtrW(hwnd(h), GWL_EXSTYLE, if on { ex & !noactivate } else { ex | noactivate });
        if on {
            let _ = SetForegroundWindow(hwnd(h));
        }
    }
}
