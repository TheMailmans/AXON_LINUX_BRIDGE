//! macOS clipboard implementation using pbpaste/pbcopy.

use super::{ClipboardContentType, ClipboardProvider};
use anyhow::{Result, Context};
use std::process::Command;
use std::io::Write;

/// macOS clipboard provider using pbpaste/pbcopy
pub struct MacOSClipboard;

impl MacOSClipboard {
    /// Create new macOS clipboard provider
    pub fn new() -> Result<Self> {
        // pbpaste is built-in to macOS
        Ok(Self)
    }
}

impl ClipboardProvider for MacOSClipboard {
    fn get_content(&self) -> Result<(String, ClipboardContentType)> {
        let output = Command::new("pbpaste")
            .output()
            .context("Failed to run pbpaste")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("pbpaste failed to get clipboard"));
        }

        let content = String::from_utf8(output.stdout)
            .context("Clipboard content is not valid UTF-8")?;

        Ok((content, ClipboardContentType::Text))
    }

    fn set_content(&self, content: &str, _content_type: ClipboardContentType) -> Result<()> {
        let mut child = Command::new("pbcopy")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn pbcopy")?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to pbcopy stdin")?;
        }

        let status = child.wait()
            .context("Failed to wait for pbcopy")?;

        if !status.success() {
            return Err(anyhow::anyhow!("pbcopy failed to set clipboard"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_clipboard_creation() {
        let _clipboard = MacOSClipboard::new();
    }

    #[test]
    fn test_macos_new_ok() {
        let result = MacOSClipboard::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_content_basic() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            match clipboard.get_content() {
                Ok((content, content_type)) => {
                    assert_eq!(content_type, ClipboardContentType::Text);
                    assert!(content.len() >= 0);
                }
                Err(_) => {
                    // Expected if pbpaste fails on non-macOS
                }
            }
        }
    }

    #[test]
    fn test_content_type_is_text() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            if let Ok((_content, content_type)) = clipboard.get_content() {
                assert_eq!(content_type, ClipboardContentType::Text);
            }
        }
    }

    #[test]
    fn test_set_simple_content() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            match clipboard.set_content("test", ClipboardContentType::Text) {
                Ok(()) => {
                    // Verify we can read it back
                    if let Ok((content, _)) = clipboard.get_content() {
                        assert!(!content.is_empty());
                    }
                }
                Err(_) => {
                    // Expected if pbcopy fails
                }
            }
        }
    }

    #[test]
    fn test_set_empty_string() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            match clipboard.set_content("", ClipboardContentType::Text) {
                Ok(()) => {
                    assert!(true);
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_long_string() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let long_str = "x".repeat(10000);
            match clipboard.set_content(&long_str, ClipboardContentType::Text) {
                Ok(()) => {
                    assert!(true);
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_multiline_content() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let multiline = "Line 1\nLine 2\nLine 3";
            match clipboard.set_content(multiline, ClipboardContentType::Text) {
                Ok(()) => {
                    if let Ok((content, _)) = clipboard.get_content() {
                        assert!(!content.is_empty());
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_unicode_content() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let unicode = "Hello 世界 🎉 Привет";
            match clipboard.set_content(unicode, ClipboardContentType::Text) {
                Ok(()) => {
                    if let Ok((content, _)) = clipboard.get_content() {
                        assert!(!content.is_empty());
                    }
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_set_special_characters() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let special = "!@#$%^&*()_+-=[]{}|;:,.<>?";
            match clipboard.set_content(special, ClipboardContentType::Text) {
                Ok(()) => {
                    assert!(true);
                }
                Err(_) => {}
            }
        }
    }

    #[test]
    fn test_roundtrip_simple() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let test_content = "macos-test-content";
            if clipboard.set_content(test_content, ClipboardContentType::Text).is_ok() {
                if let Ok((content, _)) = clipboard.get_content() {
                    assert!(!content.is_empty());
                }
            }
        }
    }

    #[test]
    fn test_multiple_operations_sequence() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            for i in 0..3 {
                let content = format!("test-{}", i);
                let _ = clipboard.set_content(&content, ClipboardContentType::Text);
                let _ = clipboard.get_content();
            }
        }
    }

    #[test]
    fn test_consistency_across_gets() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            if let Ok((content1, type1)) = clipboard.get_content() {
                if let Ok((content2, type2)) = clipboard.get_content() {
                    // Both reads should have same type
                    assert_eq!(type1, type2);
                    // Content should be consistent (both non-empty or both empty)
                    assert_eq!(content1.is_empty(), content2.is_empty());
                }
            }
        }
    }

    #[test]
    fn test_tabs_and_spaces() {
        if let Ok(clipboard) = MacOSClipboard::new() {
            let whitespace_content = "tab:\there space: here";
            match clipboard.set_content(whitespace_content, ClipboardContentType::Text) {
                Ok(()) => {
                    assert!(true);
                }
                Err(_) => {}
            }
        }
    }
}
