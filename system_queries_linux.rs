// AXONBRIDGE Linux - System Queries Module
// Production-ready system state queries for Linux/X11
// Window lists, process lists, active windows, etc.

use anyhow::{Context, Result};
use std::process::Command;
use tracing::{debug, warn};

/// Get list of all visible windows
/// 
/// # Returns
/// * `Vec<String>` - List of window titles
/// 
/// # Examples
/// ```
/// let windows = get_window_list()?;
/// for window in windows {
///     println!("Window: {}", window);
/// }
/// ```
pub fn get_window_list() -> Result<Vec<String>> {
    debug!("Getting window list");
    
    // Use wmctrl to list windows
    // -l = list windows
    let output = Command::new("wmctrl")
        .arg("-l")
        .output()
        .context("Failed to execute wmctrl")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("wmctrl failed: {}", stderr);
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse wmctrl output
    // Format: "0x04600003  0 hostname Window Title Here"
    let windows: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 3 {
                // Join parts 3+ as window title
                Some(parts[3..].join(" "))
            } else {
                None
            }
        })
        .filter(|title| !title.is_empty())
        .collect();
    
    debug!("Found {} windows", windows.len());
    
    Ok(windows)
}

/// Get list of window IDs and titles
/// 
/// # Returns
/// * `Vec<(String, String)>` - List of (window_id, title) tuples
pub fn get_window_list_with_ids() -> Result<Vec<(String, String)>> {
    debug!("Getting window list with IDs");
    
    let output = Command::new("wmctrl")
        .arg("-l")
        .output()
        .context("Failed to execute wmctrl")?;
    
    if !output.status.success() {
        anyhow::bail!("wmctrl failed");
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let windows: Vec<(String, String)> = stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 3 {
                let window_id = parts[0].to_string();
                let title = parts[3..].join(" ");
                Some((window_id, title))
            } else {
                None
            }
        })
        .collect();
    
    Ok(windows)
}

/// Get active (focused) window title
/// 
/// # Returns
/// * `String` - Title of currently focused window
pub fn get_active_window() -> Result<String> {
    debug!("Getting active window");
    
    // Use xdotool to get active window title
    let output = Command::new("xdotool")
        .args(&["getactivewindow", "getwindowname"])
        .output()
        .context("Failed to execute xdotool")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("xdotool getactivewindow failed: {}", stderr);
    }
    
    let title = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    
    debug!("Active window: '{}'", title);
    
    Ok(title)
}

/// Get active window ID
/// 
/// # Returns
/// * `String` - Window ID in hex format (e.g., "0x1234567")
pub fn get_active_window_id() -> Result<String> {
    let output = Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .context("Failed to get active window ID")?;
    
    if !output.status.success() {
        anyhow::bail!("xdotool getactivewindow failed");
    }
    
    let window_id = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();
    
    // Convert decimal to hex format
    if let Ok(id_dec) = window_id.parse::<u64>() {
        Ok(format!("0x{:x}", id_dec))
    } else {
        Ok(window_id)
    }
}

/// Get list of running processes
/// 
/// # Returns
/// * `Vec<String>` - List of process names
pub fn get_process_list() -> Result<Vec<String>> {
    debug!("Getting process list");
    
    // Use ps to list processes
    let output = Command::new("ps")
        .args(&["aux"])
        .output()
        .context("Failed to execute ps")?;
    
    if !output.status.success() {
        anyhow::bail!("ps command failed");
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Parse ps output (skip header line)
    let processes: Vec<String> = stdout
        .lines()
        .skip(1)
        .filter_map(|line| {
            // Column 10 is the command
            line.split_whitespace().nth(10).map(|s| {
                // Remove path, keep just process name
                s.split('/').last().unwrap_or(s).to_string()
            })
        })
        .collect();
    
    debug!("Found {} processes", processes.len());
    
    Ok(processes)
}

/// Get list of running application processes (GUI apps)
/// 
/// Filters for common GUI applications
/// 
/// # Returns
/// * `Vec<String>` - List of application names
pub fn get_application_list() -> Result<Vec<String>> {
    debug!("Getting application list");
    
    let all_processes = get_process_list()?;
    
    // Filter for common GUI applications
    let gui_apps: Vec<String> = all_processes
        .into_iter()
        .filter(|proc| {
            // Common GUI app patterns
            !proc.starts_with('[')  // Skip kernel threads
                && !proc.contains("systemd")
                && !proc.contains("dbus")
                && !proc.contains("gvfs")
                && (
                    proc.contains("firefox")
                    || proc.contains("chrome")
                    || proc.contains("code")
                    || proc.contains("gimp")
                    || proc.contains("libreoffice")
                    || proc.contains("thunderbird")
                    || proc.contains("vlc")
                    || proc.contains("gnome")
                    || proc.contains("nautilus")
                    || proc.contains("terminal")
                )
        })
        .collect();
    
    Ok(gui_apps)
}

/// Get window ID by window title (partial match)
/// 
/// # Arguments
/// * `title_pattern` - Pattern to match in window title
/// 
/// # Returns
/// * `Option<String>` - Window ID if found
pub fn find_window_by_title(title_pattern: &str) -> Result<Option<String>> {
    debug!("Finding window by title: '{}'", title_pattern);
    
    let windows = get_window_list_with_ids()?;
    
    let pattern_lower = title_pattern.to_lowercase();
    
    for (window_id, title) in windows {
        if title.to_lowercase().contains(&pattern_lower) {
            debug!("Found window: id={}, title='{}'", window_id, title);
            return Ok(Some(window_id));
        }
    }
    
    debug!("No window found matching '{}'", title_pattern);
    Ok(None)
}

/// Focus (activate) a window by its ID
/// 
/// # Arguments
/// * `window_id` - Window ID in hex format
pub fn focus_window(window_id: &str) -> Result<()> {
    debug!("Focusing window: {}", window_id);
    
    let output = Command::new("wmctrl")
        .args(&["-i", "-a", window_id])
        .output()
        .context("Failed to focus window")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to focus window: {}", stderr);
    }
    
    Ok(())
}

/// Close a window by its ID
/// 
/// # Arguments
/// * `window_id` - Window ID in hex format
pub fn close_window(window_id: &str) -> Result<()> {
    debug!("Closing window: {}", window_id);
    
    let output = Command::new("wmctrl")
        .args(&["-i", "-c", window_id])
        .output()
        .context("Failed to close window")?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to close window: {}", stderr);
    }
    
    Ok(())
}

/// Get desktop/workspace information
/// 
/// # Returns
/// * `(current_desktop, total_desktops)` - Desktop numbers
pub fn get_desktop_info() -> Result<(u32, u32)> {
    let output = Command::new("wmctrl")
        .arg("-d")
        .output()
        .context("Failed to get desktop info")?;
    
    if !output.status.success() {
        return Ok((0, 1)); // Default to single desktop
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let mut current = 0;
    let mut total = 0;
    
    for line in stdout.lines() {
        total += 1;
        if line.contains('*') {
            // Current desktop is marked with *
            if let Some(num_str) = line.split_whitespace().next() {
                current = num_str.parse().unwrap_or(0);
            }
        }
    }
    
    Ok((current, total))
}

/// Check if a specific application is running
/// 
/// # Arguments
/// * `app_name` - Application name to search for
/// 
/// # Returns
/// * `bool` - true if application is running
pub fn is_app_running(app_name: &str) -> Result<bool> {
    let processes = get_process_list()?;
    let app_lower = app_name.to_lowercase();
    
    Ok(processes.iter().any(|proc| proc.to_lowercase().contains(&app_lower)))
}

/// Get system information
/// 
/// # Returns
/// * `(os_name, os_version, desktop_environment)`
pub fn get_system_info() -> Result<(String, String, String)> {
    // Get OS name/version from /etc/os-release
    let os_info = std::fs::read_to_string("/etc/os-release")
        .unwrap_or_else(|_| "NAME=Linux\nVERSION=Unknown".to_string());
    
    let mut os_name = "Linux".to_string();
    let mut os_version = "Unknown".to_string();
    
    for line in os_info.lines() {
        if let Some(name) = line.strip_prefix("NAME=") {
            os_name = name.trim_matches('"').to_string();
        } else if let Some(version) = line.strip_prefix("VERSION=") {
            os_version = version.trim_matches('"').to_string();
        }
    }
    
    // Detect desktop environment
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_else(|_| "Unknown".to_string());
    
    Ok((os_name, os_version, desktop))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_get_window_list() {
        let windows = get_window_list().unwrap();
        assert!(!windows.is_empty(), "Should have at least one window");
    }
    
    #[test]
    fn test_get_process_list() {
        let processes = get_process_list().unwrap();
        assert!(!processes.is_empty(), "Should have running processes");
    }
    
    #[test]
    fn test_get_active_window() {
        let window = get_active_window();
        assert!(window.is_ok(), "Should be able to get active window");
    }
    
    #[test]
    fn test_get_system_info() {
        let (os_name, os_version, desktop) = get_system_info().unwrap();
        assert!(!os_name.is_empty());
        assert!(!os_version.is_empty());
        assert!(!desktop.is_empty());
    }
}
