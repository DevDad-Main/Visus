use std::collections::{HashMap, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

const MAX_BYTES: u64 = 256 * 1024;
const MAX_SAMPLES: usize = 240;
const STALE: Duration = Duration::from_secs(5);

static CONFIG: Mutex<Option<PathBuf>> = Mutex::new(None);
static NOTE: Mutex<String> = Mutex::new(String::new());

pub fn set_frame_log(path: Option<&str>) {
    *CONFIG.lock().unwrap_or_else(|error| error.into_inner()) = path
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
}

pub fn frame_source_note() -> String {
    let message = NOTE.lock().unwrap_or_else(|error| error.into_inner());
    if message.is_empty() {
        "PresentMon CSV: waiting for active game sampling".into()
    } else {
        message.clone()
    }
}

fn note(message: &str) {
    message.clone_into(&mut NOTE.lock().unwrap_or_else(|error| error.into_inner()));
}

#[derive(Default)]
pub(super) struct Source {
    path: Option<PathBuf>,
    stamp: Option<(u64, SystemTime)>,
    last_sample: Option<Instant>,
    value: Option<(f32, f32)>,
    sample_end: u64,
}

impl Source {
    pub(super) fn poll(&mut self) -> Option<(f32, f32)> {
        let configured = CONFIG
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let path = configured.or_else(|| {
            std::env::var_os("VISUS_PRESENTMON_CSV")
                .filter(|path| !path.is_empty())
                .map(PathBuf::from)
        });
        if self.path != path {
            *self = Self {
                path,
                ..Self::default()
            };
        }
        let Some(path) = self.path.clone() else {
            note("PresentMon CSV is not configured");
            return None;
        };
        let result = self.read(&path);
        match result {
            Ok(value) => value,
            Err(message) => {
                note(message);
                None
            }
        }
    }

    fn read(&mut self, path: &std::path::Path) -> Result<Option<(f32, f32)>, &'static str> {
        let mut file = std::fs::File::open(path).map_err(|_| "PresentMon CSV cannot be opened")?;
        let metadata = file
            .metadata()
            .map_err(|_| "PresentMon CSV metadata unavailable")?;
        let modified = metadata
            .modified()
            .map_err(|_| "PresentMon CSV modification time unavailable")?;
        if SystemTime::now()
            .duration_since(modified)
            .unwrap_or_default()
            > STALE
        {
            return Err("PresentMon CSV is stale");
        }
        let stamp = (metadata.len(), modified);
        if self.stamp != Some(stamp) {
            if self
                .stamp
                .is_some_and(|(length, _)| metadata.len() < length)
            {
                self.value = None;
                self.last_sample = None;
                self.sample_end = 0;
            }
            let mut head = Vec::new();
            (&mut file)
                .take(32 * 1024)
                .read_to_end(&mut head)
                .map_err(|_| "PresentMon CSV read failed")?;
            let header_end = head
                .iter()
                .position(|byte| *byte == b'\n')
                .ok_or("PresentMon CSV header incomplete")?
                + 1;
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(&head[..header_end]);
            let header = reader
                .records()
                .next()
                .and_then(Result::ok)
                .ok_or("PresentMon CSV header invalid")?;
            let columns = Columns::new(&header)
                .ok_or("PresentMon CSV needs CPUFrameTime, MsBetweenPresents, or FPS")?;
            let start = (metadata.len().saturating_sub(MAX_BYTES)).max(header_end as u64);
            file.seek(SeekFrom::Start(start))
                .map_err(|_| "PresentMon CSV seek failed")?;
            let mut bytes = Vec::new();
            file.take(MAX_BYTES)
                .read_to_end(&mut bytes)
                .map_err(|_| "PresentMon CSV read failed")?;
            let skip = if start > header_end as u64 {
                bytes
                    .iter()
                    .position(|byte| *byte == b'\n')
                    .map(|end| end + 1)
                    .unwrap_or(bytes.len())
            } else {
                0
            };
            let (value, sample_end) = representative(&header, &columns, &bytes[skip..])
                .ok_or("PresentMon CSV has no complete valid frame samples")?;
            let sample_end = start + skip as u64 + sample_end as u64;
            if sample_end != self.sample_end {
                self.last_sample = Some(Instant::now());
            }
            self.value = Some(value);
            self.sample_end = sample_end;
            self.stamp = Some(stamp);
        }
        if self.last_sample.is_none_or(|time| time.elapsed() > STALE) {
            return Err("PresentMon CSV has no recent valid frame samples");
        }
        note("PresentMon CSV: rolling mean of dominant PID/swapchain (not foreground matched)");
        Ok(self.value)
    }
}

struct Columns {
    milliseconds: Option<usize>,
    fps: Option<usize>,
    pid: Option<usize>,
    swapchain: Option<usize>,
    application: Option<usize>,
}

impl Columns {
    fn new(header: &csv::StringRecord) -> Option<Self> {
        let find = |name: &str| {
            header.iter().position(|field| {
                field
                    .trim()
                    .trim_start_matches('\u{feff}')
                    .eq_ignore_ascii_case(name)
            })
        };
        let columns = Self {
            milliseconds: find("CPUFrameTime").or_else(|| find("MsBetweenPresents")),
            fps: find("FPS"),
            pid: find("ProcessID").or_else(|| find("PID")),
            swapchain: find("SwapChainAddress").or_else(|| find("SwapChain")),
            application: find("Application").or_else(|| find("ApplicationName")),
        };
        (columns.milliseconds.is_some() || columns.fps.is_some()).then_some(columns)
    }

    fn sample(&self, row: &csv::StringRecord) -> Option<f32> {
        let number = |index: usize| {
            row.get(index)?
                .trim()
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite() && *value > 0.0)
        };
        let milliseconds = self
            .milliseconds
            .and_then(number)
            .or_else(|| self.fps.and_then(number).map(|fps| 1000.0 / fps))?;
        (0.1..=1000.0)
            .contains(&milliseconds)
            .then_some(milliseconds)
    }
}

fn representative(
    header: &csv::StringRecord,
    columns: &Columns,
    bytes: &[u8],
) -> Option<((f32, f32), usize)> {
    let mut groups: HashMap<(String, String), VecDeque<(f32, usize)>> = HashMap::new();
    let mut rows = VecDeque::new();
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(bytes);
    let mut previous_end = 0;
    let mut row = csv::StringRecord::new();
    while reader.read_record(&mut row).ok()? {
        let end = reader.position().byte() as usize;
        let record_bytes = &bytes[previous_end..end];
        previous_end = end;
        let terminated = record_bytes.ends_with(b"\n")
            || (record_bytes.ends_with(b"\r") && bytes.get(end) == Some(&b'\n'));
        if !terminated
            || record_bytes.iter().filter(|byte| **byte == b'"').count() % 2 != 0
            || row.len() != header.len()
        {
            continue;
        }
        let Some(milliseconds) = columns.sample(&row) else {
            continue;
        };
        let identity = columns
            .pid
            .or(columns.application)
            .and_then(|index| row.get(index))
            .unwrap_or("")
            .trim()
            .to_owned();
        let swapchain = columns
            .swapchain
            .and_then(|index| row.get(index))
            .unwrap_or("")
            .trim()
            .to_owned();
        rows.push_back(((identity, swapchain), (milliseconds, end)));
        if rows.len() > MAX_SAMPLES {
            rows.pop_front();
        }
    }
    for (key, sample) in rows {
        groups.entry(key).or_default().push_back(sample);
    }
    let (_, samples) = groups
        .into_iter()
        .max_by(|(left_key, left), (right_key, right)| {
            left.len()
                .cmp(&right.len())
                .then_with(|| left_key.cmp(right_key))
        })?;
    let sample_end = samples.back()?.1;
    let mean = samples
        .iter()
        .map(|(milliseconds, _)| milliseconds)
        .sum::<f32>()
        / samples.len() as f32;
    Some(((1000.0 / mean, mean), sample_end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<(f32, f32)> {
        let end = text.find('\n').unwrap() + 1;
        let header = csv::Reader::from_reader(text.as_bytes())
            .headers()
            .unwrap()
            .clone();
        let columns = Columns::new(&header)?;
        representative(&header, &columns, &text.as_bytes()[end..]).map(|(value, _)| value)
    }

    #[test]
    fn game_presentmon_quoted_dominant_stream_and_mean() {
        let value = parse("Application,ProcessID,SwapChainAddress,CPUFrameTime\r\n\"game, \"\"deluxe\"\".exe\",12,0x1,10\r\nother,13,0x2,1\r\ngame,12,0x1,30\r\ngame,12,0x3,2\r\n").unwrap();
        assert_eq!(value, (50.0, 20.0));
    }

    #[test]
    fn game_presentmon_headers_fallback_and_invalid_values() {
        assert_eq!(
            parse("MsBetweenPresents,FPS\n20,900\n0,50\nNaN,NaN\n-1,-1\n"),
            Some((50.0, 20.0))
        );
        assert_eq!(parse("FPS\n100\n"), Some((100.0, 10.0)));
        assert_eq!(parse("GPUTime\n10\n"), None);
        assert_eq!(parse("CPUFrameTime\n0\nNaN\ninf\n-10\n"), None);
    }

    #[test]
    fn game_presentmon_incomplete_and_multiline_records() {
        assert_eq!(
            parse("Application,MsBetweenPresents\n\"game\nname\",20\n\"partial\n"),
            Some((50.0, 20.0))
        );
        assert_eq!(parse("CPUFrameTime\n20\n10"), Some((50.0, 20.0)));
    }

    #[test]
    fn game_presentmon_rolling_samples() {
        let text = format!("CPUFrameTime\n100\n{}", "10\n".repeat(MAX_SAMPLES));
        assert_eq!(parse(&text), Some((100.0, 10.0)));
    }

    #[test]
    fn game_presentmon_file_lifecycle() {
        let path =
            std::env::temp_dir().join(format!("visus-presentmon-{}.csv", std::process::id()));
        std::fs::write(&path, "ProcessID,CPUFrameTime\n1,20\n").unwrap();
        let mut source = Source::default();
        assert_eq!(source.read(&path).unwrap(), Some((50.0, 20.0)));
        assert_eq!(source.read(&path).unwrap(), Some((50.0, 20.0)));
        source.last_sample = Some(Instant::now() - STALE - Duration::from_secs(1));
        assert!(source.read(&path).is_err());
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"1,NaN\n")
            .unwrap();
        assert!(source.read(&path).is_err());
        std::fs::write(&path, "FPS\n100\n").unwrap();
        assert_eq!(source.read(&path).unwrap(), Some((100.0, 10.0)));
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_modified(SystemTime::now() - STALE - Duration::from_secs(1))
            .unwrap();
        assert!(source.read(&path).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(source.read(&path).is_err());
    }

    #[test]
    fn game_presentmon_runtime_configuration() {
        let path = std::env::temp_dir().join(format!(
            "visus-presentmon-config-{}.csv",
            std::process::id()
        ));
        std::fs::write(&path, "FPS\n80\n").unwrap();
        super::super::set_frame_log(path.to_str());
        let mut source = Source::default();
        assert_eq!(source.poll(), Some((80.0, 12.5)));
        assert!(frame_source_note().contains("dominant"));
        assert!(!super::super::frame_source_note().is_empty());
        source.last_sample = Some(Instant::now() - STALE - Duration::from_secs(1));
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"NaN\n")
            .unwrap();
        assert_eq!(source.poll(), None);
        assert_eq!(source.poll(), None);
        let missing = path.with_extension("missing");
        super::super::set_frame_log(missing.to_str());
        assert_eq!(source.poll(), None);
        assert!(frame_source_note().contains("cannot be opened"));
        super::super::set_frame_log(None);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn game_presentmon_bounded_tail_and_partial_append() {
        let path =
            std::env::temp_dir().join(format!("visus-presentmon-tail-{}.csv", std::process::id()));
        let text = format!(
            "Application,PID,SwapChainAddress,CPUFrameTime\n{}{}game,2,0x2,5",
            "old,1,0x1,40\n".repeat(MAX_BYTES as usize / 10),
            "\"game, deluxe\",2,0x2,10\n".repeat(MAX_SAMPLES)
        );
        std::fs::write(&path, text).unwrap();
        let mut source = Source::default();
        assert_eq!(source.read(&path).unwrap(), Some((100.0, 10.0)));
        use std::io::Write;
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"\n")
            .unwrap();
        let (_, milliseconds) = source.read(&path).unwrap().unwrap();
        assert!((milliseconds - (2390.0 + 5.0) / 240.0).abs() < 0.001);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn game_presentmon_other_process_does_not_refresh_selected_stream() {
        let header = csv::StringRecord::from(vec!["PID", "CPUFrameTime"]);
        let columns = Columns::new(&header).unwrap();
        let initial = representative(&header, &columns, b"1,20\n1,20\n").unwrap();
        let background = representative(&header, &columns, b"1,20\n1,20\n2,10\n").unwrap();
        assert_eq!(initial, background);
    }
}
