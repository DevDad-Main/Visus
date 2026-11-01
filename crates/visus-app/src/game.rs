//! Background sampler for the in-game HUD: CPU, RAM, and GPU where the machine exposes it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use sysinfo::System;
use visus_core::{Event, GameStats};
use visus_platform::Emit;

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
    let mut gpu: Option<(f32, f32)> = None;
    let mut smi_ok = true;
    let mut tick = 0u32;
    loop {
        std::thread::sleep(Duration::from_millis(1000));
        if !ACTIVE.load(Ordering::Relaxed) {
            tick = 0;
            continue;
        }
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        if tick % 2 == 0 {
            gpu = if smi_ok { nvidia().or_else(|| { smi_ok = false; None }) } else { None }.or_else(amd);
        }
        tick += 1;
        const GB: f32 = 1024.0 * 1024.0 * 1024.0;
        let (total, used) = (sys.total_memory() as f32, sys.used_memory() as f32);
        emit(Event::GameStats(GameStats {
            cpu: (sys.global_cpu_usage() / 100.0).clamp(0.0, 1.0),
            cpu_temp: cpu_temp(),
            gpu: gpu.map(|g| (g.0 / 100.0).clamp(0.0, 1.0)),
            gpu_temp: gpu.map(|g| g.1),
            ram: if total > 0.0 { (used / total).clamp(0.0, 1.0) } else { 0.0 },
            ram_used_gb: used / GB,
        }));
    }
}

/// (utilisation %, temperature C) from the NVIDIA driver tool, if installed.
fn nvidia() -> Option<(f32, f32)> {
    let mut cmd = std::process::Command::new("nvidia-smi");
    cmd.args(["--query-gpu=utilization.gpu,temperature.gpu", "--format=csv,noheader,nounits"]);
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000);
    let out = cmd.output().ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let mut it = text.lines().next()?.split(',').map(|v| v.trim().parse::<f32>());
    Some((it.next()?.ok()?, it.next()?.ok()?))
}

#[cfg(target_os = "linux")]
fn amd() -> Option<(f32, f32)> {
    for card in std::fs::read_dir("/sys/class/drm").ok()?.flatten() {
        let dev = card.path().join("device");
        let Ok(busy) = std::fs::read_to_string(dev.join("gpu_busy_percent")) else { continue };
        let temp = std::fs::read_dir(dev.join("hwmon")).ok()?.flatten().find_map(|h| std::fs::read_to_string(h.path().join("temp1_input")).ok())?;
        return Some((busy.trim().parse().ok()?, temp.trim().parse::<f32>().ok()? / 1000.0));
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn amd() -> Option<(f32, f32)> {
    None
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
