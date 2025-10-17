/*!
 * macOS System Control Platform Implementation
 *
 * Uses osascript to control volume via AppleScript
 */

use std::process::Command;
use anyhow::{bail, Context, Result};
use tracing::{debug, info};

/// Get current volume using osascript
pub fn get_volume_via_osascript() -> Result<f32> {
    debug!("Getting volume via osascript");

    let output = Command::new("osascript")
        .args(&["-e", "output volume of (get volume settings)"])
        .output()
        .context("Failed to execute osascript for volume")?;

    if !output.status.success() {
        bail!("osascript volume query failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let volume_int: i32 = stdout.trim().parse()?;

    let normalized = (volume_int as f32 / 100.0).clamp(0.0, 1.0);
    debug!("Parsed volume from osascript: {} -> {}", volume_int, normalized);

    Ok(normalized)
}

/// Set volume using osascript
pub fn set_volume_via_osascript(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Volume level must be between 0.0 and 1.0, got {}", level);
    }

    let volume_int = (level * 100.0) as i32;
    let script = format!("set volume output volume {}", volume_int);

    debug!("Setting volume via osascript to {}%", volume_int);

    let output = Command::new("osascript")
        .args(&["-e", &script])
        .output()
        .context("Failed to execute osascript for volume set")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("osascript volume set failed: {}", stderr);
    }

    info!("Volume set to {}% via osascript", volume_int);
    Ok(())
}

/// Check if audio is muted using osascript
pub fn is_muted_via_osascript() -> Result<bool> {
    debug!("Checking mute status via osascript");

    let output = Command::new("osascript")
        .args(&["-e", "output muted of (get volume settings)"])
        .output()
        .context("Failed to execute osascript for mute status")?;

    if !output.status.success() {
        bail!("osascript mute status query failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let is_muted = stdout.trim() == "true";

    debug!("Mute status from osascript: {}", is_muted);
    Ok(is_muted)
}

/// Mute/unmute audio using osascript
pub fn mute_via_osascript(muted: bool) -> Result<()> {
    let script = if muted {
        "set volume output muted true".to_string()
    } else {
        "set volume output muted false".to_string()
    };

    debug!("Setting mute via osascript to {}", muted);

    let output = Command::new("osascript")
        .args(&["-e", &script])
        .output()
        .context("Failed to execute osascript for mute")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("osascript mute set failed: {}", stderr);
    }

    info!("Mute set to {} via osascript", muted);
    Ok(())
}

/// Get current brightness using osascript
pub fn get_brightness_via_osascript() -> Result<f32> {
    debug!("Getting brightness via osascript");

    let output = Command::new("osascript")
        .args(&["-e", "tell application \"System Events\" to tell appearance preferences to get brightness"])
        .output()
        .context("Failed to execute osascript for brightness")?;

    if !output.status.success() {
        bail!("osascript brightness query failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let brightness: f32 = stdout.trim().parse()?;

    let normalized = (brightness).clamp(0.0, 1.0);
    debug!("Parsed brightness from osascript: {} -> {}", brightness, normalized);

    Ok(normalized)
}

/// Set brightness using osascript
pub fn set_brightness_via_osascript(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Brightness level must be between 0.0 and 1.0, got {}", level);
    }

    let brightness_int = (level * 100.0) as i32;
    let script = format!(
        "tell application \"System Events\" to tell appearance preferences to set brightness to {}",
        brightness_int
    );

    debug!("Setting brightness via osascript to {}%", brightness_int);

    let output = Command::new("osascript")
        .args(&["-e", &script])
        .output()
        .context("Failed to execute osascript for brightness set")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("osascript brightness set failed: {}", stderr);
    }

    info!("Brightness set to {}% via osascript", brightness_int);
    Ok(())
}

// === MEDIA CONTROL ===

/// Execute media action using osascript
pub fn execute_media_action_osascript(action: super::super::MediaAction) -> Result<()> {
    use super::super::MediaAction;
    
    let script = match action {
        MediaAction::Play => "tell application \"Music\" to play",
        MediaAction::Pause => "tell application \"Music\" to pause",
        MediaAction::PlayPause => "tell application \"Music\" to playpause",
        MediaAction::Next => "tell application \"Music\" to next track",
        MediaAction::Previous => "tell application \"Music\" to previous track",
        MediaAction::Stop => "tell application \"Music\" to stop",
    };

    debug!("Executing media action via osascript: {}", action);

    let output = Command::new("osascript")
        .args(&["-e", script])
        .output()
        .context("Failed to execute osascript for media")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("osascript media action failed: {}", stderr);
    }

    info!("Media action executed via osascript: {}", action);
    Ok(())
}

/// Execute media action using keyboard simulation
pub fn execute_media_action_keyboard_macos(action: super::super::MediaAction) -> Result<()> {
    use super::super::MediaAction;
    
    let script = match action {
        MediaAction::Play => "tell application \"System Events\" to key code 104",
        MediaAction::Pause => "tell application \"System Events\" to key code 113", 
        MediaAction::PlayPause => "tell application \"System Events\" to key code 104",
        MediaAction::Next => "tell application \"System Events\" to key code 124",
        MediaAction::Previous => "tell application \"System Events\" to key code 123",
        MediaAction::Stop => "tell application \"System Events\" to key code 101",
    };

    debug!("Executing media action via keyboard: {}", action);

    let output = Command::new("osascript")
        .args(&["-e", script])
        .output()
        .context("Failed to execute keyboard media action")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Keyboard media action failed: {}", stderr);
    }

    info!("Media action executed via keyboard: {}", action);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_range_validation() {
        // Should fail with out-of-range
        assert!(set_volume_via_osascript(-0.1).is_err());
        assert!(set_volume_via_osascript(1.5).is_err());
        // Valid range should pass or fail based on osascript availability
        let _ = set_volume_via_osascript(0.5);
    }

    #[test]
    fn test_mute_validation() {
        let _ = mute_via_osascript(true);
        let _ = mute_via_osascript(false);
    }

    #[test]
    fn test_brightness_range_validation() {
        assert!(set_brightness_via_osascript(-0.1).is_err());
        assert!(set_brightness_via_osascript(1.5).is_err());
        let _ = set_brightness_via_osascript(0.5);
    }
}
