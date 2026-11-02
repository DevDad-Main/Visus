use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use super::frame_log;

struct Capture {
    process_id: u32,
    child: Child,
    csv: PathBuf,
}

static CAPTURE: Mutex<Option<Capture>> = Mutex::new(None);
static LAST_START: Mutex<Option<(u32, Instant)>> = Mutex::new(None);

pub fn set_active(active: bool, process_id: Option<u32>) {
    if frame_log::manual_log_configured() || std::env::var_os("VISUS_PRESENTMON_CSV").is_some() {
        stop();
        frame_log::set_auto_log(None);
        frame_log::set_source_note("Using manual PresentMon CSV override");
        return;
    }

    let target = active.then_some(process_id).flatten();
    let mut slot = CAPTURE.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(capture) = slot.as_mut() {
        if Some(capture.process_id) == target {
            match capture.child.try_wait() {
                Ok(None) => return,
                Ok(Some(status)) => {
                    *LAST_START.lock().unwrap_or_else(|error| error.into_inner()) =
                        Some((capture.process_id, Instant::now()));
                    let code = status.code().map_or_else(
                        || "unknown exit code".to_owned(),
                        |code| format!("exit code {code}"),
                    );
                    frame_log::set_source_note(&format!(
                        "Bundled PresentMon exited ({code}); checking capture permissions"
                    ));
                }
                Err(error) => frame_log::set_source_note(&format!(
                    "Could not check PresentMon capture: {error}"
                )),
            }
        }
        if let Some(capture) = slot.take() {
            stop_one(capture);
        }
        frame_log::set_auto_log(None);
    }

    let Some(process_id) = target else {
        frame_log::set_auto_log(None);
        frame_log::set_source_note("PresentMon capture idle; waiting for an active game");
        return;
    };
    if LAST_START
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .is_some_and(|(previous, at)| {
            previous == process_id && at.elapsed() < Duration::from_secs(8)
        })
    {
        return;
    }
    *LAST_START.lock().unwrap_or_else(|error| error.into_inner()) =
        Some((process_id, Instant::now()));

    match start(process_id) {
        Ok(capture) => {
            frame_log::set_auto_log(Some(capture.csv.clone()));
            *LAST_START.lock().unwrap_or_else(|error| error.into_inner()) = None;
            frame_log::set_source_note(
                "Bundled PresentMon capturing the active game's presentations",
            );
            *slot = Some(capture);
        }
        Err(error) => {
            frame_log::set_auto_log(None);
            frame_log::set_source_note(&error);
        }
    }
}

fn executable() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("VISUS_PRESENTMON_EXE").filter(|path| !path.is_empty()) {
        let path = PathBuf::from(path);
        return path
            .is_file()
            .then_some(path)
            .ok_or_else(|| format!("PresentMon executable not found: {}", path.display()));
    }
    if let Ok(app) = std::env::current_exe() {
        if let Some(directory) = app.parent() {
            let bundled = directory.join("PresentMon.exe");
            if bundled.is_file() {
                return Ok(bundled);
            }
        }
    }
    let path = PathBuf::from("PresentMon.exe");
    Err(format!(
        "Bundled PresentMon is missing; reinstall Visus or configure {}",
        path.display()
    ))
}

fn start(process_id: u32) -> Result<Capture, String> {
    let binary = executable()?;
    let directory = std::env::temp_dir().join("Visus");
    std::fs::create_dir_all(&directory)
        .map_err(|error| format!("Cannot create temporary capture directory: {error}"))?;
    let csv = capture_csv_path(process_id);
    let _ = std::fs::remove_file(&csv);
    let mut command = Command::new(binary);
    command
        .args(["--process_id", &process_id.to_string(), "--output_file"])
        .arg(&csv)
        .args([
            "--no_console_stats",
            "--v2_metrics",
            "--terminate_on_proc_exit",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    std::os::windows::process::CommandExt::creation_flags(&mut command, 0x0800_0000);
    let mut child = command.spawn().map_err(|error| {
        format!(
            "Could not start bundled PresentMon ({error}); capture permissions may be restricted"
        )
    })?;
    if let Some(mut stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            use std::io::Read;
            let mut output = String::new();
            let _ = stderr.by_ref().take(4096).read_to_string(&mut output);
            let message = output.trim();
            if !message.is_empty() {
                frame_log::set_source_note(&format!(
                    "PresentMon: {}",
                    message.chars().take(180).collect::<String>()
                ));
            }
        });
    }
    Ok(Capture {
        process_id,
        child,
        csv,
    })
}

fn capture_csv_path(process_id: u32) -> PathBuf {
    std::env::temp_dir().join("Visus").join(format!(
        "PresentMon-{}-{process_id}.csv",
        std::process::id()
    ))
}

fn stop_one(mut capture: Capture) {
    if capture.child.try_wait().ok().flatten().is_none() {
        let _ = capture.child.kill();
        let _ = capture.child.wait();
    }
    let directory = capture.csv.parent().map(PathBuf::from);
    let _ = std::fs::remove_file(capture.csv);
    if let Some(directory) = directory {
        let _ = std::fs::remove_dir(directory);
    }
}

fn stop() {
    if let Some(capture) = CAPTURE
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
    {
        stop_one(capture);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_path_is_unique_per_visus_and_game_process() {
        let first = capture_csv_path(123);
        let second = capture_csv_path(456);
        assert_ne!(first, second);
        assert_eq!(first.parent().unwrap(), std::env::temp_dir().join("Visus"));
    }
}
