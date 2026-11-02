use std::collections::BTreeMap;
use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmQueryThumbnailSourceSize, DwmRegisterThumbnail, DwmUnregisterThumbnail,
    DwmUpdateThumbnailProperties, DWM_THUMBNAIL_PROPERTIES, DWM_TNP_OPACITY,
    DWM_TNP_RECTDESTINATION, DWM_TNP_SOURCECLIENTAREAONLY, DWM_TNP_VISIBLE,
};

#[derive(Default)]
pub struct WindowThumbnails {
    registered: BTreeMap<isize, isize>,
}

impl WindowThumbnails {
    pub fn sync(&mut self, destination: isize, targets: &[(isize, visus_core::Rect)]) {
        self.registered.retain(|source, thumbnail| {
            if targets.iter().any(|(handle, _)| handle == source) { true }
            else { unsafe { let _ = DwmUnregisterThumbnail(*thumbnail); } false }
        });
        for (source, bounds) in targets {
            if *source == destination || destination == 0 { continue; }
            let thumbnail = match self.registered.get(source) {
                Some(handle) => *handle,
                None => {
                    let Ok(handle) = (unsafe { DwmRegisterThumbnail(HWND(destination as *mut _), HWND(*source as *mut _)) }) else { continue };
                    self.registered.insert(*source, handle);
                    handle
                }
            };
            let Ok(size) = (unsafe { DwmQueryThumbnailSourceSize(thumbnail) }) else { continue };
            if size.cx <= 0 || size.cy <= 0 { continue; }
            let scale = (bounds.w / size.cx as f32).min(bounds.h / size.cy as f32);
            let width = size.cx as f32 * scale;
            let height = size.cy as f32 * scale;
            let left = bounds.x + (bounds.w - width) / 2.0;
            let top = bounds.y + (bounds.h - height) / 2.0;
            let properties = DWM_THUMBNAIL_PROPERTIES {
                dwFlags: DWM_TNP_RECTDESTINATION | DWM_TNP_VISIBLE | DWM_TNP_OPACITY | DWM_TNP_SOURCECLIENTAREAONLY,
                rcDestination: RECT { left: left.round() as i32, top: top.round() as i32,
                    right: (left + width).round() as i32, bottom: (top + height).round() as i32 },
                opacity: 255, fVisible: BOOL(1), fSourceClientAreaOnly: BOOL(0),
                ..Default::default()
            };
            if unsafe { DwmUpdateThumbnailProperties(thumbnail, &properties) }.is_err() {
                unsafe { let _ = DwmUnregisterThumbnail(thumbnail); }
                self.registered.remove(source);
            }
        }
    }
}

impl Drop for WindowThumbnails {
    fn drop(&mut self) {
        for thumbnail in self.registered.values() { unsafe { let _ = DwmUnregisterThumbnail(*thumbnail); } }
    }
}