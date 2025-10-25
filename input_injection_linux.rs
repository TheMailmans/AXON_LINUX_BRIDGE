// AXONBRIDGE Linux - Input Injection Module
// Production-ready keyboard and mouse control for Linux/X11
// Uses xdotool for reliability and compatibility

use anyhow::{Context, Result};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Delay between key presses (milliseconds)
const KEY_DELAY_MS: u64 = 10;

/// Delay between modifier and key (milliseconds)
const MODIFIER_DELAY_MS: u64 = 50;

/// Maximum retries for flaky operations
const MAX_RETRIES: u32 = 3;

/// Map common key names to xdotool key codes
fn map_key_to_xdotool(key: &str) -> String {
    match key.to_lowercase().as_str() {
        // Special keys
        "return" | "enter" => "Return".to_string(),
        "escape" | "esc" => "Escape".to_string(),
        "tab" => "Tab".to_string(),
        "space" => "space".to_string(),
        "backspace" => "BackSpace".to_string(),
        "delete" | "del" => "Delete".to_string(),
        
        // Arrow keys
        "up" => "Up".to_string(),
        "down" => "Down".to_string(),
        "left" => "Left".to_string(),
        "right" => "Right".to_string(),
        
        // Modifiers
        "cmd" | "command" | "super" | "win" => "Super_L".to_string(),
        "ctrl" | "control" => "Control_L".to_string(),
        "alt" | "option" => "Alt_L".to_string(),
        "shift" => "Shift_L".to_string(),
        
        // Function keys
        k if k.starts_with('f') && k.len() <= 3 => {
            if let Ok(num) = k[1..].parse::<u32>() {
                if (1..=12).contains(&num) {
                    return format!("F{}", num);
                }
            }
            key.to_string()
        }
        
        // Default: pass through as-is
        _ => key.to_string(),
    }
}

/// Map modifier names to xdotool format
fn map_modifier_to_xdotool(modifier: &str) -> String {
    match modifier.to_lowercase().as_str() {
        "cmd" | "command" | "super" | "win" => "super".to_string(),
        "ctrl" | "control" => "ctrl".to_string(),
        "alt" | "option" => "alt".to_string(),
        "shift" => "shift".to_string(),
        _ => modifier.to_lowercase(),
    }
}

/// Execute xdotool command with retry logic
fn execute_xdotool(args: &[&str]) -> Result<()> {
    for attempt in 1..=MAX_RETRIES {
        let output = Command::new("xdotool")
            .args(args)
            .output()
            .context("Failed to execute xdotool")?;
        
        if output.status.success() {
            debug!("xdotool command succeeded: {:?}", args);
            return Ok(());
        }
        
        if attempt < MAX_RETRIES {
            warn!(
                "xdotool command failed (attempt {}/{}): {:?}",
                attempt, MAX_RETRIES, args
            );
            thread::sleep(Duration::from_millis(100));
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("xdotool command failed after {} attempts: {}", MAX_RETRIES, stderr);
            anyhow::bail!(
                "xdotool command failed: {} (stderr: {})",
                args.join(" "),
                stderr
            );
        }
    }
    
    unreachable!()
}

/// Inject a key press with optional modifiers
/// 
/// # Arguments
/// * `key` - The key to press (e.g., "a", "return", "f1")
/// * `modifiers` - Modifier keys (e.g., ["ctrl", "shift"])
/// 
/// # Examples
/// ```
/// // Press 'a'
/// inject_key_press("a", &[])?;
/// 
/// // Press Cmd+C (copy)
/// inject_key_press("c", &["cmd"])?;
/// 
/// // Press Ctrl+Shift+T
/// inject_key_press("t", &["ctrl", "shift"])?;
/// ```
pub fn inject_key_press(key: &str, modifiers: &[String]) -> Result<()> {
    debug!("Injecting key press: key='{}', modifiers={:?}", key, modifiers);
    
    // Map key and modifiers to xdotool format
    let mapped_key = map_key_to_xdotool(key);
    let mapped_modifiers: Vec<String> = modifiers
        .iter()
        .map(|m| map_modifier_to_xdotool(m))
        .collect();
    
    // Build key combination string
    let key_combo = if mapped_modifiers.is_empty() {
        mapped_key.clone()
    } else {
        format!("{}+{}", mapped_modifiers.join("+"), mapped_key)
    };
    
    // Execute xdotool key command
    execute_xdotool(&["key", "--clearmodifiers", &key_combo])?;
    
    // Small delay for key to register
    thread::sleep(Duration::from_millis(KEY_DELAY_MS));
    
    Ok(())
}

/// Inject multiple key presses in sequence
/// 
/// # Arguments
/// * `keys` - Array of keys to press
/// * `delay_ms` - Delay between keys (optional, defaults to KEY_DELAY_MS)
pub fn inject_key_sequence(keys: &[&str], delay_ms: Option<u64>) -> Result<()> {
    let delay = delay_ms.unwrap_or(KEY_DELAY_MS);
    
    for key in keys {
        inject_key_press(key, &[])?;
        thread::sleep(Duration::from_millis(delay));
    }
    
    Ok(())
}

/// Type a string of text
/// 
/// # Arguments
/// * `text` - The text to type
/// 
/// # Examples
/// ```
/// inject_text("Hello, World!")?;
/// ```
pub fn inject_text(text: &str) -> Result<()> {
    debug!("Injecting text: '{}'", text);
    
    // Use xdotool type for natural typing
    execute_xdotool(&["type", "--delay", "50", "--clearmodifiers", text])?;
    
    Ok(())
}

/// Inject a mouse click at current position
/// 
/// # Arguments
/// * `button` - Mouse button ("left", "right", "middle", or button number 1-3)
/// 
/// # Examples
/// ```
/// inject_mouse_click("left")?;
/// inject_mouse_click("right")?;
/// ```
pub fn inject_mouse_click(button: &str) -> Result<()> {
    debug!("Injecting mouse click: button='{}'", button);
    
    // Map button name to number
    let button_num = match button.to_lowercase().as_str() {
        "left" | "1" => "1",
        "middle" | "2" => "2",
        "right" | "3" => "3",
        _ => {
            warn!("Unknown mouse button '{}', defaulting to left", button);
            "1"
        }
    };
    
    // Click at current mouse position
    execute_xdotool(&["click", button_num])?;
    
    Ok(())
}

/// Move mouse to absolute coordinates
/// 
/// # Arguments
/// * `x` - X coordinate (pixels from left)
/// * `y` - Y coordinate (pixels from top)
/// 
/// # Examples
/// ```
/// inject_mouse_move(100, 200)?;
/// ```
pub fn inject_mouse_move(x: i32, y: i32) -> Result<()> {
    debug!("Moving mouse to: x={}, y={}", x, y);
    
    // Move mouse to absolute position
    execute_xdotool(&["mousemove", &x.to_string(), &y.to_string()])?;
    
    // Small delay for mouse to settle
    thread::sleep(Duration::from_millis(10));
    
    Ok(())
}

/// Click at specific coordinates
/// 
/// # Arguments
/// * `x` - X coordinate
/// * `y` - Y coordinate  
/// * `button` - Mouse button ("left", "right", "middle")
/// 
/// # Examples
/// ```
/// inject_mouse_click_at(100, 200, "left")?;
/// ```
pub fn inject_mouse_click_at(x: i32, y: i32, button: &str) -> Result<()> {
    debug!("Clicking at: x={}, y={}, button='{}'", x, y, button);
    
    // Move to position
    inject_mouse_move(x, y)?;
    
    // Small delay
    thread::sleep(Duration::from_millis(50));
    
    // Click
    inject_mouse_click(button)?;
    
    Ok(())
}

/// Drag mouse from one position to another
/// 
/// # Arguments
/// * `from_x` - Starting X coordinate
/// * `from_y` - Starting Y coordinate
/// * `to_x` - Ending X coordinate
/// * `to_y` - Ending Y coordinate
/// 
/// # Examples
/// ```
/// inject_mouse_drag(100, 100, 300, 300)?;
/// ```
pub fn inject_mouse_drag(from_x: i32, from_y: i32, to_x: i32, to_y: i32) -> Result<()> {
    debug!(
        "Dragging mouse from ({}, {}) to ({}, {})",
        from_x, from_y, to_x, to_y
    );
    
    // Move to start position
    inject_mouse_move(from_x, from_y)?;
    thread::sleep(Duration::from_millis(100));
    
    // Press and hold left button
    execute_xdotool(&["mousedown", "1"])?;
    thread::sleep(Duration::from_millis(100));
    
    // Move to end position
    inject_mouse_move(to_x, to_y)?;
    thread::sleep(Duration::from_millis(100));
    
    // Release button
    execute_xdotool(&["mouseup", "1"])?;
    
    Ok(())
}

/// Scroll mouse wheel
/// 
/// # Arguments
/// * `amount` - Scroll amount (positive = up, negative = down)
/// 
/// # Examples
/// ```
/// inject_mouse_scroll(5)?;   // Scroll up 5 units
/// inject_mouse_scroll(-3)?;  // Scroll down 3 units
/// ```
pub fn inject_mouse_scroll(amount: i32) -> Result<()> {
    debug!("Scrolling mouse: amount={}", amount);
    
    let button = if amount > 0 { "4" } else { "5" }; // 4 = up, 5 = down
    let count = amount.abs();
    
    for _ in 0..count {
        execute_xdotool(&["click", button])?;
        thread::sleep(Duration::from_millis(50));
    }
    
    Ok(())
}

/// Get current mouse position
/// 
/// # Returns
/// * `(x, y)` - Current mouse coordinates
pub fn get_mouse_position() -> Result<(i32, i32)> {
    let output = Command::new("xdotool")
        .args(&["getmouselocation", "--shell"])
        .output()
        .context("Failed to get mouse position")?;
    
    if !output.status.success() {
        anyhow::bail!("xdotool getmouselocation failed");
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut x = 0;
    let mut y = 0;
    
    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("X=") {
            x = value.parse().unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("Y=") {
            y = value.parse().unwrap_or(0);
        }
    }
    
    Ok((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_mapping() {
        assert_eq!(map_key_to_xdotool("return"), "Return");
        assert_eq!(map_key_to_xdotool("cmd"), "Super_L");
        assert_eq!(map_key_to_xdotool("ctrl"), "Control_L");
    }
    
    #[test]
    fn test_modifier_mapping() {
        assert_eq!(map_modifier_to_xdotool("cmd"), "super");
        assert_eq!(map_modifier_to_xdotool("ctrl"), "ctrl");
    }
}
