/*!
 * Audio Stream Manager
 * 
 * Manages audio capture, encoding, and streaming pipeline.
 * Similar architecture to video StreamManager.
 */

use anyhow::Result;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tracing::{info, warn, error, debug};

use super::{AudioConfig, AudioSource, AudioCapturer, EncodedAudioFrame};
use super::encoder::AudioEncoder;

#[cfg(target_os = "macos")]
use super::macos::MacOSAudioCapturer;

#[cfg(target_os = "windows")]
use super::windows::WindowsAudioCapturer;

#[cfg(target_os = "linux")]
use super::linux::LinuxAudioCapturer;

/// Audio streaming configuration
#[derive(Debug, Clone)]
pub struct AudioStreamConfig {
    pub source: AudioSource,
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate: u32,
    pub buffer_size: usize,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            source: AudioSource::System,
            sample_rate: 48000,
            channels: 2,
            bitrate: 128000,
            buffer_size: 100, // frames
        }
    }
}

/// Audio stream statistics
#[derive(Debug, Clone)]
pub struct AudioStreamStats {
    pub frames_captured: u64,
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub total_bytes_captured: u64,
    pub total_bytes_encoded: u64,
    pub average_encode_time_us: u64,
    pub is_streaming: bool,
}

/// Audio Stream Manager
/// 
/// Manages the complete audio streaming pipeline:
/// 1. Capture audio from OS (CoreAudio/WASAPI/PulseAudio)
/// 2. Encode with Opus
/// 3. Broadcast to subscribers (gRPC clients)
pub struct AudioStreamManager {
    config: AudioStreamConfig,
    is_streaming: Arc<AtomicBool>,
    
    // Statistics
    frames_captured: Arc<AtomicU64>,
    frames_encoded: Arc<AtomicU64>,
    frames_dropped: Arc<AtomicU64>,
    total_encode_time_us: Arc<AtomicU64>,
    
    // Broadcast channel for encoded audio
    audio_tx: broadcast::Sender<EncodedAudioFrame>,
    
    // Task handles
    tasks: Option<(JoinHandle<Result<()>>, JoinHandle<Result<()>>)>,
}

impl AudioStreamManager {
    /// Create new audio stream manager
    pub fn new(config: AudioStreamConfig) -> Self {
        let (audio_tx, _) = broadcast::channel(config.buffer_size);
        
        info!("Creating audio stream manager: {:?}", config);
        
        Self {
            config,
            is_streaming: Arc::new(AtomicBool::new(false)),
            frames_captured: Arc::new(AtomicU64::new(0)),
            frames_encoded: Arc::new(AtomicU64::new(0)),
            frames_dropped: Arc::new(AtomicU64::new(0)),
            total_encode_time_us: Arc::new(AtomicU64::new(0)),
            audio_tx,
            tasks: None,
        }
    }
    
    /// Start audio streaming
    pub fn start(&mut self) -> Result<()> {
        if self.is_streaming.load(Ordering::Relaxed) {
            warn!("Audio streaming already running");
            return Ok(());
        }
        
        info!("Starting audio streaming pipeline");
        
        self.is_streaming.store(true, Ordering::Relaxed);
        
        // Spawn capture and encode tasks
        let capture_handle = self.spawn_capture_task();
        let encode_handle = self.spawn_encode_task();
        
        self.tasks = Some((capture_handle, encode_handle));
        
        info!("Audio streaming pipeline started");
        
        Ok(())
    }
    
    /// Stop audio streaming
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_streaming.load(Ordering::Relaxed) {
            warn!("Audio streaming not running");
            return Ok(());
        }
        
        info!("Stopping audio streaming pipeline");
        
        self.is_streaming.store(false, Ordering::Relaxed);
        
        // Take task handles (cleanup on drop)
        self.tasks.take();
        
        info!("Audio streaming pipeline stopped");
        
        Ok(())
    }
    
    /// Stop audio streaming asynchronously
    pub async fn stop_async(&mut self) -> Result<()> {
        if !self.is_streaming.load(Ordering::Relaxed) {
            warn!("Audio streaming not running");
            return Ok(());
        }
        
        info!("Stopping audio streaming pipeline (async)");
        
        self.is_streaming.store(false, Ordering::Relaxed);
        
        // Wait for tasks to complete
        if let Some((capture_handle, encode_handle)) = self.tasks.take() {
            let _ = capture_handle.await;
            let _ = encode_handle.await;
        }
        
        info!("Audio streaming pipeline stopped (async)");
        
        Ok(())
    }
    
    /// Subscribe to audio stream
    pub fn subscribe(&self) -> broadcast::Receiver<EncodedAudioFrame> {
        self.audio_tx.subscribe()
    }
    
    /// Get streaming statistics
    pub fn get_stats(&self) -> AudioStreamStats {
        let captured = self.frames_captured.load(Ordering::Relaxed);
        let encoded = self.frames_encoded.load(Ordering::Relaxed);
        let total_encode_us = self.total_encode_time_us.load(Ordering::Relaxed);
        
        let avg_encode_us = if encoded > 0 {
            total_encode_us / encoded
        } else {
            0
        };
        
        AudioStreamStats {
            frames_captured: captured,
            frames_encoded: encoded,
            frames_dropped: self.frames_dropped.load(Ordering::Relaxed),
            total_bytes_captured: 0, // TODO: Track from capturer
            total_bytes_encoded: 0,  // TODO: Track from encoder
            average_encode_time_us: avg_encode_us,
            is_streaming: self.is_streaming.load(Ordering::Relaxed),
        }
    }
    
    fn spawn_capture_task(&self) -> JoinHandle<Result<()>> {
        let config = self.config.clone();
        let is_streaming = self.is_streaming.clone();
        let frames_captured = self.frames_captured.clone();
        let audio_tx = self.audio_tx.clone();
        
        tokio::spawn(async move {
            info!("Audio capture task started");
            
            // Create platform-specific capturer
            #[cfg(target_os = "macos")]
            let mut capturer = MacOSAudioCapturer::new(&config.source)?;
            
            #[cfg(target_os = "windows")]
            let mut capturer = WindowsAudioCapturer::new(&config.source)?;
            
            #[cfg(target_os = "linux")]
            let mut capturer = LinuxAudioCapturer::new(&config.source)?;
            
            // Start capture
            let audio_config = AudioConfig {
                sample_rate: config.sample_rate,
                channels: config.channels,
                bitrate: config.bitrate,
            };
            capturer.start(&audio_config)?;
            
            // Capture loop
            while is_streaming.load(Ordering::Relaxed) {
                match capturer.get_frame() {
                    Ok(frame) => {
                        frames_captured.fetch_add(1, Ordering::Relaxed);
                        
                        // For now, just track captures
                        // In full implementation, send to encoder via channel
                        debug!("Captured audio frame: {} samples", frame.data.len());
                    }
                    Err(e) => {
                        error!("Audio capture error: {}", e);
                        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    }
                }
                
                // Sleep for frame duration (20ms typical)
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            }
            
            capturer.stop()?;
            info!("Audio capture task stopped");
            
            Ok(())
        })
    }
    
    fn spawn_encode_task(&self) -> JoinHandle<Result<()>> {
        let config = self.config.clone();
        let is_streaming = self.is_streaming.clone();
        let frames_encoded = self.frames_encoded.clone();
        let total_encode_time_us = self.total_encode_time_us.clone();
        let audio_tx = self.audio_tx.clone();
        
        tokio::spawn(async move {
            info!("Audio encode task started");
            
            // Create encoder
            let mut encoder = AudioEncoder::new(
                config.sample_rate,
                config.channels,
                config.bitrate,
            )?;
            
            // Encode loop (receives from capture via channel in full implementation)
            while is_streaming.load(Ordering::Relaxed) {
                // TODO: Receive raw frames from capture task
                // For now, just sleep
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            }
            
            info!("Audio encode task stopped");
            
            Ok(())
        })
    }
}

impl Drop for AudioStreamManager {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_stream_manager_creation() {
        let config = AudioStreamConfig::default();
        let manager = AudioStreamManager::new(config);
        
        let stats = manager.get_stats();
        assert_eq!(stats.frames_captured, 0);
        assert!(!stats.is_streaming);
    }

    #[tokio::test]
    async fn test_audio_stream_lifecycle() {
        let config = AudioStreamConfig::default();
        let mut manager = AudioStreamManager::new(config);
        
        // Start streaming
        manager.start().unwrap();
        assert!(manager.is_streaming.load(Ordering::Relaxed));
        
        // Let it run briefly
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        
        // Check stats
        let stats = manager.get_stats();
        assert!(stats.is_streaming);
        
        // Stop streaming (async version)
        manager.stop_async().await.unwrap();
        
        assert!(!manager.is_streaming.load(Ordering::Relaxed));
    }

    #[test]
    fn test_audio_stream_subscribe() {
        let config = AudioStreamConfig::default();
        let manager = AudioStreamManager::new(config);
        
        let _rx1 = manager.subscribe();
        let _rx2 = manager.subscribe();
        
        // Multiple subscribers should work
    }
}
