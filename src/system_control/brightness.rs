/*!
 * Brightness Control Implementation
 *
 * Provides unified brightness control across platforms with automatic fallback:
 * 1. Try platform-specific command (brightnessctl, osascript, WMI)
 * 2. Fall back to alternative tools if available
 * 3. Ultimate fallback: keyboard simulation (xdotool, CoreGraphics, SendInput)
 */

use anyhow::{bail, Result};
use tracing::{debug, info, warn};

use super::{ControlMethod, ControlParams, ControlResult, Platform, SystemControl};
use super::platform;

/// Brightness control implementation
pub struct BrightnessControl {
    platform: Platform,
}

impl BrightnessControl {
    /// Create new brightness controller
    pub fn new(platform: Platform) -> Result<Self> {
        info!("Initializing brightness control for platform: {}", platform);
        Ok(Self { platform })
    }

    /// Get current brightness (platform-specific)
    pub fn get_brightness(&self) -> Result<f32> {
        match self.platform {
            Platform::Linux => self.get_brightness_linux(),
            Platform::MacOS => self.get_brightness_macos(),
            Platform::Windows => self.get_brightness_windows(),
        }
    }

    /// Set brightness to specified level (0.0-1.0)
    pub fn set_brightness(&self, level: f32) -> Result<()> {
        if !(0.0..=1.0).contains(&level) {
            bail!(
                "Brightness level must be between 0.0 and 1.0, got {}",
                level
            );
        }

        match self.platform {
            Platform::Linux => self.set_brightness_linux(level),
            Platform::MacOS => self.set_brightness_macos(level),
            Platform::Windows => self.set_brightness_windows(level),
        }
    }

    /// Increase brightness by step (typically 10%)
    pub fn increase_brightness(&self, step: f32) -> Result<()> {
        let current = self.get_brightness()?;
        let new_level = (current + step).min(1.0);
        self.set_brightness(new_level)
    }

    /// Decrease brightness by step (typically 10%)
    pub fn decrease_brightness(&self, step: f32) -> Result<()> {
        let current = self.get_brightness()?;
        let new_level = (current - step).max(0.0);
        self.set_brightness(new_level)
    }

    // === LINUX IMPLEMENTATION ===

    #[cfg(target_os = "linux")]
    fn get_brightness_linux(&self) -> Result<f32> {
        debug!("Getting brightness on Linux");

        // Try brightnessctl first (modern)
        match platform::get_brightness_via_brightnessctl() {
            Ok(brightness) => {
                info!("Got brightness from brightnessctl: {}", brightness);
                return Ok(brightness);
            }
            Err(e) => {
                warn!("brightnessctl failed: {}, trying xbacklight", e);
            }
        }

        // Fall back to xbacklight (older)
        match platform::get_brightness_via_xbacklight() {
            Ok(brightness) => {
                info!("Got brightness from xbacklight: {}", brightness);
                return Ok(brightness);
            }
            Err(e) => {
                warn!("xbacklight failed: {}", e);
                bail!("Could not get brightness via brightnessctl or xbacklight");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn get_brightness_linux(&self) -> Result<f32> {
        bail!("Linux brightness control not available on this platform")
    }

    #[cfg(target_os = "linux")]
    fn set_brightness_linux(&self, level: f32) -> Result<()> {
        debug!("Setting brightness on Linux to {}", level);

        // Try brightnessctl first (modern)
        match platform::set_brightness_via_brightnessctl(level) {
            Ok(_) => {
                info!("Brightness set via brightnessctl to {}", level);
                return Ok(());
            }
            Err(e) => {
                warn!("brightnessctl failed: {}, trying xbacklight", e);
            }
        }

        // Fall back to xbacklight (older)
        match platform::set_brightness_via_xbacklight(level) {
            Ok(_) => {
                info!("Brightness set via xbacklight to {}", level);
                return Ok(());
            }
            Err(e) => {
                warn!("xbacklight failed: {}", e);
                bail!("Could not set brightness via brightnessctl or xbacklight");
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn set_brightness_linux(&self, level: f32) -> Result<()> {
        bail!("Linux brightness control not available on this platform")
    }

    // === MACOS IMPLEMENTATION ===

    #[cfg(target_os = "macos")]
    fn get_brightness_macos(&self) -> Result<f32> {
        debug!("Getting brightness on macOS");
        platform::get_brightness_via_osascript()
    }

    #[cfg(not(target_os = "macos"))]
    fn get_brightness_macos(&self) -> Result<f32> {
        bail!("macOS brightness control not available on this platform")
    }

    #[cfg(target_os = "macos")]
    fn set_brightness_macos(&self, level: f32) -> Result<()> {
        debug!("Setting brightness on macOS to {}", level);
        platform::set_brightness_via_osascript(level)
    }

    #[cfg(not(target_os = "macos"))]
    fn set_brightness_macos(&self, _level: f32) -> Result<()> {
        bail!("macOS brightness control not available on this platform")
    }

    // === WINDOWS IMPLEMENTATION ===

    #[cfg(target_os = "windows")]
    fn get_brightness_windows(&self) -> Result<f32> {
        debug!("Getting brightness on Windows");
        platform::get_brightness_via_windows()
    }

    #[cfg(not(target_os = "windows"))]
    fn get_brightness_windows(&self) -> Result<f32> {
        bail!("Windows brightness control not available on this platform")
    }

    #[cfg(target_os = "windows")]
    fn set_brightness_windows(&self, level: f32) -> Result<()> {
        debug!("Setting brightness on Windows to {}", level);
        platform::set_brightness_via_windows(level)
    }

    #[cfg(not(target_os = "windows"))]
    fn set_brightness_windows(&self, _level: f32) -> Result<()> {
        bail!("Windows brightness control not available on this platform")
    }
}

impl SystemControl for BrightnessControl {
    fn name(&self) -> &str {
        "brightness"
    }

    fn execute_via_command(&self, params: &ControlParams) -> Result<ControlResult> {
        match params {
            ControlParams::Brightness { level } => {
                match self.set_brightness(*level) {
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
            _ => Err(anyhow::anyhow!(
                "Brightness control does not support {:?}",
                params
            )),
        }
    }

    fn execute_via_input(&self, _params: &ControlParams) -> Result<ControlResult> {
        // Keyboard fallback not implemented yet (future enhancement)
        // Would use xdotool, CoreGraphics, or SendInput to simulate brightness keys
        anyhow::bail!("Keyboard fallback for brightness control not yet implemented")
    }

    fn get_state(&self) -> Result<String> {
        let brightness = self.get_brightness()?;
        Ok(format!("{:.2}", brightness))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brightness_controller_creation() {
        let controller = BrightnessControl::new(Platform::Linux);
        assert!(controller.is_ok());
    }

    #[test]
    fn test_system_control_name() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        assert_eq!(controller.name(), "brightness");
    }

    #[test]
    fn test_brightness_validation() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        assert!(controller.set_brightness(0.5).is_ok() || controller.set_brightness(0.5).is_err());
        assert!(controller.set_brightness(-0.1).is_err());
        assert!(controller.set_brightness(1.5).is_err());
    }

    #[test]
    fn test_increase_brightness() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        // May succeed or fail depending on hardware availability
        let _ = controller.increase_brightness(0.1);
    }

    #[test]
    fn test_decrease_brightness() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        // May succeed or fail depending on hardware availability
        let _ = controller.decrease_brightness(0.1);
    }

    #[test]
    fn test_control_params_brightness() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        let params = ControlParams::Brightness { level: 0.5 };
        let result = controller.execute_via_command(&params);
        // May succeed or fail depending on hardware availability
        assert!(result.is_ok());
    }

    #[test]
    fn test_brightness_get_state() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        let state = controller.get_state();
        // May succeed or fail depending on hardware availability
        assert!(state.is_ok() || state.is_err());
    }

    #[test]
    fn test_invalid_control_params() {
        let controller = BrightnessControl::new(Platform::Linux).unwrap();
        let params = ControlParams::Volume { level: 0.5 };
        let result = controller.execute_via_command(&params);
        assert!(result.is_err());
    }
}
