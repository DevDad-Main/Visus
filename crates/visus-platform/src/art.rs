use std::sync::Arc;

use visus_core::Art;

const SIZE: u32 = 160;

/// Wraps already-decoded square RGBA pixels (e.g. a Windows icon).
#[allow(dead_code)]
pub fn from_rgba(size: u32, rgba: Vec<u8>) -> Arc<Art> {
    let id = rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3));
    Arc::new(Art { size, rgba, id })
}

/// Decodes PNG/JPEG cover bytes into a centre-cropped square thumbnail.
#[allow(dead_code)]
pub fn decode(bytes: &[u8]) -> Option<Arc<Art>> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    let side = w.min(h);
    if side == 0 {
        return None;
    }
    let square = image::imageops::crop_imm(&img, (w - side) / 2, (h - side) / 2, side, side).to_image();
    let small = image::imageops::resize(&square, SIZE, SIZE, image::imageops::FilterType::Triangle);
    let rgba = small.into_raw();
    let id = rgba.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ *b as u64).wrapping_mul(0x0100_0000_01b3));
    Some(Arc::new(Art { size: SIZE, rgba, id }))
}
