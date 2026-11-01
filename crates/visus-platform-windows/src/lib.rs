//! OS integration for Visus. Real implementation on Windows, inert stubs elsewhere
//! so the rest of the workspace builds and runs on any platform.

use std::sync::Arc;
use visus_core::Event;

pub type Emit = Arc<dyn Fn(Event) + Send + Sync + 'static>;

#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::*;
