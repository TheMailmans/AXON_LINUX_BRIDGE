//! System clipboard access for v2.4.
//!
//! Provides cross-platform clipboard read/write support for text content.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

use anyhow::Result;

/// Clipboard content type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardContentType {
    Text,
    Image,
    Html,
}

impl ClipboardContentType {
    /// Convert to string for proto
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Html => "html",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Self {
        match s {
            "image" => Self::Image,
            "html" => Self::Html,
            _ => Self::Text,
        }
    }
}

/// Clipboard provider trait
pub trait ClipboardProvider: Send + Sync {
    /// Get clipboard content
    fn get_content(&self) -> Result<(String, ClipboardContentType)>;

    /// Set clipboard content
    fn set_content(&self, content: &str, content_type: ClipboardContentType) -> Result<()>;
}

/// Get platform-specific clipboard provider
pub fn get_clipboard() -> Result<Box<dyn ClipboardProvider>> {
    #[cfg(target_os = "linux")]
    {
        Ok(Box::new(linux::LinuxClipboard::new()?))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(Box::new(macos::MacOSClipboard::new()?))
    }

    #[cfg(target_os = "windows")]
    {
        Ok(Box::new(windows::WindowsClipboard::new()?))
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!("Clipboard not supported on this platform"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_as_str() {
        assert_eq!(ClipboardContentType::Text.as_str(), "text");
        assert_eq!(ClipboardContentType::Image.as_str(), "image");
        assert_eq!(ClipboardContentType::Html.as_str(), "html");
    }

    #[test]
    fn test_content_type_from_str() {
        assert_eq!(ClipboardContentType::from_str("text"), ClipboardContentType::Text);
        assert_eq!(ClipboardContentType::from_str("image"), ClipboardContentType::Image);
        assert_eq!(ClipboardContentType::from_str("html"), ClipboardContentType::Html);
        assert_eq!(ClipboardContentType::from_str("unknown"), ClipboardContentType::Text);
    }
}
