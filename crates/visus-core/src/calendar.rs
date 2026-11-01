/// One calendar entry; times are Unix seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct CalEvent {
    pub start: i64,
    pub end: i64,
    pub title: String,
    pub all_day: bool,
    /// First web link found in the event (video-call link, usually).
    pub url: Option<String>,
}
