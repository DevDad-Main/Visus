//! Platform-independent notch state and animation.

mod anim;
mod calendar;
mod controls;
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
pub use calendar::CalEvent;
pub use controls::{Controls, Radio};
pub use lyrics::{LyricLine, Lyrics, current_index, parse_lrc};
pub use event::{Art, Event, Hud, MediaCommand, MediaInfo};
pub use notch::{VIZ_BARS, Alert, EyeMood, EyeState, Notch, Page, Rect, Shape};
pub use settings::{Rgb, Settings};
pub use palette::{Action, App, AppKind, guess_kind, Command, Item, Kind, Outcome, Palette, MAX_RESULTS};
pub use extras::{UpdateStatus, Weather, is_newer, weather_label};
pub use settings::{ClockStyle, EyeStyle, GameStyle};
pub use notch::{GAME_HIST, GAME_CARD_HEAD, GAME_CARD_ROW, GAME_CARD_W, GAME_ITEM_W, WIN_GRID};
pub use stats::{GameItem, GameStats, Stats};
pub use todo::{Todo, parse_task};
