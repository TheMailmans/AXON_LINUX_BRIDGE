/*!
 * Linux Input Injection
 * 
 * Uses xdotool to inject keyboard and mouse events on Linux.
 */

use std::process::Command;
use anyhow::{Result, bail};
use tracing::{info, debug};

/// Inject keyboard shortcut using xdotool
/// 
/// Examples:
/// - inject_key_press("g", &[]) -> xdotool key g
/// - inject_key_press("l", &["ctrl"]) -> xdotool key ctrl+l
/// - inject_key_press("n", &["ctrl", "shift"]) -> xdotool key ctrl+shift+n
pub fn inject_key_press(key: &str, modifiers: &[String]) -> Result<()> {
    info!("Injecting key press: key={}, modifiers={:?}", key, modifiers);
    
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
        bail!("xdotool key failed: {}", stderr);
    }
    
    info!("Key press successful: {}", key_combo);
    Ok(())
}

/// Inject mouse click using xdotool
/// 
/// Moves mouse to (x, y) then clicks the specified button.
pub fn inject_mouse_click(x: i32, y: i32, button: &str) -> Result<()> {
    info!("Injecting mouse click: x={}, y={}, button={}", x, y, button);
    
    // Move mouse first
    inject_mouse_move(x, y)?;
    
    // Small delay to ensure move completes
    std::thread::sleep(std::time::Duration::from_millis(10));
    
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
    info!("Typing string: {} chars", text.len());
    
    let output = Command::new("xdotool")
        .arg("type")
        .arg("--delay")
        .arg("12")  // 12ms delay between keys (natural typing speed)
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
