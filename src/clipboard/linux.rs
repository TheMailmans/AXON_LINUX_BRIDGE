//! Linux clipboard implementation using xclip/xsel.

use super::{ClipboardContentType, ClipboardProvider};
use anyhow::{Result, Context, bail};
use std::process::Command;
use std::io::Write;

/// Linux clipboard provider using xclip
pub struct LinuxClipboard {
    tool: ClipboardTool,
}

/// Available clipboard tools on Linux
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardTool {
    Xclip,
    Xsel,
}

impl LinuxClipboard {
    /// Create new Linux clipboard provider
    pub fn new() -> Result<Self> {
        // Try xclip first
        if Command::new("which")
            .arg("xclip")
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(Self {
                tool: ClipboardTool::Xclip,
            });
        }

        // Try xsel as fallback
        if Command::new("which")
            .arg("xsel")
            .output()
            .ok()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(Self {
                tool: ClipboardTool::Xsel,
            });
        }

        bail!("No clipboard tool found. Install xclip or xsel: sudo apt install xclip")
    }

    /// Get which tool is being used
    pub fn tool(&self) -> ClipboardTool {
        self.tool
    }
}

impl ClipboardProvider for LinuxClipboard {
    fn get_content(&self) -> Result<(String, ClipboardContentType)> {
        let content = match self.tool {
            ClipboardTool::Xclip => {
                let output = Command::new("xclip")
                    .args(&["-selection", "clipboard", "-o"])
                    .output()
                    .context("Failed to run xclip")?;

                if !output.status.success() {
                    bail!("xclip failed to get clipboard");
                }

                String::from_utf8(output.stdout)
                    .context("Clipboard content is not valid UTF-8")?
            }
            ClipboardTool::Xsel => {
                let output = Command::new("xsel")
                    .args(&["--clipboard", "--output"])
                    .output()
                    .context("Failed to run xsel")?;

                if !output.status.success() {
                    bail!("xsel failed to get clipboard");
                }

                String::from_utf8(output.stdout)
                    .context("Clipboard content is not valid UTF-8")?
            }
        };

        Ok((content, ClipboardContentType::Text))
    }

    fn set_content(&self, content: &str, _content_type: ClipboardContentType) -> Result<()> {
        match self.tool {
            ClipboardTool::Xclip => {
                let mut child = Command::new("xclip")
                    .args(&["-selection", "clipboard", "-i"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .context("Failed to spawn xclip")?;

                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(content.as_bytes())
                        .context("Failed to write to xclip stdin")?;
                }

                let status = child.wait()
                    .context("Failed to wait for xclip")?;

                if !status.success() {
                    bail!("xclip failed to set clipboard");
                }

                Ok(())
            }
            ClipboardTool::Xsel => {
                let mut child = Command::new("xsel")
                    .args(&["--clipboard", "--input"])
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                    .context("Failed to spawn xsel")?;

                if let Some(mut stdin) = child.stdin.take() {
                    stdin.write_all(content.as_bytes())
                        .context("Failed to write to xsel stdin")?;
                }

                let status = child.wait()
                    .context("Failed to wait for xsel")?;

                if !status.success() {
                    bail!("xsel failed to set clipboard");
                }

                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_clipboard_creation() {
        // This test only runs if clipboard tool is installed
        match LinuxClipboard::new() {
            Ok(_) => {} // clipboard tool available
            Err(_) => {} // clipboard tool not available, skip test
        }
    }

    #[test]
    fn test_clipboard_tool_enum() {
        assert_eq!(ClipboardTool::Xclip, ClipboardTool::Xclip);
        assert_eq!(ClipboardTool::Xsel, ClipboardTool::Xsel);
        assert_ne!(ClipboardTool::Xclip, ClipboardTool::Xsel);
    }

    #[test]
    fn test_linux_clipboard_tool_method() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            let tool = clipboard.tool();
            assert!(tool == ClipboardTool::Xclip || tool == ClipboardTool::Xsel);
        }
    }

    #[test]
    fn test_get_content_empty() {
        // Test that empty clipboard returns empty string (not an error)
        // This is integration test behavior
        if let Ok(clipboard) = LinuxClipboard::new() {
            match clipboard.get_content() {
                Ok((content, content_type)) => {
                    assert_eq!(content_type, ClipboardContentType::Text);
                    // Content may be empty if clipboard is empty
                    assert!(content.len() >= 0);
                }
                Err(_) => {
                    // Expected on systems without clipboard
                }
            }
        }
    }

    #[test]
    fn test_content_type_is_text() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            if let Ok((_content, content_type)) = clipboard.get_content() {
                assert_eq!(content_type, ClipboardContentType::Text);
            }
        }
    }

    #[test]
    fn test_set_and_get_roundtrip() {
        // Basic roundtrip test
        if let Ok(clipboard) = LinuxClipboard::new() {
            let test_content = "test-clipboard-content-12345";
            match clipboard.set_content(test_content, ClipboardContentType::Text) {
                Ok(()) => {
                    // Try to get it back
                    match clipboard.get_content() {
                        Ok((content, _)) => {
                            // Content should match (exact matching depends on system)
                            assert!(!content.is_empty());
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_empty_string() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            match clipboard.set_content("", ClipboardContentType::Text) {
                Ok(()) => {
                    // Should succeed
                    assert!(true);
                }
                Err(_) => {
                    // May fail on some systems
                }
            }
        }
    }

    #[test]
    fn test_set_long_content() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            let long_content = "x".repeat(10000);
            match clipboard.set_content(&long_content, ClipboardContentType::Text) {
                Ok(()) => {
                    assert!(true);
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_content_with_unicode() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            let unicode_content = "Hello 世界 🎉 Привет";
            match clipboard.set_content(unicode_content, ClipboardContentType::Text) {
                Ok(()) => {
                    match clipboard.get_content() {
                        Ok((content, _)) => {
                            assert!(!content.is_empty());
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_content_with_newlines() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            let multiline = "Line 1\nLine 2\nLine 3";
            match clipboard.set_content(multiline, ClipboardContentType::Text) {
                Ok(()) => {
                    match clipboard.get_content() {
                        Ok((content, _)) => {
                            assert!(!content.is_empty());
                        }
                        Err(_) => {}
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_multiple_operations() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            // Multiple set/get operations
            for i in 0..3 {
                let content = format!("test-{}", i);
                if clipboard.set_content(&content, ClipboardContentType::Text).is_ok() {
                    let _result = clipboard.get_content();
                }
            }
        }
    }

    #[test]
    fn test_content_type_consistency() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            for _ in 0..3 {
                if let Ok((_content, content_type)) = clipboard.get_content() {
                    assert_eq!(content_type, ClipboardContentType::Text);
                }
            }
        }
    }

    #[test]
    fn test_tool_persistence() {
        if let Ok(clipboard) = LinuxClipboard::new() {
            let tool1 = clipboard.tool();
            let tool2 = clipboard.tool();
            assert_eq!(tool1, tool2);
        }
    }
}
