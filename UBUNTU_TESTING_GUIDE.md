# Ubuntu Bridge Testing Guide

**Purpose**: Verify AXONBRIDGE-Linux works on Ubuntu 22.04/24.04
**Status**: Ready for testing
**Estimated Time**: 4 hours

---

## 📋 12-POINT TESTING CHECKLIST

### Prerequisites

**Ubuntu System Requirements**:
- Ubuntu 22.04 LTS or 24.04 LTS
- X11 desktop environment (not Wayland)
- sudo access for installation

**Install Dependencies**:
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install build dependencies
sudo apt install -y \
    build-essential \
    pkg-config \
    libdbus-1-dev \
    libx11-dev \
    libxi-dev \
    libxtst-dev \
    libssl-dev \
    libgtk-3-dev \
    libappindicator3-dev \
    xdotool \
    scrot \
    gnome-screenshot \
    imagemagick

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

## ✅ TEST 1: Compilation

**Test**: Verify Bridge compiles on Ubuntu

```bash
cd /path/to/AXONBRIDGE-Linux

# Build Bridge
cargo build --release

# Expected output
# Compiling...
# Finished `release` profile [optimized] target(s) in XX.XXs
```

**Success Criteria**:
- [ ] Compiles without errors
- [ ] Binary created at `target/release/axonbridge-linux`
- [ ] Binary size reasonable (10-50MB)

---

## ✅ TEST 2: System Tray Icon

**Test**: Verify system tray icon appears

```bash
# Start Bridge
./target/release/axonbridge-linux

# Expected:
# - Bridge starts
# - System tray icon appears in top panel
# - Icon shows "Idle" state (gray/blue)
```

**Success Criteria**:
- [ ] System tray icon visible
- [ ] Icon shows correct state
- [ ] Right-click menu works
- [ ] Menu shows options: Status, Emergency Unlock, Quit

**Troubleshooting**:
- If tray icon doesn't appear, check if libappindicator3 is installed
- Try: `sudo apt install libappindicator3-1`

---

## ✅ TEST 3: Input Locking

**Test**: Verify input lock works correctly

```bash
# Method 1: Via tray icon (when implemented)
# Right-click tray icon → Lock Inputs

# Method 2: Via Core GUI pairing
# (Requires Core running and connected)
```

**Success Criteria**:
- [ ] Keyboard inputs blocked
- [ ] Mouse inputs blocked
- [ ] Can still see screen (not blackedout)
- [ ] Tray icon changes to "AI Control" state (red)
- [ ] xdotool shows inputs disabled:
  ```bash
  xinput list | grep "disabled"
  ```

**Testing Manual**:
```bash
# While locked:
# - Try typing → Should be blocked
# - Try clicking → Should be blocked
# - Try Ctrl+C → Should be blocked
# - Try Alt+Tab → Should be blocked
```

---

## ✅ TEST 4: Screenshot Capture

**Test**: Verify all 3 screenshot methods work

```bash
# Test method 1: scrot
scrot /tmp/test_scrot.png
ls -lh /tmp/test_scrot.png

# Test method 2: gnome-screenshot
gnome-screenshot -f /tmp/test_gnome.png
ls -lh /tmp/test_gnome.png

# Test method 3: ImageMagick
import -window root /tmp/test_im.png
ls -lh /tmp/test_im.png

# Run Bridge screenshot test
cargo test screenshot_tests --release -- --nocapture
```

**Success Criteria**:
- [ ] At least one method works
- [ ] Screenshot files are valid PNG
- [ ] Screenshot shows full desktop
- [ ] File size reasonable (500KB - 5MB)
- [ ] Bridge test passes

---

## ✅ TEST 5: gRPC Connection

**Test**: Verify gRPC server accepts connections

```bash
# Terminal 1: Start Bridge
./target/release/axonbridge-linux

# Terminal 2: Test connection with grpcurl
# Install grpcurl first
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest

# List services
grpcurl -plaintext localhost:50051 list

# Expected output:
# axon.agent.DesktopAgent
# grpc.reflection.v1alpha.ServerReflection

# Test RegisterAgent
grpcurl -plaintext -d '{"session_id": "test-session"}' \
    localhost:50051 \
    axon.agent.DesktopAgent/RegisterAgent

# Expected:
# {
#   "agent_id": "test-session",
#   "status": "connected",
#   "system_info": {...}
# }
```

**Success Criteria**:
- [ ] gRPC server listening on 50051
- [ ] RegisterAgent works
- [ ] Returns valid system info
- [ ] No connection errors

---

## ✅ TEST 6: Control Handoff

**Test**: Verify AI/Human control transitions

**Setup**:
- Start Bridge
- Connect Core (via pairing or direct gRPC)

**Test Sequence**:
```
1. Initial state: Idle (inputs unlocked)
2. AI requests control → AI Control (inputs locked)
3. Human requests control → Human Control (inputs unlocked)
4. Return to AI → AI Control (inputs locked)
5. Task complete → Idle (inputs unlocked)
```

**Success Criteria**:
- [ ] All transitions work smoothly
- [ ] Tray icon updates for each state
- [ ] Input lock state matches control mode
- [ ] No race conditions
- [ ] Notifications shown for state changes

---

## ✅ TEST 7: Emergency Unlock

**Test**: Verify emergency unlock mechanisms work

**Method 1: Emergency Hotkey**
```bash
# While inputs are locked:
# Press: Ctrl+Alt+Shift+U

# Expected:
# - Inputs immediately unlocked
# - Tray icon returns to Idle
# - Notification: "Emergency unlock activated"
```

**Method 2: Tray Menu**
```bash
# While inputs are locked:
# Right-click tray icon → Emergency Unlock

# Expected:
# - Inputs immediately unlocked
# - Returns to Idle state
```

**Success Criteria**:
- [ ] Hotkey works (Ctrl+Alt+Shift+U)
- [ ] Tray menu option works
- [ ] Unlock happens immediately (<100ms)
- [ ] User regains control
- [ ] No residual lock state

---

## ✅ TEST 8: Watchdog Timeout

**Test**: Verify auto-unlock after timeout

```bash
# Lock inputs (via Core or test command)
# Wait 15 minutes (default timeout)

# Expected:
# - After 15 minutes, inputs auto-unlock
# - Notification: "Input lock timeout - auto-unlocked"
# - Tray returns to Idle
```

**Success Criteria**:
- [ ] Timeout triggers correctly (15 min)
- [ ] Auto-unlock executes
- [ ] Notification shown
- [ ] State resets to Idle
- [ ] No crashes or panics

---

## ✅ TEST 9: Resource Usage

**Test**: Verify Bridge doesn't consume excessive resources

```bash
# Start Bridge
./target/release/axonbridge-linux &

# Monitor resources
# Terminal 1: CPU usage
top -p $(pgrep axonbridge-linux)

# Terminal 2: Memory usage
ps aux | grep axonbridge-linux

# Let run for 30 minutes, check periodically
```

**Success Criteria**:
- [ ] CPU usage < 5% (idle)
- [ ] CPU usage < 20% (active)
- [ ] Memory usage < 100MB
- [ ] No memory leaks over time
- [ ] No CPU spikes

**Benchmarks**:
```
Idle state: ~1-2% CPU, ~30-50MB RAM
AI Control: ~5-10% CPU, ~50-80MB RAM
Screenshot capture: Brief spike to 20-30% CPU
```

---

## ✅ TEST 10: Multi-Desktop Support

**Test**: Verify works across different Ubuntu desktops

**Desktops to Test**:
1. **Ubuntu GNOME** (default)
2. **Ubuntu with X11** (not Wayland)
3. **Xubuntu** (XFCE)
4. **Kubuntu** (KDE Plasma)

**Test on Each**:
```bash
# Check desktop environment
echo $XDG_CURRENT_DESKTOP

# Run Bridge
./target/release/axonbridge-linux

# Verify:
# - Tray icon appears
# - Input lock works
# - Screenshot works
# - No crashes
```

**Success Criteria**:
- [ ] Works on Ubuntu GNOME
- [ ] Works on Ubuntu X11
- [ ] Works on Xubuntu (bonus)
- [ ] Works on Kubuntu (bonus)
- [ ] Screenshot method selection adapts

---

## ✅ TEST 11: Network Resilience

**Test**: Verify handles network issues gracefully

**Test Scenarios**:

**Scenario 1: Core Disconnects**
```bash
# 1. Connect Core to Bridge
# 2. Stop Core (Ctrl+C)
# 3. Observe Bridge behavior

# Expected:
# - Bridge remains running
# - Tray shows "Disconnected"
# - Inputs auto-unlock if locked
# - No crash
# - Reconnects when Core restarts
```

**Scenario 2: Network Interruption**
```bash
# 1. Connect Core to Bridge
# 2. Disable network: sudo ifconfig eth0 down
# 3. Wait 30 seconds
# 4. Re-enable: sudo ifconfig eth0 up

# Expected:
# - Bridge detects disconnection
# - Auto-unlocks inputs
# - Reconnects when network restored
# - No data loss
```

**Success Criteria**:
- [ ] Handles Core disconnect gracefully
- [ ] Auto-unlocks on disconnect (safety)
- [ ] Reconnects automatically
- [ ] No crashes or panics
- [ ] State recovery works

---

## ✅ TEST 12: Error Handling

**Test**: Verify error handling is robust

**Error Scenarios to Test**:

**1. Missing Screenshot Tool**
```bash
# Temporarily rename scrot
sudo mv /usr/bin/scrot /usr/bin/scrot.bak

# Request screenshot from Core
# Expected: Falls back to gnome-screenshot or ImageMagick
```

**2. Invalid gRPC Request**
```bash
# Send malformed request
grpcurl -plaintext -d '{"invalid": "data"}' \
    localhost:50051 \
    axon.agent.DesktopAgent/RegisterAgent

# Expected: Proper error response, no crash
```

**3. Input Lock Failure**
```bash
# Simulate lock failure (remove xinput)
sudo mv /usr/bin/xinput /usr/bin/xinput.bak

# Try to lock inputs
# Expected: Error logged, graceful failure, notification
```

**Success Criteria**:
- [ ] Errors logged clearly
- [ ] No panics or crashes
- [ ] Fallback mechanisms work
- [ ] User notified of issues
- [ ] Bridge remains operational

---

## 📊 FINAL VERIFICATION

### All Tests Passing Checklist

- [ ] **Test 1**: Compilation ✅
- [ ] **Test 2**: System Tray ✅
- [ ] **Test 3**: Input Locking ✅
- [ ] **Test 4**: Screenshot Capture ✅
- [ ] **Test 5**: gRPC Connection ✅
- [ ] **Test 6**: Control Handoff ✅
- [ ] **Test 7**: Emergency Unlock ✅
- [ ] **Test 8**: Watchdog Timeout ✅
- [ ] **Test 9**: Resource Usage ✅
- [ ] **Test 10**: Multi-Desktop Support ✅
- [ ] **Test 11**: Network Resilience ✅
- [ ] **Test 12**: Error Handling ✅

### Performance Benchmarks

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Idle CPU | < 5% | ___% | [ ] |
| Active CPU | < 20% | ___% | [ ] |
| Memory | < 100MB | ___MB | [ ] |
| Startup Time | < 3s | ___s | [ ] |
| Lock/Unlock | < 100ms | ___ms | [ ] |
| Screenshot | < 500ms | ___ms | [ ] |

---

## 🐛 ISSUE TRACKING

### Issues Found During Testing

| # | Issue | Severity | Status | Resolution |
|---|-------|----------|--------|------------|
| 1 | | Critical/Major/Minor | Open/Fixed | |
| 2 | | | | |
| 3 | | | | |

---

## 📝 TEST REPORT TEMPLATE

```markdown
# Ubuntu Bridge Test Report

**Date**: YYYY-MM-DD
**Tester**: [Name]
**System**: Ubuntu XX.XX
**Desktop**: GNOME/KDE/XFCE
**Bridge Version**: v1.0.0

## Summary
- Tests Passed: X/12
- Tests Failed: X/12
- Critical Issues: X
- Minor Issues: X

## Test Results
1. Compilation: PASS/FAIL
2. System Tray: PASS/FAIL
3. Input Locking: PASS/FAIL
4. Screenshot Capture: PASS/FAIL
5. gRPC Connection: PASS/FAIL
6. Control Handoff: PASS/FAIL
7. Emergency Unlock: PASS/FAIL
8. Watchdog Timeout: PASS/FAIL
9. Resource Usage: PASS/FAIL
10. Multi-Desktop: PASS/FAIL
11. Network Resilience: PASS/FAIL
12. Error Handling: PASS/FAIL

## Performance Metrics
- Idle CPU: ____%
- Active CPU: ____%
- Memory: ____MB
- Startup: ____s

## Issues Found
[List any issues]

## Recommendations
[Any recommendations for improvements]

## Sign-Off
- [ ] All critical tests passing
- [ ] Performance acceptable
- [ ] Ready for production

**Approved By**: [Name]
**Date**: YYYY-MM-DD
```

---

## 🚀 DEPLOYMENT PACKAGE CREATION

After all tests pass, create deployment package:

```bash
# Build release binary
cargo build --release

# Create deployment directory
mkdir -p axonhub-bridge-linux-v1.0.0
cd axonhub-bridge-linux-v1.0.0

# Copy binary
cp ../target/release/axonbridge-linux ./

# Copy documentation
cp ../README.md ./
cp ../LICENSE ./

# Create install script
cat > install.sh << 'EOF'
#!/bin/bash
set -e

echo "Installing AXONBRIDGE-Linux..."

# Check for X11
if [ "$XDG_SESSION_TYPE" != "x11" ]; then
    echo "⚠️  Warning: X11 required (Wayland not supported)"
    echo "   Switch to X11 session before running Bridge"
fi

# Install dependencies
sudo apt update
sudo apt install -y \
    libdbus-1-3 \
    libx11-6 \
    libxi6 \
    libxtst6 \
    libgtk-3-0 \
    libappindicator3-1 \
    xdotool \
    scrot

# Install binary
sudo cp axonbridge-linux /usr/local/bin/
sudo chmod +x /usr/local/bin/axonbridge-linux

# Create systemd service
sudo tee /etc/systemd/system/axonbridge.service > /dev/null << 'SERVICE'
[Unit]
Description=AxonHub Linux Bridge
After=network.target graphical.target

[Service]
Type=simple
User=$USER
ExecStart=/usr/local/bin/axonbridge-linux
Restart=on-failure
RestartSec=5s

[Install]
WantedBy=graphical.target
SERVICE

echo "✅ Installation complete!"
echo ""
echo "To start Bridge:"
echo "  sudo systemctl enable axonbridge"
echo "  sudo systemctl start axonbridge"
echo ""
echo "To check status:"
echo "  sudo systemctl status axonbridge"
EOF

chmod +x install.sh

# Create tarball
cd ..
tar -czf axonhub-bridge-linux-v1.0.0.tar.gz axonhub-bridge-linux-v1.0.0/

echo "✅ Deployment package created: axonhub-bridge-linux-v1.0.0.tar.gz"
```

---

## ✅ SUCCESS CRITERIA

**Bridge is production-ready when:**

### Critical
- [x] Screenshot implementation complete (3 fallback methods)
- [ ] All 12 tests passing
- [ ] Performance within targets
- [ ] No critical bugs
- [ ] Error handling robust

### Important
- [ ] Documentation complete
- [ ] Deployment package created
- [ ] Install script tested
- [ ] Multi-desktop support verified

### Quality
- [ ] Code reviewed
- [ ] Security verified
- [ ] Resource usage optimized
- [ ] User experience smooth

---

**Testing Status**: Ready to begin
**Prerequisites**: Ubuntu VM with X11
**Estimated Time**: 4 hours
**Next Step**: Execute 12-point checklist on Ubuntu system
