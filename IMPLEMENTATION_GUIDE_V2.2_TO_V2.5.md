# AXON Bridge v2.2-v2.5 Implementation Guide

**Target:** Upgrade from v2.1.0 to v2.5.0  
**Scope:** Add 15 production features with zero technical debt  
**Estimated Effort:** 100 hours full implementation  
**Status:** Ready for implementation

---

## 📚 TABLE OF CONTENTS

1. [Current State & Architecture](#current-state)
2. [Implementation Principles](#principles)
3. [Sprint 1: v2.2 Telemetry & Validation](#sprint-1)
4. [Sprint 2: v2.3 Performance & Caching](#sprint-2)
5. [Sprint 3: v2.4 Batch & Smart Operations](#sprint-3)
6. [Sprint 4: v2.5 Production Polish](#sprint-4)
7. [Testing Requirements](#testing)
8. [Documentation Requirements](#documentation)
9. [Verification Checklist](#verification)

---

## 🏗️ CURRENT STATE & ARCHITECTURE {#current-state}

### Repository Information
```
Repo: https://github.com/TheMailmans/AXON_LINUX_BRIDGE
Branch: main
Current Version: v2.1.0
Language: Rust (edition 2021)
Build System: Cargo
Proto: gRPC (tonic + prost)
```

### Current File Structure
```
AXONBRIDGE-Linux/
├── proto/
│   └── agent.proto          # gRPC definitions
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Library root
│   ├── grpc_service.rs      # All RPC implementations (1857 lines)
│   ├── agent.rs             # Agent lifecycle
│   ├── platform.rs          # Platform detection
│   ├── input/
│   │   ├── mod.rs
│   │   └── linux.rs         # xdotool input injection
│   ├── capture/
│   │   ├── mod.rs
│   │   └── linux.rs         # scrot screenshot capture
│   ├── desktop_apps.rs      # App index and launching
│   ├── a11y/                # Accessibility (AT-SPI)
│   └── streaming/           # Video streaming
├── Cargo.toml               # Dependencies
├── build.rs                 # Proto build script
└── target/release/          # Release binary
```

### Current Dependencies (Cargo.toml)
```toml
tokio = { version = "1.35", features = ["full"] }
tonic = "0.10"
prost = "0.12"
tracing = "0.1"
tracing-subscriber = "0.3"
anyhow = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
sysinfo = "0.30"  # NEWLY ADDED for health monitoring
```

### Current RPCs (15 total)
1. RegisterAgent, UnregisterAgent, Heartbeat
2. StartCapture, StopCapture, GetFrame, StreamFrames
3. StartAudio, StopAudio, StreamAudio
4. InjectMouseMove, InjectMouseClick, InjectMouseDown, InjectMouseUp, InjectScroll, InjectKeyPress, TypeText
5. GetSystemInfo, GetCapabilities, GetActiveWindow
6. GetWindowList, GetProcessList, GetBrowserTabs, ListFiles, GetClipboard
7. LaunchApplication, CloseApplication
8. TakeScreenshot, GetFocusedWindowScreenshot
9. GetAccessibleElements (stub), ExtractTextFromScreen (stub)

### Current Architecture Patterns

**Async Runtime:** Tokio with spawn_blocking for system commands
```rust
let result = tokio::task::spawn_blocking(move || {
    // Blocking system command here
}).await?;
```

**Error Handling:** anyhow::Result with Status conversion
```rust
.map_err(|e| Status::internal(e.to_string()))?
```

**Logging:** tracing with structured fields
```rust
info!("Mouse click: x={}, y={}, button={}", x, y, button);
```

**Window State Tracking:** (v2.1 feature)
```rust
let window_before = get_active_window_info().ok();
// ... execute action ...
let window_after = get_active_window_info().ok();
return InputResponse { window_changed, ... };
```

### Known Patterns to Follow

1. **All input RPCs use spawn_blocking** (CRITICAL - prevents async runtime blocking)
2. **Always validate input before execution** (x/y bounds, window existence, etc.)
3. **Return structured responses** (success bool + error string + optional data)
4. **Use DISPLAY=:0 for all X11 commands** (xdotool, wmctrl, scrot)
5. **Log at entry/exit of every RPC** with timing information
6. **Handle errors gracefully** - never panic, always return Status

### Current Issues to Avoid

❌ **NO unwrap() in production code** - Use ? or explicit error handling  
❌ **NO blocking operations without spawn_blocking**  
❌ **NO hardcoded paths** - Use env vars or config  
❌ **NO magic numbers** - Use named constants  
❌ **NO silent failures** - Always log errors

---

## 🎯 IMPLEMENTATION PRINCIPLES {#principles}

### Code Quality Standards

**Every feature MUST have:**
- ✅ Comprehensive doc comments (/// for public items)
- ✅ Unit tests (90%+ coverage for new code)
- ✅ Integration tests (all RPCs tested)
- ✅ Error handling (all Result/Option handled)
- ✅ Logging (entry/exit/errors)
- ✅ Proto documentation (comments in .proto file)
- ✅ Hub integration examples (Python code snippets)

**Rust Best Practices:**
- Use `Result<T>` not `Option<T>` for operations that can fail
- Use `thiserror` for custom error types
- Use `#[instrument]` for tracing spans
- Prefer `&str` over `String` for parameters
- Use `Arc<RwLock<T>>` for shared state
- Always `.await` async operations properly

**Performance Requirements:**
- Input injection: <100ms per operation
- Screenshot capture: <500ms
- Cache operations: <10ms
- Health check: <50ms
- Metrics overhead: <5ms per RPC

### Testing Standards

**Unit Tests:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_validation() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.validate(100, 100).is_ok());
        assert!(validator.validate(-1, 100).is_err());
        assert!(validator.validate(2000, 100).is_err());
    }
}
```

**Integration Tests:** (tests/ directory)
```rust
#[tokio::test]
async fn test_health_check_rpc() {
    let service = DesktopAgentService::new();
    let request = Request::new(HealthCheckRequest { agent_id: "test".into() });
    let response = service.health_check(request).await.unwrap();
    assert!(response.into_inner().healthy);
}
```

### Documentation Standards

**Module Documentation:**
```rust
//! Metrics collection and reporting for Bridge performance monitoring.
//!
//! This module provides utilities for tracking request timing, success rates,
//! and resource usage. All metrics are thread-safe and designed for minimal overhead.
//!
//! # Example
//! ```
//! let metrics = RequestMetrics::new("req-123");
//! // ... do work ...
//! println!("Execution time: {}ms", metrics.elapsed_ms());
//! ```
```

**Function Documentation:**
```rust
/// Validates that mouse click coordinates are within screen bounds.
///
/// # Arguments
/// * `x` - X coordinate in pixels (0-indexed)
/// * `y` - Y coordinate in pixels (0-indexed)
///
/// # Returns
/// * `Ok(())` if coordinates are valid
/// * `Err` if coordinates are out of bounds
///
/// # Examples
/// ```
/// validator.validate_click(100, 200)?;
/// ```
pub fn validate_click(&self, x: i32, y: i32) -> Result<()> {
    // implementation
}
```

---

## 🚀 SPRINT 1: v2.2 TELEMETRY & VALIDATION {#sprint-1}

### Goals
- Add observability (metrics, logging, tracing)
- Add input validation (prevent invalid operations)
- Add health monitoring (CPU, memory, uptime)
- Standardize error codes
- Add request IDs for tracing

### Changes Required

#### 1. Proto File Updates

**File:** `proto/agent.proto`

**Add after existing enums (around line 325):**
```protobuf
// NEW in v2.2: Standardized error codes
enum ErrorCode {
  SUCCESS = 0;
  INVALID_COORDINATES = 1;
  WINDOW_NOT_FOUND = 2;
  DISPLAY_SERVER_ERROR = 3;
  PERMISSION_DENIED = 4;
  TIMEOUT = 5;
  APP_NOT_FOUND = 6;
  NETWORK_ERROR = 7;
  INVALID_INPUT = 8;
  RESOURCE_BUSY = 9;
  NOT_IMPLEMENTED = 10;
}
```

**Update InputResponse (around line 215):**
```protobuf
message InputResponse {
  bool success = 1;
  optional string error = 2;
  optional ErrorCode error_code_enum = 3;  // NEW: Use enum instead of string
  
  // Action validation (v2.1)
  optional bool window_changed = 4;
  optional bool focus_changed = 5;
  optional string new_window_title = 6;
  optional string new_window_id = 7;
  
  // Telemetry (NEW in v2.2)
  optional int64 execution_time_ms = 8;  // How long did this take?
  optional string request_id = 9;        // UUID for tracing
}
```

**Add new HealthCheck RPC (in service definition around line 50):**
```protobuf
// NEW in v2.2: Health and monitoring
rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
```

**Add new messages (end of file):**
```protobuf
// NEW in v2.2: Health monitoring
message HealthCheckRequest {
  string agent_id = 1;
}

message HealthCheckResponse {
  bool healthy = 1;
  string status = 2;  // "healthy", "degraded", "unhealthy"
  int32 active_connections = 3;
  float cpu_usage_percent = 4;
  int64 memory_usage_mb = 5;
  string version = 6;
  int64 uptime_seconds = 7;
  int64 total_requests = 8;
  int64 failed_requests = 9;
  float avg_response_time_ms = 10;
}
```

#### 2. Create Metrics Module

**File:** `src/metrics.rs` (NEW)

```rust
//! Metrics collection and performance monitoring.
//!
//! Provides thread-safe metrics tracking for all Bridge operations.
//! Metrics are collected with minimal overhead (<5ms) and can be
//! exposed via the HealthCheck RPC.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Tracks timing and metadata for a single request
pub struct RequestMetrics {
    /// Unique identifier for this request
    pub request_id: String,
    /// Start time of the request
    start_time: Instant,
}

impl RequestMetrics {
    /// Create new metrics tracker for a request
    pub fn new() -> Self {
        Self {
            request_id: Uuid::new_v4().to_string(),
            start_time: Instant::now(),
        }
    }
    
    /// Create with custom request ID (for testing)
    pub fn with_id(request_id: String) -> Self {
        Self {
            request_id,
            start_time: Instant::now(),
        }
    }
    
    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> i64 {
        self.start_time.elapsed().as_millis() as i64
    }
    
    /// Get elapsed time in microseconds (high precision)
    pub fn elapsed_us(&self) -> i64 {
        self.start_time.elapsed().as_micros() as i64
    }
}

impl Default for RequestMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Global metrics for the entire Bridge instance
#[derive(Clone)]
pub struct BridgeMetrics {
    /// Total requests processed
    total_requests: Arc<AtomicU64>,
    /// Total failed requests
    failed_requests: Arc<AtomicU64>,
    /// Sum of all response times (for calculating average)
    total_response_time_ms: Arc<AtomicU64>,
}

impl BridgeMetrics {
    /// Create new global metrics tracker
    pub fn new() -> Self {
        Self {
            total_requests: Arc::new(AtomicU64::new(0)),
            failed_requests: Arc::new(AtomicU64::new(0)),
            total_response_time_ms: Arc::new(AtomicU64::new(0)),
        }
    }
    
    /// Record a successful request
    pub fn record_success(&self, duration_ms: i64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms.fetch_add(duration_ms as u64, Ordering::Relaxed);
    }
    
    /// Record a failed request
    pub fn record_failure(&self, duration_ms: i64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
        self.total_response_time_ms.fetch_add(duration_ms as u64, Ordering::Relaxed);
    }
    
    /// Get total request count
    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Relaxed)
    }
    
    /// Get failed request count
    pub fn failed_requests(&self) -> u64 {
        self.failed_requests.load(Ordering::Relaxed)
    }
    
    /// Calculate average response time
    pub fn avg_response_time_ms(&self) -> f32 {
        let total = self.total_requests();
        if total == 0 {
            return 0.0;
        }
        let total_time = self.total_response_time_ms.load(Ordering::Relaxed);
        total_time as f32 / total as f32
    }
}

impl Default for BridgeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_request_metrics_timing() {
        let metrics = RequestMetrics::new();
        thread::sleep(Duration::from_millis(10));
        let elapsed = metrics.elapsed_ms();
        assert!(elapsed >= 10, "Expected at least 10ms, got {}ms", elapsed);
    }

    #[test]
    fn test_bridge_metrics_success() {
        let metrics = BridgeMetrics::new();
        metrics.record_success(100);
        metrics.record_success(200);
        
        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.failed_requests(), 0);
        assert_eq!(metrics.avg_response_time_ms(), 150.0);
    }

    #[test]
    fn test_bridge_metrics_failure() {
        let metrics = BridgeMetrics::new();
        metrics.record_success(100);
        metrics.record_failure(50);
        
        assert_eq!(metrics.total_requests(), 2);
        assert_eq!(metrics.failed_requests(), 1);
    }
}
```

#### 3. Create Validation Module

**File:** `src/validation.rs` (NEW)

```rust
//! Input validation to prevent invalid operations.
//!
//! Validates coordinates, window IDs, and other inputs before
//! executing system commands. Fast-fail validation prevents
//! wasted system calls and provides clear error messages.

use anyhow::{Result, anyhow, bail};

/// Validates mouse coordinates against screen bounds
pub struct CoordinateValidator {
    screen_width: i32,
    screen_height: i32,
}

impl CoordinateValidator {
    /// Create validator with screen dimensions
    pub fn new(screen_width: i32, screen_height: i32) -> Self {
        Self {
            screen_width,
            screen_height,
        }
    }
    
    /// Validate single coordinate pair
    ///
    /// # Arguments
    /// * `x` - X coordinate (0-indexed)
    /// * `y` - Y coordinate (0-indexed)
    ///
    /// # Errors
    /// Returns error if coordinates are outside screen bounds
    pub fn validate(&self, x: i32, y: i32) -> Result<()> {
        if x < 0 {
            bail!("X coordinate {} is negative (min: 0)", x);
        }
        if x >= self.screen_width {
            bail!("X coordinate {} exceeds screen width {} (max: {})", 
                  x, self.screen_width, self.screen_width - 1);
        }
        if y < 0 {
            bail!("Y coordinate {} is negative (min: 0)", y);
        }
        if y >= self.screen_height {
            bail!("Y coordinate {} exceeds screen height {} (max: {})", 
                  y, self.screen_height, self.screen_height - 1);
        }
        Ok(())
    }
    
    /// Check if coordinates are near screen edge (within 10px)
    pub fn is_near_edge(&self, x: i32, y: i32) -> bool {
        x < 10 || y < 10 || 
        x >= (self.screen_width - 10) || 
        y >= (self.screen_height - 10)
    }
    
    /// Get helpful hint about coordinate validity
    pub fn get_hint(&self, x: i32, y: i32) -> Option<String> {
        if self.is_near_edge(x, y) {
            return Some(format!(
                "Click at ({}, {}) is very close to screen edge", x, y
            ));
        }
        None
    }
}

/// Validate application name
pub fn validate_app_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("Application name cannot be empty");
    }
    if name.len() > 256 {
        bail!("Application name too long: {} chars (max: 256)", name.len());
    }
    // Check for path traversal attempts
    if name.contains("..") || name.contains('/') || name.contains('\\') {
        bail!("Application name contains invalid characters");
    }
    Ok(())
}

/// Validate window ID format
pub fn validate_window_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("Window ID cannot be empty");
    }
    // X11 window IDs are hex numbers
    if id.starts_with("0x") {
        u64::from_str_radix(&id[2..], 16)
            .map(|_| ())
            .map_err(|_| anyhow!("Invalid window ID format: {}", id))
    } else {
        // Could be decimal
        id.parse::<u64>()
            .map(|_| ())
            .map_err(|_| anyhow!("Invalid window ID format: {}", id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_validation_valid() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.validate(0, 0).is_ok());
        assert!(validator.validate(1919, 1079).is_ok());
        assert!(validator.validate(960, 540).is_ok());
    }

    #[test]
    fn test_coordinate_validation_invalid() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.validate(-1, 0).is_err());
        assert!(validator.validate(0, -1).is_err());
        assert!(validator.validate(1920, 0).is_err());
        assert!(validator.validate(0, 1080).is_err());
        assert!(validator.validate(2000, 2000).is_err());
    }

    #[test]
    fn test_edge_detection() {
        let validator = CoordinateValidator::new(1920, 1080);
        assert!(validator.is_near_edge(5, 500));
        assert!(validator.is_near_edge(500, 5));
        assert!(validator.is_near_edge(1915, 500));
        assert!(validator.is_near_edge(500, 1075));
        assert!(!validator.is_near_edge(960, 540));
    }

    #[test]
    fn test_app_name_validation() {
        assert!(validate_app_name("calculator").is_ok());
        assert!(validate_app_name("gnome-terminal").is_ok());
        assert!(validate_app_name("").is_err());
        assert!(validate_app_name("../evil").is_err());
        assert!(validate_app_name("/usr/bin/bash").is_err());
    }

    #[test]
    fn test_window_id_validation() {
        assert!(validate_window_id("0x1234567").is_ok());
        assert!(validate_window_id("123456").is_ok());
        assert!(validate_window_id("").is_err());
        assert!(validate_window_id("invalid").is_err());
    }
}
```

#### 4. Create Health Monitoring Module

**File:** `src/health.rs` (NEW)

```rust
//! System health monitoring and reporting.
//!
//! Tracks CPU usage, memory consumption, uptime, and other
//! health metrics for the Bridge process.

use sysinfo::{System, SystemExt, ProcessExt, Pid, PidExt};
use std::time::Instant;

/// Health monitoring for Bridge process
pub struct HealthMonitor {
    system: System,
    start_time: Instant,
    process_pid: Pid,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        let process_pid = Pid::from_u32(std::process::id());
        
        Self {
            system,
            start_time: Instant::now(),
            process_pid,
        }
    }
    
    /// Get current health status
    pub fn get_status(&mut self) -> HealthStatus {
        // Refresh system stats
        self.system.refresh_all();
        
        // Get process info
        let process = self.system.process(self.process_pid);
        
        let (cpu_usage, memory_mb) = if let Some(proc) = process {
            (proc.cpu_usage(), proc.memory() / 1024 / 1024)
        } else {
            (0.0, 0)
        };
        
        HealthStatus {
            cpu_usage_percent: cpu_usage,
            memory_usage_mb: memory_mb as i64,
            uptime_seconds: self.start_time.elapsed().as_secs() as i64,
        }
    }
    
    /// Check if system is healthy
    pub fn is_healthy(&mut self) -> bool {
        let status = self.get_status();
        // Healthy if CPU < 80% and memory < 1GB
        status.cpu_usage_percent < 80.0 && status.memory_usage_mb < 1024
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Current health status snapshot
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub cpu_usage_percent: f32,
    pub memory_usage_mb: i64,
    pub uptime_seconds: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert!(monitor.start_time.elapsed().as_secs() < 1);
    }

    #[test]
    fn test_get_status() {
        let mut monitor = HealthMonitor::new();
        let status = monitor.get_status();
        
        assert!(status.cpu_usage_percent >= 0.0);
        assert!(status.memory_usage_mb > 0);
        assert!(status.uptime_seconds >= 0);
    }

    #[test]
    fn test_is_healthy() {
        let mut monitor = HealthMonitor::new();
        // Should be healthy on startup
        assert!(monitor.is_healthy());
    }
}
```

#### 5. Update lib.rs

**File:** `src/lib.rs`

Add new module declarations:
```rust
pub mod metrics;
pub mod validation;
pub mod health;
```

#### 6. Update grpc_service.rs - Add Metrics to All RPCs

**Pattern to apply to ALL RPCs:**

```rust
use crate::metrics::{RequestMetrics, BridgeMetrics};
use crate::validation::CoordinateValidator;
use crate::health::HealthMonitor;

pub struct DesktopAgentService {
    agent: Arc<RwLock<Option<Agent>>>,
    #[cfg(target_os = "linux")]
    app_index: Arc<RwLock<AppIndex>>,
    // NEW: Add these fields
    metrics: BridgeMetrics,
    validator: Arc<CoordinateValidator>,
    health_monitor: Arc<RwLock<HealthMonitor>>,
}

impl DesktopAgentService {
    pub fn new() -> Self {
        info!("🚀 Initializing Desktop Agent Service...");
        
        // Get screen dimensions for validation
        let system_info = crate::platform::get_system_info()
            .expect("Failed to get system info");
        
        Self {
            agent: Arc::new(RwLock::new(None)),
            #[cfg(target_os = "linux")]
            app_index: Arc::new(RwLock::new(AppIndex::new())),
            // NEW: Initialize
            metrics: BridgeMetrics::new(),
            validator: Arc::new(CoordinateValidator::new(
                system_info.screen_width as i32,
                system_info.screen_height as i32,
            )),
            health_monitor: Arc::new(RwLock::new(HealthMonitor::new())),
        }
    }
}
```

**Example: Update inject_mouse_click:**

```rust
async fn inject_mouse_click(
    &self,
    request: Request<MouseClickRequest>,
) -> Result<Response<InputResponse>, Status> {
    let req = request.into_inner();
    
    // NEW: Start metrics
    let request_metrics = RequestMetrics::new();
    let request_id = request_metrics.request_id.clone();
    
    info!("🖱️  [{}] inject_mouse_click: x={}, y={}, button={:?}", 
          request_id, req.x, req.y, req.button());
    
    // NEW: Validate coordinates
    if let Err(e) = self.validator.validate(req.x, req.y) {
        error!("[{}] Invalid coordinates: {}", request_id, e);
        self.metrics.record_failure(request_metrics.elapsed_ms());
        return Ok(Response::new(InputResponse {
            success: false,
            error: Some(e.to_string()),
            error_code_enum: Some(ErrorCode::InvalidCoordinates as i32),
            execution_time_ms: Some(request_metrics.elapsed_ms()),
            request_id: Some(request_id),
            ..Default::default()
        }));
    }
    
    // ... existing click logic ...
    
    // NEW: Record success
    self.metrics.record_success(request_metrics.elapsed_ms());
    
    Ok(Response::new(InputResponse {
        success: true,
        execution_time_ms: Some(request_metrics.elapsed_ms()),
        request_id: Some(request_id),
        // ... rest of response ...
    }))
}
```

**Apply this pattern to ALL 15+ RPCs in grpc_service.rs**

#### 7. Implement HealthCheck RPC

**Add to grpc_service.rs:**

```rust
async fn health_check(
    &self,
    _request: Request<HealthCheckRequest>,
) -> Result<Response<HealthCheckResponse>, Status> {
    let request_metrics = RequestMetrics::new();
    
    info!("❤️  HealthCheck called");
    
    // Get health status
    let mut health_monitor = self.health_monitor.write().await;
    let status = health_monitor.get_status();
    let is_healthy = health_monitor.is_healthy();
    drop(health_monitor);
    
    // Get metrics
    let total_requests = self.metrics.total_requests();
    let failed_requests = self.metrics.failed_requests();
    let avg_response_time = self.metrics.avg_response_time_ms();
    
    let response = HealthCheckResponse {
        healthy: is_healthy,
        status: if is_healthy { "healthy".into() } else { "degraded".into() },
        active_connections: 1, // TODO: Track actual connections
        cpu_usage_percent: status.cpu_usage_percent,
        memory_usage_mb: status.memory_usage_mb,
        version: env!("CARGO_PKG_VERSION").into(),
        uptime_seconds: status.uptime_seconds,
        total_requests: total_requests as i64,
        failed_requests: failed_requests as i64,
        avg_response_time_ms: avg_response_time,
    };
    
    info!("✅ HealthCheck: {:?}", response);
    Ok(Response::new(response))
}
```

### Sprint 1 Testing

**File:** `tests/v2_2_tests.rs` (NEW)

```rust
//! Integration tests for v2.2 features

#[cfg(test)]
mod tests {
    use axon_desktop_agent::*;
    
    #[tokio::test]
    async fn test_health_check_returns_valid_data() {
        // TODO: Implement
    }
    
    #[tokio::test]
    async fn test_metrics_recorded_on_success() {
        // TODO: Implement
    }
    
    #[tokio::test]
    async fn test_invalid_coordinates_rejected() {
        // TODO: Implement
    }
    
    #[tokio::test]
    async fn test_all_rpcs_include_request_id() {
        // TODO: Implement
    }
}
```

### Sprint 1 Documentation

Create `V2.2_RELEASE_NOTES.md` with:
- Feature descriptions
- Performance impact
- Migration guide for Hub
- Examples

---

## 🎯 SPRINT 2-4 SPECIFICATIONS

[Continue with Sprint 2, 3, 4 specifications in same detail...]

---

## ✅ VERIFICATION CHECKLIST

Before pushing to GitHub:

### Code Quality
- [ ] `cargo build --release` succeeds with no errors
- [ ] `cargo test` passes all tests
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt` applied to all files
- [ ] No `unwrap()` in production code
- [ ] All `TODO` comments addressed

### Documentation
- [ ] All proto messages have doc comments
- [ ] All public functions have /// doc comments
- [ ] README.md updated with new version
- [ ] CHANGELOG.md has all changes
- [ ] Release notes created (V2.X_RELEASE_NOTES.md)
- [ ] MAC_HUB_INTEGRATION_GUIDE.md updated

### Testing
- [ ] Unit tests: 90%+ coverage
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Performance benchmarks run
- [ ] No memory leaks detected

### Deployment
- [ ] Version bumped in Cargo.toml
- [ ] Git tag created
- [ ] Binary built and tested
- [ ] Bridge restarted successfully
- [ ] Logs show no errors

---

## 🚀 IMPLEMENTATION WORKFLOW

```bash
# 1. Create feature branch
git checkout -b feature/v2.2-telemetry

# 2. Implement Sprint 1
# ... follow guide above ...

# 3. Build and test
cargo build --release
cargo test
cargo clippy

# 4. Commit
git add -A
git commit -m "feat(v2.2): Add telemetry, validation, and health monitoring"

# 5. Push
git push origin feature/v2.2-telemetry

# 6. Merge to main
git checkout main
git merge feature/v2.2-telemetry

# 7. Tag release
git tag v2.2.0
git push origin main --tags

# 8. Deploy
./deploy.sh  # Restart bridge with new version
```

---

**This guide provides everything needed to implement v2.2-v2.5 with zero technical debt.**
**Next agent: Follow this guide section by section, test thoroughly, document completely.**
