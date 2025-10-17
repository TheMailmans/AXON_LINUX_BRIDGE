//! Input validation utilities for preventing invalid or unsafe operations.
//!
//! Provides coordinate, window identifier, and application validation helpers
//! with descriptive error messages suitable for exposing to upstream clients.

use anyhow::{anyhow, bail, Result};

/// Validates screen-space coordinates used for pointer events.
#[derive(Debug)]
pub struct CoordinateValidator {
    screen_width: i32,
    screen_height: i32,
}

impl CoordinateValidator {
    /// Creates a new validator with the given screen resolution in pixels.
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }

    /// Ensures the provided coordinates are within screen bounds (inclusive lower, exclusive upper).
    pub fn validate(&self, x: i32, y: i32) -> Result<()> {
        if x < 0 {
            bail!("X coordinate {} is negative (min: 0)", x);
        }
        if y < 0 {
            bail!("Y coordinate {} is negative (min: 0)", y);
        }
        if x >= self.screen_width {
            bail!(
                "X coordinate {} exceeds screen width {} (max: {})",
                x,
                self.screen_width,
                self.screen_width - 1
            );
        }
        if y >= self.screen_height {
            bail!(
                "Y coordinate {} exceeds screen height {} (max: {})",
                y,
                self.screen_height,
                self.screen_height - 1
            );
        }
        Ok(())
    }

    /// Returns true if coordinates are within 10px of a screen edge.
    pub fn is_near_edge(&self, x: i32, y: i32) -> bool {
        x < 10 || y < 10 || x >= (self.screen_width - 10) || y >= (self.screen_height - 10)
    }

    /// Provides contextual hint for borderline coordinates.
    pub fn hint(&self, x: i32, y: i32) -> Option<String> {
        if self.is_near_edge(x, y) {
            Some(format!(
                "Coordinates ({}, {}) are close to the screen edge; verify target area.",
                x, y
            ))
        } else {
            None
        }
    }
}

/// Validates user-provided application name or desktop entry identifiers.
pub fn validate_app_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("Application name cannot be empty");
    }
    if name.len() > 256 {
        bail!(
            "Application name is too long: {} characters (max: 256)",
            name.len()
        );
    }
    if name.contains(['/', '\\']) || name.contains("..") {
        bail!("Application name contains invalid path characters");
    }
    Ok(())
}

/// Validates X11 window identifiers provided as hex (0xABC) or decimal strings.
pub fn validate_window_id(id: &str) -> Result<()> {
    if id.trim().is_empty() {
        bail!("Window ID cannot be empty");
    }
    if id.starts_with("0x") {
        u64::from_str_radix(&id[2..], 16)
            .map(|_| ())
            .map_err(|_| anyhow!("Invalid hexadecimal window ID: {}", id))
    } else {
        id.parse::<u64>()
            .map(|_| ())
            .map_err(|_| anyhow!("Invalid decimal window ID: {}", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_coordinates_successfully() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.validate(0, 0).is_ok());
        assert!(validator.validate(1919, 1079).is_ok());
        assert!(validator.validate(500, 600).is_ok());
    }

    #[test]
    fn rejects_out_of_bounds_coordinates() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.validate(-1, 0).is_err());
        assert!(validator.validate(0, -1).is_err());
        assert!(validator.validate(1920, 0).is_err());
        assert!(validator.validate(0, 1080).is_err());
    }

    #[test]
    fn detects_edge_coordinates() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.is_near_edge(5, 100));
        assert!(validator.is_near_edge(1915, 100));
        assert!(validator.is_near_edge(100, 5));
        assert!(validator.is_near_edge(100, 1075));
        assert!(!validator.is_near_edge(960, 540));
    }

    #[test]
    fn provides_hints_for_edges() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.hint(2, 2).is_some());
        assert!(validator.hint(960, 540).is_none());
    }

    #[test]
    fn validates_application_names() {
        assert!(validate_app_name("calculator").is_ok());
        assert!(validate_app_name("gnome-terminal").is_ok());
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("../bad").is_err());
    }

    #[test]
    fn validates_window_ids() {
        assert!(validate_window_id("0x123abc").is_ok());
        assert!(validate_window_id("123456").is_ok());
        assert!(validate_window_id("").is_err());
        assert!(validate_window_id("xyz").is_err());
    }
}

/// Batch operation validation configuration (NEW in v2.4)
#[derive(Debug, Clone, Copy)]
pub struct BatchValidationConfig {
    /// Maximum operations per batch
    pub max_operations: usize,
    /// Maximum clipboard content size in bytes
    pub max_clipboard_size: usize,
}

impl Default for BatchValidationConfig {
    fn default() -> Self {
        Self {
            max_operations: 100,
            max_clipboard_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Validate batch operation count (NEW in v2.4)
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

/// Validate clipboard content size (NEW in v2.4)
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

/// Validate clipboard content type (NEW in v2.4)
pub fn validate_clipboard_content_type(content_type: &str) -> Result<()> {
    match content_type {
        "text" | "image" | "html" => Ok(()),
        _ => bail!("Invalid clipboard content type: {}", content_type),
    }
}

/// Comprehensive clipboard validation (NEW in v2.4)
pub fn validate_clipboard(
    content: &str,
    content_type: &str,
    config: BatchValidationConfig,
) -> Result<()> {
    validate_clipboard_size(content, config)?;
    validate_clipboard_content_type(content_type)?;
    Ok(())
}

#[cfg(test)]
mod batch_validation_tests {
    use super::*;

    #[test]
    fn test_batch_validation_config_default() {
        let config = BatchValidationConfig::default();
        assert_eq!(config.max_operations, 100);
        assert_eq!(config.max_clipboard_size, 10 * 1024 * 1024);
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
    }

    #[test]
    fn test_validate_clipboard_size_valid() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard_size("hello", config).is_ok());
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
    fn test_validate_clipboard_content_type_valid() {
        assert!(validate_clipboard_content_type("text").is_ok());
        assert!(validate_clipboard_content_type("image").is_ok());
        assert!(validate_clipboard_content_type("html").is_ok());
    }

    #[test]
    fn test_validate_clipboard_content_type_invalid() {
        assert!(validate_clipboard_content_type("json").is_err());
        assert!(validate_clipboard_content_type("invalid").is_err());
    }

    #[test]
    fn test_validate_clipboard_comprehensive() {
        let config = BatchValidationConfig::default();
        assert!(validate_clipboard("hello", "text", config).is_ok());
        assert!(validate_clipboard("", "text", config).is_err());
        assert!(validate_clipboard("hello", "invalid", config).is_err());
    }
}
