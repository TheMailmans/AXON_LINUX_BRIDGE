# AxonHub Desktop Agent

Rust-based desktop capture agent for AxonHub. Provides cross-platform screen capture, audio streaming, and input injection capabilities.

## Architecture

```
┌─────────────────┐         gRPC          ┌──────────────────┐
│   AxonHub       │ ←──────────────────→   │ Desktop Agent    │
│   (TypeScript)  │                        │   (Rust)         │
└─────────────────┘                        └──────────────────┘
                                                     │
                                      ┌──────────────┼──────────────┐
                                      │              │              │
                                ┌──────▼────┐  ┌────▼─────┐  ┌────▼──────┐
                                │  Windows  │  │  macOS   │  │  Linux    │
                                │ Graphics  │  │ SCKit    │  │ PipeWire  │
                                │  Capture  │  │          │  │           │
                                └───────────┘  └──────────┘  └───────────┘
```

## Features

### Phase 1 (Current - Sprint 1.5)
- ✅ Platform detection (Windows/macOS/Linux)
- ✅ System information gathering
- ✅ Agent lifecycle management
- ✅ gRPC protocol definition
- ✅ Capture manager architecture
- 🚧 Hub connection (in progress)

### Phase 1 (Upcoming - Sprint 1.6-1.7)
- ⏳ Windows GraphicsCapture API integration
- ⏳ macOS ScreenCaptureKit integration
- ⏳ Linux PipeWire integration
- ⏳ Audio capture and streaming
- ⏳ Input injection (mouse/keyboard)

## Platform Support

| Platform | API | Status |
|----------|-----|--------|
| Windows 10+ | Windows.Graphics.Capture | Sprint 1.6 |
| macOS 13+ | ScreenCaptureKit | Sprint 1.6 |
| Linux | PipeWire | Sprint 1.6 |

## Building

### Prerequisites
- Rust 1.70+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Protocol Buffers compiler (`brew install protobuf` on macOS)

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run -- <session-id> [hub-url]
```

Example:
```bash
cargo run -- sess_abc123 http://localhost:4545
```

## Development

### Run tests
```bash
cargo test
```

### Run with logging
```bash
RUST_LOG=info cargo run -- sess_test
```

### Format code
```bash
cargo fmt
```

### Lint
```bash
cargo clippy
```

## Protocol

Communication with hub via gRPC (defined in `proto/agent.proto`):

### Lifecycle
- `RegisterAgent` - Register agent with hub
- `UnregisterAgent` - Unregister and cleanup
- `Heartbeat` - Keep-alive pings

### Capture
- `StartCapture` - Begin screen capture
- `StopCapture` - End capture
- `GetFrame` - Request single frame
- `StreamFrames` - Stream frames continuously

### Audio
- `StartAudio` - Begin audio capture
- `StopAudio` - End audio
- `StreamAudio` - Stream audio continuously

### Input
- `InjectMouseMove` - Move mouse cursor
- `InjectMouseClick` - Click mouse button
- `InjectKeyPress` - Press keyboard key

## Project Structure

```
desktop-agent/
├── src/
│   ├── main.rs          # Entry point
│   ├── agent.rs         # Agent core logic
│   ├── platform.rs      # Platform detection
│   ├── capture.rs       # Capture manager
│   └── proto_gen.rs     # Generated protobuf code
├── proto/
│   └── agent.proto      # gRPC protocol definition
├── build.rs             # Build script (protobuf compilation)
├── Cargo.toml           # Dependencies
└── README.md            # This file
```

## License

AGPL-3.0-only (same as parent AxonHub project)
