# CloseApplication RPC Documentation

**Status:** ✅ Implemented  
**Date Added:** October 11, 2024  
**Platforms:** Linux (wmctrl), macOS (AppleScript), Windows (future)

---

## Overview

The `CloseApplication` RPC allows the Hub to gracefully close application windows on the desktop. This complements the `LaunchApplication` RPC to provide complete lifecycle management for desktop applications.

---

## Proto Definition

```protobuf
service DesktopAgent {
  // ... other methods ...
  
  // Application control
  rpc LaunchApplication(LaunchApplicationRequest) returns (LaunchApplicationResponse);
  rpc CloseApplication(CloseApplicationRequest) returns (CloseApplicationResponse);
}

message CloseApplicationRequest {
  string agent_id = 1;
  string app_name = 2;  // Window title or app name to close
}

message CloseApplicationResponse {
  bool success = 1;
  string error = 2;
}
```

---

## Platform Implementations

### **Linux (wmctrl)**

Uses `wmctrl -c` to close windows by title match:

```rust
Command::new("wmctrl")
    .arg("-c")
    .arg(&req.app_name)
    .output()
```

**Requirements:**
- `wmctrl` must be installed (`sudo apt install wmctrl`)
- X11 environment (Wayland not supported)

**How it works:**
- `wmctrl -c` matches window titles (partial match)
- Sends close event to window manager
- Window closes gracefully (app can save state)

**Example:**
```
app_name: "Calculator"
→ Closes any window with "Calculator" in title
→ "axonhub Calculator" matches
→ "GNOME Calculator" matches
```

### **macOS (AppleScript)**

Uses AppleScript to quit applications gracefully:

```rust
let script = format!("tell application \"{}\" to quit", req.app_name);
Command::new("osascript")
    .arg("-e")
    .arg(&script)
    .output()
```

**Requirements:**
- macOS 10.5+
- Application must respond to quit events

**How it works:**
- Sends quit event via AppleScript
- Application quits gracefully
- Allows app to save state and close cleanly

**Example:**
```
app_name: "Calculator"
→ Runs: tell application "Calculator" to quit
→ Calculator saves state and quits
```

### **Windows (Future Implementation)**

**Planned approach:**
```rust
// Use Windows API to find and close window
use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, SendMessageW, WM_CLOSE};

let hwnd = FindWindowW(None, window_title);
SendMessageW(hwnd, WM_CLOSE, None, None);
```

**Requirements:**
- Windows 7+
- `windows` crate for Win32 API bindings

---

## Usage from Hub

### **Hub Client Method:**

```rust
// In axon-hub/src/bridge/client.rs
pub async fn close_window(&mut self, title: &str) -> Result<()> {
    let agent_id = self.agent_id.clone()
        .context("Not connected")?;
    
    let response = self.client.close_application(CloseApplicationRequest {
        agent_id,
        app_name: title.to_string(),
    }).await?;
    
    let result = response.into_inner();
    if result.success {
        log::info!("[Bridge] Successfully closed window: {}", title);
        Ok(())
    } else {
        log::warn!("[Bridge] Failed to close: {}", result.error);
        Ok(()) // Don't fail cleanup
    }
}
```

### **Universal Cleanup System:**

```rust
// In orchestrator/task.rs
// At end of task:
let initial_windows = /* captured at start */;
let final_windows = self.bridge.get_window_list().await?;

// Close windows that were opened during task
for window in &final_windows {
    if !initial_windows.contains(window) {
        log::info!("🧹 Closing new window: {}", window);
        self.bridge.close_window(window).await?;
    }
}
```

---

## Error Handling

### **Success Response:**

```json
{
  "success": true,
  "error": ""
}
```

### **Failure Response:**

```json
{
  "success": false,
  "error": "wmctrl failed to close window: No such window"
}
```

### **Common Errors:**

| Error | Cause | Solution |
|-------|-------|----------|
| "Failed to execute wmctrl" | wmctrl not installed | `sudo apt install wmctrl` |
| "No such window" | Window title doesn't match | Check exact window title |
| "Window not found" | Window already closed | Ignore (not an error) |

---

## Testing

### **Manual Test (Linux):**

```bash
# Start Bridge
cd desktop-agent
cargo run --release

# In another terminal:
grpcurl -plaintext \
  -d '{
    "agent_id": "test-agent",
    "app_name": "Calculator"
  }' \
  localhost:50051 \
  axon.agent.DesktopAgent/CloseApplication
```

### **Integration Test:**

```rust
#[tokio::test]
async fn test_close_application() {
    let mut client = BridgeClient::new("localhost:50051".to_string()).await.unwrap();
    client.register().await.unwrap();
    
    // Launch calculator
    client.launch_application("calculator".to_string()).await.unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Close calculator
    let result = client.close_window("Calculator").await;
    assert!(result.is_ok());
}
```

---

## Logs

### **Successful Close:**

```
[INFO] CloseApplication called: app_name=Calculator
[INFO] Closing window by title: Calculator
[INFO] Successfully closed window: Calculator
```

### **Failed Close:**

```
[INFO] CloseApplication called: app_name=NonExistent
[INFO] Closing window by title: NonExistent
[ERROR] wmctrl failed to close window: Cannot find window "NonExistent"
```

---

## Future Enhancements

### **Planned Features:**

1. **Force Close Option**
   ```protobuf
   message CloseApplicationRequest {
     string agent_id = 1;
     string app_name = 2;
     bool force = 3;  // Kill process if graceful close fails
   }
   ```

2. **Close by PID**
   ```protobuf
   message CloseApplicationRequest {
     string agent_id = 1;
     oneof target {
       string app_name = 2;
       int32 pid = 3;
     }
   }
   ```

3. **Batch Close**
   ```protobuf
   message CloseApplicationsRequest {
     string agent_id = 1;
     repeated string app_names = 2;
   }
   ```

---

## Platform Comparison

| Platform | Method | Graceful | Requires Tool | Partial Match |
|----------|--------|----------|---------------|---------------|
| **Linux** | wmctrl | ✅ Yes | wmctrl | ✅ Yes |
| **macOS** | AppleScript | ✅ Yes | Built-in | ❌ No (exact name) |
| **Windows** | Win32 API | ✅ Yes | windows crate | ✅ Yes |

---

## Production Considerations

### **Reliability:**
- Always check `success` field in response
- Don't fail task if cleanup fails
- Log warnings for failed closes
- Implement timeout (wmctrl hangs if X11 issues)

### **Security:**
- Validate window titles (no shell injection)
- Limit which applications can be closed
- Audit close operations
- Rate limit close requests

### **Performance:**
- Close operations take 100-300ms
- Batch closes when possible
- Don't block on close failures
- Use async/await properly

---

## Related RPCs

- **LaunchApplication** - Launch applications
- **GetWindowList** - List open windows
- **GetProcessList** - List running processes
- **InjectKeyPress** - Alternative close via Alt+F4 (not recommended)

---

**Implemented by:** Tyler Mailman  
**Last Updated:** October 11, 2024  
**Version:** 1.0
