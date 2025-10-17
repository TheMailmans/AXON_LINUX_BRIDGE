//! Batch operation execution for v2.4.
//!
//! Allows Hub to submit multiple operations in a single request for improved
//! throughput and reduced latency.

use anyhow::{Result, bail};

/// Batch execution configuration
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Maximum operations per batch (safety limit)
    pub max_operations: usize,
    /// Stop execution on first error
    pub stop_on_error: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_operations: 100,
            stop_on_error: false,
        }
    }
}

/// Batch executor for v2.4
pub struct BatchExecutor {
    config: BatchConfig,
}

impl BatchExecutor {
    /// Create new batch executor
    pub fn new(config: BatchConfig) -> Self {
        Self { config }
    }

    /// Validate batch size and configuration
    pub fn validate_batch_size(&self, operation_count: usize) -> Result<()> {
        if operation_count == 0 {
            bail!("Batch cannot be empty");
        }
        if operation_count > self.config.max_operations {
            bail!(
                "Batch size {} exceeds maximum {}",
                operation_count,
                self.config.max_operations
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_executor_creation() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert_eq!(executor.config.max_operations, 100);
    }

    #[test]
    fn test_validate_empty_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(0).is_err());
    }

    #[test]
    fn test_validate_valid_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(5).is_ok());
        assert!(executor.validate_batch_size(100).is_ok());
    }

    #[test]
    fn test_validate_oversized_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(101).is_err());
    }
}
