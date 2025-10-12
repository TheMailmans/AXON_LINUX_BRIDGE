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

#[cfg(target_os = "linux")]
pub mod desktop_apps;

// Re-export commonly used types
pub use audio::{AudioConfig, AudioFrame, AudioSource, AudioCapturer, EncodedAudioFrame};
pub use video::{VideoEncoder, EncodedFrame};
// pub use capture::ScreenCapturer;  // TODO: Export when ScreenCapturer trait exists
pub use streaming::StreamManager;
