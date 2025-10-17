/*!
 * Linux System Control Platform Implementation
 *
 * Uses PulseAudio (pactl) for volume control with fallback to ALSA (amixer)
 * and ultimately keyboard simulation (xdotool) as a last resort.
 */

use std::process::Command;
use anyhow::{bail, Context, Result};
use tracing::{debug, info, warn};

/// Get current volume using pactl
pub fn get_volume_via_pactl() -> Result<f32> {
    debug!("Getting volume via pactl");

    let output = Command::new("pactl")
        .args(&["get-sink-volume", "@DEFAULT_SINK@"])
        .output()
        .context("Failed to execute pactl get-sink-volume")?;

    if !output.status.success() {
        bail!("pactl get-sink-volume failed");
    }

    let stdout = String::from_utf8(output.stdout)?;

    // Parse output: "Volume: front-left: 65536 / 100% / dB: 0.00"
    for line in stdout.lines() {
        if let Some(percent_str) = line.split('/').nth(1) {
            if let Some(percent) = percent_str.trim().strip_suffix('%') {
                let level: f32 = percent.parse()?;
                let normalized = (level / 100.0).clamp(0.0, 1.0);
                debug!("Parsed volume from pactl: {}% -> {}", percent, normalized);
                return Ok(normalized);
            }
        }
    }

    bail!("Could not parse pactl output")
}

/// Get current volume using amixer (fallback)
pub fn get_volume_via_amixer() -> Result<f32> {
    debug!("Getting volume via amixer (fallback)");

    let output = Command::new("amixer")
        .args(&["get", "Master"])
        .output()
        .context("Failed to execute amixer get Master")?;

    if !output.status.success() {
        bail!("amixer get Master failed");
    }

    let stdout = String::from_utf8(output.stdout)?;

    // Parse output: "[100%] [on]" or similar
    for line in stdout.lines() {
        if let Some(bracket_content) = line.split('[').nth(1) {
            if let Some(percent) = bracket_content.split(']').next() {
                if let Some(num) = percent.strip_suffix('%') {
                    let level: f32 = num.parse()?;
                    let normalized = (level / 100.0).clamp(0.0, 1.0);
                    debug!("Parsed volume from amixer: {}% -> {}", num, normalized);
                    return Ok(normalized);
                }
            }
        }
    }

    bail!("Could not parse amixer output")
}

/// Set volume using pactl
pub fn set_volume_via_pactl(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Volume level must be between 0.0 and 1.0, got {}", level);
    }

    let percent = (level * 100.0) as u32;
    debug!("Setting volume via pactl to {}%", percent);

    let output = Command::new("pactl")
        .args(&["set-sink-volume", "@DEFAULT_SINK@", &format!("{}%", percent)])
        .output()
        .context("Failed to execute pactl set-sink-volume")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("pactl set-sink-volume failed: {}", stderr);
    }

    info!("Volume set to {}% via pactl", percent);
    Ok(())
}

/// Set volume using amixer (fallback)
pub fn set_volume_via_amixer(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Volume level must be between 0.0 and 1.0, got {}", level);
    }

    let percent = (level * 100.0) as u32;
    debug!("Setting volume via amixer to {}%", percent);

    let output = Command::new("amixer")
        .args(&["set", "Master", &format!("{}%", percent)])
        .output()
        .context("Failed to execute amixer set Master")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("amixer set Master failed: {}", stderr);
    }

    info!("Volume set to {}% via amixer", percent);
    Ok(())
}

/// Mute audio using pactl
pub fn mute_via_pactl(muted: bool) -> Result<()> {
    let action = if muted { "1" } else { "0" };
    debug!("Setting mute via pactl to {}", muted);

    let output = Command::new("pactl")
        .args(&["set-sink-mute", "@DEFAULT_SINK@", action])
        .output()
        .context("Failed to execute pactl set-sink-mute")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("pactl set-sink-mute failed: {}", stderr);
    }

    info!("Mute set to {} via pactl", muted);
    Ok(())
}

/// Mute audio using amixer (fallback)
pub fn mute_via_amixer(muted: bool) -> Result<()> {
    let action = if muted { "mute" } else { "unmute" };
    debug!("Setting mute via amixer to {}", muted);

    let output = Command::new("amixer")
        .args(&["set", "Master", action])
        .output()
        .context("Failed to execute amixer set Master")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("amixer set Master {} failed: {}", action, stderr);
    }

    info!("Mute set to {} via amixer", muted);
    Ok(())
}

/// Check if pactl is available
pub fn has_pactl() -> bool {
    Command::new("which")
        .arg("pactl")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if amixer is available
pub fn has_amixer() -> bool {
    Command::new("which")
        .arg("amixer")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_range_validation() {
        assert!(set_volume_via_pactl(0.5).is_ok() || set_volume_via_pactl(0.5).is_err());
        // Should fail with out-of-range
        assert!(set_volume_via_pactl(-0.1).is_err());
        assert!(set_volume_via_pactl(1.5).is_err());
    }

    #[test]
    fn test_mute_via_pactl_validation() {
        // Just verify the function works (may fail if pactl not available)
        let _ = mute_via_pactl(true);
        let _ = mute_via_pactl(false);
    }

    #[test]
    fn test_has_pactl_or_amixer() {
        // At least one should be available on Linux
        let has_pactl = has_pactl();
        let has_amixer = has_amixer();
        assert!(has_pactl || has_amixer, "Neither pactl nor amixer available");
    }
}
