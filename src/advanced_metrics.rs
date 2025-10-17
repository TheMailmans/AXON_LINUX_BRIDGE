//! Advanced metrics and observability for v2.7.
//!
//! Comprehensive metrics collection, histograms, percentile tracking,
//! and distributed tracing support.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::time::Instant;

/// Histogram for tracking latency distributions
#[derive(Debug)]
pub struct Histogram {
    buckets: Vec<AtomicU64>,
    min: AtomicU64,
    max: AtomicU64,
    sum: AtomicU64,
}

impl Histogram {
    /// Create new histogram with predefined buckets (ms)
    pub fn new() -> Self {
        let mut buckets = Vec::new();
        // Buckets: 1, 5, 10, 25, 50, 100, 250, 500, 1000, 2500, 5000+
        for _ in 0..11 {
            buckets.push(AtomicU64::new(0));
        }
        
        Self {
            buckets,
            min: AtomicU64::new(u64::MAX),
            max: AtomicU64::new(0),
            sum: AtomicU64::new(0),
        }
    }

    /// Record observation (in milliseconds)
    pub fn record(&self, value_ms: u64) {
        // Update min/max
        let mut current_min = self.min.load(Ordering::Relaxed);
        while value_ms < current_min {
            match self.min.compare_exchange(
                current_min,
                value_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        let mut current_max = self.max.load(Ordering::Relaxed);
        while value_ms > current_max {
            match self.max.compare_exchange(
                current_max,
                value_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        self.sum.fetch_add(value_ms, Ordering::Relaxed);

        // Find bucket
        let bucket_idx = match value_ms {
            0..=1 => 0,
            2..=5 => 1,
            6..=10 => 2,
            11..=25 => 3,
            26..=50 => 4,
            51..=100 => 5,
            101..=250 => 6,
            251..=500 => 7,
            501..=1000 => 8,
            1001..=2500 => 9,
            _ => 10,
        };

        if bucket_idx < self.buckets.len() {
            self.buckets[bucket_idx].fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get percentile (0-100)
    pub fn percentile(&self, p: usize) -> u64 {
        let total: u64 = self.buckets.iter().map(|b| b.load(Ordering::Relaxed)).sum();
        if total == 0 {
            return 0;
        }

        let target = (total * p as u64) / 100;
        let mut count = 0u64;

        for (i, bucket) in self.buckets.iter().enumerate() {
            count += bucket.load(Ordering::Relaxed);
            if count >= target {
                return match i {
                    0 => 1,
                    1 => 5,
                    2 => 10,
                    3 => 25,
                    4 => 50,
                    5 => 100,
                    6 => 250,
                    7 => 500,
                    8 => 1000,
                    9 => 2500,
                    _ => 5000,
                };
            }
        }

        5000
    }

    /// Get statistics
    pub fn stats(&self) -> HistogramStats {
        let total: u64 = self.buckets.iter().map(|b| b.load(Ordering::Relaxed)).sum();
        let min = self.min.load(Ordering::Relaxed);
        let max = self.max.load(Ordering::Relaxed);
        let sum = self.sum.load(Ordering::Relaxed);

        HistogramStats {
            count: total,
            min: if min == u64::MAX { 0 } else { min },
            max,
            avg: if total > 0 { sum / total } else { 0 },
            p50: self.percentile(50),
            p95: self.percentile(95),
            p99: self.percentile(99),
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram statistics
#[derive(Debug, Clone)]
pub struct HistogramStats {
    pub count: u64,
    pub min: u64,
    pub max: u64,
    pub avg: u64,
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
}

/// Request trace for distributed tracing
#[derive(Debug, Clone)]
pub struct RequestTrace {
    pub id: String,
    pub parent_id: Option<String>,
    pub operation: String,
    pub start_time: Instant,
    pub spans: Arc<RwLock<Vec<Span>>>,
}

/// Trace span
#[derive(Debug, Clone)]
pub struct Span {
    pub name: String,
    pub duration_ms: u64,
    pub tags: HashMap<String, String>,
}

impl RequestTrace {
    /// Create new trace
    pub fn new(id: String, operation: String) -> Self {
        Self {
            id,
            parent_id: None,
            operation,
            start_time: Instant::now(),
            spans: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add span
    pub async fn add_span(&self, span: Span) {
        let mut spans = self.spans.write().await;
        spans.push(span);
    }

    /// Get total duration
    pub fn duration_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }
}

/// Advanced metrics collector
pub struct MetricsCollector {
    histograms: Arc<RwLock<HashMap<String, Arc<Histogram>>>>,
    counters: Arc<RwLock<HashMap<String, u64>>>,
    gauges: Arc<RwLock<HashMap<String, u64>>>,
    traces: Arc<RwLock<Vec<RequestTrace>>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            histograms: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            traces: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record histogram observation
    pub async fn record_histogram(&self, name: &str, value_ms: u64) {
        let mut histograms = self.histograms.write().await;
        let histogram = histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new()));
        histogram.record(value_ms);
    }

    /// Increment counter
    pub async fn increment_counter(&self, name: &str, delta: u64) {
        let mut counters = self.counters.write().await;
        *counters.entry(name.to_string()).or_insert(0) += delta;
    }

    /// Set gauge
    pub async fn set_gauge(&self, name: &str, value: u64) {
        let mut gauges = self.gauges.write().await;
        gauges.insert(name.to_string(), value);
    }

    /// Add trace
    pub async fn add_trace(&self, trace: RequestTrace) {
        let mut traces = self.traces.write().await;
        traces.push(trace);
        
        // Keep only last 1000 traces
        if traces.len() > 1000 {
            traces.remove(0);
        }
    }

    /// Get histogram stats
    pub async fn get_histogram_stats(&self, name: &str) -> Option<HistogramStats> {
        let histograms = self.histograms.read().await;
        histograms.get(name).map(|h| h.stats())
    }

    /// Get counter value
    pub async fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.read().await;
        *counters.get(name).unwrap_or(&0)
    }

    /// Get gauge value
    pub async fn get_gauge(&self, name: &str) -> u64 {
        let gauges = self.gauges.read().await;
        *gauges.get(name).unwrap_or(&0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_creation() {
        let hist = Histogram::new();
        let stats = hist.stats();
        assert_eq!(stats.count, 0);
    }

    #[test]
    fn test_histogram_record() {
        let hist = Histogram::new();
        hist.record(50);
        hist.record(100);
        
        let stats = hist.stats();
        assert_eq!(stats.count, 2);
    }

    #[test]
    fn test_histogram_min_max() {
        let hist = Histogram::new();
        hist.record(10);
        hist.record(100);
        hist.record(50);
        
        let stats = hist.stats();
        assert_eq!(stats.min, 10);
        assert_eq!(stats.max, 100);
    }

    #[test]
    fn test_histogram_percentile() {
        let hist = Histogram::new();
        for i in 1..=100 {
            hist.record(i);
        }
        
        let p50 = hist.percentile(50);
        assert!(p50 > 0);
    }

    #[test]
    fn test_request_trace() {
        let trace = RequestTrace::new("trace1".to_string(), "get_frame".to_string());
        assert_eq!(trace.id, "trace1");
        assert_eq!(trace.operation, "get_frame");
    }

    #[test]
    fn test_request_trace_duration() {
        let trace = RequestTrace::new("trace1".to_string(), "op".to_string());
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        let duration = trace.duration_ms();
        assert!(duration >= 10);
    }

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_counter("test").await, 0);
    }

    #[tokio::test]
    async fn test_metrics_collector_histogram() {
        let collector = MetricsCollector::new();
        collector.record_histogram("latency", 50).await;
        
        let stats = collector.get_histogram_stats("latency").await;
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().count, 1);
    }

    #[tokio::test]
    async fn test_metrics_collector_counter() {
        let collector = MetricsCollector::new();
        collector.increment_counter("requests", 5).await;
        
        let count = collector.get_counter("requests").await;
        assert_eq!(count, 5);
    }

    #[tokio::test]
    async fn test_metrics_collector_gauge() {
        let collector = MetricsCollector::new();
        collector.set_gauge("temperature", 75).await;
        
        let value = collector.get_gauge("temperature").await;
        assert_eq!(value, 75);
    }

    #[tokio::test]
    async fn test_metrics_collector_trace() {
        let collector = MetricsCollector::new();
        let trace = RequestTrace::new("t1".to_string(), "op".to_string());
        collector.add_trace(trace).await;
    }

    #[test]
    fn test_histogram_average() {
        let hist = Histogram::new();
        hist.record(10);
        hist.record(20);
        hist.record(30);
        
        let stats = hist.stats();
        assert_eq!(stats.avg, 20);
    }

    #[test]
    fn test_histogram_p95() {
        let hist = Histogram::new();
        for i in 1..=100 {
            hist.record(i);
        }
        
        let p95 = hist.percentile(95);
        assert!(p95 > 0);
    }

    #[test]
    fn test_histogram_p99() {
        let hist = Histogram::new();
        for i in 1..=1000 {
            hist.record(i);
        }
        
        let p99 = hist.percentile(99);
        assert!(p99 > 0);
    }
}
