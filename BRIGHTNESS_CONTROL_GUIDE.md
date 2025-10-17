# Brightness Control Guide

## Overview

The brightness control module provides unified APIs for getting and setting display brightness across Linux, macOS, and Windows platforms.

## Quick Start

### Getting Current Brightness

```bash
# Via gRPC
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/GetBrightness
```

**Response:**
```json
{
  "level": 0.75,
  "method_used": "command",
  "timestamp": 1234567890000
}
```

### Setting Brightness

```bash
# Set to 50% brightness
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1", "level": 0.5}' \
  localhost:50051 axonbridge.DesktopAgent/SetBrightness
```

**Response:**
```json
{
  "success": true,
  "actual_level": 0.5,
  "method_used": "command",
  "timestamp": 1234567890000
}
```

## API Reference

### GetBrightness RPC

**Request:**
```protobuf
message GetBrightnessRequest {
  string agent_id = 1;
}
```

**Response:**
```protobuf
message GetBrightnessResponse {
  float level = 1;              // 0.0 (darkest) to 1.0 (brightest)
  string method_used = 2;       // "command" or "keyboard"
  int64 timestamp = 3;          // milliseconds since epoch
}
```

**Returns:**
- **level:** Current brightness as float from 0.0 to 1.0
- **method_used:** How the brightness was obtained (command-line tool or keyboard)
- **timestamp:** When the operation completed

**Errors:**
- `UNAVAILABLE (503)` - Brightness control not available
- `INTERNAL (500)` - Failed to read brightness

### SetBrightness RPC

**Request:**
```protobuf
message SetBrightnessRequest {
  string agent_id = 1;
  float level = 2;              // 0.0 to 1.0
}
```

**Response:**
```protobuf
message SetBrightnessResponse {
  bool success = 1;
  float actual_level = 2;       // Level after setting
  string method_used = 3;       // "command" or "keyboard"
  optional string error = 4;
  int64 timestamp = 5;
}
```

**Parameters:**
- **level:** Desired brightness from 0.0 (off) to 1.0 (full brightness)

**Returns:**
- **success:** True if brightness was set successfully
- **actual_level:** Brightness level after the operation
- **method_used:** "command" (tool-based) or "keyboard" (key simulation)
- **error:** Error message if operation failed
- **timestamp:** When the operation completed

**Validation:**
- level must be between 0.0 and 1.0 (inclusive)
- Returns `INVALID_ARGUMENT (400)` if level is out of range

**Errors:**
- `INVALID_ARGUMENT (400)` - Level outside 0.0-1.0 range
- `UNAVAILABLE (503)` - Brightness control not available
- `INTERNAL (500)` - Failed to set brightness

## Platform-Specific Details

### Linux

#### Primary Method: brightnessctl

**Requirements:**
```bash
# Install on Ubuntu/Debian
sudo apt-get install brightnessctl

# Install on Fedora/RHEL
sudo dnf install brightnessctl

# Install on Arch
sudo pacman -S brightnessctl
```

**How it works:**
- Reads from `/sys/class/backlight/`
- Supports multiple backlight devices
- Returns brightness as percentage (0-100)

**Example:**
```bash
# Read brightness
brightnessctl get

# Set brightness to 50%
brightnessctl set 50%

# Get max brightness
brightnessctl max
```

#### Fallback Method: xbacklight

**Requirements:**
```bash
# Install on Ubuntu/Debian
sudo apt-get install xbacklight

# Install on Fedora
sudo dnf install xbacklight
```

**How it works:**
- Uses Intel or AMD backlight driver
- Works via X11 (not Wayland)
- Returns brightness as percentage (0-100)

**Example:**
```bash
# Read brightness
xbacklight -get

# Set brightness to 75%
xbacklight -set 75

# Increase by 10%
xbacklight -inc 10
```

**Note:** If brightnessctl fails, framework automatically falls back to xbacklight.

#### Wayland Considerations

On Wayland systems:
- xbacklight may not work (no X11 support)
- brightnessctl should work (uses sysfs directly)
- Keyboard brightness keys (if available) may work as fallback

### macOS

#### Primary Method: osascript

**Requirements:**
- Built-in to macOS (no installation needed)
- Requires display access permissions

**How it works:**
- Uses System Events (AppleScript)
- Sets brightness in "display preferences"
- Range: 0 to 1 (internally normalized to 0-100%)

**Example:**
```bash
# Set brightness to 75%
osascript -e "tell application \"System Events\" to set brightness of (displays) to 75"

# Get brightness
osascript -e "tell application \"System Events\" to get brightness of (displays)"
```

#### Fallback Method: Keyboard Simulation

Uses key codes for brightness control:
- `key code 144` - Increase brightness
- `key code 145` - Decrease brightness

**Limitations:**
- Brightness adjusts by 6.25% per press
- Less precise than direct setting
- Respects system keyboard preferences

### Windows

#### Primary Method: PowerShell/WMI

**Requirements:**
- PowerShell 5.0+ (built-in on Windows 10+)
- WMI support (standard on all Windows versions)

**How it works:**
- Uses `Get-WmiObject Win32_MonitorBrightness`
- Works via Windows Management Instrumentation
- Requires appropriate driver support

**Limitations:**
- May not work with all display drivers
- Requires WMI to be enabled

#### Fallback Method: Virtual Key Codes

Uses `VK_MEDIA_*` constants:
- `VK_MEDIA_PLAY_PAUSE` for brightness control simulation

**Limitations:**
- Less reliable than direct WMI
- Depends on driver support

## Advanced Usage

### Brightness Profiles

You can create brightness profiles for different scenarios:

```rust
// Example: Create morning, afternoon, evening profiles
const PROFILES: &[(f32, &str)] = &[
    (0.3, "night"),
    (0.7, "morning"),
    (1.0, "afternoon"),
    (0.4, "evening"),
];

for (level, name) in PROFILES {
    set_brightness(*level)?;
    println!("Brightness set to {} ({})", name, level);
}
```

### Automatic Brightness Adjustment

```rust
// Gradually increase brightness
async fn gradually_increase() -> Result<()> {
    for i in 0..10 {
        let level = i as f32 / 10.0;
        set_brightness(level)?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Ok(())
}
```

### Brightness Polling

```rust
// Monitor brightness changes
async fn monitor_brightness() -> Result<()> {
    loop {
        let brightness = get_brightness()?;
        println!("Current brightness: {}", brightness);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

## Troubleshooting

### "Brightness control not available"

**Linux Solutions:**
1. Check if brightnessctl is installed:
   ```bash
   which brightnessctl
   # or
   brightnessctl get
   ```

2. If not installed, install it:
   ```bash
   sudo apt-get install brightnessctl
   ```

3. Check if backlight device exists:
   ```bash
   ls /sys/class/backlight/
   ```

4. Try xbacklight:
   ```bash
   xbacklight -get
   ```

**macOS Solutions:**
1. Check System Events access:
   ```bash
   osascript -e "tell application \"System Events\" to get brightness of (displays)"
   ```

2. Grant display permissions if needed:
   - System Preferences > Security & Privacy > Accessibility
   - Ensure terminal has access

**Windows Solutions:**
1. Check PowerShell version:
   ```powershell
   $PSVersionTable.PSVersion
   ```

2. Verify WMI is available:
   ```powershell
   Get-WmiObject Win32_MonitorBrightness
   ```

3. Update display drivers

### Brightness not changing

**Linux:**
1. Check current brightness:
   ```bash
   brightnessctl get
   ```

2. Verify you can change it manually:
   ```bash
   brightnessctl set 50%
   ```

3. Check for driver issues:
   ```bash
   dmesg | grep brightness
   ```

**macOS:**
1. Try manual change via System Preferences
2. Check if System Events access is granted
3. Restart Finder if necessary

**Windows:**
1. Try Manual brightness change via keyboard
2. Update video drivers
3. Check if monitor supports DDC-CI

### Wayland (Linux) Issues

If using Wayland on Linux:
1. brightnessctl should work (uses sysfs)
2. xbacklight will NOT work (needs X11)
3. Keyboard brightness keys might work (driver-dependent)

Solution: Upgrade system to latest Linux/driver versions

## Performance Notes

### Latency

- **brightnessctl:** ~50-100ms
- **xbacklight:** ~100-200ms
- **osascript:** ~100-150ms
- **PowerShell/WMI:** ~200-300ms
- **Keyboard fallback:** <50ms (less reliable)

### Optimization Tips

1. **Batch operations:** Set multiple brightness levels in quick succession
2. **Cache brightness:** Don't poll repeatedly (use 5-10 second intervals)
3. **Use keyboard fallback:** If command-line is slow, fallback is faster
4. **Monitor system load:** Brightness control has minimal impact

## Integration Examples

### Python Client

```python
import grpc
from axonbridge.agent_pb2 import GetBrightnessRequest
from axonbridge.agent_pb2_grpc import DesktopAgentStub

# Connect to bridge
channel = grpc.insecure_channel('localhost:50051')
stub = DesktopAgentStub(channel)

# Get brightness
request = GetBrightnessRequest(agent_id='desktop-1')
response = stub.GetBrightness(request)
print(f"Current brightness: {response.level}")

# Set brightness
from axonbridge.agent_pb2 import SetBrightnessRequest
request = SetBrightnessRequest(agent_id='desktop-1', level=0.5)
response = stub.SetBrightness(request)
print(f"Brightness set to: {response.actual_level}")
```

### Go Client

```go
import "github.com/axonbridge/client"

// Connect to bridge
conn, err := grpc.Dial("localhost:50051")
defer conn.Close()
stub := axonbridge.NewDesktopAgentClient(conn)

// Get brightness
resp, err := stub.GetBrightness(context.Background(), 
    &axonbridge.GetBrightnessRequest{AgentId: "desktop-1"})
log.Printf("Brightness: %v", resp.Level)

// Set brightness
resp2, err := stub.SetBrightness(context.Background(),
    &axonbridge.SetBrightnessRequest{AgentId: "desktop-1", Level: 0.5})
log.Printf("Set to: %v", resp2.ActualLevel)
```

## Related Documentation

- [Volume Control Guide](VOLUME_CONTROL_GUIDE.md)
- [Media Control Guide](MEDIA_CONTROL_GUIDE.md)
- [System Control Architecture](SYSTEM_CONTROL_ARCHITECTURE.md)
- [Bridge Connection Info](BRIDGE_CONNECTION_INFO.txt)

## Support & Issues

For issues or feature requests:
1. Check troubleshooting section above
2. Verify platform-specific requirements
3. Check system logs (`dmesg` on Linux, `log show` on macOS)
4. Report with OS version and error message

## Version History

**v3.1.0** (Current)
- Cross-platform brightness control
- Hybrid execution (command + keyboard)
- 11 unit tests
- gRPC GetBrightness and SetBrightness RPCs
- Linux: brightnessctl + xbacklight
- macOS: osascript + keyboard
- Windows: PowerShell/WMI + virtual keys
