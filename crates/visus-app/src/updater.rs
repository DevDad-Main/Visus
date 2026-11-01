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
    zip: Option<String>,
    sha: Option<String>,
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
        zip: asset("Visus-windows-x64.zip"),
        sha: asset("Visus.exe.sha256"),
    };
    Ok((tag, release))
}

pub fn check(emit: Emit) {
    std::thread::spawn(move || {
        let status = match fetch_latest() {
            Ok((tag, rel)) => {
                let newer = is_newer(&tag, env!("CARGO_PKG_VERSION"));
                *LATEST.lock().unwrap() = Some(rel);
                if newer { UpdateStatus::Available { version: tag } } else { UpdateStatus::UpToDate }
            }
            Err(e) => UpdateStatus::Failed(e),
        };
        emit(Event::Update(status));
    });
}

pub fn install(emit: Emit) {
    std::thread::spawn(move || match install_inner() {
        // The new copy is already running.
        Ok(()) => std::process::exit(0),
        Err(e) => emit(Event::Update(UpdateStatus::Failed(e))),
    });
}

#[cfg(not(windows))]
fn install_inner() -> Result<(), String> {
    let page = LATEST.lock().unwrap().clone().map(|r| r.page).ok_or("Check for updates first")?;
    visus_platform::open_url(&page);
    Err("Opened the release page".into())
}

#[cfg(windows)]
fn install_inner() -> Result<(), String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let rel = LATEST.lock().unwrap().clone().ok_or("Check for updates first")?;
    let (zip_url, sha_url) = rel.zip.zip(rel.sha).ok_or("Release has no Windows build")?;
    let get = http_get;
    let zip_bytes = get(&zip_url)?;
    let expected = String::from_utf8_lossy(&get(&sha_url)?).split_whitespace().next().unwrap_or("").to_lowercase();

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).map_err(|e| e.to_string())?;
    let mut exe = Vec::new();
    archive.by_name("Visus.exe").map_err(|e| e.to_string())?.read_to_end(&mut exe).map_err(|e| e.to_string())?;

    let actual: String = Sha256::digest(&exe).iter().map(|b| format!("{b:02x}")).collect();
    if actual != expected {
        return Err("Checksum mismatch, update not installed".into());
    }

    // A running exe can be renamed but not overwritten.
    let current = std::env::current_exe().map_err(|e| e.to_string())?;
    let old = current.with_extension("old");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&current, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::write(&current, &exe) {
        let _ = std::fs::rename(&old, &current);
        return Err(e.to_string());
    }
    std::process::Command::new(&current).spawn().map_err(|e| e.to_string())?;
    Ok(())
}
