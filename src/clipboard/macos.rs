//! macOS clipboard implementation using pbpaste/pbcopy.

use super::{ClipboardContentType, ClipboardProvider};
use anyhow::{Result, Context};
use std::process::Command;

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

        use std::io::Write;
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
}
