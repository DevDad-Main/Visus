/// Latest system readings shown on the Stats page.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// 0.0..=1.0
    pub cpu: f32,
    /// 0.0..=1.0
    pub ram: f32,
    pub ram_used_gb: f32,
    pub ram_total_gb: f32,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
}

/// Live readings for the in-game HUD; `None` means the machine does not expose that value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GameStats {
    /// 0.0..=1.0
    pub cpu: f32,
    pub cpu_temp: Option<f32>,
    pub gpu: Option<f32>,
    pub gpu_temp: Option<f32>,
    /// 0.0..=1.0
    pub ram: f32,
    pub ram_used_gb: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameItem {
    Cpu,
    CpuTemp,
    Gpu,
    GpuTemp,
    Ram,
}
