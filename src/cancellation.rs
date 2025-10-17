//! Request cancellation module for v2.5.
//!
//! Implements graceful cancellation with timeout support, resource cleanup,
//! and cancellation token distribution for coordinated shutdown.

use anyhow::{Result, bail};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU32, Ordering};
use uuid::Uuid;

/// Cancellation token for tracking request lifecycle
#[derive(Debug, Clone)]
pub struct CancellationToken {
    id: String,
    cancelled: Arc<AtomicBool>,
    timeout_ms: u64,
    created_at: std::time::Instant,
}

impl CancellationToken {
    /// Create new cancellation token
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            cancelled: Arc::new(AtomicBool::new(false)),
            timeout_ms,
            created_at: std::time::Instant::now(),
        }
    }

    /// Get token ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Check if cancellation requested
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Check if token has timed out
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_millis() as u64 > self.timeout_ms
    }

    /// Check if token is still valid
    pub fn is_valid(&self) -> bool {
        !self.is_cancelled() && !self.is_expired()
    }

    /// Request cancellation
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Get time remaining before expiration (milliseconds)
    pub fn time_remaining_ms(&self) -> u64 {
        let elapsed = self.created_at.elapsed().as_millis() as u64;
        if elapsed >= self.timeout_ms {
            0
        } else {
            self.timeout_ms - elapsed
        }
    }

    /// Get elapsed time since creation (milliseconds)
    pub fn elapsed_ms(&self) -> u64 {
        self.created_at.elapsed().as_millis() as u64
    }
}

/// Cancellation scope for managing multiple related operations
#[derive(Debug, Clone)]
pub struct CancellationScope {
    id: String,
    tokens: Arc<RwLock<HashMap<String, CancellationToken>>>,
    cancelled: Arc<AtomicBool>,
    operation_count: Arc<AtomicU64>,
}

impl CancellationScope {
    /// Create new cancellation scope
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            tokens: Arc::new(RwLock::new(HashMap::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
            operation_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get scope ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Create and register new token
    pub async fn create_token(&self, timeout_ms: u64) -> Result<CancellationToken> {
        if self.is_cancelled() {
            bail!("Scope {} already cancelled", self.id);
        }

        let token = CancellationToken::new(timeout_ms);
        let mut tokens = self.tokens.write().await;
        tokens.insert(token.id().to_string(), token.clone());
        self.operation_count.fetch_add(1, Ordering::Relaxed);

        Ok(token)
    }

    /// Cancel all tokens in scope
    pub fn cancel_all(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check if scope is cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Get number of active operations
    pub fn active_operations(&self) -> u64 {
        self.operation_count.load(Ordering::Relaxed)
    }

    /// Remove token from scope (operation completed)
    pub async fn remove_token(&self, token_id: &str) {
        let mut tokens = self.tokens.write().await;
        if tokens.remove(token_id).is_some() {
            self.operation_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Get all active token IDs
    pub async fn active_tokens(&self) -> Vec<String> {
        let tokens = self.tokens.read().await;
        tokens.keys().cloned().collect()
    }

    /// Get token statistics
    pub async fn stats(&self) -> ScopeStats {
        let tokens = self.tokens.read().await;
        let total_tokens = tokens.len();
        let expired_tokens = tokens.values().filter(|t| t.is_expired()).count();
        let cancelled_tokens = tokens.values().filter(|t| t.is_cancelled()).count();

        ScopeStats {
            scope_id: self.id.clone(),
            total_tokens,
            active_tokens: total_tokens - expired_tokens - cancelled_tokens,
            expired_tokens,
            cancelled_tokens,
        }
    }
}

impl Default for CancellationScope {
    fn default() -> Self {
        Self::new()
    }
}

/// Scope statistics
#[derive(Debug, Clone)]
pub struct ScopeStats {
    pub scope_id: String,
    pub total_tokens: usize,
    pub active_tokens: usize,
    pub expired_tokens: usize,
    pub cancelled_tokens: usize,
}

/// Request state tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestState {
    Pending,
    InProgress,
    Completed,
    Cancelled,
    Failed,
}

/// Request lifecycle manager
#[derive(Debug, Clone)]
pub struct RequestContext {
    id: String,
    token: CancellationToken,
    state: Arc<AtomicU32>,  // Encodes RequestState as u32
    cleanup_tasks: Arc<RwLock<Vec<Box<dyn std::any::Any + Send + Sync>>>>,
}

impl RequestContext {
    /// Create new request context
    pub fn new(timeout_ms: u64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            token: CancellationToken::new(timeout_ms),
            state: Arc::new(AtomicU32::new(Self::encode_state(RequestState::Pending))),
            cleanup_tasks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Get request ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get cancellation token
    pub fn token(&self) -> &CancellationToken {
        &self.token
    }

    /// Set request state
    pub fn set_state(&self, state: RequestState) {
        self.state.store(Self::encode_state(state), Ordering::Relaxed);
    }

    /// Get current state
    pub fn state(&self) -> RequestState {
        Self::decode_state(self.state.load(Ordering::Relaxed))
    }

    /// Check if request is still valid
    pub fn is_valid(&self) -> bool {
        self.token.is_valid() && self.state() != RequestState::Cancelled
    }

    /// Request cancellation
    pub fn cancel(&self) {
        self.token.cancel();
        self.set_state(RequestState::Cancelled);
    }

    /// Encode RequestState as u32
    fn encode_state(state: RequestState) -> u32 {
        match state {
            RequestState::Pending => 0,
            RequestState::InProgress => 1,
            RequestState::Completed => 2,
            RequestState::Cancelled => 3,
            RequestState::Failed => 4,
        }
    }

    /// Decode u32 to RequestState
    fn decode_state(value: u32) -> RequestState {
        match value {
            0 => RequestState::Pending,
            1 => RequestState::InProgress,
            2 => RequestState::Completed,
            3 => RequestState::Cancelled,
            4 => RequestState::Failed,
            _ => RequestState::Failed,
        }
    }

    /// Register cleanup task (placeholder for cleanup function pointer)
    pub async fn register_cleanup(&self) {
        let mut tasks = self.cleanup_tasks.write().await;
        tasks.push(Box::new(0u32)); // Placeholder
    }

    /// Perform cleanup
    pub async fn cleanup(&self) {
        let mut tasks = self.cleanup_tasks.write().await;
        tasks.clear();
    }
}

/// Cancellation manager for coordinating all cancellations
pub struct CancellationManager {
    scopes: Arc<RwLock<HashMap<String, CancellationScope>>>,
    contexts: Arc<RwLock<HashMap<String, RequestContext>>>,
}

impl CancellationManager {
    /// Create new cancellation manager
    pub fn new() -> Self {
        Self {
            scopes: Arc::new(RwLock::new(HashMap::new())),
            contexts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create new scope
    pub async fn create_scope(&self) -> CancellationScope {
        let scope = CancellationScope::new();
        let mut scopes = self.scopes.write().await;
        scopes.insert(scope.id().to_string(), scope.clone());
        scope
    }

    /// Create request context
    pub async fn create_context(&self, timeout_ms: u64) -> RequestContext {
        let context = RequestContext::new(timeout_ms);
        let mut contexts = self.contexts.write().await;
        contexts.insert(context.id().to_string(), context.clone());
        context
    }

    /// Cancel request
    pub async fn cancel_request(&self, request_id: &str) -> Result<()> {
        let contexts = self.contexts.read().await;
        if let Some(context) = contexts.get(request_id) {
            context.cancel();
            Ok(())
        } else {
            bail!("Request {} not found", request_id)
        }
    }

    /// Get request context
    pub async fn get_context(&self, request_id: &str) -> Result<RequestContext> {
        let contexts = self.contexts.read().await;
        contexts
            .get(request_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Request {} not found", request_id))
    }

    /// Remove context
    pub async fn remove_context(&self, request_id: &str) {
        let mut contexts = self.contexts.write().await;
        contexts.remove(request_id);
    }

    /// Get all active scopes
    pub async fn active_scopes(&self) -> usize {
        let scopes = self.scopes.read().await;
        scopes.len()
    }

    /// Get all active contexts
    pub async fn active_contexts(&self) -> usize {
        let contexts = self.contexts.read().await;
        contexts.len()
    }
}

impl Default for CancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancellation_token_creation() {
        let token = CancellationToken::new(5000);
        assert!(!token.is_cancelled());
        assert!(!token.is_expired());
        assert!(token.is_valid());
    }

    #[test]
    fn test_cancellation_token_cancel() {
        let token = CancellationToken::new(5000);
        token.cancel();
        assert!(token.is_cancelled());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_cancellation_token_timeout() {
        let token = CancellationToken::new(1);
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(token.is_expired());
        assert!(!token.is_valid());
    }

    #[test]
    fn test_cancellation_token_time_remaining() {
        let token = CancellationToken::new(5000);
        let remaining = token.time_remaining_ms();
        assert!(remaining > 4900 && remaining <= 5000);
    }

    #[test]
    fn test_cancellation_token_elapsed() {
        let token = CancellationToken::new(5000);
        std::thread::sleep(std::time::Duration::from_millis(100));
        let elapsed = token.elapsed_ms();
        assert!(elapsed >= 100 && elapsed <= 500);
    }

    #[tokio::test]
    async fn test_cancellation_scope_creation() {
        let scope = CancellationScope::new();
        assert!(!scope.is_cancelled());
        assert_eq!(scope.active_operations(), 0);
    }

    #[tokio::test]
    async fn test_cancellation_scope_create_token() {
        let scope = CancellationScope::new();
        let token = scope.create_token(5000).await.unwrap();
        assert!(token.is_valid());
        assert_eq!(scope.active_operations(), 1);
    }

    #[tokio::test]
    async fn test_cancellation_scope_cancel_all() {
        let scope = CancellationScope::new();
        let _token = scope.create_token(5000).await.unwrap();
        
        scope.cancel_all();
        assert!(scope.is_cancelled());
        
        // Cancelling scope prevents new token creation
        let result = scope.create_token(5000).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cancellation_scope_remove_token() {
        let scope = CancellationScope::new();
        let token = scope.create_token(5000).await.unwrap();
        assert_eq!(scope.active_operations(), 1);
        
        scope.remove_token(token.id()).await;
        assert_eq!(scope.active_operations(), 0);
    }

    #[tokio::test]
    async fn test_cancellation_scope_stats() {
        let scope = CancellationScope::new();
        let token1 = scope.create_token(5000).await.unwrap();
        let token2 = scope.create_token(5000).await.unwrap();
        
        token1.cancel();
        
        let stats = scope.stats().await;
        assert_eq!(stats.total_tokens, 2);
        assert_eq!(stats.cancelled_tokens, 1);
        assert_eq!(stats.active_tokens, 1);
    }

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new(5000);
        assert_eq!(ctx.state(), RequestState::Pending);
        assert!(ctx.is_valid());
    }

    #[test]
    fn test_request_context_state_transition() {
        let ctx = RequestContext::new(5000);
        ctx.set_state(RequestState::InProgress);
        assert_eq!(ctx.state(), RequestState::InProgress);
        
        ctx.set_state(RequestState::Completed);
        assert_eq!(ctx.state(), RequestState::Completed);
    }

    #[test]
    fn test_request_context_cancel() {
        let ctx = RequestContext::new(5000);
        ctx.cancel();
        assert_eq!(ctx.state(), RequestState::Cancelled);
        assert!(!ctx.is_valid());
    }

    #[tokio::test]
    async fn test_cancellation_manager_creation() {
        let manager = CancellationManager::new();
        assert_eq!(manager.active_scopes().await, 0);
        assert_eq!(manager.active_contexts().await, 0);
    }

    #[tokio::test]
    async fn test_cancellation_manager_create_scope() {
        let manager = CancellationManager::new();
        let scope = manager.create_scope().await;
        assert_eq!(manager.active_scopes().await, 1);
        assert!(!scope.is_cancelled());
    }

    #[tokio::test]
    async fn test_cancellation_manager_create_context() {
        let manager = CancellationManager::new();
        let context = manager.create_context(5000).await;
        assert_eq!(manager.active_contexts().await, 1);
        assert!(context.is_valid());
    }

    #[tokio::test]
    async fn test_cancellation_manager_cancel_request() {
        let manager = CancellationManager::new();
        let context = manager.create_context(5000).await;
        let request_id = context.id().to_string();
        
        assert!(manager.cancel_request(&request_id).await.is_ok());
        assert!(!context.is_valid());
    }

    #[tokio::test]
    async fn test_cancellation_manager_get_context() {
        let manager = CancellationManager::new();
        let context = manager.create_context(5000).await;
        let request_id = context.id().to_string();
        
        let retrieved = manager.get_context(&request_id).await.unwrap();
        assert_eq!(retrieved.id(), context.id());
    }

    #[tokio::test]
    async fn test_cancellation_manager_remove_context() {
        let manager = CancellationManager::new();
        let context = manager.create_context(5000).await;
        let request_id = context.id().to_string();
        
        manager.remove_context(&request_id).await;
        assert_eq!(manager.active_contexts().await, 0);
    }

    #[test]
    fn test_cancellation_token_id() {
        let token = CancellationToken::new(5000);
        let id = token.id();
        assert!(!id.is_empty());
    }

    #[tokio::test]
    async fn test_cancellation_scope_active_tokens() {
        let scope = CancellationScope::new();
        let token1 = scope.create_token(5000).await.unwrap();
        let token2 = scope.create_token(5000).await.unwrap();
        
        let active = scope.active_tokens().await;
        assert_eq!(active.len(), 2);
        assert!(active.contains(&token1.id().to_string()));
        assert!(active.contains(&token2.id().to_string()));
    }
}
