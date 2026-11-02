//! Background sampler for the in-game HUD: CPU, RAM, and GPU where the machine exposes it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use sysinfo::System;
use visus_core::{Event, GameStats};
use visus_platform::Emit;

#[path = "game_gpu.rs"]
mod gpu;

#[cfg(target_os = "linux")]
use std::io::{Read, Seek, SeekFrom};
#[cfg(target_os = "linux")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "linux")]
use std::time::{Instant, SystemTime};

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Called every frame; the sampler thread starts on first use and idles while no game is running.
pub fn set_active(on: bool, emit: &Emit) {
    static START: std::sync::Once = std::sync::Once::new();
    ACTIVE.store(on, Ordering::Relaxed);
    if on {
        let emit = emit.clone();
        START.call_once(move || {
            std::thread::spawn(move || run(emit));
        });
    }
}

fn run(emit: Emit) {
    let Ok(mut sys) = System::new() else { return };
    let mut gpu = gpu::Metrics::default();
    let mut fps = FpsSource::new();
    let mut tick = 0u32;
    loop {
        std::thread::sleep(Duration::from_millis(1000));
        if !ACTIVE.load(Ordering::Relaxed) {
            tick = 0;
            fps.reset();
            continue;
        }
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        if tick % 2 == 0 {
            gpu = gpu::poll();
        }
        tick += 1;
        let frame = fps.poll();
        const GB: f32 = 1024.0 * 1024.0 * 1024.0;
        let (total, used) = (sys.total_memory() as f32, sys.used_memory() as f32);
        emit(Event::GameStats(GameStats {
            cpu: (sys.global_cpu_usage() / 100.0).clamp(0.0, 1.0),
            cpu_temp: cpu_temp(),
            gpu: gpu.usage.map(|usage| usage / 100.0),
            gpu_temp: gpu.temperature,
            gpu_vram_used_gb: gpu.used_gb,
            gpu_vram_total_gb: gpu.total_gb,
            ram: if total > 0.0 { (used / total).clamp(0.0, 1.0) } else { 0.0 },
            ram_used_gb: used / GB,
            fps: frame.map(|(fps, _)| fps),
            frame_ms: frame.map(|(_, ms)| ms),
        }));
    }
}

/// Frame-rate telemetry for the HUD. On Linux this follows the most recent MangoHud benchmark
/// log; when no log is being written the card shows `--` instead of inventing a frame rate.
struct FpsSource {
    #[cfg(target_os = "linux")]
    tail: Option<LogTail>,
    #[cfg(target_os = "linux")]
    next_scan: Option<Instant>,
    #[cfg(target_os = "linux")]
    dirs: Vec<PathBuf>,
}

impl FpsSource {
    fn new() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self { tail: None, next_scan: None, dirs: log_dirs() }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self {}
        }
    }

    fn reset(&mut self) {
        #[cfg(target_os = "linux")]
        {
            self.tail = None;
            self.next_scan = None;
        }
    }

    fn poll(&mut self) -> Option<(f32, f32)> {
        #[cfg(target_os = "linux")]
        {
            // A log that stopped growing is from a finished session; look for a fresh one.
            if self.tail.as_ref().is_some_and(LogTail::stale) {
                self.tail = None;
            }
            if self.tail.is_none() {
                self.discover();
            }
            self.tail.as_mut().and_then(LogTail::read_latest)
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn discover(&mut self) {
        let now = Instant::now();
        if self.next_scan.is_some_and(|t| now < t) {
            return;
        }
        self.next_scan = Some(now + Duration::from_secs(2));
        let mut newest: Option<(SystemTime, PathBuf)> = None;
        for dir in &self.dirs {
            let Ok(entries) = std::fs::read_dir(dir) else { continue };
            for entry in entries.flatten() {
                let path = entry.path();
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if !name.to_ascii_lowercase().ends_with(".csv") || name.to_ascii_lowercase().ends_with("_summary.csv") {
                    continue;
                }
                let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else { continue };
                let Ok(age) = SystemTime::now().duration_since(modified) else { continue };
                if age > Duration::from_secs(30) || !is_mangohud_log(&path) {
                    continue;
                }
                if newest.as_ref().is_none_or(|(t, _)| modified > *t) {
                    newest = Some((modified, path));
                }
            }
        }
        if let Some((_, path)) = newest {
            if self.tail.as_ref().is_none_or(|t| t.path != path) {
                self.tail = Some(LogTail::new(path));
            }
        }
    }
}

#[cfg(target_os = "linux")]
struct LogTail {
    path: PathBuf,
    offset: u64,
    last: Option<(f32, f32)>,
}

#[cfg(target_os = "linux")]
impl LogTail {
    fn new(path: PathBuf) -> Self {
        Self { path, offset: 0, last: None }
    }

    fn stale(&self) -> bool {
        std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_some_and(|age| age > Duration::from_secs(15))
    }

    /// Read only the newly appended complete lines, keeping the last frame rate seen.
    fn read_latest(&mut self) -> Option<(f32, f32)> {
        let len = std::fs::metadata(&self.path).ok()?.len();
        if len < self.offset {
            self.offset = 0;
        }
        if len == self.offset {
            return self.last;
        }
        // A log can already contain a whole session; only the tail matters.
        let start = if self.offset == 0 && len > 64 * 1024 { len - 64 * 1024 } else { self.offset };
        let mut file = std::fs::File::open(&self.path).ok()?;
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut bytes = Vec::new();
        file.take(len - start).read_to_end(&mut bytes).ok()?;
        let text = String::from_utf8_lossy(&bytes);
        let Some(end) = text.rfind('\n') else { return self.last };
        let complete = &text[..=end];
        let found = complete.lines().rev().find_map(parse_mangohud_line);
        self.offset = start + complete.len() as u64;
        if found.is_some() {
            self.last = found;
        }
        self.last
    }
}

/// MangoHud benchmark CSV rows are `fps,frametime,cpu_load,...`.
#[cfg(target_os = "linux")]
fn parse_mangohud_line(line: &str) -> Option<(f32, f32)> {
    let mut columns = line.split(',');
    let fps = columns.next()?.trim().parse::<f32>().ok()?;
    let frame_ms = columns.next()?.trim().parse::<f32>().ok()?;
    if !(0.1..=1000.0).contains(&fps) || !(0.1..=1000.0).contains(&frame_ms) {
        return None;
    }
    Some((fps, frame_ms))
}

/// MangoHud benchmark logs start with system info followed by a `fps,frametime,...` header.
#[cfg(target_os = "linux")]
fn is_mangohud_log(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else { return false };
    let mut buf = [0u8; 8192];
    let Ok(read) = file.read(&mut buf) else { return false };
    String::from_utf8_lossy(&buf[..read]).contains("fps,frametime")
}

/// Places MangoHud can be configured to write benchmark logs.
#[cfg(target_os = "linux")]
fn log_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.clone());
        for sub in ["Documents", "Desktop", "Videos", "Games"] {
            dirs.push(home.join(sub));
        }
    }
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("MangoHud"));
    }
    if let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR") {
        dirs.push(PathBuf::from(runtime));
    }
    dirs.push(PathBuf::from("/tmp"));
    if let Some(configured) = configured_log_dir() {
        dirs.insert(0, configured);
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

#[cfg(target_os = "linux")]
fn configured_log_dir() -> Option<PathBuf> {
    if let Ok(config) = std::env::var("MANGOHUD_CONFIG") {
        if let Some(dir) = config.split(',').find_map(|v| v.trim().strip_prefix("output_folder=")).filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(dir));
        }
    }
    let config = dirs::config_dir()?.join("MangoHud").join("MangoHud.conf");
    let text = std::fs::read_to_string(config).ok()?;
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("output_folder=")?.trim();
        (!value.is_empty()).then(|| PathBuf::from(value))
    })
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn parses_mangohud_rows() {
        assert_eq!(parse_mangohud_line("fps,frametime,cpu_load"), None);
        let (fps, ms) = parse_mangohud_line("143.7,6.96,12.0,1.0").unwrap();
        assert!((fps - 143.7).abs() < 0.01);
        assert!((ms - 6.96).abs() < 0.01);
        assert_eq!(parse_mangohud_line("0,0,1"), None);
    }

    #[test]
    fn tails_complete_log_rows_only() {
        let path = std::env::temp_dir().join(format!("visus-mangohud-{}.csv", std::process::id()));
        std::fs::write(&path, "fps,frametime,cpu_load\n60,16.6,10\n").unwrap();
        let mut tail = LogTail::new(path.clone());
        assert_eq!(tail.read_latest(), Some((60.0, 16.6)));
        std::fs::write(&path, "").unwrap();
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"120,8.3,12\n90,11.1").unwrap();
        assert_eq!(tail.read_latest(), Some((120.0, 8.3)));
        file.write_all(b",14\n").unwrap();
        assert_eq!(tail.read_latest(), Some((90.0, 11.1)));
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(target_os = "linux")]
fn cpu_temp() -> Option<f32> {
    for h in std::fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let name = std::fs::read_to_string(h.path().join("name")).unwrap_or_default();
        if matches!(name.trim(), "coretemp" | "k10temp" | "zenpower") {
            if let Ok(t) = std::fs::read_to_string(h.path().join("temp1_input")) {
                return t.trim().parse::<f32>().ok().map(|v| v / 1000.0);
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn cpu_temp() -> Option<f32> {
    None
}
