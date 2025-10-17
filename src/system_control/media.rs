/*!
 * Media Control Implementation
 *
 * Provides unified media playback control across platforms:
 * - Play/Pause/Stop
 * - Next/Previous track
 * - Get playback status
 *
 * Hybrid execution: command-line tools + keyboard fallback
 */

use anyhow::{bail, Result};
use tracing::{debug, info, warn};

use super::{ControlMethod, ControlParams, ControlResult, MediaAction, Platform, SystemControl};
use super::platform;

/// Media control implementation
pub struct MediaControl {
    platform: Platform,
}

impl MediaControl {
    /// Create new media controller
    pub fn new(platform: Platform) -> Result<Self> {
        info!("Initializing media control for platform: {}", platform);
        Ok(Self { platform })
    }

    /// Play media
    pub fn play(&self) -> Result<()> {
        self.execute_action(MediaAction::Play)
    }

    /// Pause media
    pub fn pause(&self) -> Result<()> {
        self.execute_action(MediaAction::Pause)
    }

    /// Toggle play/pause
    pub fn play_pause(&self) -> Result<()> {
        self.execute_action(MediaAction::PlayPause)
    }

    /// Next track
    pub fn next(&self) -> Result<()> {
        self.execute_action(MediaAction::Next)
    }

    /// Previous track
    pub fn previous(&self) -> Result<()> {
        self.execute_action(MediaAction::Previous)
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        self.execute_action(MediaAction::Stop)
    }

    /// Execute media action (platform-specific)
    fn execute_action(&self, action: MediaAction) -> Result<()> {
        match self.platform {
            Platform::Linux => self.execute_action_linux(action),
            Platform::MacOS => self.execute_action_macos(action),
            Platform::Windows => self.execute_action_windows(action),
        }
    }

    // === LINUX IMPLEMENTATION ===

    #[cfg(target_os = "linux")]
    fn execute_action_linux(&self, action: MediaAction) -> Result<()> {
        debug!("Executing media action on Linux: {}", action);

        // Try playerctl first (modern)
        match platform::execute_media_action_playerctl(action) {
            Ok(_) => {
                info!("Media action executed via playerctl: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("playerctl failed: {}, trying keyboard", e);
            }
        }

        // Fall back to keyboard simulation
        match platform::execute_media_action_keyboard_linux(action) {
            Ok(_) => {
                info!("Media action executed via keyboard: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("Keyboard fallback failed: {}", e);
                bail!(
                    "Could not execute media action via playerctl or keyboard: {}",
                    action
                );
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn execute_action_linux(&self, _action: MediaAction) -> Result<()> {
        bail!("Linux media control not available on this platform")
    }

    // === MACOS IMPLEMENTATION ===

    #[cfg(target_os = "macos")]
    fn execute_action_macos(&self, action: MediaAction) -> Result<()> {
        debug!("Executing media action on macOS: {}", action);

        // Try osascript first
        match platform::execute_media_action_osascript(action) {
            Ok(_) => {
                info!("Media action executed via osascript: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("osascript failed: {}, trying keyboard", e);
            }
        }

        // Fall back to keyboard
        match platform::execute_media_action_keyboard_macos(action) {
            Ok(_) => {
                info!("Media action executed via keyboard: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("Keyboard fallback failed: {}", e);
                bail!("Could not execute media action: {}", action);
            }
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn execute_action_macos(&self, _action: MediaAction) -> Result<()> {
        bail!("macOS media control not available on this platform")
    }

    // === WINDOWS IMPLEMENTATION ===

    #[cfg(target_os = "windows")]
    fn execute_action_windows(&self, action: MediaAction) -> Result<()> {
        debug!("Executing media action on Windows: {}", action);

        // Try nircmd first
        match platform::execute_media_action_nircmd(action) {
            Ok(_) => {
                info!("Media action executed via nircmd: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("nircmd failed: {}, trying keyboard", e);
            }
        }

        // Fall back to keyboard
        match platform::execute_media_action_keyboard_windows(action) {
            Ok(_) => {
                info!("Media action executed via keyboard: {}", action);
                return Ok(());
            }
            Err(e) => {
                warn!("Keyboard fallback failed: {}", e);
                bail!("Could not execute media action: {}", action);
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn execute_action_windows(&self, _action: MediaAction) -> Result<()> {
        bail!("Windows media control not available on this platform")
    }
}

impl SystemControl for MediaControl {
    fn name(&self) -> &str {
        "media"
    }

    fn execute_via_command(&self, params: &ControlParams) -> Result<ControlResult> {
        match params {
            ControlParams::MediaControl { action } => {
                match self.execute_action(*action) {
                    Ok(_) => Ok(ControlResult {
                        success: true,
                        method_used: ControlMethod::Command,
                        value: Some(action.to_string()),
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
                "Media control does not support {:?}",
                params
            )),
        }
    }

    fn execute_via_input(&self, _params: &ControlParams) -> Result<ControlResult> {
        // Keyboard fallback already integrated into execute_action
        anyhow::bail!("Direct keyboard mode not supported for media control")
    }

    fn get_state(&self) -> Result<String> {
        Ok("media_control_active".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_controller_creation() {
        let controller = MediaControl::new(Platform::Linux);
        assert!(controller.is_ok());
    }

    #[test]
    fn test_system_control_name() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        assert_eq!(controller.name(), "media");
    }

    #[test]
    fn test_play() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.play();
    }

    #[test]
    fn test_pause() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.pause();
    }

    #[test]
    fn test_play_pause() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.play_pause();
    }

    #[test]
    fn test_next() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.next();
    }

    #[test]
    fn test_previous() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.previous();
    }

    #[test]
    fn test_stop() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let _ = controller.stop();
    }

    #[test]
    fn test_control_params_media() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let params = ControlParams::MediaControl {
            action: MediaAction::Play,
        };
        let result = controller.execute_via_command(&params);
        // May succeed or fail depending on media player availability
        assert!(result.is_ok());
    }

    #[test]
    fn test_media_get_state() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let state = controller.get_state();
        assert!(state.is_ok());
    }

    #[test]
    fn test_invalid_control_params() {
        let controller = MediaControl::new(Platform::Linux).unwrap();
        let params = ControlParams::Volume { level: 0.5 };
        let result = controller.execute_via_command(&params);
        assert!(result.is_err());
    }
}
