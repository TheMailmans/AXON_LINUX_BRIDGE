use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub arch: String,
    pub hostname: String,
    pub screen_width: u32,
    pub screen_height: u32,
}

/// Get platform name
pub fn get_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    return "windows";
    
    #[cfg(target_os = "macos")]
    return "macos";
    
    #[cfg(target_os = "linux")]
    return "linux";
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "unknown";
}

/// Get system information
pub fn get_system_info() -> Result<SystemInfo> {
    let os = get_platform_name().to_string();
    let arch = std::env::consts::ARCH.to_string();
    
    let hostname = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());
    
    let os_version = get_os_version();
    
    // Get primary display dimensions
    let (screen_width, screen_height) = get_primary_display_size()?;
    
    Ok(SystemInfo {
        os,
        os_version,
        arch,
        hostname,
        screen_width,
        screen_height,
    })
}

/// Get OS version string
fn get_os_version() -> String {
    #[cfg(target_os = "windows")]
    {
        // Will implement Windows-specific version detection
        "Windows 10+".to_string()
    }
    
    #[cfg(target_os = "macos")]
    {
        // Will implement macOS-specific version detection
        "macOS 13+".to_string()
    }
    
    #[cfg(target_os = "linux")]
    {
        // Will implement Linux-specific version detection
        "Linux".to_string()
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

/// Get primary display size
pub fn get_primary_display_size() -> Result<(u32, u32)> {
    #[cfg(target_os = "windows")]
    {
        windows_display_size()
    }
    
    #[cfg(target_os = "macos")]
    {
        macos_display_size()
    }
    
    #[cfg(target_os = "linux")]
    {
        linux_display_size()
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok((1920, 1080)) // Default fallback
    }
}

#[cfg(target_os = "windows")]
fn windows_display_size() -> Result<(u32, u32)> {
    use windows::Win32::Graphics::Gdi::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    
    unsafe {
        let width = GetSystemMetrics(SM_CXSCREEN) as u32;
        let height = GetSystemMetrics(SM_CYSCREEN) as u32;
        Ok((width, height))
    }
}

#[cfg(target_os = "macos")]
fn macos_display_size() -> Result<(u32, u32)> {
    use core_graphics::display::CGDisplay;
    
    let display = CGDisplay::main();
    let width = display.pixels_wide() as u32;
    let height = display.pixels_high() as u32;
    
    Ok((width, height))
}

#[cfg(target_os = "linux")]
fn linux_display_size() -> Result<(u32, u32)> {
    // For now, return a default
    // Will implement X11/Wayland detection in Sprint 1.6
    Ok((1920, 1080))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_platform_detection() {
        let platform = get_platform_name();
        assert!(["windows", "macos", "linux"].contains(&platform));
    }
    
    #[test]
    fn test_system_info() {
        let info = get_system_info().unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
        assert!(info.screen_width > 0);
        assert!(info.screen_height > 0);
    }
}
