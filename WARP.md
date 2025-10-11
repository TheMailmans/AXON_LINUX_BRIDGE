# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

AXONBRIDGE (formerly AxonHub Desktop Agent) is a cross-platform Rust desktop agent that provides screen capture, audio streaming, and input injection capabilities. It communicates with the AxonHub system via gRPC and supports OSWorld benchmarking compatibility.

**Purpose**: Enable AI-based computer control through real-time screenshot capture, system state queries, and input injection (keyboard/mouse).

**Primary Language**: Rust 2021 edition

## Common Development Commands

### Building

```bash
# Development build
cargo build

# Production/release build (optimized)
cargo build --release

# Linux-specific build with dependencies
./BUILD_LINUX.sh
```

### Running

```bash
# Basic run (requires session-id)
cargo run -- <session-id> [hub-url] [grpc-port]

# With logging
RUST_LOG=info cargo run -- test-session http://localhost:4545 50051

# Debug-level logging
RUST_LOG=debug cargo run -- test-session http://localhost:4545 50051

# Run release binary directly
./target/release/axon-desktop-agent test-session http://localhost:4545 50051
```

**Arguments**:
- `<session-id>`: Required session identifier
- `[hub-url]`: Optional hub URL (default: `http://localhost:4545`)
- `[grpc-port]`: Optional gRPC port (default: `50051`)

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests for specific module
cargo test capture::
```

### Code Quality

```bash
# Format code
cargo fmt

# Check formatting without applying
cargo fmt --check

# Run linter
cargo clippy

# Run clippy with all warnings
cargo clippy -- -W clippy::all

# Check without building
cargo check
```

### Protocol Buffer Regeneration

Protocol buffers are automatically compiled by `build.rs` during cargo build. If you modify `proto/agent.proto`, just rebuild:

```bash
cargo build
```

## Architecture Overview

### High-Level Design

The agent follows a **service-oriented architecture** with clear separation between platform-specific implementations and cross-platform abstractions:

```
┌──────────────────────────────────────────────────────┐
│                   main.rs                             │
│         (Entry point + gRPC server setup)             │
└────────────────┬─────────────────────┬────────────────┘
                 │                     │
        ┌────────▼────────┐   ┌───────▼──────────┐
        │   Agent         │   │  gRPC Service     │
        │  (agent.rs)     │   │ (grpc_service.rs) │
        │                 │   │                   │
        │ - Lifecycle     │   │ - RPC handlers    │
        │ - State mgmt    │   │ - Request routing │
        └────────┬────────┘   └───────┬───────────┘
                 │                    │
         ┌───────┴────────────────────┴───────┐
         │                                    │
    ┌────▼──────────┐              ┌─────────▼──────┐
    │ StreamManager │              │ CaptureManager │
    │ (streaming/)  │              │  (capture.rs)  │
    │               │              │                │
    │ - Frame queue │              │ - Platform     │
    │ - Encoding    │              │   abstraction  │
    └────┬──────────┘              └─────────┬──────┘
         │                                   │
         │                         ┌─────────┴──────────────┐
         │                         │                        │
    ┌────▼────────┐      ┌────────▼────────┐    ┌─────────▼────────┐
    │   Video     │      │  Linux (X11)    │    │  macOS (SCKit)   │
    │  Encoder    │      │  capture/       │    │  capture/        │
    │ (video/)    │      │  linux.rs       │    │  macos.rs        │
    └─────────────┘      └─────────────────┘    └──────────────────┘
```

### Core Components

#### 1. **Agent** (`src/agent.rs`)
- **Responsibility**: Central coordinator for agent lifecycle, state management, and capture orchestration
- **Key Methods**:
  - `new()`: Initialize agent with session and hub URL
  - `start_capture()` / `stop_capture()`: Manage capture lifecycle
  - `subscribe_frames()`: Get broadcast receiver for encoded frames
  - `disconnect()`: Clean shutdown
- **State Machine**: `Initializing → Connected → Capturing → Disconnected`

#### 2. **gRPC Service** (`src/grpc_service.rs`)
- **Responsibility**: Implements the DesktopAgent gRPC protocol (defined in `proto/agent.proto`)
- **Key RPCs**:
  - `RegisterAgent` / `UnregisterAgent`: Connection lifecycle
  - `GetFrame`: On-demand screenshot (creates temporary capturer)
  - `StartCapture` / `StopCapture`: Streaming control
  - `InjectKeyPress` / `InjectMouseClick`: Input injection
  - `GetWindowList` / `GetSystemInfo`: System state queries (OSWorld support)
- **Design Note**: `GetFrame` creates a separate short-lived capturer independent of streaming pipeline

#### 3. **StreamManager** (`src/streaming/`)
- **Responsibility**: Real-time capture-encode-transmit pipeline
- **Architecture**: 
  - Capture thread → Frame queue → Encoding thread → Broadcast channel
  - Backpressure handling via bounded queues
  - Adaptive quality (future)
- **Usage Pattern**:
  ```rust
  let mut manager = StreamManager::new(config);
  manager.start().await?;
  let mut rx = manager.subscribe();
  while let Ok(frame) = rx.recv().await { /* process frame */ }
  ```

#### 4. **CaptureManager** (`src/capture.rs` + `src/capture/{platform}.rs`)
- **Responsibility**: Platform abstraction for screen capture
- **Pattern**: Trait-based platform dispatch via `PlatformCapturer` trait
- **Platforms**:
  - **Linux**: X11/XCB direct capture (no PipeWire yet)
  - **macOS**: ScreenCaptureKit (SCKit) via Objective-C FFI
  - **Windows**: Graphics.Capture API (planned)
- **Key Methods**:
  - `start(&config)`: Initialize capture with mode (Desktop/Window/Region)
  - `get_raw_frame()`: Returns `RawFrame` with RGBA data + dimensions
  - `stop()`: Clean teardown

#### 5. **Video Encoder** (`src/video/encoder.rs`)
- **Responsibility**: Encode raw RGBA frames to PNG/JPEG
- **Current Implementation**: Software encoding via `image` crate
- **Future**: Hardware acceleration (VideoToolbox for macOS, etc.)
- **Output**: `EncodedFrame { data: Vec<u8>, format: FrameFormat, ... }`

#### 6. **Input Injection** (`src/input/`)
- **Responsibility**: Keyboard and mouse control
- **Platform-specific**:
  - Linux: X11 `XTestFakeKeyEvent` / `XTestFakeButtonEvent`
  - macOS: CGEvent API
- **Key Types**:
  - `keyboard.rs`: Key code translation and key press/release
  - `mouse.rs`: Mouse movement, clicks, buttons

### Key Architectural Patterns

#### Pattern 1: Platform Abstraction via Traits
Capture, audio, and input use trait-based dispatch to keep platform code isolated:
```rust
trait PlatformCapturer {
    fn start(&mut self, config: &CaptureConfig) -> Result<()>;
    fn get_frame(&mut self) -> Result<Vec<u8>>;
}

#[cfg(target_os = "linux")]
impl PlatformCapturer for LinuxCapturer { ... }
```

#### Pattern 2: Dual Capture Modes
- **On-demand** (`GetFrame` RPC): Short-lived capturer for single screenshots
- **Streaming** (`StreamManager`): Long-lived pipeline for continuous capture

**Why?** Streaming has overhead (threads, queues, encoding); one-shot queries should be fast.

#### Pattern 3: Async Runtime + Sync Capture
- **Async**: gRPC handlers, agent lifecycle (Tokio)
- **Sync**: Platform capture APIs (blocking FFI)
- **Bridge**: `tokio::task::spawn_blocking` for capture in async contexts

#### Pattern 4: Broadcast-based Frame Distribution
`StreamManager` uses `tokio::sync::broadcast` to distribute frames to multiple consumers:
```rust
let rx1 = manager.subscribe();
let rx2 = manager.subscribe();  // Independent consumers
```

### Module Breakdown

```
src/
├── main.rs              # Entry point, CLI parsing, server init
├── agent.rs             # Agent state machine and lifecycle
├── grpc_service.rs      # gRPC RPC implementations
├── platform.rs          # Platform detection, system info
├── proto_gen.rs         # Generated protobuf code (module import)
├── lib.rs               # Library exports
│
├── capture/             # Screen capture (platform-specific)
│   ├── linux.rs         # X11/XCB capture
│   ├── macos.rs         # ScreenCaptureKit (SCKit)
│   └── windows.rs       # Graphics.Capture (stub)
│
├── streaming/           # Real-time streaming pipeline
│   ├── mod.rs           # Config, timing, quality types
│   └── stream_manager.rs # Capture→Encode→Broadcast orchestrator
│
├── video/               # Video encoding
│   ├── encoder.rs       # Software PNG/JPEG encoder
│   ├── frame.rs         # Frame types (RawFrame, EncodedFrame)
│   └── videotoolbox_ffi.rs # macOS VideoToolbox (future)
│
├── input/               # Input injection
│   ├── keyboard.rs      # Key code mapping, key press
│   ├── mouse.rs         # Mouse movement, clicks
│   ├── linux.rs         # X11 XTest
│   └── macos_keys.rs    # macOS key mappings
│
├── audio/               # Audio capture (disabled for MVP)
│   ├── linux.rs         # PulseAudio
│   ├── macos.rs         # CoreAudio
│   └── windows.rs       # WASAPI
│
└── a11y/                # Accessibility tree parsing (OSWorld)
```

### Important Cross-File Dependencies

1. **Agent ↔ StreamManager**: Agent creates and manages StreamManager lifecycle
2. **gRPC Service ↔ Agent**: Service holds `Option<Agent>` to route requests
3. **StreamManager ↔ CaptureManager**: StreamManager owns capture instance
4. **CaptureManager ↔ Platform Capturers**: Dynamic dispatch via trait
5. **StreamManager ↔ VideoEncoder**: Encoding happens in StreamManager thread

## Platform-Specific Notes

### Linux (Primary Target)
- **Display Server**: Currently X11 only (use `echo $XDG_SESSION_TYPE` to check)
- **Capture**: Direct X11 via `xcb` crate (no Portal/PipeWire yet)
- **Input**: X11 XTest extension
- **Dependencies**: See `BUILD_LINUX.sh` - requires X11 dev headers
- **Wayland**: Not yet supported (planned for future)

### macOS
- **Capture**: ScreenCaptureKit (requires macOS 13+)
- **FFI**: Manual Objective-C bindings in `capture/macos.rs`
- **Input**: CGEvent API
- **Permissions**: Requires Screen Recording permission in System Preferences

### Windows
- **Status**: Stub implementation only
- **Planned**: Graphics.Capture API (Windows 10+)

## Development Guidelines

### When Adding Platform-Specific Code

1. **Use conditional compilation**:
   ```rust
   #[cfg(target_os = "linux")]
   mod linux;
   ```

2. **Implement platform trait** in `capture.rs`, `audio/`, or `input/`:
   ```rust
   impl PlatformCapturer for NewPlatformCapturer { ... }
   ```

3. **Update `build.rs`** for platform-specific linking

4. **Add platform checks** in `platform.rs` if needed

### When Modifying gRPC Protocol

1. Edit `proto/agent.proto`
2. Run `cargo build` (build.rs regenerates Rust bindings)
3. Update corresponding RPC handler in `grpc_service.rs`
4. Update client code in hub (if applicable)

### When Adding New RPC Handlers

1. Add RPC definition to `proto/agent.proto`
2. Implement in `grpc_service.rs`:
   ```rust
   async fn new_rpc(&self, request: Request<Req>) -> Result<Response<Res>, Status> {
       // Route to agent method or handle directly
   }
   ```
3. If agent state needed, add method to `Agent` in `agent.rs`

### Async vs Sync Guidelines

- **Use async** for: I/O, network, gRPC handlers, agent coordination
- **Use sync + spawn_blocking** for: Platform FFI, capture loops, encoding
- **Never block** the Tokio runtime with long-running sync operations

### Error Handling

- **Library code**: Return `anyhow::Result<T>` for flexibility
- **gRPC handlers**: Convert to `tonic::Status` with `.map_err(|e| Status::internal(...))`
- **FFI boundaries**: Validate pointers, handle null returns, log errors

## Testing Notes

- **Unit tests**: In module files (e.g., `streaming/mod.rs` has timing tests)
- **Integration tests**: Currently minimal (add to `tests/` directory)
- **Manual testing**: Use Python gRPC client (see README_NEW.md for example)
- **Platform testing**: Test on actual hardware for capture/input (no VMs for capture)

## Logging

Use `tracing` macros for structured logging:

```rust
use tracing::{info, warn, error, debug};

info!("Starting capture");
error!("Failed to connect: {}", err);
debug!("Frame size: {}x{}", width, height);
```

**Log levels**: Set via `RUST_LOG` environment variable:
- `RUST_LOG=info`: Standard operation
- `RUST_LOG=debug`: Detailed debugging
- `RUST_LOG=axon_desktop_agent=trace`: Module-specific verbose logging

## Build System

- **build.rs**: Compiles protobuf, links platform frameworks
- **Cargo features**:
  - `h264`: Software H.264 encoding (optional, disabled by default)
  - `webrtc`: WebRTC support (optional, disabled by default)
- **Release profile**: Aggressive optimization (`opt-level=3`, LTO, strip)

## Important Caveats

1. **Audio is disabled**: Agent was built for MVP without audio (see `agent.rs` comments)
2. **GetFrame independence**: `GetFrame` RPC creates its own capturer, doesn't use StreamManager
3. **Linux requires X11**: No Wayland support yet (check with `echo $XDG_SESSION_TYPE`)
4. **macOS permissions**: Needs Screen Recording permission grant
5. **Streaming overhead**: StreamManager has thread+queue overhead, use `GetFrame` for one-shot queries

## Related Projects

This agent is designed to work with **AxonHub** (Rust/TypeScript hub) for AI-based computer control. It implements the OSWorld-compatible protocol for benchmarking LLM agents.

## Useful References

- **Protocol**: `proto/agent.proto` - Full gRPC API definition
- **READMEs**: `README.md` (overview), `README_NEW.md` (Linux-specific guide)
- **Build**: `BUILD_LINUX.sh` - Linux dependency installation
