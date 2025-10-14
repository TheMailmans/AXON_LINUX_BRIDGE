/*!
 * Linux Input Injection
 * 
 * Uses xdotool to inject keyboard and mouse events on Linux.
 * Enhanced with robustness improvements:
 * - Smart keypress handling (type vs key)
 * - Press/release support
 * - Scroll injection
 * - Window targeting
 * - Better error handling with error codes
 */

use std::process::Command;
use anyhow::{Result, bail};
use tracing::{info, debug, warn};

/// Error codes for structured error reporting
pub const ERROR_CODE_NO_FOCUS: &str = "NO_FOCUS";
pub const ERROR_CODE_WINDOW_NOT_FOUND: &str = "WINDOW_NOT_FOUND";
pub const ERROR_CODE_XDOTOOL_FAILED: &str = "XDOTOOL_FAILED";
pub const ERROR_CODE_INVALID_INPUT: &str = "INVALID_INPUT";

/// Smart key injection that automatically chooses between `xdotool key` and `xdotool type`
/// 
/// For printable single characters without modifiers, uses `type` to avoid keysym issues.
/// For shortcuts or special keys, uses `key` with proper keysym mapping.
/// 
/// Examples:
/// - inject_key_press("g", &[]) -> xdotool type g
/// - inject_key_press(".", &[]) -> xdotool type .  (FIXES the '.' error!)
/// - inject_key_press("l", &["ctrl"]) -> xdotool key ctrl+l
/// - inject_key_press("Escape", &[]) -> xdotool key Escape
pub fn inject_key_press(key: &str, modifiers: &[String]) -> Result<()> {
    info!("Injecting key press: key={}, modifiers={:?}", key, modifiers);
    
    // Determine if we should use 'type' instead of 'key'
    let is_printable = key.chars().count() == 1 && !key.chars().any(|c| c.is_control());
    let has_modifiers = !modifiers.is_empty();
    
    if is_printable && !has_modifiers {
        // Use type for single printable characters - more reliable for punctuation
        debug!("Using xdotool type for printable character: {}", key);
        return type_string(key);
    }
    
    // Build key combination string for xdotool
    let key_combo = if modifiers.is_empty() {
        key.to_string()
    } else {
        format!("{}+{}", modifiers.join("+"), key)
    };
    
    debug!("xdotool command: key {}", key_combo);
    
    // Execute xdotool
    let output = Command::new("xdotool")
        .arg("key")
        .arg(&key_combo)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!("xdotool key failed: {}", stderr);
        bail!("xdotool key failed: {}", stderr);
    }
    
    info!("Key press successful: {}", key_combo);
    Ok(())
}

/// Inject mouse click using xdotool
/// 
/// Moves mouse to (x, y) then clicks the specified button.
/// Supports optional click-to-focus for better reliability on unfocused windows.
pub fn inject_mouse_click(x: i32, y: i32, button: &str) -> Result<()> {
    inject_mouse_click_with_options(x, y, button, false, false)
}

/// Advanced mouse click with options for focus and timing
pub fn inject_mouse_click_with_options(
    x: i32,
    y: i32,
    button: &str,
    click_to_focus: bool,
    ensure_window_active: bool,
) -> Result<()> {
    info!("Injecting mouse click: x={}, y={}, button={}, focus={}, active={}",
          x, y, button, click_to_focus, ensure_window_active);
    
    // Move mouse first
    inject_mouse_move(x, y)?;
    
    // Small delay to ensure move completes
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // Optional: Click to focus first (improves reliability)
    if click_to_focus {
        debug!("Clicking to focus window first");
        let output = Command::new("xdotool")
            .arg("click")
            .arg("1")  // Left click to focus
            .output()?;
        
        if output.status.success() {
            std::thread::sleep(std::time::Duration::from_millis(30));
        } else {
            warn!("Focus click failed, continuing anyway");
        }
    }
    
    // Map button names to xdotool button numbers
    let button_num = match button {
        "left" => "1",
        "right" => "3",
        "middle" => "2",
        _ => {
            info!("Unknown button '{}', defaulting to left", button);
            "1"
        }
    };
    
    debug!("xdotool command: click {}", button_num);
    
    // Click
    let output = Command::new("xdotool")
        .arg("click")
        .arg(button_num)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("xdotool click failed: {}", stderr);
    }
    
    info!("Mouse click successful at ({}, {})", x, y);
    Ok(())
}

/// Inject mouse button press (down)
pub fn inject_mouse_press(x: i32, y: i32, button: &str) -> Result<()> {
    info!("Injecting mouse press: x={}, y={}, button={}", x, y, button);
    
    // Move mouse first
    inject_mouse_move(x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    let button_num = match button {
        "left" => "1",
        "right" => "3",
        "middle" => "2",
        _ => "1",
    };
    
    debug!("xdotool command: mousedown {}", button_num);
    
    let output = Command::new("xdotool")
        .arg("mousedown")
        .arg(button_num)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("xdotool mousedown failed: {}", stderr);
    }
    
    info!("Mouse press successful at ({}, {})", x, y);
    Ok(())
}

/// Inject mouse button release (up)
pub fn inject_mouse_release(x: i32, y: i32, button: &str) -> Result<()> {
    info!("Injecting mouse release: x={}, y={}, button={}", x, y, button);
    
    let button_num = match button {
        "left" => "1",
        "right" => "3",
        "middle" => "2",
        _ => "1",
    };
    
    debug!("xdotool command: mouseup {}", button_num);
    
    let output = Command::new("xdotool")
        .arg("mouseup")
        .arg(button_num)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("xdotool mouseup failed: {}", stderr);
    }
    
    info!("Mouse release successful");
    Ok(())
}

/// Inject scroll event
pub fn inject_scroll(x: i32, y: i32, delta_x: i32, delta_y: i32) -> Result<()> {
    info!("Injecting scroll: x={}, y={}, dx={}, dy={}", x, y, delta_x, delta_y);
    
    // Move mouse to position first
    inject_mouse_move(x, y)?;
    std::thread::sleep(std::time::Duration::from_millis(10));
    
    // xdotool uses button 4 (up) and 5 (down) for vertical scroll
    // button 6 (left) and 7 (right) for horizontal scroll
    
    // Handle vertical scroll
    if delta_y != 0 {
        let button = if delta_y > 0 { "4" } else { "5" };  // 4=up, 5=down
        let repeats = delta_y.abs();
        
        for _ in 0..repeats {
            Command::new("xdotool")
                .arg("click")
                .arg(button)
                .output()?;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    
    // Handle horizontal scroll
    if delta_x != 0 {
        let button = if delta_x > 0 { "7" } else { "6" };  // 6=left, 7=right
        let repeats = delta_x.abs();
        
        for _ in 0..repeats {
            Command::new("xdotool")
                .arg("click")
                .arg(button)
                .output()?;
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
    
    info!("Scroll successful");
    Ok(())
}

/// Inject mouse move using xdotool
pub fn inject_mouse_move(x: i32, y: i32) -> Result<()> {
    debug!("Injecting mouse move: x={}, y={}", x, y);
    
    let output = Command::new("xdotool")
        .arg("mousemove")
        // Note: --sync flag removed as it causes 10-15 second hangs with some WMs
        .arg(x.to_string())
        .arg(y.to_string())
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("xdotool mousemove failed: {}", stderr);
    }
    
    debug!("Mouse move successful");
    Ok(())
}

/// Type string using xdotool
/// 
/// Types the string character by character.
/// Note: Uses xdotool type which handles special characters.
pub fn type_string(text: &str) -> Result<()> {
    type_string_with_delay(text, 12)
}

/// Type string with custom delay between characters
pub fn type_string_with_delay(text: &str, delay_ms: i32) -> Result<()> {
    info!("Typing string: {} chars (delay: {}ms)", text.len(), delay_ms);
    
    let output = Command::new("xdotool")
        .arg("type")
        .arg("--delay")
        .arg(delay_ms.to_string())
        .arg("--")  // End of options
        .arg(text)
        .output()?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("xdotool type failed: {}", stderr);
    }
    
    info!("String typed successfully");
    Ok(())
}

/// Get information about the currently active window
pub struct WindowInfo {
    pub window_id: String,
    pub title: String,
    pub app_name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub fn get_active_window() -> Result<WindowInfo> {
    info!("Getting active window info");
    
    // Get active window ID
    let output = Command::new("xdotool")
        .arg("getactivewindow")
        .output()?;
    
    if !output.status.success() {
        bail!("Failed to get active window ID");
    }
    
    let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Get window name
    let output = Command::new("xdotool")
        .arg("getwindowname")
        .arg(&window_id)
        .output()?;
    
    let title = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        String::new()
    };
    
    // Get window geometry
    let output = Command::new("xdotool")
        .arg("getwindowgeometry")
        .arg(&window_id)
        .output()?;
    
    let (x, y, width, height) = if output.status.success() {
        let geometry_str = String::from_utf8_lossy(&output.stdout);
        parse_window_geometry(&geometry_str).unwrap_or((0, 0, 0, 0))
    } else {
        (0, 0, 0, 0)
    };
    
    // Get WM_CLASS (app name) using xprop
    let output = Command::new("xprop")
        .arg("-id")
        .arg(&window_id)
        .arg("WM_CLASS")
        .output()?;
    
    let app_name = if output.status.success() {
        let class_str = String::from_utf8_lossy(&output.stdout);
        parse_wm_class(&class_str).unwrap_or_else(|| "unknown".to_string())
    } else {
        "unknown".to_string()
    };
    
    Ok(WindowInfo {
        window_id,
        title,
        app_name,
        x,
        y,
        width,
        height,
    })
}

fn parse_window_geometry(geometry_str: &str) -> Option<(i32, i32, i32, i32)> {
    // Parse xdotool getwindowgeometry output
    // Position: X,Y
    // Geometry: WxH
    let mut x = 0;
    let mut y = 0;
    let mut width = 0;
    let mut height = 0;
    
    for line in geometry_str.lines() {
        if line.contains("Position:") {
            if let Some(pos) = line.split_whitespace().nth(1) {
                let parts: Vec<&str> = pos.split(',').collect();
                if parts.len() == 2 {
                    x = parts[0].parse().unwrap_or(0);
                    y = parts[1].parse().unwrap_or(0);
                }
            }
        } else if line.contains("Geometry:") {
            if let Some(geo) = line.split_whitespace().nth(1) {
                let parts: Vec<&str> = geo.split('x').collect();
                if parts.len() == 2 {
                    width = parts[0].parse().unwrap_or(0);
                    height = parts[1].parse().unwrap_or(0);
                }
            }
        }
    }
    
    Some((x, y, width, height))
}

fn parse_wm_class(class_str: &str) -> Option<String> {
    // Parse WM_CLASS output: WM_CLASS(STRING) = "app", "App"
    if let Some(start) = class_str.find('"') {
        if let Some(end) = class_str[start + 1..].find('"') {
            return Some(class_str[start + 1..start + 1 + end].to_string());
        }
    }
    None
}

/// Check system capabilities for input injection
pub struct Capabilities {
    pub display_server: String,
    pub input_method: String,
    pub capture_method: String,
    pub supports_x11: bool,
    pub supports_wayland: bool,
    pub supports_press_release: bool,
    pub supports_scroll: bool,
    pub supports_a11y: bool,
}

pub fn get_capabilities() -> Capabilities {
    let session_type = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_else(|_| "unknown".to_string());
    
    let xdotool_available = Command::new("which")
        .arg("xdotool")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    let scrot_available = Command::new("which")
        .arg("scrot")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    
    Capabilities {
        display_server: session_type.clone(),
        input_method: if xdotool_available { "xdotool" } else { "none" }.to_string(),
        capture_method: if scrot_available { "scrot" } else { "none" }.to_string(),
        supports_x11: session_type == "x11",
        supports_wayland: session_type == "wayland",
        supports_press_release: xdotool_available,
        supports_scroll: xdotool_available,
        supports_a11y: true, // at-spi support is checked at runtime
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore] // Only run manually on Linux with X11
    fn test_key_press_simple() {
        let result = inject_key_press("g", &[]);
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_key_press_with_modifiers() {
        let result = inject_key_press("l", &["ctrl".to_string()]);
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_mouse_click() {
        let result = inject_mouse_click(100, 100, "left");
        assert!(result.is_ok());
    }
    
    #[test]
    #[ignore]
    fn test_type_string() {
        let result = type_string("Hello World");
        assert!(result.is_ok());
    }
}
