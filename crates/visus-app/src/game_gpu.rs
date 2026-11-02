//! GPU readings are local to the host running Visus. To read a cloud rig, Visus must run
//! inside that rig; a streaming client's GPU counters cannot describe the remote host.
//! NVAPI uses the public nvapi.h/nvapi_lite_common.h layouts and nvapi_interface.h IDs.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Metrics {
    pub usage: Option<f32>,
    pub temperature: Option<f32>,
    pub used_gb: Option<f32>,
    pub total_gb: Option<f32>,
}

impl Metrics {
    fn available(self) -> bool {
        self != Self::default()
    }
}

fn number(text: &str) -> Option<f32> {
    text.trim().parse::<f32>().ok().filter(|value| value.is_finite() && *value >= 0.0)
}

fn select(adapters: impl Iterator<Item = Metrics>) -> Metrics {
    adapters
        .filter(|metrics| metrics.available())
        .max_by(|left, right| {
            left.total_gb
                .unwrap_or(0.0)
                .total_cmp(&right.total_gb.unwrap_or(0.0))
                .then_with(|| left.used_gb.unwrap_or(0.0).total_cmp(&right.used_gb.unwrap_or(0.0)))
        })
        .unwrap_or_default()
}

fn parse_smi(text: &str) -> Metrics {
    select(text.lines().filter_map(|line| {
        let fields: Vec<_> = line.split(',').collect();
        if fields.len() != 4 {
            return None;
        }
        Some(Metrics {
            usage: number(fields[0]).filter(|value| *value <= 100.0),
            temperature: number(fields[1]).filter(|value| *value <= 150.0),
            used_gb: number(fields[2]).map(|value| value / 1024.0),
            total_gb: number(fields[3]).filter(|value| *value > 0.0).map(|value| value / 1024.0),
        })
    }))
}

fn command_output(command: &mut Command, deadline: Instant) -> Option<String> {
    if Instant::now() >= deadline {
        return None;
    }
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(command, 0x0800_0000);
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.by_ref().take(128 * 1024 + 1).read_to_end(&mut bytes).ok()?;
        (bytes.len() <= 128 * 1024).then_some(bytes)
    });
    let success = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.success(),
            Ok(None) if Instant::now() < deadline => std::thread::park_timeout(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break false;
            }
        }
    };
    if !success {
        return None;
    }
    while !reader.is_finished() {
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
    let bytes = reader.join().ok()??;
    Some(decode(&bytes))
}

fn decode(bytes: &[u8]) -> String {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.iter().skip(1).step_by(2).take(256).filter(|byte| **byte == 0).count() > 16 {
        let bytes = bytes.strip_prefix(&[0xff, 0xfe]).unwrap_or(bytes);
        String::from_utf16_lossy(
            &bytes
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect::<Vec<_>>(),
        )
    } else {
        String::from_utf8_lossy(bytes).trim_start_matches('\u{feff}').to_owned()
    }
}

fn nvidia(deadline: Instant) -> Metrics {
    let mut candidates = Vec::new();
    if let Some(path) = std::env::var_os("NVIDIA_SMI_PATH") {
        candidates.push(std::path::PathBuf::from(path));
    }
    #[cfg(windows)]
    {
        if let Some(root) = std::env::var_os("SystemRoot") {
            candidates.push(std::path::PathBuf::from(root).join("System32/nvidia-smi.exe"));
        }
        if let Some(root) = std::env::var_os("ProgramW6432").or_else(|| std::env::var_os("ProgramFiles")) {
            candidates.push(std::path::PathBuf::from(root).join("NVIDIA Corporation/NVSMI/nvidia-smi.exe"));
        }
    }
    candidates.push(std::path::PathBuf::from("nvidia-smi"));
    for path in candidates {
        if path.is_absolute() && !path.is_file() {
            continue;
        }
        let mut command = Command::new(path);
        command.args([
            "--query-gpu=utilization.gpu,temperature.gpu,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ]);
        if let Some(text) = command_output(&mut command, deadline) {
            let metrics = parse_smi(&text);
            if metrics.available() {
                return metrics;
            }
        }
    }
    Metrics::default()
}

pub(super) fn poll() -> Metrics {
    #[cfg(windows)]
    {
        let deadline = Instant::now() + Duration::from_secs(5);
        let primary = native::poll();
        if primary.usage.is_some() && primary.used_gb.is_some() && primary.total_gb.is_some() {
            return primary;
        }
        let metrics = fallback(primary, || nvidia(Instant::now() + Duration::from_secs(1)));
        if metrics.usage.is_some() && metrics.used_gb.is_some() {
            return metrics;
        }
        fallback(metrics, || windows_counters(deadline))
    }
    #[cfg(not(windows))]
    {
        fallback(nvidia(Instant::now() + Duration::from_secs(3)), amd)
    }
}

fn fallback(primary: Metrics, secondary: impl FnOnce() -> Metrics) -> Metrics {
    let secondary = secondary();
    let score = |metrics: Metrics| {
        (
            metrics.usage.is_some(),
            [metrics.temperature, metrics.used_gb, metrics.total_gb]
                .iter()
                .filter(|value| value.is_some())
                .count(),
        )
    };
    if score(secondary) > score(primary) { secondary } else { primary }
}

#[cfg(any(windows, test))]
fn counter_adapter(name: &str) -> Option<String> {
    let name = name.to_ascii_lowercase();
    let start = name.find("luid_")?;
    let physical = name[start..].find("_phys_")? + start + 6;
    let end = physical + name[physical..].bytes().take_while(u8::is_ascii_digit).count();
    (end > physical).then(|| name[start..end].to_owned())
}

#[cfg(any(windows, test))]
fn parse_counters(text: &str) -> Metrics {
    use std::collections::BTreeMap;
    let mut adapters: BTreeMap<String, Metrics> = BTreeMap::new();
    let mut engines: BTreeMap<(String, String), f32> = BTreeMap::new();
    for block in text.replace('\r', "").split("\n\n") {
        let fields: BTreeMap<_, _> = block
            .lines()
            .filter_map(|line| {
                let (key, value) = line.trim().split_once('=')?;
                Some((key.trim().to_ascii_lowercase(), value.trim()))
            })
            .collect();
        let Some(name) = fields.get("name") else { continue };
        let Some(adapter) = counter_adapter(name) else { continue };
        if let Some(usage) = fields.get("dedicatedusage").and_then(|value| number(value)) {
            adapters.entry(adapter.clone()).or_default().used_gb = Some(usage / 1073741824.0);
        }
        if let Some(usage) = fields.get("utilizationpercentage").and_then(|value| number(value)) {
            let name = name.to_ascii_lowercase();
            let Some((_, engine)) = name.split_once("engtype_") else { continue };
            if engine.is_empty() {
                continue;
            }
            *engines.entry((adapter, engine.to_owned())).or_default() += usage;
        }
    }
    for ((adapter, _), usage) in engines {
        let metrics = adapters.entry(adapter).or_default();
        metrics.usage = Some(metrics.usage.unwrap_or(0.0).max(usage.min(100.0)));
    }
    select(adapters.into_values())
}

#[cfg(windows)]
fn windows_counters(deadline: Instant) -> Metrics {
    let mut text = String::new();
    for (class, properties) in [
        (
            "Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory",
            "Name,DedicatedUsage",
        ),
        (
            "Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine",
            "Name,UtilizationPercentage",
        ),
    ] {
        let mut command = Command::new("wmic.exe");
        command.args(["path", class, "get", properties, "/format:list"]);
        if let Some(output) = command_output(&mut command, deadline.min(Instant::now() + Duration::from_secs(1))) {
            text.push_str(&output);
            text.push_str("\n\n");
        }
    }
    let metrics = parse_counters(&text);
    if metrics.usage.is_some() {
        return metrics;
    }
    let mut command = Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        concat!(
            "$ErrorActionPreference='SilentlyContinue'; ",
            "Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUAdapterMemory | ",
            "ForEach-Object { 'Name='+$_.Name; 'DedicatedUsage='+$_.DedicatedUsage; '' }; ",
            "Get-CimInstance Win32_PerfFormattedData_GPUPerformanceCounters_GPUEngine | ",
            "ForEach-Object { 'Name='+$_.Name; 'UtilizationPercentage='+$_.UtilizationPercentage; '' }"
        ),
    ]);
    fallback(metrics, || {
        command_output(&mut command, deadline)
            .map(|text| parse_counters(&text))
            .unwrap_or_default()
    })
}

#[cfg(target_os = "linux")]
fn amd() -> Metrics {
    let Ok(cards) = std::fs::read_dir("/sys/class/drm") else {
        return Metrics::default();
    };
    select(cards.flatten().filter_map(|card| {
        if !card
            .file_name()
            .to_string_lossy()
            .strip_prefix("card")
            .is_some_and(|suffix| !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return None;
        }
        let device = card.path().join("device");
        let read = |name| std::fs::read_to_string(device.join(name)).ok().and_then(|text| number(&text));
        let temperature = std::fs::read_dir(device.join("hwmon")).ok().and_then(|entries| {
            entries.flatten().find_map(|entry| {
                std::fs::read_to_string(entry.path().join("temp1_input"))
                    .ok()
                    .and_then(|text| number(&text))
            })
        });
        Some(Metrics {
            usage: read("gpu_busy_percent").filter(|value| *value <= 100.0),
            temperature: temperature.map(|value| value / 1000.0),
            used_gb: read("mem_info_vram_used").map(|value| value / 1073741824.0),
            total_gb: read("mem_info_vram_total")
                .filter(|value| *value > 0.0)
                .map(|value| value / 1073741824.0),
        })
    }))
}

#[cfg(all(not(windows), not(target_os = "linux")))]
fn amd() -> Metrics {
    Metrics::default()
}

#[cfg(any(windows, test))]
mod native {
    use super::Metrics;
    use std::ffi::c_void;

    type Query = unsafe extern "C" fn(u32) -> *const c_void;
    type Handle = *mut c_void;
    type GetUsage = unsafe extern "C" fn(Handle, *mut Usage) -> i32;
    type GetThermal = unsafe extern "C" fn(Handle, u32, *mut Thermal) -> i32;
    type GetMemory = unsafe extern "C" fn(Handle, *mut Memory) -> i32;

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Domain {
        present: u32,
        percentage: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Usage {
        version: u32,
        flags: u32,
        domains: [Domain; 8],
    }

    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct Sensor {
        controller: i32,
        minimum: i32,
        maximum: i32,
        current: i32,
        target: i32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct Thermal {
        version: u32,
        count: u32,
        sensors: [Sensor; 3],
    }

    #[repr(C)]
    #[derive(Default)]
    struct Memory {
        version: u32,
        dedicated: u32,
        available: u32,
        system: u32,
        shared: u32,
        current_available: u32,
    }

    fn version<T>(revision: u32) -> u32 {
        std::mem::size_of::<T>() as u32 | revision << 16
    }

    unsafe fn resolve<T: Copy>(query: Query, id: u32) -> Option<T> {
        let address = unsafe { query(id) };
        if address.is_null() {
            return None;
        }
        assert_eq!(std::mem::size_of::<T>(), std::mem::size_of_val(&address));
        Some(unsafe { std::mem::transmute_copy(&address) })
    }

    unsafe fn sample(handle: Handle, query: Query) -> Metrics {
        let mut metrics = Metrics::default();
        if let Some(get) = unsafe { resolve::<GetUsage>(query, 0x60de_d2ed) } {
            let mut usage = Usage {
                version: version::<Usage>(1),
                ..Default::default()
            };
            if unsafe { get(handle, &mut usage) } == 0 {
                let gpu = usage.domains[0];
                if gpu.present & 1 != 0 && gpu.percentage <= 100 {
                    metrics.usage = Some(gpu.percentage as f32);
                }
            }
        }
        if let Some(get) = unsafe { resolve::<GetThermal>(query, 0xe364_0a56) } {
            let mut thermal = Thermal {
                version: version::<Thermal>(2),
                ..Default::default()
            };
            if unsafe { get(handle, 15, &mut thermal) } == 0 {
                metrics.temperature = thermal
                    .sensors
                    .iter()
                    .take(thermal.count as usize)
                    .find(|sensor| sensor.target == 1 && (-40..=150).contains(&sensor.current))
                    .map(|sensor| sensor.current as f32);
            }
        }
        if let Some(get) = unsafe { resolve::<GetMemory>(query, 0x07f9_b368) } {
            let mut memory = Memory {
                version: version::<Memory>(2),
                ..Default::default()
            };
            if unsafe { get(handle, &mut memory) } == 0 && memory.dedicated > 0 {
                metrics.total_gb = Some(memory.dedicated as f32 / 1048576.0);
                metrics.used_gb = memory
                    .dedicated
                    .checked_sub(memory.current_available)
                    .map(|used| used as f32 / 1048576.0);
            }
        }
        metrics
    }

    #[cfg(windows)]
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LoadLibraryExW(name: *const u16, file: Handle, flags: u32) -> Handle;
        fn GetProcAddress(module: Handle, name: *const u8) -> *const c_void;
        fn FreeLibrary(module: Handle) -> i32;
    }

    #[cfg(windows)]
    struct Library {
        module: Handle,
        query: Query,
        initialized: bool,
    }

    #[cfg(windows)]
    impl Drop for Library {
        fn drop(&mut self) {
            unsafe {
                if self.initialized {
                    if let Some(unload) = resolve::<unsafe extern "C" fn() -> i32>(self.query, 0xd22b_dd7e) {
                        unload();
                    }
                }
                FreeLibrary(self.module);
            }
        }
    }

    #[cfg(windows)]
    pub(super) fn poll() -> Metrics {
        unsafe {
            let name = if cfg!(target_pointer_width = "64") {
                "nvapi64.dll"
            } else {
                "nvapi.dll"
            };
            let name: Vec<_> = name.encode_utf16().chain(Some(0)).collect();
            let module = LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0x0000_0800);
            if module.is_null() {
                return Metrics::default();
            }
            let address = GetProcAddress(module, c"nvapi_QueryInterface".as_ptr().cast());
            if address.is_null() {
                FreeLibrary(module);
                return Metrics::default();
            }
            let mut library = Library {
                module,
                query: std::mem::transmute::<*const c_void, Query>(address),
                initialized: false,
            };
            let Some(initialize) = resolve::<unsafe extern "C" fn() -> i32>(library.query, 0x0150_e828) else {
                return Metrics::default();
            };
            if initialize() != 0 {
                return Metrics::default();
            }
            library.initialized = true;
            type Enumerate = unsafe extern "C" fn(*mut Handle, *mut u32) -> i32;
            let Some(enumerate) = resolve::<Enumerate>(library.query, 0xe5ac_921f) else {
                return Metrics::default();
            };
            let mut handles = [std::ptr::null_mut(); 64];
            let mut count = 0;
            if enumerate(handles.as_mut_ptr(), &mut count) != 0 {
                return Metrics::default();
            }
            super::select(
                handles
                    .into_iter()
                    .take(count as usize)
                    .filter(|handle| !handle.is_null())
                    .map(|handle| sample(handle, library.query)),
            )
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        unsafe extern "C" fn usage(handle: Handle, output: *mut Usage) -> i32 {
            let output = unsafe { &mut *output };
            assert_eq!(output.version, 0x0001_0048);
            output.domains[0] = Domain {
                present: u32::from(!handle.is_null()),
                percentage: 0,
            };
            0
        }

        unsafe extern "C" fn memory(_: Handle, output: *mut Memory) -> i32 {
            let output = unsafe { &mut *output };
            assert_eq!(output.version, 0x0002_0018);
            output.dedicated = 8 * 1048576;
            output.current_available = 6 * 1048576;
            0
        }

        unsafe extern "C" fn query(id: u32) -> *const c_void {
            match id {
                0x60de_d2ed => usage as *const c_void,
                0x07f9_b368 => memory as *const c_void,
                _ => std::ptr::null(),
            }
        }

        #[test]
        fn game_gpu_nvapi_layout_and_presence() {
            assert_eq!(version::<Thermal>(2), 0x0002_0044);
            let absent = unsafe { sample(std::ptr::null_mut(), query) };
            assert_eq!(absent.usage, None);
            assert_eq!(absent.temperature, None);
            assert_eq!(absent.used_gb, Some(2.0));
            assert_eq!(absent.total_gb, Some(8.0));
            let present = unsafe { sample(std::ptr::dangling_mut::<u8>().cast(), query) };
            assert_eq!(present.usage, Some(0.0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_gpu_missing_temperature_preserves_usage_and_memory() {
        let metrics = parse_smi("51, N/A, 2048, 8192");
        assert_eq!(
            metrics,
            Metrics {
                usage: Some(51.0),
                temperature: None,
                used_gb: Some(2.0),
                total_gb: Some(8.0)
            }
        );
        assert_eq!(parse_smi("N/A, [Not Supported], NaN, 0"), Metrics::default());
        assert_eq!(parse_smi("0, N/A, 0, 4096").usage, Some(0.0));
    }

    #[test]
    fn game_gpu_selects_one_adapter_without_combining_readings() {
        let metrics = parse_smi("90, 60, 1024, 4096\n20, N/A, 2048, 16384");
        assert_eq!(metrics.usage, Some(20.0));
        assert_eq!(metrics.temperature, None);
        assert_eq!(metrics.total_gb, Some(16.0));
    }

    #[test]
    fn game_gpu_decodes_windows_output() {
        let bytes: Vec<_> = "\u{feff}51, N/A, 2048, 8192".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(parse_smi(&decode(&bytes)).usage, Some(51.0));
    }

    #[test]
    fn game_gpu_counters_keep_adapter_and_engine_identity() {
        let metrics = parse_counters(concat!(
            "Name=luid_0x0_0x1_phys_0\r\r\nDedicatedUsage=2147483648\r\r\n\r\r\n",
            "Name=pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D\nUtilizationPercentage=30\n\n",
            "Name=pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D\nUtilizationPercentage=40\n\n",
            "Name=pid_1_luid_0x0_0x1_phys_0_eng_1_engtype_Copy\nUtilizationPercentage=50\n\n",
            "Name=luid_0x0_0x2_phys_0\nDedicatedUsage=1048576\n\n",
            "Name=pid_3_luid_0x0_0x2_phys_0_eng_0_engtype_3D\nUtilizationPercentage=99\n\n",
        ));
        assert_eq!(
            metrics,
            Metrics {
                usage: Some(70.0),
                used_gb: Some(2.0),
                ..Default::default()
            }
        );
        assert_eq!(parse_counters("Name=unknown\nDedicatedUsage=0"), Metrics::default());
        assert_eq!(
            parse_counters("Name=luid_x_phys_0\nTotalCommitted=123\nSharedUsage=456"),
            Metrics::default()
        );
    }

    #[test]
    fn game_gpu_fallback_retries_and_never_merges_adapters() {
        let partial = Metrics {
            temperature: Some(60.0),
            total_gb: Some(8.0),
            ..Default::default()
        };
        let usage = Metrics {
            usage: Some(25.0),
            ..Default::default()
        };
        assert_eq!(fallback(partial, || usage), usage);
        assert_eq!(fallback(usage, Metrics::default), usage);
        let mut attempts = [Metrics::default(), usage].into_iter();
        assert_eq!(fallback(Metrics::default(), || attempts.next().unwrap()), Metrics::default());
        assert_eq!(fallback(Metrics::default(), || attempts.next().unwrap()), usage);
    }

    #[test]
    fn game_gpu_command_child() {
        match std::env::var("VISUS_TEST_GPU_CHILD").as_deref() {
            Ok("spin") => loop {
                std::hint::spin_loop();
            },
            Ok("overflow") => {
                use std::io::Write;
                let _ = std::io::stdout().write_all(&vec![b'x'; 129 * 1024]);
            }
            _ => {}
        }
    }

    #[test]
    fn game_gpu_commands_are_bounded() {
        for mode in ["spin", "overflow"] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args(["--exact", "game::gpu::tests::game_gpu_command_child", "--nocapture"]);
            command.env("VISUS_TEST_GPU_CHILD", mode);
            let start = Instant::now();
            assert!(command_output(&mut command, start + Duration::from_millis(150)).is_none());
            assert!(start.elapsed() < Duration::from_secs(2));
        }
    }
}
