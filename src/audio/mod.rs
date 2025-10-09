/*!
 * Audio Capture and Streaming Module
 * 
 * Provides cross-platform audio capture with Opus encoding.
 */

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub mod coreaudio_ffi;

#[cfg(target_os = "macos")]
pub mod macos_capture;

pub mod ring_buffer;

#[cfg(target_os = "windows")]
pub mod wasapi_ffi;

#[cfg(target_os = "windows")]
pub mod wasapi_capture;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod pulseaudio_ffi;

#[cfg(target_os = "linux")]
pub mod pulseaudio_capture;

#[cfg(target_os = "linux")]
pub mod linux;

pub mod encoder;
pub mod stream_manager;
pub mod grpc_streaming;
pub mod av_sync;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Audio capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate: u32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,  // 48kHz standard
            channels: 2,          // Stereo
            bitrate: 128000,      // 128 kbps
        }
    }
}

/// Audio source type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AudioSource {
    /// System audio (what the computer is playing)
    System,
    /// Microphone input
    Microphone,
    /// Specific application audio
    Application(String),
}

/// Raw audio frame (PCM data)
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub timestamp: u64,
    pub data: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u32,
}

/// Encoded audio frame (Opus data)
#[derive(Debug, Clone)]
pub struct EncodedAudioFrame {
    pub timestamp_ms: u64,
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u32,
    pub sequence: u64,
}

/// Platform-agnostic audio capturer trait
pub trait AudioCapturer {
    fn start(&mut self, config: &AudioConfig) -> Result<()>;
    fn stop(&mut self) -> Result<()>;
    fn get_frame(&mut self) -> Result<AudioFrame>;
    fn is_capturing(&self) -> bool;
}

/// Create platform-specific audio capturer
#[cfg(target_os = "macos")]
pub fn create_capturer(source: &AudioSource) -> Result<Box<dyn AudioCapturer>> {
    Ok(Box::new(macos::MacOSAudioCapturer::new(source)?))
}

#[cfg(target_os = "windows")]
pub fn create_capturer(source: &AudioSource) -> Result<Box<dyn AudioCapturer>> {
    Ok(Box::new(windows::WindowsAudioCapturer::new(source)?))
}

#[cfg(target_os = "linux")]
pub fn create_capturer(source: &AudioSource) -> Result<Box<dyn AudioCapturer>> {
    Ok(Box::new(linux::LinuxAudioCapturer::new(source)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_config_default() {
        let config = AudioConfig::default();
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.bitrate, 128000);
    }

    #[test]
    fn test_create_capturer() {
        let capturer = create_capturer(&AudioSource::System);
        assert!(capturer.is_ok());
    }
}
