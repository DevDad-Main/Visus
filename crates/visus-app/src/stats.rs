use std::time::{Duration, Instant};

use sysinfo::{Networks, System};
use visus_core::Stats;

/// Reads CPU, memory and network throughput; keep one around so rates are measured between calls.
pub struct Sampler {
    sys: System,
    nets: Networks,
    last: Instant,
    cpu: f32,
    stats: Stats,
}

impl Sampler {
    pub fn new() -> Option<Self> {
        Some(Self { sys: System::new().ok()?, nets: Networks::new_with_refreshed_list().ok()?, last: Instant::now(), cpu: 0.0, stats: Stats::default() })
    }

    /// Returns the latest readings; measures at most once a second so rates cover a full window.
    pub fn sample(&mut self) -> Stats {
        let elapsed = self.last.elapsed();
        if elapsed < Duration::from_millis(900) {
            return self.stats;
        }
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.nets.refresh(true);
        self.last = Instant::now();
        // After a long pause (page was closed) the counters span the whole gap; only re-baseline.
        if elapsed > Duration::from_secs(3) {
            return self.stats;
        }

        let secs = elapsed.as_secs_f64();
        let (rx, tx) = self.nets.values().fold((0u64, 0u64), |(r, t), n| (r + n.received(), t + n.transmitted()));
        let smooth = |prev: u64, now: f64| (prev as f64 + (now - prev as f64) * 0.5) as u64 / 10_240 * 10_240;

        const GB: f32 = 1024.0 * 1024.0 * 1024.0;
        self.cpu += (self.sys.global_cpu_usage() / 100.0 - self.cpu) * 0.4;
        let total = self.sys.total_memory() as f32;
        let used = self.sys.used_memory() as f32;
        self.stats = Stats {
            cpu: ((self.cpu * 100.0).round() / 100.0).clamp(0.0, 1.0),
            ram: if total > 0.0 { ((used / total * 100.0).round() / 100.0).clamp(0.0, 1.0) } else { 0.0 },
            ram_used_gb: used / GB,
            ram_total_gb: total / GB,
            rx_bytes_per_sec: smooth(self.stats.rx_bytes_per_sec, rx as f64 / secs),
            tx_bytes_per_sec: smooth(self.stats.tx_bytes_per_sec, tx as f64 / secs),
        };
        self.stats
    }
}
