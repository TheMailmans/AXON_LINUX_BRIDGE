//! Windows clipboard implementation using Win32 API.

use super::{ClipboardContentType, ClipboardProvider};
use anyhow::Result;

/// Windows clipboard provider using Win32 API
pub struct WindowsClipboard;

impl WindowsClipboard {
    /// Create new Windows clipboard provider
    pub fn new() -> Result<Self> {
        Ok(Self)
    }
}

impl ClipboardProvider for WindowsClipboard {
    fn get_content(&self) -> Result<(String, ClipboardContentType)> {
        // TODO: Implement using Win32 API
        // For now, return stub error
        Err(anyhow::anyhow!("Windows clipboard not yet implemented"))
    }

    fn set_content(&self, _content: &str, _content_type: ClipboardContentType) -> Result<()> {
        // TODO: Implement using Win32 API
        // For now, return stub error
        Err(anyhow::anyhow!("Windows clipboard not yet implemented"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_clipboard_creation() {
        let _clipboard = WindowsClipboard::new();
    }
}
