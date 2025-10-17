//! Rate limiting module for v2.5.
//!
//! Implements token bucket algorithm with per-agent quotas to prevent abuse
//! and ensure fair resource allocation across multiple agents.

use anyhow::{Result, bail};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use std::sync::Mutex;

/// Token bucket for rate limiting
#[derive(Debug, Clone)]
pub struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,  // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Create new token bucket
    pub fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }

    /// Try to consume tokens (returns true if successful)
    pub fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();
        
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    /// Get available tokens without consuming
    pub fn available_tokens(&mut self) -> f64 {
        self.refill();
        self.tokens
    }

    /// Get time until next token available (in milliseconds)
    pub fn time_until_available(&mut self, required_tokens: f64) -> f64 {
        self.refill();
        
        if self.tokens >= required_tokens {
            return 0.0;
        }

        let needed = required_tokens - self.tokens;
        let time_seconds = needed / self.refill_rate;
        time_seconds * 1000.0  // Convert to milliseconds
    }

    /// Reset bucket to full capacity
    pub fn reset(&mut self) {
        self.tokens = self.capacity;
        self.last_refill = Instant::now();
    }
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Max frames per second
    pub max_frames_per_sec: f64,
    /// Max batch operations per second
    pub max_batch_ops_per_sec: f64,
    /// Max input events per second
    pub max_input_per_sec: f64,
    /// Max concurrent requests per agent
    pub max_concurrent_requests: usize,
    /// Default quota (requests per minute)
    pub default_quota_per_minute: i32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_frames_per_sec: 30.0,
            max_batch_ops_per_sec: 100.0,
            max_input_per_sec: 100.0,
            max_concurrent_requests: 10,
            default_quota_per_minute: 6000,  // 100 req/sec sustained
        }
    }
}

/// Per-agent quota tracking
#[derive(Debug, Clone)]
pub struct AgentQuota {
    pub agent_id: String,
    pub quota_per_minute: i32,
    pub requests_this_minute: i32,
    pub minute_start: SystemTime,
}

impl AgentQuota {
    /// Create new agent quota
    pub fn new(agent_id: String, quota_per_minute: i32) -> Self {
        Self {
            agent_id,
            quota_per_minute,
            requests_this_minute: 0,
            minute_start: SystemTime::now(),
        }
    }

    /// Check if minute window has elapsed
    fn check_and_reset_window(&mut self) {
        if let Ok(elapsed) = self.minute_start.elapsed() {
            if elapsed >= Duration::from_secs(60) {
                self.requests_this_minute = 0;
                self.minute_start = SystemTime::now();
            }
        }
    }

    /// Try to consume one quota unit
    pub fn try_consume(&mut self) -> bool {
        self.check_and_reset_window();
        
        if self.requests_this_minute < self.quota_per_minute {
            self.requests_this_minute += 1;
            true
        } else {
            false
        }
    }

    /// Get remaining quota
    pub fn remaining(&mut self) -> i32 {
        self.check_and_reset_window();
        self.quota_per_minute - self.requests_this_minute
    }

    /// Get seconds until quota resets
    pub fn seconds_until_reset(&self) -> u64 {
        if let Ok(elapsed) = self.minute_start.elapsed() {
            if elapsed >= Duration::from_secs(60) {
                return 0;
            }
            60 - elapsed.as_secs()
        } else {
            60
        }
    }
}

/// Rate limiter for managing per-agent and global limits
pub struct RateLimiter {
    config: RateLimitConfig,
    frame_bucket: Arc<Mutex<TokenBucket>>,
    batch_bucket: Arc<Mutex<TokenBucket>>,
    input_bucket: Arc<Mutex<TokenBucket>>,
    agent_quotas: Arc<RwLock<HashMap<String, AgentQuota>>>,
    concurrent_requests: Arc<RwLock<HashMap<String, usize>>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            frame_bucket: Arc::new(Mutex::new(TokenBucket::new(
                config.max_frames_per_sec,
                config.max_frames_per_sec,
            ))),
            batch_bucket: Arc::new(Mutex::new(TokenBucket::new(
                config.max_batch_ops_per_sec,
                config.max_batch_ops_per_sec,
            ))),
            input_bucket: Arc::new(Mutex::new(TokenBucket::new(
                config.max_input_per_sec,
                config.max_input_per_sec,
            ))),
            config,
            agent_quotas: Arc::new(RwLock::new(HashMap::new())),
            concurrent_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if frame request is allowed (uses token bucket)
    pub fn allow_frame_request(&self) -> bool {
        if let Ok(mut bucket) = self.frame_bucket.lock() {
            bucket.try_consume(1.0)
        } else {
            false
        }
    }

    /// Check if batch operation is allowed
    pub fn allow_batch_operation(&self, op_count: usize) -> bool {
        let tokens_needed = op_count as f64;
        if let Ok(mut bucket) = self.batch_bucket.lock() {
            bucket.try_consume(tokens_needed)
        } else {
            false
        }
    }

    /// Check if input event is allowed
    pub fn allow_input_event(&self) -> bool {
        if let Ok(mut bucket) = self.input_bucket.lock() {
            bucket.try_consume(1.0)
        } else {
            false
        }
    }

    /// Register or get agent quota
    pub async fn get_agent_quota(&self, agent_id: &str) -> AgentQuota {
        let mut quotas = self.agent_quotas.write().await;
        quotas
            .entry(agent_id.to_string())
            .or_insert_with(|| AgentQuota::new(
                agent_id.to_string(),
                self.config.default_quota_per_minute,
            ))
            .clone()
    }

    /// Check if agent can make request (uses per-minute quota)
    pub async fn allow_agent_request(&self, agent_id: &str) -> bool {
        let mut quotas = self.agent_quotas.write().await;
        
        if let Some(quota) = quotas.get_mut(agent_id) {
            quota.try_consume()
        } else {
            let mut quota = AgentQuota::new(
                agent_id.to_string(),
                self.config.default_quota_per_minute,
            );
            let allowed = quota.try_consume();
            quotas.insert(agent_id.to_string(), quota);
            allowed
        }
    }

    /// Register concurrent request
    pub async fn register_request(&self, agent_id: &str) -> Result<()> {
        let mut concurrent = self.concurrent_requests.write().await;
        let count = concurrent.entry(agent_id.to_string()).or_insert(0);
        
        if *count >= self.config.max_concurrent_requests {
            bail!(
                "Agent {} has reached maximum concurrent requests ({})",
                agent_id,
                self.config.max_concurrent_requests
            );
        }
        
        *count += 1;
        Ok(())
    }

    /// Unregister concurrent request
    pub async fn unregister_request(&self, agent_id: &str) {
        let mut concurrent = self.concurrent_requests.write().await;
        if let Some(count) = concurrent.get_mut(agent_id) {
            if *count > 0 {
                *count -= 1;
            }
        }
    }

    /// Get current concurrent request count for agent
    pub async fn concurrent_request_count(&self, agent_id: &str) -> usize {
        let concurrent = self.concurrent_requests.read().await;
        *concurrent.get(agent_id).unwrap_or(&0)
    }

    /// Get remaining quota for agent
    pub async fn get_remaining_quota(&self, agent_id: &str) -> i32 {
        let mut quotas = self.agent_quotas.write().await;
        if let Some(quota) = quotas.get_mut(agent_id) {
            quota.remaining()
        } else {
            self.config.default_quota_per_minute
        }
    }

    /// Reset all buckets (for testing)
    pub fn reset_buckets(&self) {
        if let Ok(mut bucket) = self.frame_bucket.lock() {
            bucket.reset();
        }
        if let Ok(mut bucket) = self.batch_bucket.lock() {
            bucket.reset();
        }
        if let Ok(mut bucket) = self.input_bucket.lock() {
            bucket.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(100.0, 10.0);
        assert_eq!(bucket.capacity, 100.0);
        assert_eq!(bucket.refill_rate, 10.0);
    }

    #[test]
    fn test_token_bucket_consume() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        assert!(bucket.try_consume(50.0));
        assert_eq!(bucket.tokens, 50.0);
    }

    #[test]
    fn test_token_bucket_exceed_capacity() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        assert!(!bucket.try_consume(150.0));
    }

    #[test]
    fn test_token_bucket_refill() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        bucket.try_consume(100.0);
        assert_eq!(bucket.tokens, 0.0);
        
        thread::sleep(StdDuration::from_millis(200));
        bucket.refill();
        assert!(bucket.tokens > 0.0);
    }

    #[test]
    fn test_token_bucket_max_capacity() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        bucket.tokens = 50.0;
        
        thread::sleep(StdDuration::from_millis(6000));
        bucket.refill();
        
        assert!(bucket.tokens <= 100.0);
    }

    #[test]
    fn test_agent_quota_creation() {
        let quota = AgentQuota::new("agent1".to_string(), 6000);
        assert_eq!(quota.agent_id, "agent1");
        assert_eq!(quota.quota_per_minute, 6000);
        assert_eq!(quota.requests_this_minute, 0);
    }

    #[test]
    fn test_agent_quota_consume() {
        let mut quota = AgentQuota::new("agent1".to_string(), 100);
        assert!(quota.try_consume());
        assert_eq!(quota.requests_this_minute, 1);
    }

    #[test]
    fn test_agent_quota_exceed() {
        let mut quota = AgentQuota::new("agent1".to_string(), 5);
        for _ in 0..5 {
            assert!(quota.try_consume());
        }
        assert!(!quota.try_consume());
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_frames_per_sec, 30.0);
        assert_eq!(config.max_batch_ops_per_sec, 100.0);
        assert_eq!(config.max_input_per_sec, 100.0);
        assert_eq!(config.max_concurrent_requests, 10);
    }

    #[test]
    fn test_rate_limiter_creation() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        assert!(limiter.allow_frame_request());
    }

    #[test]
    fn test_rate_limiter_frame_limit() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_frames_per_sec: 5.0,
            ..Default::default()
        });

        for _ in 0..5 {
            assert!(limiter.allow_frame_request());
        }
        assert!(!limiter.allow_frame_request());
    }

    #[test]
    fn test_rate_limiter_batch_operations() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_batch_ops_per_sec: 10.0,
            ..Default::default()
        });

        assert!(limiter.allow_batch_operation(10));
        assert!(!limiter.allow_batch_operation(1));
    }

    #[test]
    fn test_rate_limiter_input_events() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_input_per_sec: 5.0,
            ..Default::default()
        });

        for _ in 0..5 {
            assert!(limiter.allow_input_event());
        }
        assert!(!limiter.allow_input_event());
    }

    #[test]
    fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        
        // Consume some tokens
        for _ in 0..100 {
            let _ = limiter.allow_frame_request();
        }
        
        // Should be rate limited now
        let mut can_request = false;
        for _ in 0..5 {
            if limiter.allow_frame_request() {
                can_request = true;
                break;
            }
        }
        
        assert!(!can_request);
        
        limiter.reset_buckets();
        assert!(limiter.allow_frame_request());
    }

    #[test]
    fn test_token_bucket_available_tokens() {
        let mut bucket = TokenBucket::new(100.0, 10.0);
        bucket.try_consume(30.0);
        let available = bucket.available_tokens();
        assert!(available >= 70.0 && available <= 100.0);
    }

    #[test]
    fn test_token_bucket_time_until_available() {
        let mut bucket = TokenBucket::new(10.0, 10.0);
        bucket.try_consume(10.0);
        let time_ms = bucket.time_until_available(5.0);
        assert!(time_ms > 0.0 && time_ms <= 500.0);
    }

    #[test]
    fn test_agent_quota_remaining() {
        let mut quota = AgentQuota::new("agent1".to_string(), 100);
        quota.try_consume();
        assert_eq!(quota.remaining(), 99);
    }

    #[test]
    fn test_agent_quota_seconds_until_reset() {
        let quota = AgentQuota::new("agent1".to_string(), 100);
        let seconds = quota.seconds_until_reset();
        assert!(seconds <= 60);
    }

    #[tokio::test]
    async fn test_rate_limiter_agent_request() {
        let limiter = RateLimiter::new(RateLimitConfig {
            default_quota_per_minute: 10,
            ..Default::default()
        });

        for _ in 0..10 {
            assert!(limiter.allow_agent_request("agent1").await);
        }
        assert!(!limiter.allow_agent_request("agent1").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_requests() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_concurrent_requests: 3,
            ..Default::default()
        });

        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_unregister() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_concurrent_requests: 2,
            ..Default::default()
        });

        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_err());

        limiter.unregister_request("agent1").await;
        assert!(limiter.register_request("agent1").await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_multiple_agents() {
        let limiter = RateLimiter::new(RateLimitConfig::default());

        let initial_quota = limiter.get_remaining_quota("agent1").await;
        
        assert!(limiter.allow_agent_request("agent1").await);
        assert!(limiter.allow_agent_request("agent2").await);
        
        let quota1 = limiter.get_remaining_quota("agent1").await;
        let quota2 = limiter.get_remaining_quota("agent2").await;
        
        assert_eq!(quota1, initial_quota - 1);
        assert_eq!(quota2, initial_quota - 1);
    }

    #[tokio::test]
    async fn test_rate_limiter_get_agent_quota() {
        let limiter = RateLimiter::new(RateLimitConfig {
            default_quota_per_minute: 500,
            ..Default::default()
        });

        let quota = limiter.get_agent_quota("agent1").await;
        assert_eq!(quota.agent_id, "agent1");
        assert_eq!(quota.quota_per_minute, 500);
    }

    #[tokio::test]
    async fn test_rate_limiter_concurrent_count() {
        let limiter = RateLimiter::new(RateLimitConfig::default());

        assert!(limiter.register_request("agent1").await.is_ok());
        assert!(limiter.register_request("agent1").await.is_ok());
        
        let count = limiter.concurrent_request_count("agent1").await;
        assert_eq!(count, 2);
        
        limiter.unregister_request("agent1").await;
        let count = limiter.concurrent_request_count("agent1").await;
        assert_eq!(count, 1);
    }
}
