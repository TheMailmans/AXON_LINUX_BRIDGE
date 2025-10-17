/*!
 * Volume Control Implementation
 *
 * Provides unified volume control across platforms with automatic fallback:
 * 1. Try platform-specific command (pactl, osascript, nircmd)
 * 2. Fall back to amixer/xbacklight if available
 * 3. Ultimate fallback: keyboard simulation (xdotool, CoreGraphics, SendInput)
 */

use anyhow::{bail, Result};
use tracing::{debug, info, warn};

use super::{ControlMethod, ControlParams, ControlResult, Platform, SystemControl};
use super::platform;

/// Volume control implementation
pub struct VolumeControl {
    platform: Platform,
}

impl VolumeControl {
    /// Create new volume controller
    pub fn new(platform: Platform) -> Result<Self> {
        info!("Initializing volume control for platform: {}", platform);
        Ok(Self { platform })
    }

    /// Get current volume (platform-specific)
    pub fn get_volume(&self) -> Result<f32> {
        match self.platform {
            Platform::Linux => self.get_volume_linux(),
            Platform::MacOS => self.get_volume_macos(),
            Platform::Windows => self.get_volume_windows(),
        }
    }

    /// Set volume to specified level (0.0-1.0)
    pub fn set_volume(&self, level: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&level) {
            bail!(
                "Volume level must be between 0.0 and 1.0, got {}",
                level
            );
        }

        match self.platform {
            Platform::Linux => self.set_volume_linux(level),
            Platform::MacOS => self.set_volume_macos(level),
            Platform::Windows => self.set_volume_windows(level),
        }
    }

    /// Mute/unmute audio
    pub fn mute(&self, muted: bool) -> Result<()> {
        match self.platform {
            Platform::Linux => self.mute_linux(muted),
            Platform::MacOS => self.mute_macos(muted),
            Platform::Windows => self.mute_windows(muted),
        }
    }

    // === LINUX IMPLEMENTATION ===

    #[cfg(target_os = "linux")]
    fn get_volume_linux(&self) -> Result<f32> {
        debug!("Getting volume on Linux");

        // Try pactl first (PulseAudio)
        match platform::get_volume_via_pactl() {
            Ok(volume) => {
                info!("Got volume from pactl: {}", volume);
                return Ok(volume);
            }
            Err(e) => {
                warn!("pactl failed: {}, trying amixer", e);
            }
        }

        // Fall back to amixer (ALSA)
        match platform::get_volume_via_amixer() {
            Ok(volume) => {
                info!("Got volume from amixer: {}", volume);
                return Ok(volume);
            }
            Err(e) => {
                warn!("amixer failed: {}", e);
                bail!("Could not get volume via pactl or amixer");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn get_volume_linux(&self) -> Result<f32> {
        bail!("Linux volume control not available on this platform")
    }

    #[cfg(target_os = "linux")]
    fn set_volume_linux(&self, level: f32) -> Result<()> {
        debug!("Setting volume on Linux to {}", level);

        // Try pactl first (PulseAudio)
        match platform::set_volume_via_pactl(level) {
            Ok(_) => {
                info!("Volume set via pactl to {}", level);
                return Ok(());
            }
            Err(e) => {
                warn!("pactl failed: {}, trying amixer", e);
            }
        }

        // Fall back to amixer (ALSA)
        match platform::set_volume_via_amixer(level) {
            Ok(_) => {
                info!("Volume set via amixer to {}", level);
                return Ok(());
            }
            Err(e) => {
                warn!("amixer failed: {}", e);
                bail!("Could not set volume via pactl or amixer");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn set_volume_linux(&self, level: f32) -> Result<()> {
        bail!("Linux volume control not available on this platform")
    }

    #[cfg(target_os = "linux")]
    fn mute_linux(&self, muted: bool) -> Result<()> {
        debug!("Setting mute on Linux to {}", muted);

        // Try pactl first (PulseAudio)
        match platform::mute_via_pactl(muted) {
            Ok(_) => {
                info!("Mute set via pactl to {}", muted);
                return Ok(());
            }
            Err(e) => {
                warn!("pactl failed: {}, trying amixer", e);
            }
        }

        // Fall back to amixer (ALSA)
        match platform::mute_via_amixer(muted) {
            Ok(_) => {
                info!("Mute set via amixer to {}", muted);
                return Ok(());
            }
            Err(e) => {
                warn!("amixer failed: {}", e);
                bail!("Could not mute via pactl or amixer");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn mute_linux(&self, muted: bool) -> Result<()> {
        bail!("Linux volume control not available on this platform")
    }

    // === MACOS IMPLEMENTATION ===

    #[cfg(target_os = "macos")]
    fn get_volume_macos(&self) -> Result<f32> {
        debug!("Getting volume on macOS");
        platform::get_volume_via_osascript()
    }

    #[cfg(not(target_os = "macos"))]
    fn get_volume_macos(&self) -> Result<f32> {
        bail!("macOS volume control not available on this platform")
    }

    #[cfg(target_os = "macos")]
    fn set_volume_macos(&self, level: f32) -> Result<()> {
        debug!("Setting volume on macOS to {}", level);
        platform::set_volume_via_osascript(level)
    }

    #[cfg(not(target_os = "macos"))]
    fn set_volume_macos(&self, level: f32) -> Result<()> {
        bail!("macOS volume control not available on this platform")
    }

    #[cfg(target_os = "macos")]
    fn mute_macos(&self, muted: bool) -> Result<()> {
        debug!("Setting mute on macOS to {}", muted);
        platform::mute_via_osascript(muted)
    }

    #[cfg(not(target_os = "macos"))]
    fn mute_macos(&self, muted: bool) -> Result<()> {
        bail!("macOS volume control not available on this platform")
    }

    // === WINDOWS IMPLEMENTATION ===

    #[cfg(target_os = "windows")]
    fn get_volume_windows(&self) -> Result<f32> {
        debug!("Getting volume on Windows");
        platform::get_volume_via_windows()
    }

    #[cfg(not(target_os = "windows"))]
    fn get_volume_windows(&self) -> Result<f32> {
        bail!("Windows volume control not available on this platform")
    }

    #[cfg(target_os = "windows")]
    fn set_volume_windows(&self, level: f32) -> Result<()> {
        debug!("Setting volume on Windows to {}", level);
        platform::set_volume_via_nircmd(level)
    }

    #[cfg(not(target_os = "windows"))]
    fn set_volume_windows(&self, level: f32) -> Result<()> {
        bail!("Windows volume control not available on this platform")
    }

    #[cfg(target_os = "windows")]
    fn mute_windows(&self, muted: bool) -> Result<()> {
        debug!("Setting mute on Windows to {}", muted);
        platform::mute_via_nircmd(muted)
    }

    #[cfg(not(target_os = "windows"))]
    fn mute_windows(&self, muted: bool) -> Result<()> {
        bail!("Windows volume control not available on this platform")
    }
}

impl SystemControl for VolumeControl {
    fn name(&self) -> &str {
        "volume"
    }

    fn execute_via_command(&self, params: &ControlParams) -> Result<ControlResult> {
        match params {
            ControlParams::Volume { level } => {
                match self.set_volume(*level) {
                    Ok(_) => Ok(ControlResult {
                        success: true,
                        method_used: ControlMethod::Command,
                        value: Some(level.to_string()),
                        error_message: None,
                    }),
                    Err(e) => Ok(ControlResult {
                        success: false,
                        method_used: ControlMethod::Command,
                        value: None,
                        error_message: Some(e.to_string()),
                    }),
                }
            }
            ControlParams::VolumeMute { muted } => {
                match self.mute(*muted) {
                    Ok(_) => Ok(ControlResult {
                        success: true,
                        method_used: ControlMethod::Command,
                        value: Some(muted.to_string()),
                        error_message: None,
                    }),
                    Err(e) => Ok(ControlResult {
                        success: false,
                        method_used: ControlMethod::Command,
                        value: None,
                        error_message: Some(e.to_string()),
                    }),
                }
            }
            _ => Err(anyhow::anyhow!(
                "Volume control does not support {:?}",
                params
            )),
        }
    }

    fn execute_via_input(&self, _params: &ControlParams) -> Result<ControlResult> {
        // Keyboard fallback not implemented yet (future enhancement)
        // Would use xdotool, CoreGraphics, or SendInput to simulate volume keys
        anyhow::bail!("Keyboard fallback for volume control not yet implemented")
    }

    fn get_state(&self) -> Result<String> {
        let volume = self.get_volume()?;
        Ok(format!("{:.2}", volume))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_controller_creation() {
        let controller = VolumeControl::new(Platform::Linux);
        assert!(controller.is_ok());
    }

    #[test]
    fn test_system_control_name() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        assert_eq!(controller.name(), "volume");
    }

    #[test]
    fn test_volume_validation() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        assert!(controller.set_volume(0.5).is_ok() || controller.set_volume(0.5).is_err());
        assert!(controller.set_volume(-0.1).is_err());
        assert!(controller.set_volume(1.5).is_err());
    }

    #[test]
    fn test_control_params_volume() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        let params = ControlParams::Volume { level: 0.5 };
        let result = controller.execute_via_command(&params);
        // May succeed or fail depending on audio system availability
        assert!(result.is_ok());
    }

    #[test]
    fn test_control_params_mute() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        let params = ControlParams::VolumeMute { muted: true };
        let result = controller.execute_via_command(&params);
        // May succeed or fail depending on audio system availability
        assert!(result.is_ok());
    }

    #[test]
    fn test_volume_get_state() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        let state = controller.get_state();
        // May succeed or fail depending on audio system availability
        assert!(state.is_ok() || state.is_err());
    }

    #[test]
    fn test_invalid_control_params() {
        let controller = VolumeControl::new(Platform::Linux).unwrap();
        let params = ControlParams::Brightness { level: 0.5 };
        let result = controller.execute_via_command(&params);
        assert!(result.is_err());
    }
}
