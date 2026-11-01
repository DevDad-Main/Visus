//! Platform-independent notch state and animation.

mod anim;
mod event;
mod notch;
mod settings;
mod stats;
mod timer;

pub use timer::{FOCUS, Timer};
pub use anim::Spring;
pub use event::{Event, Hud, MediaCommand, MediaInfo};
pub use notch::{Alert, EyeState, Notch, Page, Rect, Shape};
pub use settings::{Rgb, Settings};
pub use stats::Stats;
