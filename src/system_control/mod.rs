/*!
 * System Control Module
 *
 * Provides unified system control APIs across platforms (Linux, macOS, Windows)
 * with automatic fallback from command-line tools to keyboard simulation.
 *
 * Architecture:
 * - SystemControl trait: Define platform-agnostic control interface
 * - Platform modules: OS-specific implementations
 * - Hybrid execution: Try command first, fallback to keyboard
 * - Universal RPC: Expose system controls via gRPC
 *
 * Supported Controls:
 * - Volume (get/set/mute)
 * - Brightness (get/set)
 * - Media (play/pause/next/previous/stop)
 * - (Extensible for future controls)
 */

use anyhow::Result;
use std::fmt;
use tracing::info;

pub mod platform;
pub mod volume;
pub mod brightness;
pub mod media;

use volume::VolumeControl;
use brightness::BrightnessControl;
use media::MediaControl;

/// Generic control parameters (extensible for future controls)
#[derive(Debug, Clone)]
pub enum ControlParams {
    Volume { level: f32 },
    VolumeMute { muted: bool },
    Brightness { level: f32 },
    MediaControl { action: MediaAction },
}

/// Media control actions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaAction {
    Play,
    Pause,
    PlayPause,
    Next,
    Previous,
    Stop,
}

impl fmt::Display for MediaAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaAction::Play => write!(f, "play"),
            MediaAction::Pause => write!(f, "pause"),
            MediaAction::PlayPause => write!(f, "play-pause"),
            MediaAction::Next => write!(f, "next"),
            MediaAction::Previous => write!(f, "previous"),
            MediaAction::Stop => write!(f, "stop"),
        }
    }
}

/// Result from system control operation
#[derive(Debug, Clone)]
pub struct ControlResult {
    pub success: bool,
    pub method_used: ControlMethod,
    pub value: Option<String>,
    pub error_message: Option<String>,
}

/// Which method was used to execute the control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMethod {
    Command,
    Keyboard,
}

impl fmt::Display for ControlMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlMethod::Command => write!(f, "command"),
            ControlMethod::Keyboard => write!(f, "keyboard"),
        }
    }
}

/// Generic system control trait
/// 
/// Provides interface for controlling system features with automatic fallback.
/// Primary execution uses platform-specific commands (pactl, osascript, etc.)
/// Fallback uses keyboard simulation (xdotool, CoreGraphics, etc.)
pub trait SystemControl: Send + Sync {
    /// Get control name (e.g., "volume", "brightness")
    fn name(&self) -> &str;

    /// Execute control via command-line tool (fast, precise)
    fn execute_via_command(&self, params: &ControlParams) -> Result<ControlResult>;

    /// Execute control via keyboard simulation (slower, universal fallback)
    fn execute_via_input(&self, params: &ControlParams) -> Result<ControlResult>;

    /// Hybrid execution: try command first, fallback to keyboard
    fn execute(&self, params: &ControlParams) -> Result<ControlResult> {
        info!("{}: executing control via command first", self.name());
        
        match self.execute_via_command(params) {
            Ok(result) if result.success => {
                info!("{}: command execution succeeded", self.name());
                Ok(result)
            }
            Ok(ControlResult { error_message: Some(ref msg), .. }) => {
                tracing::warn!(
                    "{}: command failed ({}), falling back to keyboard",
                    self.name(),
                    msg
                );
                self.execute_via_input(params)
            }
            Ok(_) => {
                tracing::warn!(
                    "{}: command failed, falling back to keyboard",
                    self.name()
                );
                self.execute_via_input(params)
            }
            Err(e) => {
                tracing::warn!(
                    "{}: command failed ({}), falling back to keyboard",
                    self.name(),
                    e
                );
                self.execute_via_input(params)
            }
        }
    }

    /// Get current state (optional, not all controls support)
    fn get_state(&self) -> Result<String> {
        anyhow::bail!("{}: get_state not implemented", self.name())
    }
}

/// System control manager (registry for all controls)
pub struct SystemControlManager {
    platform: Platform,
    volume_control: VolumeControl,
    brightness_control: BrightnessControl,
    media_control: MediaControl,
}

/// Detected platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Platform::Linux => write!(f, "linux"),
            Platform::MacOS => write!(f, "macos"),
            Platform::Windows => write!(f, "windows"),
        }
    }
}

impl SystemControlManager {
    /// Create new system control manager
    pub fn new() -> Result<Self> {
        let platform = detect_platform();
        info!("Initializing system control manager for platform: {}", platform);

        let volume_control = VolumeControl::new(platform)?;
        let brightness_control = BrightnessControl::new(platform)?;
        let media_control = MediaControl::new(platform)?;

        Ok(Self {
            platform,
            volume_control,
            brightness_control,
            media_control,
        })
    }

    /// Get current platform
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Execute system control
    pub fn execute(&self, params: &ControlParams) -> Result<ControlResult> {
        match params {
            ControlParams::Volume { .. } | ControlParams::VolumeMute { .. } => {
                self.volume_control.execute(params)
            }
            ControlParams::Brightness { .. } => {
                self.brightness_control.execute(params)
            }
            ControlParams::MediaControl { .. } => {
                self.media_control.execute(params)
            }
        }
    }

    /// Get volume controller
    pub fn volume_control(&self) -> &VolumeControl {
        &self.volume_control
    }

    /// Get brightness controller
    pub fn brightness_control(&self) -> &BrightnessControl {
        &self.brightness_control
    }

    /// Get media controller
    pub fn media_control(&self) -> &MediaControl {
        &self.media_control
    }
}

impl Default for SystemControlManager {
    fn default() -> Self {
        Self::new().expect("Failed to create system control manager")
    }
}

/// Detect platform at runtime
pub fn detect_platform() -> Platform {
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        panic!("Unsupported platform")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_platform() {
        let platform = detect_platform();
        assert!(
            platform == Platform::Linux
                || platform == Platform::MacOS
                || platform == Platform::Windows
        );
    }

    #[test]
    fn test_system_control_manager_creation() {
        let manager = SystemControlManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_system_control_manager_platform() {
        let manager = SystemControlManager::new().unwrap();
        let platform = manager.platform();
        assert!(
            platform == Platform::Linux
                || platform == Platform::MacOS
                || platform == Platform::Windows
        );
    }

    #[test]
    fn test_media_action_display() {
        assert_eq!(MediaAction::Play.to_string(), "play");
        assert_eq!(MediaAction::Pause.to_string(), "pause");
        assert_eq!(MediaAction::PlayPause.to_string(), "play-pause");
        assert_eq!(MediaAction::Next.to_string(), "next");
        assert_eq!(MediaAction::Previous.to_string(), "previous");
        assert_eq!(MediaAction::Stop.to_string(), "stop");
    }

    #[test]
    fn test_control_method_display() {
        assert_eq!(ControlMethod::Command.to_string(), "command");
        assert_eq!(ControlMethod::Keyboard.to_string(), "keyboard");
    }

    #[test]
    fn test_platform_display() {
        assert_eq!(Platform::Linux.to_string(), "linux");
        assert_eq!(Platform::MacOS.to_string(), "macos");
        assert_eq!(Platform::Windows.to_string(), "windows");
    }
}
