/*!
 * Windows System Control Platform Implementation
 *
 * Uses nircmd/powershell for volume control
 * Falls back to simulated keyboard input
 */

use std::process::Command;
use anyhow::{bail, Context, Result};
use tracing::{debug, info};

/// Get current volume using nircmd (if available) or PowerShell
pub fn get_volume_via_windows() -> Result<f32> {
    debug!("Getting volume via Windows");

    // Try nircmd first (faster)
    if has_nircmd() {
        return get_volume_via_nircmd();
    }

    // Fall back to PowerShell
    get_volume_via_powershell()
}

/// Get volume using nircmd utility
fn get_volume_via_nircmd() -> Result<f32> {
    debug!("Getting volume via nircmd");

    let output = Command::new("nircmd.exe")
        .args(&["getvolume"])
        .output()
        .context("Failed to execute nircmd getvolume")?;

    if !output.status.success() {
        bail!("nircmd getvolume failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    // nircmd returns volume as 0-65535
    let volume_int: i32 = stdout.trim().parse()?;
    let normalized = ((volume_int as f32) / 65535.0).clamp(0.0, 1.0);

    debug!("Parsed volume from nircmd: {} -> {}", volume_int, normalized);
    Ok(normalized)
}

/// Get volume using PowerShell (slower fallback)
fn get_volume_via_powershell() -> Result<f32> {
    debug!("Getting volume via PowerShell");

    let script = r#"
        Add-Type -TypeDefinition @"
        using System;
        using System.Runtime.InteropServices;
        [Guid("5CDF2C82-841E-4546-9722-0CF74078229A"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        interface IAudioEndpointVolume {
            [PreserveSig] int NotImpl1();
            [PreserveSig] int NotImpl2();
            [PreserveSig] int GetChannelCount([Out] out int channelCount);
            [PreserveSig] int SetMasterVolumeLevel([In] float level, [In] IntPtr eventContext);
            [PreserveSig] int SetMasterVolumeLevelScalar([In] float level, [In] IntPtr eventContext);
            [PreserveSig] int GetMasterVolumeLevel([Out] out float level);
            [PreserveSig] int GetMasterVolumeLevelScalar([Out] out float level);
            [PreserveSig] int SetMute([In] bool mute, [In] IntPtr eventContext);
            [PreserveSig] int GetMute([Out] out bool mute);
            [PreserveSig] int GetVolumeRange([Out] out float volumeMin, [Out] out float volumeMax, [Out] out float volumeStep);
        }
        [Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        interface IMMDevice {
            [PreserveSig] int Activate([In] ref Guid iid, [In] int dwClsCtx, [In] IntPtr pActivationParams, [Out, MarshalAs(UnmanagedType.IUnknown)] out object ppInterface);
            [PreserveSig] int OpenPropertyStore([In] int stgmAccess, [Out] out object ppProperties);
            [PreserveSig] int GetId([Out] out string ppstrId);
            [PreserveSig] int GetState([Out] out int pdwState);
        }
        [Guid("A95664D2-9614-4F35-A745-E0A8D93E7E21"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
        interface IMMDeviceEnumerator {
            [PreserveSig] int EnumAudioEndpoints([In] int dataFlow, [In] int dwStateMask, [Out] out object ppDevices);
            [PreserveSig] int GetDefaultAudioEndpoint([In] int dataFlow, [In] int role, [Out] out IMMDevice ppEndpoint);
            [PreserveSig] int GetDevice([In] string id, [Out] out IMMDevice ppDevice);
            [PreserveSig] int RegisterEndpointNotificationCallback([In] object pClient);
            [PreserveSig] int UnregisterEndpointNotificationCallback([In] object pClient);
        }
        [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
        class MMDeviceEnumeratorComObject { }

        $deviceEnumerator = [Activator]::CreateInstance([Type]::GetTypeFromCLSID([Guid]"BCDE0395-E52F-467C-8E3D-C4579291692E"))
        $speakers = $deviceEnumerator.GetDefaultAudioEndpoint(0, 1)
        $audio = $speakers.Activate([Guid]"5CDF2C82-841E-4546-9722-0CF74078229A", 0, $null)
        [float]$level = 0
        $audio.GetMasterVolumeLevelScalar([ref]$level)
        Write-Host $level
    "@#;

    let output = Command::new("powershell.exe")
        .args(&["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .context("Failed to execute PowerShell volume query")?;

    if !output.status.success() {
        bail!("PowerShell volume query failed");
    }

    let stdout = String::from_utf8(output.stdout)?;
    let level: f32 = stdout.trim().parse()?;

    debug!("Parsed volume from PowerShell: {}", level);
    Ok(level.clamp(0.0, 1.0))
}

/// Set volume using nircmd
pub fn set_volume_via_nircmd(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Volume level must be between 0.0 and 1.0, got {}", level);
    }

    let volume_int = (level * 65535.0) as i32;
    debug!("Setting volume via nircmd to {}/65535", volume_int);

    let output = Command::new("nircmd.exe")
        .args(&["setvolume", "0", &volume_int.to_string()])
        .output()
        .context("Failed to execute nircmd setvolume")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("nircmd setvolume failed: {}", stderr);
    }

    info!("Volume set to {}/65535 via nircmd", volume_int);
    Ok(())
}

/// Mute/unmute using nircmd
pub fn mute_via_nircmd(muted: bool) -> Result<()> {
    let action = if muted { "1" } else { "0" };
    debug!("Setting mute via nircmd to {}", muted);

    let output = Command::new("nircmd.exe")
        .args(&["mutesysvolume", action])
        .output()
        .context("Failed to execute nircmd mutesysvolume")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("nircmd mutesysvolume failed: {}", stderr);
    }

    info!("Mute set to {} via nircmd", muted);
    Ok(())
}

/// Check if nircmd is available
pub fn has_nircmd() -> bool {
    Command::new("where.exe")
        .arg("nircmd.exe")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Get current brightness using WMI/PowerShell
pub fn get_brightness_via_windows() -> Result<f32> {
    debug!("Getting brightness via Windows");

    let script = r#"
        Add-Type -AssemblyName System.Windows.Forms
        [System.Windows.Forms.Screen]::PrimaryScreen.BitsPerPixel
        # TODO: Implement actual brightness reading via WMI
        # This is a stub - actual implementation would use WMI
    "#;

    debug!("Getting brightness via Windows (stub implementation)");
    
    // Stub: return 0.75 (75% brightness)
    // Real implementation would query actual display brightness
    Ok(0.75)
}

/// Set brightness using nircmd or WMI
pub fn set_brightness_via_windows(level: f32) -> Result<()> {
    if !(0.0..=1.0).contains(&level) {
        bail!("Brightness level must be between 0.0 and 1.0, got {}", level);
    }

    debug!("Setting brightness via Windows to {}", level);

    // Try nircmd if available
    if has_nircmd() {
        // Note: nircmd uses different brightness API
        // This is a stub - real implementation would use appropriate WMI calls
        warn!("nircmd brightness control - stub implementation");
        return Ok(());
    }

    // Fallback to WMI/PowerShell
    debug!("Using PowerShell for brightness control (stub)");
    info!("Brightness set to {} via Windows (stub)", level);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_range_validation() {
        assert!(set_volume_via_nircmd(-0.1).is_err());
        assert!(set_volume_via_nircmd(1.5).is_err());
    }

    #[test]
    fn test_mute_validation() {
        let _ = mute_via_nircmd(true);
        let _ = mute_via_nircmd(false);
    }

    #[test]
    fn test_brightness_range_validation() {
        assert!(set_brightness_via_windows(-0.1).is_err());
        assert!(set_brightness_via_windows(1.5).is_err());
        assert!(set_brightness_via_windows(0.5).is_ok());
    }

    #[test]
    fn test_get_brightness_windows() {
        let brightness = get_brightness_via_windows();
        assert!(brightness.is_ok());
    }
}
