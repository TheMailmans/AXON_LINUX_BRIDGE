//! Health monitoring utilities for the desktop agent.
//!
//! Collects CPU usage, memory consumption, uptime, and basic process metrics
//! exposed through the HealthCheck RPC.

use std::time::Instant;

use sysinfo::{Pid, System};

/// Snapshot of current bridge health metrics.
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: i64,
    pub uptime_seconds: i64,
}

/// Tracks bridge process health over time.
#[derive(Debug)]
pub struct HealthMonitor {
    system: System,
    process_pid: Pid,
    start_time: Instant,
}

impl HealthMonitor {
    /// Creates a new health monitor instance.
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        let process_pid = Pid::from_u32(std::process::id());

        Self {
            system,
            process_pid,
            start_time: Instant::now(),
        }
    }

    /// Returns current health status for the bridge process.
    pub fn status(&mut self) -> HealthStatus {
        self.system.refresh_process(self.process_pid);

        let (cpu_usage, memory_mb) = self
            .system
            .process(self.process_pid)
            .map(|process| (process.cpu_usage(), process.memory() as i64 / 1024 / 1024))
            .unwrap_or((0.0, 0));

        HealthStatus {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_mb,
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
        }
    }

    /// Returns true when the bridge is considered healthy.
    pub fn is_healthy(&mut self) -> bool {
        let status = self.status();
        status.cpu_usage_percent < 85.0 && status.memory_usage_mb < 1024
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_initializes() {
        let monitor = HealthMonitor::new();
        assert!(monitor.start_time.elapsed().as_secs() < 1);
    }

    #[test]
    fn status_returns_non_negative_values() {
        let mut monitor = HealthMonitor::new();
        let status = monitor.status();
        assert!(status.cpu_usage_percent >= 0.0);
        assert!(status.memory_usage_mb >= 0);
        assert!(status.uptime_seconds >= 0);
    }

    #[test]
    fn healthy_threshold_default() {
        let mut monitor = HealthMonitor::new();
        assert!(monitor.is_healthy());
    }
}
