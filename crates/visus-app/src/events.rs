use visus_core::{Event, Notch};

use crate::lyrics::Fetcher;

/// Applies an OS or worker event to the notch and starts a lyrics lookup when the track changed.
pub fn apply(notch: &mut Notch, fetcher: Option<&Fetcher>, ev: Event) {
    let before = notch.track_key();
    notch.handle(ev);
    let after = notch.track_key();
    if after != before {
        if let (Some(f), Some((title, artist, duration))) = (fetcher, after) {
            f.request(title, artist, duration);
        }
    }
}
