#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radio {
    Wifi,
    Bluetooth,
}

/// Quick-control state reported by the OS; `None` radios are not present on this machine.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    pub volume: f32,
    pub muted: bool,
    pub has_volume: bool,
    /// Display brightness 0..1 when a controllable panel exists.
    pub brightness: Option<f32>,
    pub wifi: Option<bool>,
    pub bluetooth: Option<bool>,
}
