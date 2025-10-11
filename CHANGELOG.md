# Changelog - AXON Linux Bridge

All notable changes to the Linux Bridge (desktop-agent) will be documented in this file.

## [Unreleased]

### Added - October 11, 2024

- **CloseApplication RPC** - Gracefully close application windows
  - Added `CloseApplication` RPC method to proto definition
  - Implemented Linux support using `wmctrl -c`
  - Implemented macOS support using AppleScript
  - Added comprehensive error handling and logging
  - Enables universal cleanup system in Hub
  - See `CLOSE_APPLICATION_RPC.md` for full documentation

### Changed

- Updated proto file with CloseApplication service definition
- Added CloseApplicationRequest and CloseApplicationResponse messages
- Enhanced README.md with Application Control section

### Technical Details

**Proto Changes:**
```protobuf
rpc CloseApplication(CloseApplicationRequest) returns (CloseApplicationResponse);

message CloseApplicationRequest {
  string agent_id = 1;
  string app_name = 2;  // Window title or app name to close
}

message CloseApplicationResponse {
  bool success = 1;
  string error = 2;
}
```

**Implementation:**
- Linux: Uses `wmctrl -c <title>` for graceful window closing
- macOS: Uses `osascript -e "tell application \"<name>\" to quit"`
- Windows: Planned (Win32 API SendMessage with WM_CLOSE)

**Benefits:**
- Enables Hub to clean up windows after tasks
- Universal cleanup system works across all tests
- Graceful shutdown (apps can save state)
- Production-ready error handling

---

## [Previous Work]

### OSWorld Evaluator Support - October 2024

- Added GetWindowList RPC
- Added GetProcessList RPC
- Added GetBrowserTabs RPC
- Added ListFiles RPC
- Added GetClipboard RPC
- Added LaunchApplication RPC

### Initial Implementation - September 2024

- gRPC protocol definition
- Platform detection (Linux/macOS/Windows)
- Agent lifecycle management
- Screen capture architecture
- Audio capture planning
- Input injection methods

---

## Notes for Future Implementations

### Windows Bridge (Future)
When implementing CloseApplication for Windows:
```rust
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SendMessageW, WM_CLOSE};

// Find window by title
let hwnd = FindWindowW(None, window_title);

// Send close message
SendMessageW(hwnd, WM_CLOSE, None, None);
```

### macOS Bridge Updates (Future)
Current implementation uses AppleScript. Consider alternatives:
- Accessibility API for window-level control
- CGWindowID and window management APIs
- More precise window targeting

---

## Semantic Versioning

This project follows [Semantic Versioning](https://semver.org/):
- **MAJOR**: Breaking proto changes
- **MINOR**: New RPC methods (backwards compatible)
- **PATCH**: Bug fixes and improvements

Current Version: **0.2.0** (unreleased)
- Added CloseApplication RPC (minor version bump)

Previous Version: **0.1.0**
- Initial OSWorld evaluator support
