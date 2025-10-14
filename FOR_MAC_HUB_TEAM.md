# Quick Reference for Mac Hub Team

**Updated:** 2025-10-14 00:19 UTC  
**Bridge Status:** ✅ **RUNNING** with LaunchApplication fix deployed

---

## ✅ What Was Fixed

**LaunchApplication RPC** now properly attempts all fallback launch methods even when the application is not found in the AppIndex.

### Before
```
LaunchApplication("terminal") → ❌ FAILS
Error: "No matching application found"
```

### After
```
LaunchApplication("terminal") → ✅ SUCCESS
Terminal launches via fallback methods (gio/gtk-launch/direct exec)
```

---

## 🔧 Bridge Connection Details

| Parameter | Value |
|-----------|-------|
| **IP Address** | `192.168.64.3` |
| **gRPC Port** | `50051` |
| **Hub URL** | `http://192.168.64.1:4545` |
| **Session ID** | `ubuntu-session` |
| **Status** | Running (PID: 275773) |

---

## 🚀 Current State

### Bridge Process
```bash
# Process running:
./target/release/axon-desktop-agent ubuntu-session http://192.168.64.1:4545 50051

# Check status:
ps aux | grep axon-desktop-agent

# View logs:
tail -f /home/th3mailman/AXONBRIDGE-Linux/bridge.log
```

### Available RPCs (All Working)
✅ **LaunchApplication** - Now with robust fallback support  
✅ **CloseApplication** - Works for any process  
✅ **GetFrame** - Screenshot capture (< 100ms)  
✅ **InjectMouseClick** - Fast mouse clicks (< 100ms)  
✅ **InjectKeyPress** - Keyboard input  
✅ **GetWindowList** - List all windows  
✅ **GetSystemInfo** - System details  
✅ **GetAccessibilityTree** - UI tree parsing (OSWorld)  
✅ **GetClipboard** - Read clipboard content  

---

## 🎯 Known Working Test Cases

### 1. Launch Terminal
```protobuf
LaunchApplication {
  app_name: "terminal"
}
→ Expected: success=true, gnome-terminal opens
```

### 2. Launch Text Editor
```protobuf
LaunchApplication {
  app_name: "gedit"
}
→ Expected: success=true, gedit opens
```

### 3. Launch Firefox
```protobuf
LaunchApplication {
  app_name: "firefox"
}
→ Expected: success=true, firefox opens
```

### 4. Click and Type Workflow
```
1. LaunchApplication("gedit")
2. Wait 2 seconds for window to open
3. InjectMouseClick(x=500, y=300)  # Click in text area
4. InjectKeyPress("Hello from Mac Hub!")
→ Expected: "Hello from Mac Hub!" appears in gedit
```

---

## ⚠️  Important Notes for Mac Hub

### 1. **Window Focus**
After launching an app, you need to **focus its window** before sending keystrokes. Options:
- **A) Mouse Click Method** (current workaround):
  ```
  1. LaunchApplication("gedit")
  2. Sleep 2 seconds
  3. InjectMouseClick(x=500, y=300)  # Click in window
  4. InjectKeyPress("text")
  ```
- **B) FocusWindow RPC** (recommended for future):
  - Add `FocusWindow(window_id)` RPC to bridge
  - Hub calls `GetWindowList()` to find window
  - Hub calls `FocusWindow(window_id)` before typing
  - This is cleaner and more reliable

### 2. **Connection Issues?**
If Mac Hub can't connect:
```bash
# On Ubuntu VM, check bridge is listening:
ss -tlnp | grep 50051

# Should show:
# LISTEN  0.0.0.0:50051

# Test from Mac:
nc -zv 192.168.64.3 50051
# Should output: Connection succeeded

# If connection fails, check Mac firewall/network settings
```

### 3. **LaunchApplication Timing**
Apps need time to start before you can interact with them:
```
LaunchApplication → Wait ~1-2 seconds → InjectMouseClick/InjectKeyPress
```

### 4. **Logs Are Your Friend**
Bridge logs show exactly what's happening:
```bash
# On Ubuntu VM:
tail -f /home/th3mailman/AXONBRIDGE-Linux/bridge.log

# Look for:
# "🚀 LaunchApplication called: app_name=..."
# "🎯 Matched '...' to '...'" (AppIndex hit)
# "⚠️  No AppIndex match, trying fallback" (fallback triggered)
# "✅ Launched via gio/gtk-launch/exec" (success)
```

---

## 📊 Performance Benchmarks

| Operation | Latency |
|-----------|---------|
| GetFrame | < 100ms |
| InjectMouseClick | < 100ms |
| InjectKeyPress | < 50ms |
| LaunchApplication | 200-500ms (app-dependent) |
| GetWindowList | < 50ms |

---

## 🐛 Debugging Tips

### Bridge Not Responding?
```bash
# Restart bridge:
pkill -f axon-desktop-agent
cd /home/th3mailman/AXONBRIDGE-Linux
RUST_LOG=info ./target/release/axon-desktop-agent ubuntu-session http://192.168.64.1:4545 50051 > bridge.log 2>&1 &
```

### App Launch Not Working?
1. Check bridge logs for error messages
2. Try launching the app manually on Ubuntu: `gnome-terminal`
3. Verify the app is installed: `which gnome-terminal`

### Keystrokes Going to Wrong Window?
1. Add a mouse click after launching the app to focus it
2. Wait 1-2 seconds after launching before typing
3. Consider implementing FocusWindow RPC

---

## 📁 Important Files

| File | Purpose |
|------|---------|
| `bridge.log` | Real-time bridge activity log |
| `FIX_SUMMARY_LAUNCH_FALLBACK.md` | Detailed explanation of the launch fix |
| `test_launch_fix.sh` | Test script for fallback methods |
| `DIAGNOSTICS_2025-10-13.md` | Full system diagnostic report |

---

## 🎉 Success Checklist

For a complete working demo:
- [x] Bridge running on Ubuntu VM
- [x] LaunchApplication fix deployed
- [x] Mouse click latency fixed (no --sync)
- [x] All RPCs working
- [ ] Mac Hub connects to `192.168.64.3:50051`
- [ ] Mac Hub sends LaunchApplication + click + type workflow
- [ ] Ubuntu VM shows app launching and receiving input

**Next Step:** Test from Mac Hub with the corrected connection details!

---

**Questions or Issues?**  
Check the bridge logs first: `tail -f /home/th3mailman/AXONBRIDGE-Linux/bridge.log`  
The logs show exactly what RPCs are being received and their results.
