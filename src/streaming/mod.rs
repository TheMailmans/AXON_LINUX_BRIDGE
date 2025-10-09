/*!
 * Streaming Module
 * 
 * Manages real-time video and audio streaming from desktop capture.
 * Coordinates capture, encoding, and transmission.
 */

pub mod stream_manager;

pub use stream_manager::{StreamManager, StreamConfig, StreamStats};

use std::time::{Duration, Instant};

/// Streaming quality configuration
#[derive(Debug, Clone)]
pub struct QualityConfig {
    /// Target frame rate (frames per second)
    pub fps: u32,
    /// Video quality preset
    pub quality: crate::video::Quality,
    /// Maximum frame queue size (backpressure threshold)
    pub max_queue_size: usize,
    /// Enable adaptive bitrate
    pub adaptive_bitrate: bool,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            fps: 30,
            quality: crate::video::Quality::Medium,
            max_queue_size: 60, // 2 seconds at 30 FPS
            adaptive_bitrate: true,
        }
    }
}

/// Frame timing information
#[derive(Debug, Clone)]
pub struct FrameTiming {
    /// When the frame was captured
    pub capture_time: Instant,
    /// When encoding started
    pub encode_start: Option<Instant>,
    /// When encoding completed
    pub encode_end: Option<Instant>,
    /// When transmission started
    pub transmit_start: Option<Instant>,
    /// When transmission completed
    pub transmit_end: Option<Instant>,
}

impl FrameTiming {
    pub fn new() -> Self {
        Self {
            capture_time: Instant::now(),
            encode_start: None,
            encode_end: None,
            transmit_start: None,
            transmit_end: None,
        }
    }
    
    /// Get capture-to-encode latency
    pub fn capture_latency(&self) -> Option<Duration> {
        self.encode_end.map(|end| end.duration_since(self.capture_time))
    }
    
    /// Get encoding duration
    pub fn encode_duration(&self) -> Option<Duration> {
        if let (Some(start), Some(end)) = (self.encode_start, self.encode_end) {
            Some(end.duration_since(start))
        } else {
            None
        }
    }
    
    /// Get total latency (capture to transmission complete)
    pub fn total_latency(&self) -> Option<Duration> {
        self.transmit_end.map(|end| end.duration_since(self.capture_time))
    }
}

/// Frame drop reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// Queue full (backpressure)
    QueueFull,
    /// Encoding too slow
    EncodingSlow,
    /// Network congestion
    NetworkCongestion,
    /// Explicitly dropped (e.g., on pause)
    Explicit,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_config_default() {
        let config = QualityConfig::default();
        assert_eq!(config.fps, 30);
        assert_eq!(config.max_queue_size, 60);
        assert!(config.adaptive_bitrate);
    }

    #[test]
    fn test_frame_timing() {
        let mut timing = FrameTiming::new();
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        timing.encode_start = Some(Instant::now());
        
        std::thread::sleep(std::time::Duration::from_millis(10));
        timing.encode_end = Some(Instant::now());
        
        let duration = timing.encode_duration().unwrap();
        assert!(duration.as_millis() >= 10);
        assert!(duration.as_millis() < 50);
    }
}
