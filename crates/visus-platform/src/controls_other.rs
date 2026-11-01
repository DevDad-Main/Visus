use visus_core::{Controls, Radio};

pub fn init_thread() {}
pub fn read() -> Controls {
    Controls::default()
}
pub fn set_volume(_v: f32) {}
pub fn set_mute(_m: bool) {}
pub fn set_brightness(_v: f32) {}
pub fn set_radio(_r: Radio, _on: bool) {}

pub struct Meter;
impl Meter {
    pub fn open() -> Option<Meter> {
        None
    }
    pub fn peak(&self) -> Option<f32> {
        None
    }
}
