//! Batch operation execution for v2.4.
//!
//! Allows Hub to submit multiple operations in a single request for improved
//! throughput and reduced latency.

use anyhow::{Result, bail};
use std::time::Instant;
use crate::proto_gen::agent::{Operation, OperationResult, ErrorCode};

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

/// Result of batch execution
#[derive(Debug, Clone)]
pub struct BatchExecutionResult {
    /// Results for each operation in order
    pub results: Vec<OperationResult>,
    /// Total successful operations
    pub success_count: i32,
    /// Total failed operations
    pub failure_count: i32,
    /// Total execution time in milliseconds
    pub total_time_ms: i64,
}

impl BatchExecutionResult {
    /// Create new batch result
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            success_count: 0,
            failure_count: 0,
            total_time_ms: 0,
        }
    }

    /// Add successful operation result
    pub fn add_success(&mut self, request_id: String, execution_time_ms: i64) {
        self.results.push(OperationResult {
            success: true,
            error: None,
            error_code: Some(ErrorCode::Success as i32),
            execution_time_ms,
            request_id: Some(request_id),
        });
        self.success_count += 1;
    }

    /// Add failed operation result
    pub fn add_failure(&mut self, request_id: String, error: String, execution_time_ms: i64) {
        self.results.push(OperationResult {
            success: false,
            error: Some(error),
            error_code: Some(ErrorCode::DisplayServerError as i32),
            execution_time_ms,
            request_id: Some(request_id),
        });
        self.failure_count += 1;
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

    /// Validate operation sequence
    /// Could add more sophisticated validation rules here
    pub fn validate_operations(&self, operations: &[Operation]) -> Result<()> {
        // Basic validation - operations should have valid types
        for (_idx, op) in operations.iter().enumerate() {
            if op.op.is_none() {
                bail!("Operation has no type specified");
            }
        }
        Ok(())
    }

    /// Execute batch with early termination on error if configured
    pub fn execute_batch(
        &self,
        operations: Vec<Operation>,
        stop_on_error: bool,
    ) -> Result<BatchExecutionResult> {
        let mut result = BatchExecutionResult::new();
        let start = Instant::now();

        for (idx, _op) in operations.iter().enumerate() {
            let op_start = Instant::now();
            let op_id = format!("batch-op-{}", idx);

            // TODO: Implement actual operation execution in Phase 2
            // For now, mark as success
            let execution_time_ms = op_start.elapsed().as_millis() as i64;
            result.add_success(op_id, execution_time_ms);

            if stop_on_error && result.failure_count > 0 {
                break;
            }
        }

        result.total_time_ms = start.elapsed().as_millis() as i64;
        Ok(result)
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
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.max_operations, 100);
        assert!(!config.stop_on_error);
    }

    #[test]
    fn test_validate_empty_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(0).is_err());
    }

    #[test]
    fn test_validate_single_operation() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(1).is_ok());
    }

    #[test]
    fn test_validate_valid_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(5).is_ok());
        assert!(executor.validate_batch_size(50).is_ok());
        assert!(executor.validate_batch_size(100).is_ok());
    }

    #[test]
    fn test_validate_oversized_batch() {
        let executor = BatchExecutor::new(BatchConfig::default());
        assert!(executor.validate_batch_size(101).is_err());
        assert!(executor.validate_batch_size(200).is_err());
    }

    #[test]
    fn test_custom_max_operations() {
        let config = BatchConfig {
            max_operations: 50,
            stop_on_error: false,
        };
        let executor = BatchExecutor::new(config);
        assert!(executor.validate_batch_size(50).is_ok());
        assert!(executor.validate_batch_size(51).is_err());
    }

    #[test]
    fn test_batch_execution_result_creation() {
        let result = BatchExecutionResult::new();
        assert_eq!(result.results.len(), 0);
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.total_time_ms, 0);
    }

    #[test]
    fn test_batch_result_add_success() {
        let mut result = BatchExecutionResult::new();
        result.add_success("req-1".into(), 10);
        
        assert_eq!(result.success_count, 1);
        assert_eq!(result.failure_count, 0);
        assert_eq!(result.results.len(), 1);
        assert!(result.results[0].success);
    }

    #[test]
    fn test_batch_result_add_failure() {
        let mut result = BatchExecutionResult::new();
        result.add_failure("req-1".into(), "Test error".into(), 5);
        
        assert_eq!(result.success_count, 0);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.results.len(), 1);
        assert!(!result.results[0].success);
    }

    #[test]
    fn test_batch_result_mixed_results() {
        let mut result = BatchExecutionResult::new();
        result.add_success("req-1".into(), 10);
        result.add_failure("req-2".into(), "Failed".into(), 5);
        result.add_success("req-3".into(), 15);
        
        assert_eq!(result.success_count, 2);
        assert_eq!(result.failure_count, 1);
        assert_eq!(result.results.len(), 3);
    }

    #[test]
    fn test_batch_result_timing() {
        let mut result = BatchExecutionResult::new();
        result.total_time_ms = 100;
        assert_eq!(result.total_time_ms, 100);
    }

    #[test]
    fn test_batch_execution_empty() {
        let executor = BatchExecutor::new(BatchConfig::default());
        let result = executor.execute_batch(vec![], false);
        // Should succeed with empty batch
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_execution_records_time() {
        let executor = BatchExecutor::new(BatchConfig::default());
        let result = executor.execute_batch(vec![], false).unwrap();
        // Should have recorded execution time (>= 0)
        assert!(result.total_time_ms >= 0);
    }

    #[test]
    fn test_validate_operations_none() {
        let executor = BatchExecutor::new(BatchConfig::default());
        let ops = vec![];
        assert!(executor.validate_operations(&ops).is_ok());
    }

    #[test]
    fn test_stop_on_error_flag() {
        let config1 = BatchConfig {
            max_operations: 100,
            stop_on_error: true,
        };
        let config2 = BatchConfig {
            max_operations: 100,
            stop_on_error: false,
        };
        
        let executor1 = BatchExecutor::new(config1);
        let executor2 = BatchExecutor::new(config2);
        
        assert!(executor1.config.stop_on_error);
        assert!(!executor2.config.stop_on_error);
    }

    #[test]
    fn test_batch_result_counts_consistency() {
        let mut result = BatchExecutionResult::new();
        for i in 0..5 {
            if i % 2 == 0 {
                result.add_success(format!("req-{}", i), 10);
            } else {
                result.add_failure(format!("req-{}", i), "Error".into(), 5);
            }
        }
        
        assert_eq!(result.results.len(), 5);
        assert_eq!(result.success_count as usize + result.failure_count as usize, 5);
    }
}
