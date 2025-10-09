/*!
 * Accessibility Tree Capture
 * 
 * Captures AT-SPI accessibility tree on Linux.
 */

use anyhow::{Result, anyhow};
use std::process::Command;
use tracing::{debug, warn};

/// Capture accessibility tree using AT-SPI
/// 
/// On Linux, this uses the AT-SPI D-Bus interface to get the accessibility tree.
/// For OSWorld environments, it can also read from a Python controller.
#[cfg(target_os = "linux")]
pub fn capture_accessibility_tree() -> Result<String> {
    // Try Python controller first (for OSWorld compatibility)
    if let Ok(tree) = capture_via_python_controller() {
        return Ok(tree);
    }
    
    // Fallback to direct AT-SPI capture
    capture_via_atspi()
}

#[cfg(not(target_os = "linux"))]
pub fn capture_accessibility_tree() -> Result<String> {
    Err(anyhow!("A11y tree capture only supported on Linux"))
}

/// Capture via Python controller (OSWorld compatibility)
fn capture_via_python_controller() -> Result<String> {
    debug!("Attempting to capture a11y tree via Python controller");
    
    let output = Command::new("python3")
        .arg("-c")
        .arg(r#"
from desktop_env.controllers.python import PythonController
controller = PythonController()
tree = controller.get_accessibility_tree()
print(tree)
"#)
        .output()
        .map_err(|e| anyhow!("Failed to run Python: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Python controller failed: {}", stderr));
    }
    
    let tree = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8 in a11y tree: {}", e))?;
    
    if tree.len() < 100 {
        return Err(anyhow!("A11y tree too small: {} bytes", tree.len()));
    }
    
    debug!("Captured a11y tree via Python: {} bytes", tree.len());
    Ok(tree)
}

/// Capture directly via AT-SPI using atspi2-tool
fn capture_via_atspi() -> Result<String> {
    debug!("Attempting to capture a11y tree via AT-SPI");
    
    // Try using accerciser or at-spi2-core tools if available
    // This is a simplified implementation
    
    let output = Command::new("busctl")
        .arg("tree")
        .arg("org.a11y.atspi.Registry")
        .output()
        .map_err(|e| anyhow!("Failed to query AT-SPI: {}", e))?;
    
    if !output.status.success() {
        return Err(anyhow!("AT-SPI query failed"));
    }
    
    let tree = String::from_utf8(output.stdout)
        .map_err(|e| anyhow!("Invalid UTF-8: {}", e))?;
    
    // This is a very basic implementation
    // In production, you'd use proper AT-SPI bindings
    warn!("Using basic AT-SPI capture - may not include all shortcuts");
    Ok(tree)
}

/// Mock capture for testing
#[cfg(test)]
pub fn capture_accessibility_tree_mock() -> Result<String> {
    Ok(r#"
        <desktop-frame xmlns:attr="https://accessibility.ubuntu.example.org/ns/attributes">
            <application name="google-chrome">
                <menu name="Chrome">
                    <menuitem name="Settings" attr:keyshortcuts="g"/>
                    <entry name="Address and search bar" attr:keyshortcuts="Ctrl+L"/>
                </menu>
            </application>
        </desktop-frame>
    "#.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mock_capture() {
        let tree = capture_accessibility_tree_mock().unwrap();
        assert!(tree.contains("keyshortcuts"));
        assert!(tree.len() > 100);
    }
}
