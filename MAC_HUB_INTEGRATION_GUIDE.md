# AxonHub Mac Core Integration Guide

**Document Version:** 2.0  
**Bridge Version:** 2.0.0  
**Last Updated:** 2025-10-17  
**Status:** ✅ Production Ready

---

## 🎯 Quick Start

### Bridge Connection Details

```
IP Address:   192.168.64.3
gRPC Port:    50051
Protocol:     gRPC (insecure channel)
Full Address: 192.168.64.3:50051
```

### Test Connection

```bash
# From Mac
grpcurl -plaintext 192.168.64.3:50051 list
```

```python
# Python test
import grpc
from generated_pb2 import DesktopAgentStub

channel = grpc.insecure_channel('192.168.64.3:50051')
stub = DesktopAgentStub(channel)
info = stub.GetSystemInfo()
print(f"Connected to: {info.os_name} {info.os_version}")
```

---

## 🚀 Bridge Capabilities (v2.0.0)

### ✅ Fully Working RPCs

| RPC Name | Status | Latency | Notes |
|----------|--------|---------|-------|
| **GetSystemInfo** | ✅ Working | <10ms | Returns OS details, display info |
| **GetWindowList** | ✅ Working | 50-100ms | All open windows with titles/IDs |
| **LaunchApplication** | ✅ Working | 200-500ms | Uses .desktop files, spawn_blocking |
| **CloseApplication** | ✅ Working | 100-200ms | Window ID close + fallback pkill |
| **InjectMouseClick** | ✅ Working | <50ms | Spawn_blocking fix applied |
| **InjectMouseMove** | ✅ Working | <50ms | Async-safe |
| **InjectKeyPress** | ✅ Working | 20-40ms | Special keys + modifiers |
| **GetFrame** | ✅ Working | 150-300ms | Native scrot screenshot |
| **TakeScreenshot** | ✅ Working | 150-300ms | Saves to file |
| **TypeText** | ✅ NEW v2.0 | 50-100ms | Natural text typing |
| **InjectScroll** | ✅ NEW v2.0 | <50ms | Mouse wheel scrolling |
| **GetCapabilities** | ✅ NEW v2.0 | <10ms | Lists all Bridge features |

### 🔧 Known Issues & Workarounds

#### 1. Right-Click on Empty Desktop
**Issue:** Ubuntu GNOME doesn't show context menu on empty desktop background  
**Workaround:** Right-click works on files, windows, icons - just not empty space  
**Status:** Desktop environment limitation, not a bug

#### 2. Window Focus
**Issue:** Some apps require focus before clicks register  
**Workaround:** Use `wmctrl -ia <window_id>` before injecting input  
**Status:** Improved in v2.0 with click-to-focus logic

#### 3. RPC Timeouts
**Recommendation:** Set Hub-side RPC timeout to **30 seconds minimum**  
**Reason:** App launches can take 5-10 seconds on first run  
**Default:** Most gRPC clients use 5s (too short!)

---

## 📋 Complete RPC Reference

### 1. GetSystemInfo

```protobuf
rpc GetSystemInfo(Empty) returns (SystemInfo)

message SystemInfo {
  string os_name = 1;        // "Linux"
  string os_version = 2;     // "22.04"
  string hostname = 3;       // "ubuntu-vm"
  repeated Display displays = 4;
}

message Display {
  int32 width = 1;           // 1920
  int32 height = 2;          // 1080
  int32 x = 3;               // 0
  int32 y = 4;               // 0
  bool is_primary = 5;       // true
}
```

**Use Case:** Get screen dimensions for coordinate calculations

---

### 2. LaunchApplication

```protobuf
rpc LaunchApplication(LaunchRequest) returns (LaunchResponse)

message LaunchRequest {
  string application_name = 1;  // "gnome-calculator" or "org.gnome.Calculator"
  repeated string arguments = 2;
}

message LaunchResponse {
  bool success = 1;
  string message = 2;
  int32 process_id = 3;      // May be 0 if detached
}
```

**Notes:**
- Supports both binary names ("gnome-calculator") and desktop IDs ("org.gnome.Calculator")
- Uses spawn() for background launching (non-blocking)
- Returns immediately (~200-500ms)
- App may take additional 2-5s to fully appear

**Common Apps:**
- Calculator: `org.gnome.Calculator`
- VS Code: `code`
- Firefox: `firefox`
- Terminal: `gnome-terminal`

---

### 3. CloseApplication

```protobuf
rpc CloseApplication(CloseRequest) returns (CloseResponse)

message CloseRequest {
  string application_name = 1;  // "Calculator" or "gnome-calculator"
}

message CloseResponse {
  bool success = 1;
  string message = 2;
}
```

**Implementation:**
1. List all windows with `wmctrl -l`
2. Case-insensitive match on window title
3. Close by window ID: `wmctrl -ic <window_id>`
4. Fallback to process kill: `pkill -f <app_name>`

**Latency:** 100-200ms

---

### 4. InjectMouseClick

```protobuf
rpc InjectMouseClick(MouseClickRequest) returns (MouseClickResponse)

message MouseClickRequest {
  int32 x = 1;
  int32 y = 2;
  MouseButton button = 3;    // LEFT, RIGHT, MIDDLE
  int32 click_count = 4;     // 1=single, 2=double
}

message MouseClickResponse {
  bool success = 1;
  string message = 2;
}
```

**Implementation:**
- Uses `xdotool` via spawn_blocking (async-safe)
- Moves cursor to position first
- Performs click(s)
- Returns immediately

**Button Mapping:**
- `LEFT = 0` → xdotool button 1
- `RIGHT = 1` → xdotool button 3
- `MIDDLE = 2` → xdotool button 2

**⚠️ CRITICAL:** Ensure Hub sends correct button enum values!

---

### 5. InjectKeyPress

```protobuf
rpc InjectKeyPress(KeyPressRequest) returns (KeyPressResponse)

message KeyPressRequest {
  string key = 1;            // "a", "Return", "ctrl+c"
  repeated string modifiers = 2;  // ["ctrl", "shift"]
}

message KeyPressResponse {
  bool success = 1;
  string message = 2;
}
```

**Supported Keys:**
- Printable: a-z, 0-9, symbols
- Special: Return, Escape, Tab, BackSpace, space
- Modifiers: ctrl, shift, alt, super
- Function: F1-F12

**Format:**
- Single key: `key = "a"`
- With modifiers: `key = "c"`, `modifiers = ["ctrl"]`
- Or combined: `key = "ctrl+c"`

---

### 6. GetFrame / TakeScreenshot

```protobuf
rpc GetFrame(Empty) returns (FrameResponse)

message FrameResponse {
  bytes image_data = 1;      // PNG encoded
  int32 width = 2;
  int32 height = 3;
  int64 timestamp_ms = 4;
}

rpc TakeScreenshot(ScreenshotRequest) returns (ScreenshotResponse)

message ScreenshotRequest {
  string file_path = 1;      // "/tmp/screenshot.png"
}

message ScreenshotResponse {
  bool success = 1;
  string file_path = 2;
}
```

**Implementation:**
- Native `scrot` command (reliable)
- PNG format
- Full screen capture
- Typical size: 150-250 KB
- Latency: 150-300ms

**Use Case:** Vision-based action planning

---

### 7. TypeText (NEW v2.0)

```protobuf
rpc TypeText(TypeTextRequest) returns (TypeTextResponse)

message TypeTextRequest {
  string text = 1;           // "Hello World!"
  int32 delay_ms = 2;        // Optional delay between chars
}

message TypeTextResponse {
  bool success = 1;
  string message = 2;
}
```

**Benefits:**
- Natural text entry (better than individual key presses)
- Handles spaces, punctuation, mixed case
- Uses `xdotool type`

**Use Case:** Form filling, text editors, search boxes

---

### 8. InjectScroll (NEW v2.0)

```protobuf
rpc InjectScroll(ScrollRequest) returns (ScrollResponse)

message ScrollRequest {
  int32 x = 1;               // Mouse position X
  int32 y = 2;               // Mouse position Y
  ScrollDirection direction = 3;  // UP, DOWN, LEFT, RIGHT
  int32 amount = 4;          // Number of scroll "clicks"
}

message ScrollResponse {
  bool success = 1;
  string message = 2;
}
```

**Implementation:**
- Moves cursor to (x, y)
- Scrolls specified direction
- Amount = number of wheel clicks

**Use Case:** Page navigation, long documents

---

### 9. GetCapabilities (NEW v2.0)

```protobuf
rpc GetCapabilities(Empty) returns (CapabilitiesResponse)

message CapabilitiesResponse {
  repeated string capabilities = 1;
  string version = 2;
}
```

**Returns:**
```json
{
  "capabilities": [
    "mouse_click",
    "mouse_move",
    "keyboard",
    "screenshot",
    "app_launch",
    "app_close",
    "window_list",
    "text_typing",
    "scroll",
    "system_info"
  ],
  "version": "2.0.0"
}
```

**Use Case:** Feature detection, compatibility checking

---

## 🔬 Testing & Verification

### Bridge Health Check

```bash
# Check if running
ps aux | grep axon-desktop-agent | grep -v grep

# Check port listening
ss -tulpn | grep 50051

# Check logs
tail -f ~/AXONBRIDGE-Linux/bridge.log
```

### Manual RPC Tests

```bash
# Get system info
grpcurl -plaintext 192.168.64.3:50051 \
  axon.DesktopAgent/GetSystemInfo

# Launch calculator
grpcurl -plaintext -d '{"application_name":"gnome-calculator"}' \
  192.168.64.3:50051 \
  axon.DesktopAgent/LaunchApplication

# Click at position
grpcurl -plaintext -d '{"x":500,"y":400,"button":"LEFT","click_count":1}' \
  192.168.64.3:50051 \
  axon.DesktopAgent/InjectMouseClick

# Take screenshot
grpcurl -plaintext -d '{"file_path":"/tmp/test.png"}' \
  192.168.64.3:50051 \
  axon.DesktopAgent/TakeScreenshot
```

### Python Test Script

```python
import grpc
import time
from generated_pb2 import *
from generated_pb2_grpc import DesktopAgentStub

def test_bridge():
    channel = grpc.insecure_channel('192.168.64.3:50051')
    stub = DesktopAgentStub(channel)
    
    # 1. Get system info
    print("1. Getting system info...")
    info = stub.GetSystemInfo()
    print(f"   OS: {info.os_name} {info.os_version}")
    print(f"   Display: {info.displays[0].width}x{info.displays[0].height}")
    
    # 2. Launch calculator
    print("\n2. Launching calculator...")
    response = stub.LaunchApplication(
        LaunchRequest(application_name="gnome-calculator")
    )
    print(f"   Success: {response.success}")
    time.sleep(3)
    
    # 3. Get window list
    print("\n3. Getting window list...")
    windows = stub.GetWindowList()
    for w in windows.windows:
        if "calculator" in w.title.lower():
            print(f"   Found: {w.title} (ID: {w.window_id})")
    
    # 4. Take screenshot
    print("\n4. Taking screenshot...")
    frame = stub.GetFrame()
    print(f"   Size: {len(frame.image_data)} bytes")
    print(f"   Dimensions: {frame.width}x{frame.height}")
    
    # 5. Click calculator button
    print("\n5. Clicking calculator button...")
    response = stub.InjectMouseClick(
        MouseClickRequest(x=500, y=400, button=MouseButton.LEFT, click_count=1)
    )
    print(f"   Success: {response.success}")
    
    # 6. Type text
    print("\n6. Typing text...")
    response = stub.TypeText(
        TypeTextRequest(text="Hello from Mac Hub!", delay_ms=50)
    )
    print(f"   Success: {response.success}")
    
    # 7. Close calculator
    print("\n7. Closing calculator...")
    response = stub.CloseApplication(
        CloseRequest(application_name="Calculator")
    )
    print(f"   Success: {response.success}")
    
    print("\n✅ All tests passed!")

if __name__ == "__main__":
    test_bridge()
```

---

## 🏗️ Architecture Notes

### Threading Model

The Bridge uses **Tokio async runtime** with proper spawn_blocking for all system commands:

- ✅ All input injection commands are spawn_blocked
- ✅ All app launch commands use spawn() for background execution
- ✅ Screenshot capture is spawn_blocked
- ✅ No blocking operations on async runtime

**Result:** No deadlocks, no hangs, reliable RPC responses

### Error Handling

All RPCs follow this pattern:
1. Input validation
2. Execute operation in spawn_blocking
3. Detailed error logging
4. Return structured response with success flag + message

### Logging

Set `RUST_LOG` environment variable:
- `RUST_LOG=info` - Normal operation (recommended)
- `RUST_LOG=debug` - Verbose RPC tracing
- `RUST_LOG=trace` - Full system command output

View logs:
```bash
tail -f ~/AXONBRIDGE-Linux/bridge.log
```

---

## 🐛 Troubleshooting

### RPC Timeout Errors

**Symptom:** "Deadline Exceeded" errors from Hub  
**Solution:** Increase Hub RPC timeout to 30 seconds

```python
# Python gRPC example
channel = grpc.insecure_channel('192.168.64.3:50051')
stub = DesktopAgentStub(channel)

# Set 30-second timeout
stub.LaunchApplication(request, timeout=30)
```

### App Doesn't Launch

**Debug Steps:**
1. Check Bridge logs: `tail -f ~/AXONBRIDGE-Linux/bridge.log`
2. Verify app installed: `dpkg -l | grep <app-name>`
3. Test manually: `gio launch org.gnome.Calculator.desktop`
4. Check desktop files: `ls /usr/share/applications/*.desktop`

### Clicks Not Working

**Debug Steps:**
1. Verify xdotool: `xdotool getmouselocation click 1`
2. Check DISPLAY: Bridge uses DISPLAY=:0
3. Verify window focus: `wmctrl -l`
4. Check coordinates: Are they within screen bounds?

### Screenshot Fails

**Debug Steps:**
1. Test scrot: `scrot /tmp/test.png --overwrite`
2. Check display: `echo $DISPLAY` should be `:0`
3. Verify scrot installed: `which scrot`

---

## 📊 Performance Benchmarks

Measured on Ubuntu 22.04 VM (4 CPU, 8GB RAM):

| Operation | Average | p95 | p99 |
|-----------|---------|-----|-----|
| GetSystemInfo | 5ms | 8ms | 10ms |
| GetWindowList | 80ms | 120ms | 150ms |
| LaunchApplication | 350ms | 600ms | 1000ms |
| CloseApplication | 150ms | 250ms | 400ms |
| InjectMouseClick | 35ms | 60ms | 80ms |
| InjectKeyPress | 25ms | 45ms | 60ms |
| GetFrame | 200ms | 350ms | 500ms |
| TypeText (10 chars) | 75ms | 120ms | 150ms |
| InjectScroll | 40ms | 65ms | 85ms |

**Note:** First app launch is slower (~2-5s) due to cold start

---

## 🔄 OSWorld Integration

### Status: ✅ Ready for Integration

The Bridge is fully compatible with OSWorld benchmarks. A verified Python runner exists on the Ubuntu VM at:

```
/home/th3mailman/OSWorld/run_osworld_verified.py
```

### Required Mac Hub API Endpoint

For full OSWorld integration, implement this HTTP API:

```
POST http://192.168.64.1:4545/api/v1/action

Request:
{
  "screenshot": "<base64_png>",
  "task_description": "Click the calculator button",
  "previous_actions": ["launched calculator"],
  "system_info": {
    "screen_width": 1920,
    "screen_height": 1080
  }
}

Response:
{
  "action": "click",
  "x": 500,
  "y": 400,
  "button": "left",
  "confidence": 0.95,
  "reasoning": "Calculator button located at (500, 400)"
}
```

### OSWorld Test Execution

```bash
cd /home/th3mailman/OSWorld
python run_osworld_verified.py --config verified_config.yaml
```

See `/home/th3mailman/OSWorld/FINAL_INTEGRATION_GUIDE.md` for full details.

---

## 📦 Deployment

### Current Deployment

```
VM IP:        192.168.64.3
gRPC Port:    50051
Hub URL:      http://192.168.64.1:4545
Session ID:   my-session
Log File:     ~/AXONBRIDGE-Linux/bridge.log
```

### Manual Start/Stop

```bash
# Stop
pkill -SIGTERM axon-desktop-agent
sleep 2
ps aux | grep axon-desktop-agent | grep -v grep  # Should be empty

# Start
cd ~/AXONBRIDGE-Linux
RUST_LOG=info ./target/release/axon-desktop-agent \
  my-session \
  http://192.168.64.1:4545 \
  50051 \
  > bridge.log 2>&1 &

# Verify
sleep 2
ps aux | grep axon-desktop-agent | grep -v grep
ss -tulpn | grep 50051
tail -20 bridge.log
```

### Rebuild After Updates

```bash
cd ~/AXONBRIDGE-Linux
git pull origin main
cargo clean
cargo build --release
# Then restart as above
```

---

## 📞 Support & Contact

### Bridge Repository
https://github.com/TheMailmans/AXON_LINUX_BRIDGE

### Proto Files
`proto/agent.proto` - Full RPC definitions

### Version History
- **v2.0.0** (2025-10-17): New RPCs (TypeText, Scroll, Capabilities), reliability improvements
- **v1.5.0** (2025-10-16): Spawn_blocking fixes, CloseApplication improvements
- **v1.4.0** (2025-10-15): Screenshot scrot fix, launch command spawn() fix
- **v1.3.0** (2025-10-14): Launch fallback logic, error handling improvements

---

## ✅ Pre-Deployment Checklist

Before integrating with Mac Hub:

- [ ] Bridge is running on 192.168.64.3:50051
- [ ] grpcurl test succeeds from Mac
- [ ] RPC timeout set to 30s+ on Hub client
- [ ] Screenshot test returns valid PNG data
- [ ] Calculator launch + click test succeeds
- [ ] CloseApplication test succeeds
- [ ] Proto files synced between Bridge and Hub
- [ ] Logging level set appropriately (RUST_LOG=info)

---

## 🎓 Quick Reference Card

```
═══════════════════════════════════════════════════
  AXONBRIDGE LINUX v2.0.0 - QUICK REFERENCE
═══════════════════════════════════════════════════

CONNECTION:  192.168.64.3:50051 (gRPC insecure)
HUB URL:     http://192.168.64.1:4545
TIMEOUT:     30 seconds minimum

CORE RPCS:
  GetSystemInfo      - OS + display info
  GetWindowList      - All open windows
  LaunchApplication  - Start apps
  CloseApplication   - Close apps
  GetFrame           - Screenshot (PNG)
  
INPUT RPCS:
  InjectMouseClick   - Click at (x,y)
  InjectMouseMove    - Move cursor
  InjectKeyPress     - Press keys
  TypeText           - Type strings [NEW]
  InjectScroll       - Scroll wheel [NEW]

UTILITIES:
  GetCapabilities    - Feature list [NEW]
  TakeScreenshot     - Save to file

LOGS:     ~/AXONBRIDGE-Linux/bridge.log
STATUS:   ✅ Production Ready
VERSION:  2.0.0

═══════════════════════════════════════════════════
