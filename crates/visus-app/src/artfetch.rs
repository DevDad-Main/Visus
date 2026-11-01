//! Finds cover art online (iTunes Search, no key needed) for tracks whose player supplied none.

use visus_core::Event;
use visus_platform::Emit;

fn artwork(title: &str, artist: &str) -> Option<std::sync::Arc<visus_core::Art>> {
    let mut t = crate::lyrics::clean(title);
    let mut a = crate::lyrics::clean(artist);
    // YouTube titles often read "Artist - Song".
    if let Some((x, y)) = t.clone().split_once(" - ") {
        a = x.trim().to_string();
        t = y.trim().to_string();
    }
    let term = format!("{a} {t}");
    let url = format!("https://itunes.apple.com/search?media=music&entity=song&limit=3&term={}", crate::lyrics::encode(term.trim()));
    let json: serde_json::Value = serde_json::from_slice(&crate::updater::http_get(&url).ok()?).ok()?;
    let want = t.to_lowercase();
    let hit = json["results"].as_array()?.iter().find(|r| {
        let name = r["trackName"].as_str().unwrap_or("").to_lowercase();
        !name.is_empty() && (name.contains(&want) || want.contains(&name))
    })?;
    let img = hit["artworkUrl100"].as_str()?.replace("100x100", "600x600");
    visus_platform::decode_art(&crate::updater::http_get(&img).ok()?)
}

pub fn fetch(title: String, artist: String, emit: Emit) {
    std::thread::spawn(move || {
        let key = format!("{title}\u{1}{artist}");
        emit(Event::ArtFallback { key, art: artwork(&title, &artist) });
    });
}
