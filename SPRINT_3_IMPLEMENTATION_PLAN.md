# Sprint 3 (v2.4) Implementation Plan
## Batch Operations & Smart Features

**Target Version:** 2.4.0  
**Estimated Effort:** 80-100 hours  
**Scope:** 4 major features with zero technical debt  
**Status:** Planning Phase

---

## 📋 Overview

Sprint 3 delivers batch operations and smart features to enable Hub agents to perform multi-action workflows efficiently:

1. **Batch RPC** - Multiple RPCs in single request
2. **Clipboard Monitoring** - System clipboard access and monitoring
3. **Differential Screenshots** - Frame diffing to reduce bandwidth
4. **Smart Validation** - Enhanced validation for batch operations

---

## 🎯 Feature Breakdown

### Feature 1: Batch RPC (Highest Priority)

**Purpose:** Allow Hub to submit multiple operations in one request, improving throughput and reducing latency overhead

**Proto Changes Required:**
```protobuf
// NEW in v2.4: Batch operations
message BatchRequest {
  string agent_id = 1;
  repeated Operation operations = 2;
  bool stop_on_error = 3;  // Stop if any operation fails
}

message Operation {
  oneof op {
    MouseMoveRequest mouse_move = 1;
    MouseClickRequest mouse_click = 2;
    KeyPressRequest key_press = 3;
    ScrollRequest scroll = 4;
    TypeTextRequest type_text = 5;
  }
}

message BatchResponse {
  repeated OperationResult results = 1;
  int32 success_count = 2;
  int32 failure_count = 3;
  int64 total_time_ms = 4;
}

message OperationResult {
  bool success = 1;
  optional string error = 2;
  optional ErrorCode error_code = 3;
  int64 execution_time_ms = 4;
  optional string request_id = 5;
}
```

**Implementation Details:**
- [ ] Create `src/batch.rs` module
  - [ ] Operation enum wrapper
  - [ ] Batch executor with transaction-like semantics
  - [ ] Atomic vs. partial failure handling
- [ ] Update proto compilation
- [ ] Add `batch_execute()` RPC to gRPC service
- [ ] Integrate with existing validation
- [ ] Add metrics for batch operations
- [ ] Unit tests: 15+ tests
  - [ ] Single operation in batch
  - [ ] Multiple operations
  - [ ] Failure handling
  - [ ] Error aggregation
- [ ] Integration tests: 5+ tests
  - [ ] End-to-end batch workflow
  - [ ] Partial failures

**Performance Goals:**
- Batch of 5 ops: <100ms overhead (vs 25ms per op separately = 125ms)
- 95th percentile: <500ms for 10-operation batch

---

### Feature 2: Clipboard Monitoring

**Purpose:** Allow Hub to read/write system clipboard for copy-paste workflows

**Proto Changes:**
```protobuf
// NEW in v2.4: Clipboard operations
message GetClipboardRequest {
  string agent_id = 1;
}

message GetClipboardResponse {
  string content = 1;
  string content_type = 2;  // "text", "image", "html"
  int64 retrieved_at = 3;
}

message SetClipboardRequest {
  string agent_id = 1;
  string content = 2;
  string content_type = 3;
}

message SetClipboardResponse {
  bool success = 1;
  optional string error = 2;
}

// Monitor clipboard changes
message MonitorClipboardRequest {
  string agent_id = 1;
  int32 poll_interval_ms = 2;  // Default: 500ms
}

message ClipboardChangeEvent {
  string content = 1;
  string content_type = 2;
  int64 changed_at = 3;
}
```

**Implementation Details:**
- [ ] Create `src/clipboard/mod.rs`
  - [ ] `src/clipboard/linux.rs` - xclip/xsel based
  - [ ] `src/clipboard/macos.rs` - pbpaste/pbcopy
  - [ ] `src/clipboard/windows.rs` - Win32 API
- [ ] Trait: `ClipboardProvider`
  - [ ] `get_text() -> Result<String>`
  - [ ] `set_text(text: &str) -> Result<()>`
  - [ ] `get_image() -> Result<Vec<u8>>` (optional)
  - [ ] `monitor(callback) -> Result<()>`
- [ ] Add to `DesktopAgentService`
- [ ] Update validation to handle clipboard ops
- [ ] Unit tests: 10+ tests per platform
  - [ ] Get clipboard content
  - [ ] Set clipboard content
  - [ ] Content type detection
  - [ ] Platform-specific tests
- [ ] Integration tests: 5+ tests

**Performance Goals:**
- Get clipboard: <10ms
- Set clipboard: <5ms
- Polling overhead: <50ms per second

---

### Feature 3: Differential Screenshots

**Purpose:** Reduce bandwidth by sending only changed pixels

**Architecture:**
```
Frame N-1: Store full frame + hash
    ↓
Frame N: Capture + hash
    ↓
Compare hashes: Different?
    ├─ YES: Compute diff (changed regions)
    │       → Send diff + regions (80% smaller)
    └─ NO: Skip frame (already in cache)
```

**Proto Changes:**
```protobuf
// NEW in v2.4: Differential frames
message DiffRegion {
  int32 x = 1;
  int32 y = 2;
  int32 width = 3;
  int32 height = 4;
  bytes pixel_data = 5;
}

message DiffFrame {
  string base_frame_hash = 1;      // Hash of previous frame
  repeated DiffRegion regions = 2;
  int64 total_changed_pixels = 3;
  int32 changed_percent = 4;       // % of frame changed
}

// Updated GetFrame response
message GetFrameResponse {
  oneof content {
    FrameData full_frame = 1;
    DiffFrame diff_frame = 2;
  }
}
```

**Implementation Details:**
- [ ] Create `src/capture/diff.rs`
  - [ ] Region detection (simple: rectangular regions)
  - [ ] Pixel-by-pixel diff algorithm
  - [ ] Region merging/optimization
  - [ ] 10+ unit tests
- [ ] Integrate into GetFrame RPC
  - [ ] Track previous frame
  - [ ] Compute diff on miss
  - [ ] Return full or diff based on size
- [ ] Extend compression module with diff utilities
- [ ] Update cache to store base frames for diff
- [ ] Unit tests: 15+ tests
  - [ ] Region detection
  - [ ] Diff computation accuracy
  - [ ] Region merging
  - [ ] Size comparison
- [ ] Integration tests: 5+ tests

**Performance Goals:**
- Diff computation: <50ms for 1920x1080
- Diff size: 20-30% of full frame (typical)
- Diff latency: <100ms total

---

### Feature 4: Enhanced Validation

**Purpose:** Extend validation to batch operations and new features

**Scope:**
- [ ] Create `src/validation/batch.rs`
  - [ ] Validate batch size (max 100 ops default)
  - [ ] Validate operation sequence (e.g., don't click before window exists)
  - [ ] Cross-operation dependencies
- [ ] Extend `src/validation.rs`
  - [ ] Validate clipboard content size (max 10MB)
  - [ ] Validate diff regions
- [ ] Add to `CoordinateValidator`
  - [ ] Validate region bounds
  - [ ] Validate pixel data size
- [ ] Unit tests: 20+ tests

---

## 📊 Implementation Phases

### Phase 1: Proto & Core Modules (Week 1)
- [ ] Define all proto changes
- [ ] Update build scripts
- [ ] Create batch module skeleton
- [ ] Create clipboard module skeleton
- [ ] Create diff module skeleton
- **Deliverable:** Proto compiles, empty modules exist

### Phase 2: Batch Implementation (Week 2)
- [ ] Implement `src/batch.rs` module
- [ ] Add batch executor
- [ ] Implement `batch_execute()` RPC
- [ ] Add 15+ unit tests
- [ ] Add 5+ integration tests
- **Deliverable:** Batch RPC functional, tested, documented

### Phase 3: Clipboard Implementation (Week 2-3)
- [ ] Implement clipboard providers (Linux, macOS, Windows)
- [ ] Add to gRPC service
- [ ] Add GetClipboard/SetClipboard RPCs
- [ ] Add 30+ unit tests (10+ per platform)
- [ ] Add 5+ integration tests
- **Deliverable:** Clipboard operations functional, platform-specific

### Phase 4: Diff Implementation (Week 3)
- [ ] Implement diff algorithm
- [ ] Extend GetFrame with diff logic
- [ ] Add 15+ unit tests
- [ ] Add 5+ integration tests
- **Deliverable:** Differential frames working, size reduced

### Phase 5: Validation Enhancement (Week 4)
- [ ] Extend validation module
- [ ] Add 20+ unit tests
- [ ] Update existing validation
- **Deliverable:** Comprehensive validation across all features

### Phase 6: Testing & Documentation (Week 4)
- [ ] Write V2.4_RELEASE_NOTES.md
- [ ] Create architecture documentation
- [ ] Performance testing
- [ ] Integration testing
- [ ] End-to-end testing
- **Deliverable:** Production-ready v2.4.0

---

## 🧪 Test Plan

### Total Tests Target: 100+ new tests
- Batch module: 20 tests
- Clipboard module: 30 tests (10 per platform)
- Diff module: 20 tests
- Validation module: 20 tests
- Integration tests: 10 tests

### Test Categories
- **Unit tests:** Each function
- **Integration tests:** End-to-end workflows
- **Performance tests:** Latency/throughput
- **Platform tests:** Linux, macOS, Windows (where applicable)
- **Edge cases:** Empty batches, large clipboards, identical frames

---

## 📚 Documentation Plan

### Code Documentation
- [ ] Module-level comments with examples
- [ ] Function-level docs with error cases
- [ ] Proto message documentation
- [ ] Integration examples in docstrings

### Release Documentation
- [ ] V2.4_RELEASE_NOTES.md
  - Feature descriptions
  - Performance metrics
  - Migration guide
  - Usage examples
- [ ] SPRINT_3_IMPLEMENTATION_GUIDE.md
  - Architecture overview
  - Design decisions
  - Performance profile

### Integration Guides
- [ ] Hub integration examples (Python)
- [ ] Batch operation workflows
- [ ] Clipboard usage patterns
- [ ] Diff handling logic

---

## 🚀 Quality Standards

### Code Quality
- [ ] Zero `unwrap()` in production
- [ ] 100% error handling
- [ ] Thread-safe throughout
- [ ] Async-first design
- [ ] No blocking operations on async runtime

### Performance Requirements
- [ ] Batch execution: <500ms for 10 ops (95th percentile)
- [ ] Clipboard operations: <50ms
- [ ] Diff computation: <50ms for 1920x1080
- [ ] Memory overhead: <100MB total

### Testing Requirements
- [ ] 100+ new unit tests
- [ ] 10+ integration tests
- [ ] All tests passing before release
- [ ] Code coverage: 90%+ for new code

### Documentation Requirements
- [ ] All public APIs documented
- [ ] Migration guide for Hub developers
- [ ] Performance tuning guide
- [ ] Troubleshooting guide

---

## 📐 Architecture Decisions

### Batch Execution Model
**Option:** Sequential execution with early termination on error
- Pros: Simple, predictable, safe
- Cons: Slower than parallel
- Decision: Go sequential for safety, add parallelism in v2.5

### Clipboard Backend
**Options Considered:**
- xclip/xsel (Linux) - Good, stable
- pbpaste/pbcopy (macOS) - Standard, reliable
- Win32 API (Windows) - Direct, efficient
- Decision: Use native tools for reliability, consider native bindings later

### Diff Algorithm
**Options Considered:**
- Per-pixel comparison - Slow but accurate
- Block-based (16x16 blocks) - Faster, good enough
- Decision: Per-pixel for accuracy, optimize later if needed

---

## 🔄 Dependencies & Integration

### Internal Dependencies
- Uses v2.2 validation module
- Uses v2.3 metrics/health/caching
- Extends v2.3 proto definitions

### External Dependencies to Add
- `image` crate enhancements (if needed for diff)
- Platform-specific clipboard libraries (evaluated per OS)

### No Breaking Changes
- All existing RPCs unchanged
- New RPCs are purely additive
- Proto is backward compatible

---

## ✅ Success Criteria

| Criterion | Target | Status |
|-----------|--------|--------|
| **Batch RPC** | Functional, tested, <500ms | ⏳ Pending |
| **Clipboard** | Multi-platform, <50ms | ⏳ Pending |
| **Diff Frames** | Working, 20-30% size | ⏳ Pending |
| **Validation** | Comprehensive, <5ms | ⏳ Pending |
| **Tests** | 100+ passing | ⏳ Pending |
| **Documentation** | Complete, migration guide | ⏳ Pending |
| **Performance** | Meets goals | ⏳ Pending |
| **Zero Tech Debt** | 100% error handling, no unsafe | ⏳ Pending |

---

## 🎓 Example Usage (Hub Perspective)

### Batch Operations
```python
# Hub: Submit multiple operations
batch = BatchRequest(
    agent_id="agent1",
    operations=[
        Operation(mouse_move=MouseMoveRequest(x=100, y=200)),
        Operation(mouse_click=MouseClickRequest(x=100, y=200, button=1)),
        Operation(key_press=KeyPressRequest(key="Ctrl+A")),
        Operation(type_text=TypeTextRequest(text="Hello World")),
    ]
)
response = bridge.batch_execute(batch)
# All 4 operations executed in ~50ms vs ~100ms separately
```

### Clipboard Operations
```python
# Hub: Read clipboard
response = bridge.get_clipboard(agent_id="agent1")
content = response.content

# Hub: Write clipboard
bridge.set_clipboard(
    agent_id="agent1",
    content="New clipboard content"
)
```

### Differential Frames
```python
# Hub: Get frame (may be diff)
response = bridge.get_frame(agent_id="agent1")
if response.HasField("diff_frame"):
    # Handle differential frame
    regions = response.diff_frame.regions
else:
    # Handle full frame
    frame = response.full_frame
```

---

## 📝 Deliverables Checklist

- [ ] Sprint 3 implementation plan (this document)
- [ ] Proto definitions (all new messages)
- [ ] Batch module (src/batch.rs)
- [ ] Clipboard module (src/clipboard/*)
- [ ] Diff module (src/capture/diff.rs)
- [ ] Enhanced validation
- [ ] gRPC service updates
- [ ] 100+ unit tests
- [ ] 10+ integration tests
- [ ] V2.4_RELEASE_NOTES.md
- [ ] Architecture documentation
- [ ] Integration examples
- [ ] Performance benchmarks
- [ ] All tests passing
- [ ] Zero technical debt

---

## 🔮 Future Improvements (Post-v2.4)

- Parallel batch execution (v2.5)
- Image clipboard support (v2.5)
- Advanced diff algorithms (v2.5)
- Clipboard history monitoring (v2.6)
- Batch transaction semantics (v2.6)

---

**Status: Ready for implementation**  
**No shortcuts: Complete, production-ready approach**  
**Start date: Now**  
**Target completion: 4 weeks**
