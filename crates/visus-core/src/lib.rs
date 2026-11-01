//! Platform-independent notch state and animation.

mod anim;
mod event;
mod extras;
mod lyrics;
mod notch;
mod palette;
mod settings;
mod stats;
mod timer;
mod todo;

pub use timer::{FOCUS, Timer};
pub use anim::Spring;
pub use lyrics::{LyricLine, Lyrics, current_index, parse_lrc};
pub use event::{Art, Event, Hud, MediaCommand, MediaInfo};
pub use notch::{Alert, EyeState, Notch, Page, Rect, Shape};
pub use settings::{Rgb, Settings};
pub use palette::{Action, App, AppKind, guess_kind, Command, Item, Kind, Outcome, Palette, MAX_RESULTS};
pub use extras::{UpdateStatus, Weather, is_newer, weather_label};
pub use settings::EyeStyle;
pub use stats::Stats;
pub use todo::{Todo, parse_task};
