//! Manual update check and install from GitHub Releases. Nothing runs unless the user clicks.

use std::sync::Mutex;
use std::time::Duration;

use visus_core::{Event, UpdateStatus, is_newer};
use visus_platform::Emit;

const RELEASES: &str = "https://api.github.com/repos/DevDad-Main/Visus/releases?per_page=1";
const USER_AGENT: &str = concat!("Visus/", env!("CARGO_PKG_VERSION"), " (https://github.com/DevDad-Main/Visus)");

#[derive(Clone)]
#[cfg_attr(not(windows), allow(dead_code))]
struct Release {
    page: String,
    setup: Option<String>,
    setup_sha: Option<String>,
    #[cfg(target_os = "linux")]
    linux: Option<String>,
    #[cfg(target_os = "linux")]
    linux_sha: Option<String>,
}

static LATEST: Mutex<Option<Release>> = Mutex::new(None);

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(30))).user_agent(USER_AGENT).build().into()
}

/// Downloads a URL; falls back to the system `curl` (which trusts the OS certificate store) if the built-in client fails.
pub fn http_get(url: &str) -> Result<Vec<u8>, String> {
    let first = agent().get(url).call().map_err(|e| e.to_string()).and_then(|mut r| r.body_mut().read_to_vec().map_err(|e| e.to_string()));
    let Err(err) = first else { return first };
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-sSL", "--fail", "--max-time", "120", "-A", USER_AGENT, url]);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000);
    match cmd.output() {
        Ok(o) if o.status.success() => Ok(o.stdout),
        _ => Err(err),
    }
}

fn fetch_latest() -> Result<(String, Release), String> {
    let body = String::from_utf8(http_get(RELEASES).map_err(|e| format!("Update check failed: {e}"))?).map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let rel = json.get(0).ok_or("No releases found")?;
    let tag = rel["tag_name"].as_str().ok_or("Release has no tag")?.to_string();
    let asset = |name: &str| {
        rel["assets"].as_array()?.iter().find(|a| a["name"].as_str() == Some(name))?["browser_download_url"].as_str().map(String::from)
    };
    let release = Release {
        page: rel["html_url"].as_str().unwrap_or("https://github.com/DevDad-Main/Visus/releases").to_string(),
        setup: asset("Visus-Setup.exe"),
        setup_sha: asset("Visus-Setup.exe.sha256"),
        #[cfg(target_os = "linux")]
        linux: asset("Visus-linux-x64.tar.gz"),
        #[cfg(target_os = "linux")]
        linux_sha: asset("Visus-linux-x64.tar.gz.sha256"),
    };
    Ok((tag, release))
}

pub fn check(emit: Emit) {
    check_inner(emit, false);
}

/// Background check: reports only a newer release, never errors or "up to date".
pub fn check_silent(emit: Emit) {
    check_inner(emit, true);
}

fn check_inner(emit: Emit, silent: bool) {
    std::thread::spawn(move || {
        let status = match fetch_latest() {
            Ok((tag, rel)) => {
                let newer = is_newer(&tag, env!("CARGO_PKG_VERSION"));
                *LATEST.lock().unwrap() = Some(rel);
                if newer { UpdateStatus::Available { version: tag } } else { UpdateStatus::UpToDate }
            }
            Err(e) => UpdateStatus::Failed(e),
        };
        if silent && !matches!(status, UpdateStatus::Available { .. }) {
            return;
        }
        emit(Event::Update(status));
    });
}

pub fn install(emit: Emit) {
    std::thread::spawn(move || match install_inner() {
        // The installer has taken over and will restart Visus when it is done.
        Ok(()) => std::process::exit(0),
        Err(e) => emit(Event::Update(UpdateStatus::Failed(e))),
    });
}

#[cfg(not(any(windows, target_os = "linux")))]
fn install_inner() -> Result<(), String> {
    let page = LATEST.lock().unwrap().clone().map(|r| r.page).ok_or("Check for updates first")?;
    visus_platform::open_url(&page);
    Err("Opened the release page".into())
}

#[cfg(target_os = "linux")]
fn verify_checksum(bytes: &[u8], checksum: &[u8]) -> Result<(), String> {
    use sha2::{Digest, Sha256};
    let expected = String::from_utf8_lossy(checksum).split_whitespace().next().unwrap_or("").to_lowercase();
    let actual: String = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
    if actual != expected { return Err("Checksum mismatch, update not installed".into()); }
    Ok(())
}

#[cfg(target_os = "linux")]
fn replace_linux(executable: &std::path::Path, bytes: &[u8], restart: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let parent = executable.parent().ok_or("Executable has no parent directory")?;
    let staged = parent.join(format!(".visus-update-{}", std::process::id()));
    let backup = parent.join(format!(".visus-backup-{}", std::process::id()));
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).mode(0o755).open(&staged)
        .map_err(|error| format!("Cannot update this installation ({error}). Install Visus in a user-writable folder or update it with your package manager."))?;
    let result = (|| {
        file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|error| format!("Could not stage update: {error}"))?;
        drop(file);
        std::fs::hard_link(executable, &backup).map_err(|error| format!("Could not back up executable: {error}"))?;
        if let Err(error) = std::fs::rename(&staged, executable) {
            let _ = std::fs::remove_file(&backup);
            return Err(format!("Could not replace executable: {error}"));
        }
        if let Err(error) = restart() {
            std::fs::rename(&backup, executable).map_err(|restore| format!("{error}; restore failed: {restore}. Backup: {}", backup.display()))?;
            return Err(error);
        }
        let _ = std::fs::remove_file(&backup);
        Ok(())
    })();
    let _ = std::fs::remove_file(&staged);
    result
}

#[cfg(target_os = "linux")]
fn install_inner() -> Result<(), String> {
    if std::env::consts::ARCH != "x86_64" { return Err("Automatic Linux updates currently require x86_64".into()); }
    let release = LATEST.lock().unwrap().clone().ok_or("Check for updates first")?;
    let (archive_url, checksum_url) = release.linux.zip(release.linux_sha)
        .ok_or("Linux download is not ready yet. Try again when release packaging finishes.")?;
    let archive = http_get(&archive_url)?;
    verify_checksum(&archive, &http_get(&checksum_url)?)?;
    let temporary = std::env::temp_dir().join(format!("visus-update-{}", std::process::id()));
    std::fs::create_dir(&temporary).map_err(|error| format!("Could not prepare update: {error}"))?;
    let result = (|| {
        let archive_path = temporary.join("release.tar.gz");
        std::fs::write(&archive_path, archive).map_err(|error| error.to_string())?;
        let output = std::process::Command::new("tar").args(["-xzOf"]).arg(&archive_path).arg("visus/visus")
            .output().map_err(|error| format!("Could not extract update (tar required): {error}"))?;
        if !output.status.success() { return Err("Linux archive does not contain the Visus executable".into()); }
        if !output.stdout.starts_with(b"\x7fELF\x02\x01") { return Err("Linux update is not a 64-bit ELF executable".into()); }
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        replace_linux(&executable, &output.stdout, || {
            std::process::Command::new(&executable).arg("--restart-after").arg(std::process::id().to_string())
                .stdin(std::process::Stdio::null()).spawn().map(|_| ())
                .map_err(|error| format!("Could not restart Visus: {error}"))
        })
    })();
    let _ = std::fs::remove_dir_all(temporary);
    result
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn linux_checksum_rejects_modified_downloads() {
        use sha2::{Digest, Sha256};
        let bytes = b"verified update";
        let digest: String = Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect();
        assert!(verify_checksum(bytes, format!("{digest}  archive.tar.gz").as_bytes()).is_ok());
        assert!(verify_checksum(b"modified", digest.as_bytes()).is_err());
    }

    #[test]
    fn linux_replacement_rolls_back_if_restart_fails() {
        let directory = std::env::temp_dir().join(format!("visus-updater-test-{}", std::process::id()));
        std::fs::create_dir(&directory).unwrap();
        let executable = directory.join("visus");
        std::fs::write(&executable, b"old").unwrap();
        assert!(replace_linux(&executable, b"new", || Err("restart failed".into())).is_err());
        assert_eq!(std::fs::read(&executable).unwrap(), b"old");
        replace_linux(&executable, b"new", || Ok(())).unwrap();
        assert_eq!(std::fs::read(&executable).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(windows)]
fn install_inner() -> Result<(), String> {
    use sha2::{Digest, Sha256};

    let rel = LATEST.lock().unwrap().clone().ok_or("Check for updates first")?;

    if let Some((setup_url, sha_url)) = rel.setup.as_ref().zip(rel.setup_sha.as_ref()) {
        let setup = http_get(setup_url)?;
        let expected = String::from_utf8_lossy(&http_get(sha_url)?)
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_lowercase();
        let actual: String = Sha256::digest(&setup).iter().map(|b| format!("{b:02x}")).collect();
        if actual != expected {
            return Err("Checksum mismatch, update not installed".into());
        }

        // Run the per-user installer rather than overwriting the running exe. This works even when
        // the current copy lives somewhere write-protected (for example Program Files) and never
        // asks for administrator rights.
        let setup_path = std::env::temp_dir().join(format!("Visus-Setup-{}.exe", std::process::id()));
        std::fs::write(&setup_path, &setup).map_err(|e| format!("Could not save the update: {e}"))?;

        let mut cmd = std::process::Command::new(&setup_path);
        cmd.args([
            "/VERYSILENT",
            "/SUPPRESSMSGBOXES",
            "/NORESTART",
            "/SP-",
            "/CLOSEAPPLICATIONS",
            "/FORCECLOSEAPPLICATIONS",
            "/MERGETASKS=!autostart,!desktopicon",
            "/RESTART=1",
        ]);
        std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000);
        if let Err(e) = cmd.spawn() {
            visus_platform::open_url(&rel.page);
            return Err(format!("Could not start the installer ({e}). Opened the release page."));
        }
        return Ok(());
    }

    visus_platform::open_url(&rel.page);
    Err("No Windows installer found in this release. Opened the release page.".into())
}
