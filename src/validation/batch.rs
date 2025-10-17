//! Validation for batch operations and clipboard content.

use anyhow::{Result, bail};

/// Batch operation validation configuration
#[derive(Debug, Clone, Copy)]
pub struct BatchValidationConfig {
    /// Maximum operations per batch
    pub max_operations: usize,
    /// Maximum clipboard content size in bytes
    pub max_clipboard_size: usize,
    /// Maximum content length per operation
    pub max_operation_size: usize,
}

impl Default for BatchValidationConfig {
    fn default() -> Self {
        Self {
            max_operations: 100,
            max_clipboard_size: 10 * 1024 * 1024, // 10MB
            max_operation_size: 1024 * 1024, // 1MB per operation
        }
    }
}

/// Validate batch operation count
pub fn validate_batch_count(count: usize, config: BatchValidationConfig) -> Result<()> {
    if count == 0 {
        bail!("Batch cannot be empty");
    }
    if count > config.max_operations {
        bail!(
            "Batch size {} exceeds maximum {}",
            count,
            config.max_operations
        );
    }
    Ok(())
}

/// Validate clipboard content size
pub fn validate_clipboard_size(content: &str, config: BatchValidationConfig) -> Result<()> {
    let size_bytes = content.as_bytes().len();
    
    if size_bytes == 0 {
        bail!("Clipboard content cannot be empty");
    }
    
    if size_bytes > config.max_clipboard_size {
        bail!(
            "Clipboard content size {} bytes exceeds maximum {} bytes",
            size_bytes,
            config.max_clipboard_size
        );
    }
    
    Ok(())
}

/// Validate clipboard content for special characters
pub fn validate_clipboard_content(content: &str) -> Result<()> {
    // Check for valid UTF-8 (already guaranteed by String type)
    if !content.is_char_boundary(0) {
        bail!("Clipboard content contains invalid UTF-8");
    }
    
    // Warn about extremely long lines (might indicate data issue)
    let max_line_length = 1000000; // 1 million chars
    for (line_num, line) in content.lines().enumerate() {
        if line.len() > max_line_length {
            bail!(
                "Line {} exceeds maximum length of {} characters",
                line_num + 1,
                max_line_length
            );
        }
    }
    
    Ok(())
}

/// Validate clipboard content type
pub fn validate_clipboard_content_type(content_type: &str) -> Result<()> {
    match content_type {
        "text" | "image" | "html" => Ok(()),
        _ => bail!("Invalid clipboard content type: {}", content_type),
    }
}

/// Comprehensive clipboard validation
pub fn validate_clipboard(
    content: &str,
    content_type: &str,
    config: BatchValidationConfig,
) -> Result<()> {
    validate_clipboard_size(content, config)?;
    validate_clipboard_content(content)?;
    validate_clipboard_content_type(content_type)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_validation_config_default() {
        let config = BatchValidationConfig::default();
        assert_eq!(config.max_operations, 100);
        assert_eq!(config.max_clipboard_size, 10 * 1024 * 1024);
        assert_eq!(config.max_operation_size, 1024 * 1024);
    }

    #[test]
    fn test_validate_batch_count_valid() {
        let config = BatchValidationConfig::default();
        assert!(validate_batch_count(1, config).is_ok());
        assert!(validate_batch_count(50, config).is_ok());
        assert!(validate_batch_count(100, config).is_ok());
    }

    #[test]
    fn test_validate_batch_count_empty() {
        let config = BatchValidationConfig::default();
        assert!(validate_batch_count(0, config).is_err());
    }

    #[test]
    fn test_validate_batch_count_exceeds_max() {
        let config = BatchValidationConfig::default();
        assert!(validate_batch_count(101, config).is_err());
        assert!(validate_batch_count(1000, config).is_err());
    }

    #[test]
    fn test_validate_batch_count_custom_config() {
        let config = BatchValidationConfig {
            max_operations: 50,
            ..Default::default()
        };
        assert!(validate_batch_count(50, config).is_ok());
        assert!(validate_batch_count(51, config).is_err());
    }

    #[test]
    fn test_validate_clipboard_size_valid() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard_size("hello", config).is_ok());
        assert!(validate_clipboard_size(&"x".repeat(1000), config).is_ok());
    }

    #[test]
    fn test_validate_clipboard_size_empty() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard_size("", config).is_err());
    }

    #[test]
    fn test_validate_clipboard_size_exceeds_max() {
        let config = BatchValidationConfig {
            max_clipboard_size: 100,
            ..Default::default()
        };
        let large_content = "x".repeat(101);
        assert!(validate_clipboard_size(&large_content, config).is_err());
    }

    #[test]
    fn test_validate_clipboard_size_at_limit() {
        let config = BatchValidationConfig {
            max_clipboard_size: 100,
            ..Default::default()
        };
        let content = "x".repeat(100);
        assert!(validate_clipboard_size(&content, config).is_ok());
    }

    #[test]
    fn test_validate_clipboard_content_valid() {
        assert!(validate_clipboard_content("hello").is_ok());
        assert!(validate_clipboard_content("hello\nworld").is_ok());
        assert!(validate_clipboard_content("Hello 世界 🎉").is_ok());
    }

    #[test]
    fn test_validate_clipboard_content_multiline() {
        let multiline = "Line 1\nLine 2\nLine 3";
        assert!(validate_clipboard_content(multiline).is_ok());
    }

    #[test]
    fn test_validate_clipboard_content_type_valid() {
        assert!(validate_clipboard_content_type("text").is_ok());
        assert!(validate_clipboard_content_type("image").is_ok());
        assert!(validate_clipboard_content_type("html").is_ok());
    }

    #[test]
    fn test_validate_clipboard_content_type_invalid() {
        assert!(validate_clipboard_content_type("json").is_err());
        assert!(validate_clipboard_content_type("xml").is_err());
        assert!(validate_clipboard_content_type("").is_err());
    }

    #[test]
    fn test_validate_clipboard_comprehensive_valid() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard("hello", "text", config).is_ok());
        assert!(validate_clipboard("hello\nworld", "text", config).is_ok());
        assert!(validate_clipboard("Hello 世界", "html", config).is_ok());
    }

    #[test]
    fn test_validate_clipboard_comprehensive_empty() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard("", "text", config).is_err());
    }

    #[test]
    fn test_validate_clipboard_comprehensive_invalid_type() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard("hello", "invalid", config).is_err());
    }

    #[test]
    fn test_validate_clipboard_comprehensive_too_large() {
        let config = BatchValidationConfig {
            max_clipboard_size: 100,
            ..Default::default()
        };
        let content = "x".repeat(101);
        assert!(validate_clipboard(&content, "text", config).is_err());
    }

    #[test]
    fn test_validate_clipboard_unicode() {
        let config = BatchValidationConfig::default();
        let unicode = "Hello 世界 🎉 Привет";
        assert!(validate_clipboard(unicode, "text", config).is_ok());
    }

    #[test]
    fn test_validate_clipboard_special_chars() {
        let config = BatchValidationConfig::default();
        let special = "!@#$%^&*()_+-=[]{}|;:,.<>?";
        assert!(validate_clipboard(special, "text", config).is_ok());
    }
}
