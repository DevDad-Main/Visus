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
