use std::cell::RefCell;
use std::time::{Duration, Instant};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct Usage {
    pub cpu: Option<f32>,
    pub memory_mb: Option<f32>,
}

struct Sampler {
    system: System,
    pid: Pid,
    sampled: Option<Instant>,
    usage: Usage,
}

thread_local! {
    static SAMPLER: RefCell<Option<Sampler>> = const { RefCell::new(None) };
}

pub fn sample() -> Usage {
    SAMPLER.with_borrow_mut(|slot| {
        if slot.is_none() {
            let (Ok(system), Ok(pid)) = (System::new(), sysinfo::get_current_pid()) else { return Usage::default() };
            *slot = Some(Sampler { system, pid, sampled: None, usage: Usage::default() });
        }
        let sampler = slot.as_mut().unwrap();
        if sampler.sampled.is_some_and(|time| time.elapsed() < Duration::from_secs(1)) { return sampler.usage; }
        let warm = sampler.sampled.is_some();
        sampler.system.refresh_processes_specifics(ProcessesToUpdate::Some(&[sampler.pid]), true,
            ProcessRefreshKind::nothing().with_cpu().with_memory());
        sampler.usage = sampler.system.process(sampler.pid).map_or(Usage::default(), |process| Usage {
            cpu: warm.then(|| (process.cpu_usage() / std::thread::available_parallelism().map_or(1, usize::from) as f32).clamp(0.0, 100.0)),
            memory_mb: Some(process.memory() as f32 / 1_000_000.0),
        });
        sampler.sampled = Some(Instant::now());
        sampler.usage
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn own_process_usage_is_available_and_rate_limited() {
        let initial = sample();
        assert!(initial.memory_mb.is_some_and(|memory| memory.is_finite() && memory > 0.0));
        assert!(initial.cpu.is_none());
        assert!(sample() == initial);
        SAMPLER.with_borrow_mut(|slot| slot.as_mut().unwrap().sampled = Some(Instant::now() - Duration::from_secs(2)));
        let next = sample();
        assert!(next.cpu.is_some_and(|usage| (0.0..=100.0).contains(&usage)));
    }
}