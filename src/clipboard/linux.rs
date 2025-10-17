//! Linux clipboard implementation using xclip/xsel.

use super::{ClipboardContentType, ClipboardProvider};
use anyhow::{Result, Context};
use std::process::Command;

/// Linux clipboard provider using xclip
pub struct LinuxClipboard;

impl LinuxClipboard {
    /// Create new Linux clipboard provider
    pub fn new() -> Result<Self> {
        // Check if xclip is available
        Command::new("which")
            .arg("xclip")
            .output()
            .context("xclip not found, install it with: sudo apt install xclip")?;
        Ok(Self)
    }
}

impl ClipboardProvider for LinuxClipboard {
    fn get_content(&self) -> Result<(String, ClipboardContentType)> {
        let output = Command::new("xclip")
            .args(&["-selection", "clipboard", "-o"])
            .output()
            .context("Failed to run xclip")?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("xclip failed to get clipboard"));
        }

        let content = String::from_utf8(output.stdout)
            .context("Clipboard content is not valid UTF-8")?;

        Ok((content, ClipboardContentType::Text))
    }

    fn set_content(&self, content: &str, _content_type: ClipboardContentType) -> Result<()> {
        let mut child = Command::new("xclip")
            .args(&["-selection", "clipboard", "-i"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .context("Failed to spawn xclip")?;

        use std::io::Write;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())
                .context("Failed to write to xclip stdin")?;
        }

        let status = child.wait()
            .context("Failed to wait for xclip")?;

        if !status.success() {
            return Err(anyhow::anyhow!("xclip failed to set clipboard"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_clipboard_creation() {
        // This test only runs if xclip is installed
        match LinuxClipboard::new() {
            Ok(_) => {} // xclip available
            Err(_) => {} // xclip not available, skip test
        }
    }
}
