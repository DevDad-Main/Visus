//! Watches the Downloads folder for in-progress browser downloads and reports their combined speed.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

use visus_core::Event;
use visus_platform::Emit;

const TEMP_EXT: [&str; 5] = ["crdownload", "part", "partial", "download", "opdownload"];

pub fn start(emit: Emit) {
    std::thread::Builder::new()
        .name("visus-downloads".into())
        .spawn(move || watch(emit))
        .ok();
}

fn watch(emit: Emit) {
    let Some(dir) = dirs::download_dir() else { return };
    let mut seen: HashMap<PathBuf, (u64, Instant)> = HashMap::new();
    let mut active = false;
    loop {
        std::thread::sleep(Duration::from_millis(1500));
        let mut now_files = HashMap::new();
        let mut rate = 0.0f64;
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for e in entries.flatten() {
            let p = e.path();
            let is_temp = p.extension().and_then(|x| x.to_str()).is_some_and(|x| TEMP_EXT.contains(&x.to_lowercase().as_str()));
            if !is_temp {
                continue;
            }
            let Ok(md) = e.metadata() else { continue };
            let size = md.len();
            if let Some((prev, at)) = seen.get(&p) {
                if size > *prev {
                    rate += (size - prev) as f64 / at.elapsed().as_secs_f64().max(0.2);
                }
            }
            now_files.insert(p, (size, Instant::now()));
        }
        let finished = seen.keys().any(|k| !now_files.contains_key(k));
        seen = now_files;
        if rate > 0.0 {
            emit(Event::Download(Some(rate as u64)));
            active = true;
        } else if active {
            emit(Event::Download(None));
            active = false;
            if finished {
                if let Some(name) = newest_recent(&dir) {
                    emit(Event::DownloadDone(name));
                }
            }
        }
    }
}

/// The most recently written regular file, if it changed within the last few seconds.
fn newest_recent(dir: &std::path::Path) -> Option<String> {
    let now = SystemTime::now();
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            let age = now.duration_since(md.modified().ok()?).ok()?;
            let temp = e.path().extension().and_then(|x| x.to_str()).is_some_and(|x| TEMP_EXT.contains(&x.to_lowercase().as_str()));
            (md.is_file() && !temp && age < Duration::from_secs(10)).then(|| (age, e.file_name().to_string_lossy().into_owned()))
        })
        .min_by_key(|(age, _)| *age)
        .map(|(_, n)| n)
}
