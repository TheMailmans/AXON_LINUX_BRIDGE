//! Retry and backoff helpers for resilient capture and input operations.
//!
//! Provides exponential backoff and jittered retry strategies to handle
//! transient failures in system operations.

use std::time::Duration;
use anyhow::Result;

/// Exponential backoff with jitter configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial backoff duration
    pub initial_delay: Duration,
    /// Maximum backoff duration (cap)
    pub max_delay: Duration,
    /// Backoff multiplier (e.g., 2.0 for exponential)
    pub backoff_multiplier: f32,
    /// Add random jitter to prevent thundering herd
    pub use_jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
}

impl RetryConfig {
    /// Create with custom max attempts
    pub fn with_attempts(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            ..Default::default()
        }
    }

    /// Get delay for attempt N
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let exponential_delay = self.initial_delay.as_millis() as f32
            * self.backoff_multiplier.powi(attempt as i32);
        
        let delay_ms = exponential_delay.min(self.max_delay.as_millis() as f32) as u64;
        
        let final_delay = if self.use_jitter {
            // Add ±25% jitter using timestamp-based pseudo-randomness
            let jitter_amount = (delay_ms as f32 * 0.25) as u64;
            let pseudo_random = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64;
            let jitter_offset = pseudo_random % (jitter_amount + 1);
            let lower_bound = delay_ms.saturating_sub(jitter_amount / 2);
            lower_bound.saturating_add(jitter_offset % (jitter_amount + 1))
        } else {
            delay_ms
        };

        Duration::from_millis(final_delay)
    }
}

/// Retry a fallible operation with exponential backoff
///
/// # Example
/// ```ignore
/// let config = RetryConfig::with_attempts(3);
/// let result = retry_with_backoff(config, || async {
///     // Do something that might fail
///     Ok::<_, anyhow::Error>(())
/// }).await?;
/// ```
pub async fn retry_with_backoff<F, T, E, Fut>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Debug,
{
    let mut attempt = 0;
    loop {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts - 1 => {
                return Err(e);
            }
            Err(_) => {
                let delay = config.delay_for_attempt(attempt);
                tokio::time::sleep(delay).await;
                attempt += 1;
            }
        }
    }
}

/// Retry a synchronous operation with exponential backoff
pub fn retry_with_backoff_sync<F, T, E>(
    config: RetryConfig,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
    E: std::fmt::Debug,
{
    let mut attempt = 0;
    loop {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) if attempt >= config.max_attempts - 1 => {
                return Err(e);
            }
            Err(_) => {
                let delay = config.delay_for_attempt(attempt);
                std::thread::sleep(delay);
                attempt += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.initial_delay, Duration::from_millis(100));
        assert_eq!(config.backoff_multiplier, 2.0);
    }

    #[test]
    fn test_delay_calculation_exponential() {
        let config = RetryConfig {
            max_attempts: 5,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            use_jitter: false,
        };

        // Attempt 0: 100ms
        assert_eq!(config.delay_for_attempt(0).as_millis(), 100);
        
        // Attempt 1: 200ms
        assert_eq!(config.delay_for_attempt(1).as_millis(), 200);
        
        // Attempt 2: 400ms
        assert_eq!(config.delay_for_attempt(2).as_millis(), 400);
        
        // Attempt 3: 800ms
        assert_eq!(config.delay_for_attempt(3).as_millis(), 800);
    }

    #[test]
    fn test_delay_caps_at_max() {
        let config = RetryConfig {
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
            use_jitter: false,
        };

        // Should cap at 5 seconds
        assert_eq!(config.delay_for_attempt(5).as_secs(), 5);
        assert_eq!(config.delay_for_attempt(10).as_secs(), 5);
    }

    #[test]
    fn test_jitter_adds_variance() {
        let config_no_jitter = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            use_jitter: false,
        };

        let config_with_jitter = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(10),
            backoff_multiplier: 2.0,
            use_jitter: true,
        };

        let no_jitter_delay = config_no_jitter.delay_for_attempt(2).as_millis();
        let with_jitter_delay = config_with_jitter.delay_for_attempt(2).as_millis();
        
        // Both should be in the ballpark of 4000ms (2^2 * 1000)
        assert!(no_jitter_delay >= 3500 && no_jitter_delay <= 4500, 
                "no_jitter_delay out of range: {}", no_jitter_delay);
        assert!(with_jitter_delay >= 2500 && with_jitter_delay <= 5500, 
                "with_jitter_delay out of range: {}", with_jitter_delay);
    }

    #[test]
    fn test_retry_sync_succeeds() {
        let config = RetryConfig::with_attempts(3);
        let mut attempts = 0;

        let result = retry_with_backoff_sync(config, || {
            attempts += 1;
            if attempts < 2 {
                Err("fail")
            } else {
                Ok("success")
            }
        });

        assert!(result.is_ok());
        assert_eq!(attempts, 2);
    }

    #[test]
    fn test_retry_sync_exhausts_attempts() {
        let config = RetryConfig::with_attempts(3);
        let mut attempts = 0;

        let result = retry_with_backoff_sync(config, || {
            attempts += 1;
            Err::<(), _>("always fails")
        });

        assert!(result.is_err());
        assert_eq!(attempts, 3);
    }
}
