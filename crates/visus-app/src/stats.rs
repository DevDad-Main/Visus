use std::time::Instant;

use sysinfo::{Networks, System};
use visus_core::Stats;

/// Reads CPU, memory and network throughput; keep one around so rates are measured between calls.
pub struct Sampler {
    sys: System,
    nets: Networks,
    last: Instant,
}

impl Sampler {
    pub fn new() -> Option<Self> {
        Some(Self { sys: System::new().ok()?, nets: Networks::new_with_refreshed_list().ok()?, last: Instant::now() })
    }

    pub fn sample(&mut self) -> Stats {
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.nets.refresh(true);

        let secs = self.last.elapsed().as_secs_f64().max(0.05);
        self.last = Instant::now();
        let (rx, tx) = self
            .nets
            .values()
            .fold((0u64, 0u64), |(r, t), n| (r + n.received(), t + n.transmitted()));

        const GB: f32 = 1024.0 * 1024.0 * 1024.0;
        let total = self.sys.total_memory() as f32;
        let used = self.sys.used_memory() as f32;
        Stats {
            cpu: (self.sys.global_cpu_usage() / 100.0).clamp(0.0, 1.0),
            ram: if total > 0.0 { (used / total).clamp(0.0, 1.0) } else { 0.0 },
            ram_used_gb: used / GB,
            ram_total_gb: total / GB,
            rx_bytes_per_sec: (rx as f64 / secs) as u64,
            tx_bytes_per_sec: (tx as f64 / secs) as u64,
        }
    }
}
