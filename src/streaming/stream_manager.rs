/*!
 * Stream Manager
 * 
 * Coordinates capture, encoding, and streaming of desktop sessions.
 */

use anyhow::{Result, Context};
use tokio::sync::{mpsc, broadcast};
use tokio::time::{interval, Duration};
use std::time::Instant;
use tracing::{info, warn, debug, error};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};
use tokio::task::JoinHandle;

use crate::capture::{CaptureConfig, CaptureMode};
use crate::video::{VideoEncoder, EncoderConfig, Quality, Codec, RawFrame, EncodedFrame};
use super::{QualityConfig, FrameTiming, DropReason};

#[cfg(target_os = "macos")]
use crate::capture::macos::MacOSCapturer;

/// Stream configuration
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Capture configuration
    pub capture: CaptureConfig,
    /// Video quality configuration
    pub quality: QualityConfig,
    /// Enable audio streaming
    pub enable_audio: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            capture: CaptureConfig::default(),
            quality: QualityConfig::default(),
            enable_audio: false,
        }
    }
}

/// Stream statistics
#[derive(Debug, Clone, Default)]
pub struct StreamStats {
    /// Total frames captured
    pub frames_captured: u64,
    /// Total frames encoded
    pub frames_encoded: u64,
    /// Total frames transmitted
    pub frames_transmitted: u64,
    /// Total frames dropped
    pub frames_dropped: u64,
    /// Average encoding time (ms)
    pub avg_encode_ms: f64,
    /// Average total latency (ms)
    pub avg_latency_ms: f64,
    /// Current bitrate (kbps)
    pub current_bitrate_kbps: u32,
    /// Stream uptime (seconds)
    pub uptime_secs: u64,
}

/// Stream manager handles the complete streaming pipeline
pub struct StreamManager {
    config: StreamConfig,
    is_streaming: Arc<AtomicBool>,
    frames_captured: Arc<AtomicU64>,
    frames_encoded: Arc<AtomicU64>,
    frames_transmitted: Arc<AtomicU64>,
    frames_dropped: Arc<AtomicU64>,
    /// Total encode time in microseconds
    total_encode_us: Arc<AtomicU64>,
    /// Total latency in microseconds  
    total_latency_us: Arc<AtomicU64>,
    start_time: Option<Instant>,
    /// Broadcast channel for encoded frames (for gRPC consumers)
    frame_tx: broadcast::Sender<EncodedFrame>,
    /// Task handles for graceful shutdown
    tasks: Option<(JoinHandle<Result<()>>, JoinHandle<Result<()>>, JoinHandle<Result<()>>)>,
    /// Actual frame dimensions (from display or config)
    frame_width: u32,
    frame_height: u32,
}

impl StreamManager {
    /// Create a new stream manager
    pub fn new(config: StreamConfig) -> Self {
        // Create broadcast channel for encoded frames (capacity: 10 frames)
        let (frame_tx, _) = broadcast::channel(10);
        
        // Get actual display dimensions (with fallback to 1920x1080)
        let (frame_width, frame_height) = crate::platform::get_primary_display_size()
            .unwrap_or_else(|e| {
                warn!("Failed to get display size, using 1920x1080: {}", e);
                (1920, 1080)
            });
        
        info!("Stream manager initialized with frame size: {}x{}", frame_width, frame_height);
        
        Self {
            config,
            is_streaming: Arc::new(AtomicBool::new(false)),
            frames_captured: Arc::new(AtomicU64::new(0)),
            frames_encoded: Arc::new(AtomicU64::new(0)),
            frames_transmitted: Arc::new(AtomicU64::new(0)),
            frames_dropped: Arc::new(AtomicU64::new(0)),
            total_encode_us: Arc::new(AtomicU64::new(0)),
            total_latency_us: Arc::new(AtomicU64::new(0)),
            start_time: None,
            frame_tx,
            tasks: None,
            frame_width,
            frame_height,
        }
    }
    
    /// Subscribe to frame stream (for gRPC consumers)
    pub fn subscribe(&self) -> broadcast::Receiver<EncodedFrame> {
        self.frame_tx.subscribe()
    }
    
    /// Start streaming
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting stream manager");
        
        if self.is_streaming.load(Ordering::SeqCst) {
            warn!("Stream already running");
            return Ok(());
        }
        
        self.is_streaming.store(true, Ordering::SeqCst);
        self.start_time = Some(Instant::now());
        
        // Create channels for pipeline
        let (capture_tx, capture_rx) = mpsc::channel::<(RawFrame, FrameTiming)>(
            self.config.quality.max_queue_size
        );
        let (encode_tx, encode_rx) = mpsc::channel::<(EncodedFrame, FrameTiming)>(
            self.config.quality.max_queue_size
        );
        
        // Start capture task
        let capture_task = self.spawn_capture_task(capture_tx);
        
        // Start encode task
        let encode_task = self.spawn_encode_task(capture_rx, encode_tx);
        
        // Start transmit task (with broadcast channel)
        let transmit_task = self.spawn_transmit_task(encode_rx);
        
        // Store task handles for graceful shutdown
        self.tasks = Some((capture_task, encode_task, transmit_task));
        
        info!("Stream manager started successfully");
        
        Ok(())
    }
    
    /// Stop streaming and wait for all tasks to complete
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping stream manager");
        
        // Signal all tasks to stop
        self.is_streaming.store(false, Ordering::SeqCst);
        
        // Wait for all tasks to complete gracefully
        if let Some((capture_task, encode_task, transmit_task)) = self.tasks.take() {
            info!("Waiting for tasks to complete...");
            
            // Await all tasks in parallel
            let results = tokio::try_join!(
                async {
                    capture_task.await
                        .map_err(|e| anyhow::anyhow!("Capture task join error: {}", e))?
                },
                async {
                    encode_task.await
                        .map_err(|e| anyhow::anyhow!("Encode task join error: {}", e))?
                },
                async {
                    transmit_task.await
                        .map_err(|e| anyhow::anyhow!("Transmit task join error: {}", e))?
                }
            );
            
            match results {
                Ok(_) => info!("All tasks completed successfully"),
                Err(e) => warn!("Task completion error: {}", e),
            }
        }
        
        info!("Stream manager stopped");
        
        Ok(())
    }
    
    /// Get stream statistics
    pub fn get_stats(&self) -> StreamStats {
        let uptime_secs = self.start_time
            .map(|start| start.elapsed().as_secs())
            .unwrap_or(0);
        
        let frames_encoded = self.frames_encoded.load(Ordering::Relaxed);
        let frames_transmitted = self.frames_transmitted.load(Ordering::Relaxed);
        let total_encode_us = self.total_encode_us.load(Ordering::Relaxed);
        let total_latency_us = self.total_latency_us.load(Ordering::Relaxed);
        
        // Calculate averages (convert microseconds to milliseconds)
        let avg_encode_ms = if frames_encoded > 0 {
            (total_encode_us as f64) / (frames_encoded as f64) / 1000.0
        } else {
            0.0
        };
        
        let avg_latency_ms = if frames_transmitted > 0 {
            (total_latency_us as f64) / (frames_transmitted as f64) / 1000.0
        } else {
            0.0
        };
        
        StreamStats {
            frames_captured: self.frames_captured.load(Ordering::Relaxed),
            frames_encoded,
            frames_transmitted,
            frames_dropped: self.frames_dropped.load(Ordering::Relaxed),
            avg_encode_ms,
            avg_latency_ms,
            current_bitrate_kbps: self.config.quality.quality.bitrate_kbps(self.frame_width, self.frame_height),
            uptime_secs,
        }
    }
    
    /// Alias for get_stats() - convenience method
    pub fn stats(&self) -> StreamStats {
        self.get_stats()
    }
    
    /// Spawn capture task
    fn spawn_capture_task(
        &self,
        tx: mpsc::Sender<(RawFrame, FrameTiming)>,
    ) -> JoinHandle<Result<()>> {
        let is_streaming = self.is_streaming.clone();
        let frames_captured = self.frames_captured.clone();
        let frames_dropped = self.frames_dropped.clone();
        let fps = self.config.quality.fps;
        let capture_config = self.config.capture.clone();
        
        tokio::spawn(async move {
            #[cfg(target_os = "macos")]
            {
                Self::capture_loop_macos(
                    capture_config,
                    fps,
                    tx,
                    is_streaming,
                    frames_captured,
                    frames_dropped,
                ).await
            }
            
            #[cfg(not(target_os = "macos"))]
            {
                warn!("Capture not implemented for this platform");
                Ok(())
            }
        })
    }
    
    /// macOS capture loop
    #[cfg(target_os = "macos")]
    async fn capture_loop_macos(
        config: CaptureConfig,
        fps: u32,
        tx: mpsc::Sender<(RawFrame, FrameTiming)>,
        is_streaming: Arc<AtomicBool>,
        frames_captured: Arc<AtomicU64>,
        frames_dropped: Arc<AtomicU64>,
    ) -> Result<()> {
        let mut capturer = MacOSCapturer::new()?;
        capturer.start(&config)?;
        
        let frame_duration = Duration::from_micros(1_000_000 / fps as u64);
        let mut interval = interval(frame_duration);
        
        info!("Capture loop started at {} FPS", fps);
        
        while is_streaming.load(Ordering::Relaxed) {
            interval.tick().await;
            
            let timing = FrameTiming::new();
            
            match capturer.get_raw_frame() {
                Ok(frame) => {
                    frames_captured.fetch_add(1, Ordering::Relaxed);
                    
                    if let Err(e) = tx.try_send((frame, timing)) {
                        warn!("Failed to send frame to encoder: {}", e);
                        frames_dropped.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(e) => {
                    warn!("Frame capture error: {}", e);
                }
            }
        }
        
        capturer.stop()?;
        info!("Capture loop stopped");
        
        Ok(())
    }
    
    /// Spawn encode task
    fn spawn_encode_task(
        &self,
        mut rx: mpsc::Receiver<(RawFrame, FrameTiming)>,
        tx: mpsc::Sender<(EncodedFrame, FrameTiming)>,
    ) -> JoinHandle<Result<()>> {
        let is_streaming = self.is_streaming.clone();
        let frames_encoded = self.frames_encoded.clone();
        let total_encode_us = self.total_encode_us.clone();
        let quality = self.config.quality.quality.clone();
        let fps = self.config.quality.fps;
        let width = self.frame_width;
        let height = self.frame_height;
        
        tokio::spawn(async move {
            Self::encode_loop(
                rx,
                tx,
                is_streaming,
                frames_encoded,
                total_encode_us,
                quality,
                fps,
                width,
                height,
            ).await
        })
    }
    
    /// Encode loop
    async fn encode_loop(
        mut rx: mpsc::Receiver<(RawFrame, FrameTiming)>,
        tx: mpsc::Sender<(EncodedFrame, FrameTiming)>,
        is_streaming: Arc<AtomicBool>,
        frames_encoded: Arc<AtomicU64>,
        total_encode_us: Arc<AtomicU64>,
        quality: Quality,
        fps: u32,
        width: u32,
        height: u32,
    ) -> Result<()> {
        use crate::video::encoder::create_encoder;
        
        let encoder_config = EncoderConfig {
            width,
            height,
            fps,
            quality,
            codec: Codec::H264,
            hardware_accel: true,
            keyframe_interval: fps * 2, // Keyframe every 2 seconds
        };
        
        let mut encoder = create_encoder(encoder_config)?;
        
        info!("Encode loop started (hardware_accel: {})", encoder.is_hardware_accelerated());
        
        while is_streaming.load(Ordering::Relaxed) {
            if let Some((frame, mut timing)) = rx.recv().await {
                timing.encode_start = Some(Instant::now());
                
                match encoder.encode(&frame) {
                    Ok(encoded) => {
                        timing.encode_end = Some(Instant::now());
                        frames_encoded.fetch_add(1, Ordering::Relaxed);
                        
                        // Track encode duration
                        if let Some(duration) = timing.encode_duration() {
                            total_encode_us.fetch_add(duration.as_micros() as u64, Ordering::Relaxed);
                        }
                        
                        if let Err(e) = tx.try_send((encoded, timing)) {
                            warn!("Failed to send encoded frame: {}", e);
                        }
                    }
                    Err(e) => {
                        warn!("Encoding error: {}", e);
                    }
                }
            }
        }
        
        info!("Encode loop stopped");
        Ok(())
    }
    
    /// Spawn transmit task
    fn spawn_transmit_task(
        &self,
        mut rx: mpsc::Receiver<(EncodedFrame, FrameTiming)>,
    ) -> JoinHandle<Result<()>> {
        let is_streaming = self.is_streaming.clone();
        let frames_transmitted = self.frames_transmitted.clone();
        let total_latency_us = self.total_latency_us.clone();
        let frame_tx = self.frame_tx.clone();
        
        tokio::spawn(async move {
            Self::transmit_loop(
                rx,
                is_streaming,
                frames_transmitted,
                total_latency_us,
                frame_tx,
            ).await
        })
    }
    
    /// Transmit loop - broadcasts encoded frames to all consumers
    async fn transmit_loop(
        mut rx: mpsc::Receiver<(EncodedFrame, FrameTiming)>,
        is_streaming: Arc<AtomicBool>,
        frames_transmitted: Arc<AtomicU64>,
        total_latency_us: Arc<AtomicU64>,
        frame_tx: broadcast::Sender<EncodedFrame>,
    ) -> Result<()> {
        info!("Transmit loop started (broadcasting to consumers)");
        
        while is_streaming.load(Ordering::Relaxed) {
            if let Some((encoded, mut timing)) = rx.recv().await {
                timing.transmit_start = Some(Instant::now());
                
                // Broadcast frame to all subscribers (gRPC consumers)
                match frame_tx.send(encoded.clone()) {
                    Ok(receiver_count) => {
                        debug!(
                            "Broadcast frame {} ({} bytes, keyframe: {}) to {} receiver(s)",
                            encoded.sequence,
                            encoded.data.len(),
                            encoded.is_keyframe,
                            receiver_count
                        );
                        
                        timing.transmit_end = Some(Instant::now());
                        frames_transmitted.fetch_add(1, Ordering::Relaxed);
                        
                        // Track total latency (capture to transmit)
                        if let Some(latency) = timing.total_latency() {
                            total_latency_us.fetch_add(latency.as_micros() as u64, Ordering::Relaxed);
                            
                            // Check for high latency
                            if latency.as_millis() > 200 {
                                warn!("High latency: {} ms", latency.as_millis());
                            }
                        }
                    }
                    Err(_) => {
                        // No receivers connected - this is fine, just skip
                        debug!("No receivers for frame {}, skipping", encoded.sequence);
                    }
                }
            }
        }
        
        info!("Transmit loop stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.quality.fps, 30);
        assert!(!config.enable_audio);
    }

    #[test]
    fn test_stream_manager_creation() {
        let config = StreamConfig::default();
        let manager = StreamManager::new(config);
        assert!(!manager.is_streaming.load(Ordering::Relaxed));
    }

    #[test]
    fn test_stream_stats() {
        let config = StreamConfig::default();
        let manager = StreamManager::new(config);
        let stats = manager.get_stats();
        assert_eq!(stats.frames_captured, 0);
        assert_eq!(stats.frames_encoded, 0);
    }
}
