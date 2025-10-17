/*!
 * Axon Desktop Agent Library
 * 
 * Core modules for desktop capture and streaming.
 */

pub mod platform;
pub mod video;
pub mod capture;
pub mod streaming;
pub mod audio;
pub mod input;
pub mod a11y;
pub mod metrics;
pub mod validation;
pub mod health;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod desktop_apps;

// Re-export commonly used types
pub use audio::{AudioCapturer, AudioConfig, AudioFrame, AudioSource, EncodedAudioFrame};
pub use video::{EncodedFrame, VideoEncoder};
// pub use capture::ScreenCapturer;  // TODO: Export when ScreenCapturer trait exists
pub use streaming::StreamManager;
