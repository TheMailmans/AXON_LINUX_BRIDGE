//! Metrics collection and performance monitoring for the desktop agent.
//!
//! This module provides thread-safe utilities for tracking request timing,
//! success/failure counts, and aggregated statistics. Metrics are designed to
//! add <5ms overhead per RPC and are safe to use across threads.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Tracks timing information for a single RPC request.
#[derive(Debug)]
pub struct RequestMetrics {
    /// Unique identifier for the request.
    pub request_id: String,
    start_time: Instant,
}

impl RequestMetrics {
    /// Creates a new metrics tracker with a freshly generated request ID.
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        }
    }

    /// Creates a tracker with a provided request ID (useful for tests).
    pub fn with_id(request_id: String) -> Self {
        Self {
            request_id,
            start_time: Instant::now(),
        }
    }

    /// Returns elapsed time since construction in milliseconds.
    pub fn elapsed_ms(&self) -> i64 {
        self.start_time.elapsed().as_millis() as i64
    }

    /// Returns elapsed time since construction in microseconds.
    pub fn elapsed_us(&self) -> i64 {
        self.start_time.elapsed().as_micros() as i64
    }
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated metrics for the bridge instance.
#[derive(Clone, Debug, Default)]
pub struct BridgeMetrics {
    total_requests: Arc<AtomicU64>,
    failed_requests: Arc<AtomicU64>,
    total_response_time_ms: Arc<AtomicU64>,
}

impl BridgeMetrics {
    /// Creates a new metrics collector.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a successful request and tracks execution time.
    pub fn record_success(&self, duration_ms: i64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms
            .fetch_add(duration_ms as u64, Ordering::Relaxed);
    }

    /// Records a failed request and tracks execution time.
    pub fn record_failure(&self, duration_ms: i64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms
            .fetch_add(duration_ms as u64, Ordering::Relaxed);
    }

    /// Returns the total number of handled requests.
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }

    /// Returns the number of failed requests.
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }

    /// Returns the average response time (ms) for all requests.
    pub fn avg_response_time_ms(&self) -> f32 {
        let total = self.total_requests();
        if total == 0 {
            return 0.0;
        }
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);
        total_time as f32 / total as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn request_metrics_tracks_elapsed_time() {
        let metrics = RequestMetrics::new();
        thread::sleep(Duration::from_millis(5));
        assert!(metrics.elapsed_ms() >= 5);
        assert!(metrics.elapsed_us() >= 5_000);
    }

    #[test]
    fn bridge_metrics_records_success() {
        let metrics = BridgeMetrics::new();
        metrics.record_success(100);
        metrics.record_success(200);

        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.failed_requests(), 0);
        assert_eq!(metrics.avg_response_time_ms(), 150.0);
    }

    #[test]
    fn bridge_metrics_records_failure() {
        let metrics = BridgeMetrics::new();
        metrics.record_success(100);
        metrics.record_failure(50);

        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.failed_requests(), 1);
        assert_eq!(metrics.avg_response_time_ms(), 75.0);
    }
}
