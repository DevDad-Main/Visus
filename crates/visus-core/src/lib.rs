//! Platform-independent notch state and animation.

mod anim;
mod event;
mod notch;
mod timer;

pub use timer::{FOCUS, Timer};
pub use anim::Spring;
pub use event::{Event, Hud, MediaCommand, MediaInfo};
pub use notch::{Alert, Notch, Page, Rect, Shape};
