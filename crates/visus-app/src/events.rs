use visus_core::{Event, Notch};

use crate::lyrics::Fetcher;

/// Applies an OS or worker event to the notch and starts a lyrics lookup when the track changed.
pub fn apply(notch: &mut Notch, fetcher: Option<&Fetcher>, ev: Event) {
    let before = notch.track_key();
    if let Event::Geocode(found) = &ev {
        crate::settings_surface::location_result(found.as_ref().map(|(city, _, _)| city.as_str()));
    }
    if let Event::SettingsPaste(text) = &ev { crate::settings_surface::paste(text.clone()); }
    if let Event::SettingsChanged(settings) = &ev {
        if settings.app_folders != notch.settings().app_folders {
            visus_platform::set_app_folders(settings.app_folders.clone());
        }
    }
    notch.handle(ev);
    let after = notch.track_key();
    if after != before {
        if let (Some(f), Some((title, artist, duration))) = (fetcher, after) {
            f.request(title, artist, duration);
        }
    }
}
