// AXONBRIDGE Linux - Screenshot Capture Module
// Production-ready screen capture for Linux/X11
// Uses scrot for reliability, with X11 fallback

use anyhow::{Context, Result};
use std::process::Command;
use std::fs;
use std::path::Path;
use tracing::{debug, error, warn};

/// Screenshot quality (1-100, higher = better quality, larger file)
const DEFAULT_JPEG_QUALITY: u8 = 80;

/// Temporary screenshot path
const TEMP_SCREENSHOT_PATH: &str = "/tmp/axonbridge_screenshot.png";

/// Capture screenshot using scrot (primary method)
/// 
/// # Returns
/// * `Vec<u8>` - PNG image bytes
fn capture_with_scrot() -> Result<Vec<u8>> {
    debug!("Capturing screenshot with scrot");
    
    // Remove old temp file if exists
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    // Capture screenshot with scrot
    // -o = overwrite without asking
    // -q = quality (for JPEG, but we use PNG)
    let output = Command::new("scrot")
        .args(&[
            "-o",                    // Overwrite
            TEMP_SCREENSHOT_PATH,    // Output path
        ])
        .output()
        .context("Failed to execute scrot")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("scrot failed: {}", stderr);
    }
    
    // Read the captured image
    let image_data = fs::read(TEMP_SCREENSHOT_PATH)
        .context("Failed to read screenshot file")?;
    
    // Clean up temp file
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    debug!("Screenshot captured: {} bytes", image_data.len());
    
    Ok(image_data)
}

/// Capture screenshot using ImageMagick import (fallback method)
fn capture_with_import() -> Result<Vec<u8>> {
    debug!("Capturing screenshot with ImageMagick import");
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    // Use ImageMagick import to capture screen
    // -window root = capture entire screen
    let output = Command::new("import")
        .args(&[
            "-window", "root",
            TEMP_SCREENSHOT_PATH,
        ])
        .output()
        .context("Failed to execute import")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("import failed: {}", stderr);
    }
    
    let image_data = fs::read(TEMP_SCREENSHOT_PATH)
        .context("Failed to read screenshot file")?;
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    Ok(image_data)
}

/// Capture screenshot using GNOME screenshot tool (fallback #2)
fn capture_with_gnome_screenshot() -> Result<Vec<u8>> {
    debug!("Capturing screenshot with gnome-screenshot");
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    let output = Command::new("gnome-screenshot")
        .args(&[
            "-f", TEMP_SCREENSHOT_PATH,  // File output
            "-p",                         // Include pointer
        ])
        .output()
        .context("Failed to execute gnome-screenshot")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gnome-screenshot failed: {}", stderr);
    }
    
    let image_data = fs::read(TEMP_SCREENSHOT_PATH)
        .context("Failed to read screenshot file")?;
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    Ok(image_data)
}

/// Capture screenshot with automatic fallback
/// 
/// Tries multiple methods in order:
/// 1. scrot (fastest, most reliable)
/// 2. ImageMagick import
/// 3. gnome-screenshot
/// 
/// # Returns
/// * `Vec<u8>` - PNG image bytes
/// 
/// # Examples
/// ```
/// let screenshot = capture_screenshot()?;
/// std::fs::write("screenshot.png", screenshot)?;
/// ```
pub fn capture_screenshot() -> Result<Vec<u8>> {
    // Try scrot first (preferred)
    match capture_with_scrot() {
        Ok(data) => return Ok(data),
        Err(e) => {
            warn!("scrot failed, trying fallback: {}", e);
        }
    }
    
    // Try ImageMagick import
    match capture_with_import() {
        Ok(data) => return Ok(data),
        Err(e) => {
            warn!("import failed, trying fallback: {}", e);
        }
    }
    
    // Try GNOME screenshot
    match capture_with_gnome_screenshot() {
        Ok(data) => return Ok(data),
        Err(e) => {
            error!("All screenshot methods failed, last error: {}", e);
        }
    }
    
    anyhow::bail!("All screenshot capture methods failed")
}

/// Capture screenshot and encode as JPEG
/// 
/// # Arguments
/// * `quality` - JPEG quality (1-100, higher = better)
/// 
/// # Returns
/// * `Vec<u8>` - JPEG image bytes
pub fn capture_screenshot_jpeg(quality: Option<u8>) -> Result<Vec<u8>> {
    let quality = quality.unwrap_or(DEFAULT_JPEG_QUALITY);
    
    debug!("Capturing screenshot as JPEG (quality={})", quality);
    
    // First capture as PNG
    let png_data = capture_screenshot()?;
    
    // Write PNG to temp file
    let temp_png = "/tmp/axonbridge_temp.png";
    let temp_jpg = "/tmp/axonbridge_temp.jpg";
    
    fs::write(temp_png, &png_data)
        .context("Failed to write temporary PNG")?;
    
    // Convert PNG to JPEG using ImageMagick
    let output = Command::new("convert")
        .args(&[
            temp_png,
            "-quality", &quality.to_string(),
            temp_jpg,
        ])
        .output();
    
    // If ImageMagick not available, just return PNG
    // (Client can handle PNG as well)
    if output.is_err() {
        warn!("ImageMagick convert not available, returning PNG");
        let _ = fs::remove_file(temp_png);
        return Ok(png_data);
    }
    
    let output = output.unwrap();
    if !output.status.success() {
        warn!("JPEG conversion failed, returning PNG");
        let _ = fs::remove_file(temp_png);
        return Ok(png_data);
    }
    
    // Read JPEG
    let jpeg_data = fs::read(temp_jpg)
        .context("Failed to read JPEG file")?;
    
    // Clean up
    let _ = fs::remove_file(temp_png);
    let _ = fs::remove_file(temp_jpg);
    
    debug!("Screenshot converted to JPEG: {} bytes", jpeg_data.len());
    
    Ok(jpeg_data)
}

/// Capture screenshot of specific window
/// 
/// # Arguments
/// * `window_id` - X11 window ID (hex format like "0x1234567")
/// 
/// # Returns
/// * `Vec<u8>` - PNG image bytes of window
pub fn capture_window_screenshot(window_id: &str) -> Result<Vec<u8>> {
    debug!("Capturing window screenshot: window_id={}", window_id);
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    // Use scrot with window ID
    let output = Command::new("scrot")
        .args(&[
            "-u",                    // Capture specific window
            "-b",                    // Include window border
            window_id,
            TEMP_SCREENSHOT_PATH,
        ])
        .output()
        .context("Failed to capture window screenshot")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Window screenshot failed: {}", stderr);
    }
    
    let image_data = fs::read(TEMP_SCREENSHOT_PATH)
        .context("Failed to read window screenshot")?;
    
    let _ = fs::remove_file(TEMP_SCREENSHOT_PATH);
    
    Ok(image_data)
}

/// Get screen dimensions
/// 
/// # Returns
/// * `(width, height)` - Screen dimensions in pixels
pub fn get_screen_size() -> Result<(u32, u32)> {
    let output = Command::new("xdpyinfo")
        .output()
        .context("Failed to execute xdpyinfo")?;
    
    if !output.status.success() {
        anyhow::bail!("xdpyinfo failed");
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse output: "  dimensions:    1920x1080 pixels (508x286 millimeters)"
    for line in stdout.lines() {
        if line.contains("dimensions:") {
            if let Some(dims) = line.split_whitespace().nth(1) {
                if let Some((w, h)) = dims.split_once('x') {
                    let width = w.parse().unwrap_or(1920);
                    let height = h.parse().unwrap_or(1080);
                    return Ok((width, height));
                }
            }
        }
    }
    
    // Default to common resolution if parsing fails
    warn!("Could not parse screen size, using default 1920x1080");
    Ok((1920, 1080))
}

/// Check if screenshot tools are available
/// 
/// # Returns
/// * `Vec<String>` - List of available screenshot tools
pub fn check_available_tools() -> Vec<String> {
    let mut available = Vec::new();
    
    // Check scrot
    if Command::new("which").arg("scrot").output().is_ok() {
        available.push("scrot".to_string());
    }
    
    // Check ImageMagick import
    if Command::new("which").arg("import").output().is_ok() {
        available.push("import".to_string());
    }
    
    // Check gnome-screenshot
    if Command::new("which").arg("gnome-screenshot").output().is_ok() {
        available.push("gnome-screenshot".to_string());
    }
    
    available
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_check_tools() {
        let tools = check_available_tools();
        assert!(!tools.is_empty(), "At least one screenshot tool should be available");
    }
    
    #[test]
    fn test_screen_size() {
        let (width, height) = get_screen_size().unwrap();
        assert!(width > 0 && height > 0);
    }
}
