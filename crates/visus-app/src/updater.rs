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

#[cfg(not(windows))]
fn install_inner() -> Result<(), String> {
    let page = LATEST.lock().unwrap().clone().map(|r| r.page).ok_or("Check for updates first")?;
    visus_platform::open_url(&page);
    Err("Opened the release page".into())
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
