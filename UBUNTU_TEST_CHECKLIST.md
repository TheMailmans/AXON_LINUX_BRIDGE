# Ubuntu Integration Test Checklist
## AXONBRIDGE-Linux Phase 2 Testing

**Version**: 3.0  
**Date Prepared**: October 26, 2025  
**Status**: READY FOR BRIDGE TEAM TESTING  
**Tester**: ________________  
**Test Date**: ________________  
**Ubuntu Version**: ________________  

---

## 🎯 Testing Objective

Verify that AXONBRIDGE-Linux works correctly on Ubuntu 22.04+ with:
- Screenshot capture (3 methods)
- System tray integration
- Input locking
- Notifications
- Connection to hub
- Performance requirements

---

## ⚙️ Pre-Test Setup

### Step 1: Environment Check

```bash
# Verify Ubuntu version
lsb_release -a
# Required: Ubuntu 22.04 LTS or later

# Check display server
echo $XDG_SESSION_TYPE
# Should show: x11 or wayland

# Verify desktop environment
echo $DESKTOP_SESSION
# Common: ubuntu, gnome, kde-plasma, xfce
```

- [ ] Ubuntu version confirmed (22.04+)
- [ ] Display server identified
- [ ] Desktop environment identified

### Step 2: Install Screenshot Dependencies

```bash
# Update package list
sudo apt-get update

# Install all 3 screenshot tools
sudo apt-get install -y scrot imagemagick gnome-screenshot

# Verify installations
which scrot
which convert
which gnome-screenshot
```

- [ ] scrot installed
- [ ] imagemagick installed  
- [ ] gnome-screenshot installed

### Step 3: Clone and Build Bridge

```bash
# Clone repository (if not already done)
cd ~/
git clone https://github.com/[YOUR-ORG]/AXONBRIDGE-Linux.git
cd AXONBRIDGE-Linux

# OR pull latest changes
cd ~/AXONBRIDGE-Linux
git pull origin main

# Build release binary
cargo build --release

# Verify binary exists
ls -la target/release/axonbridge
```

- [ ] Repository cloned/updated
- [ ] Build successful (no errors)
- [ ] Binary created: `target/release/axonbridge`

---

## 📸 SECTION 1: Screenshot Tests

### Test 1.1: Screenshot Tool Verification

```bash
# Run the test script
cd ~/AXONBRIDGE-Linux
./test_screenshot.sh
```

**Expected Output**:
```
✓ scrot found
✓ scrot works (XXXXX bytes)

✓ gnome-screenshot found
✓ gnome-screenshot works (XXXXX bytes)

✓ imagemagick found
✓ imagemagick works (XXXXX bytes)
```

**Results**:
- [ ] PASS - At least ONE method works
- [ ] FAIL - No methods work (document issue below)

**Notes**: _______________________________________________________________

### Test 1.2: Screenshot Unit Test

```bash
# Run Rust unit tests
cd ~/AXONBRIDGE-Linux
cargo test screenshot_tests -- --nocapture
```

**Expected Output**:
```
running 1 test
test screenshot_tests::test_screenshot_fallback ... ok

test result: ok. 1 passed; 0 failed; 0 ignored
```

**Results**:
- [ ] PASS - Test passed
- [ ] FAIL - Test failed (document error below)

**Notes**: _______________________________________________________________

### Test 1.3: Screenshot Quality Check

```bash
# Take a test screenshot
scrot /tmp/quality_test.png --overwrite

# Check file size (should be > 10KB)
ls -lh /tmp/quality_test.png

# Verify it's a valid PNG
file /tmp/quality_test.png
# Should output: PNG image data

# View the screenshot
eog /tmp/quality_test.png  # or xdg-open /tmp/quality_test.png
```

**Results**:
- [ ] Screenshot file size > 10KB
- [ ] File is valid PNG format
- [ ] Image quality acceptable (not corrupted)
- [ ] Screenshot shows full desktop

**Notes**: _______________________________________________________________

---

## 🖥️ SECTION 2: System Tray Tests

### Test 2.1: System Tray Icon Appears

```bash
# Start the bridge in background
cd ~/AXONBRIDGE-Linux
./target/release/axonbridge &

# Wait 5 seconds for startup
sleep 5
```

**Manual Verification**:
- [ ] System tray icon appears (top-right or bottom-right of screen)
- [ ] Icon is visible (not corrupted/blank)
- [ ] Icon shows correct symbol/logo

**Notes**: _______________________________________________________________

### Test 2.2: System Tray Menu

**Manual Verification**:
- [ ] Right-click on tray icon opens menu
- [ ] Menu shows these options:
  - [ ] "Pause AI" or "Resume AI"
  - [ ] "Stop Current Task"
  - [ ] "Disconnect"
  - [ ] "About" or version info
- [ ] Menu items are clickable

**Notes**: _______________________________________________________________

### Test 2.3: System Tray Status Indicator

**Manual Verification**:
- [ ] Tray shows "Disconnected" state initially
- [ ] Status changes when connected to hub (if hub running)
- [ ] Visual indicator shows connection state clearly

**Notes**: _______________________________________________________________

---

## 🔒 SECTION 3: Input Locking Tests

**Note**: Input locking tests require the hub to be running. If hub not available, mark as "Deferred - Requires Hub".

### Test 3.1: Keyboard Lock

**Manual Verification** (with hub connected):
- [ ] AI takes control (input lock activated)
- [ ] Cannot type on keyboard during lock
- [ ] Keyboard inputs blocked across all applications
- [ ] Lock releases after AI task completes
- [ ] Can type normally after unlock

**Notes**: _______________________________________________________________

### Test 3.2: Mouse Lock

**Manual Verification** (with hub connected):
- [ ] AI takes control (input lock activated)
- [ ] Cannot move mouse during lock
- [ ] Mouse clicks blocked
- [ ] Lock releases after AI task completes
- [ ] Can control mouse normally after unlock

**Notes**: _______________________________________________________________

### Test 3.3: Input Lock Timeout

**Manual Verification** (with hub connected):
- [ ] Input lock has timeout (default 5 minutes)
- [ ] Lock auto-releases if timeout exceeded
- [ ] Notification shown when lock times out
- [ ] System returns to normal operation

**Notes**: _______________________________________________________________

---

## 🔔 SECTION 4: Notifications Tests

### Test 4.1: Connection Notification

```bash
# Start bridge (if not already running)
./target/release/axonbridge &
```

**Manual Verification**:
- [ ] Desktop notification appears on startup
- [ ] Notification shows "Connecting..." or similar
- [ ] Notification properly formatted (not garbled)

**Notes**: _______________________________________________________________

### Test 4.2: Task Notifications

**Manual Verification** (requires hub):
- [ ] Notification when task starts
- [ ] Notification when task completes
- [ ] Notification when task fails
- [ ] Notifications include task details

**Notes**: _______________________________________________________________

### Test 4.3: Error Notifications

**Manual Verification**:
- [ ] Notification shown for errors (e.g., screenshot failed)
- [ ] Error message is clear and helpful
- [ ] Notification doesn't crash the bridge

**Notes**: _______________________________________________________________

---

## 🔗 SECTION 5: Connection Tests

**Note**: These tests require hub to be running. If unavailable, mark "Deferred".

### Test 5.1: Connect to Hub

```bash
# Start bridge with hub URL
./target/release/axonbridge --hub-url http://hub-server:8080 &

# Check logs
tail -f ~/.axonbridge/logs/bridge.log
```

**Expected in logs**:
```
[Bridge] Connecting to hub: http://hub-server:8080
[Bridge] ✅ Connected successfully
[Bridge] Agent ID: xxxxx
```

**Results**:
- [ ] Bridge connects to hub successfully
- [ ] Agent ID assigned
- [ ] System tray shows "Connected" status
- [ ] No connection errors in logs

**Notes**: _______________________________________________________________

### Test 5.2: Heartbeat

**Manual Verification** (watch logs for 2 minutes):
- [ ] Heartbeat messages appear every 30 seconds
- [ ] No disconnections
- [ ] Connection stays stable

**Notes**: _______________________________________________________________

### Test 5.3: Reconnection After Network Interruption

```bash
# Simulate network interruption
sudo systemctl stop NetworkManager
sleep 5
sudo systemctl start NetworkManager

# Watch logs
tail -f ~/.axonbridge/logs/bridge.log
```

**Results**:
- [ ] Bridge detects disconnection
- [ ] Bridge attempts to reconnect
- [ ] Bridge successfully reconnects
- [ ] No crashes during reconnection

**Notes**: _______________________________________________________________

### Test 5.4: Graceful Shutdown

```bash
# Stop the bridge
pkill -SIGTERM axonbridge

# Check logs
tail -n 20 ~/.axonbridge/logs/bridge.log
```

**Expected**:
```
[Bridge] Received shutdown signal
[Bridge] Disconnecting from hub...
[Bridge] ✅ Shutdown complete
```

**Results**:
- [ ] Bridge shuts down gracefully
- [ ] No error messages during shutdown
- [ ] Logs show clean disconnect

**Notes**: _______________________________________________________________

---

## ⚡ SECTION 6: Performance Tests

### Test 6.1: CPU Usage (Idle)

```bash
# Start bridge
./target/release/axonbridge &

# Wait for startup
sleep 10

# Check CPU usage
top -b -n 1 | grep axonbridge
```

**Expected**: CPU < 5%

**Results**:
- [ ] PASS - CPU usage < 5% when idle
- [ ] FAIL - CPU usage > 5% (record actual: _____%)

**Notes**: _______________________________________________________________

### Test 6.2: Memory Usage (Idle)

```bash
# Check memory usage
top -b -n 1 | grep axonbridge
```

**Expected**: Memory < 100MB

**Results**:
- [ ] PASS - Memory usage < 100MB
- [ ] FAIL - Memory usage > 100MB (record actual: _____MB)

**Notes**: _______________________________________________________________

### Test 6.3: Screenshot Performance

```bash
# Time 10 consecutive screenshots
time for i in {1..10}; do
  scrot /tmp/perf_test_$i.png --overwrite
done
```

**Expected**: < 5 seconds total (< 0.5s per screenshot)

**Results**:
- [ ] PASS - 10 screenshots in < 5 seconds
- [ ] FAIL - Took longer (record actual: _____s)

**Notes**: _______________________________________________________________

### Test 6.4: Memory Leak Test

```bash
# Start bridge
./target/release/axonbridge &

# Record initial memory
INITIAL_MEM=$(ps aux | grep axonbridge | grep -v grep | awk '{print $6}')
echo "Initial memory: $INITIAL_MEM KB"

# Take 100 screenshots
for i in {1..100}; do
  # Trigger screenshot via gRPC if hub available
  # Or wait for periodic screenshots
  sleep 1
done

# Record final memory
FINAL_MEM=$(ps aux | grep axonbridge | grep -v grep | awk '{print $6}')
echo "Final memory: $FINAL_MEM KB"

# Calculate growth
echo "Memory growth: $(($FINAL_MEM - $INITIAL_MEM)) KB"
```

**Expected**: Memory growth < 50MB

**Results**:
- [ ] PASS - No significant memory leak
- [ ] FAIL - Memory leak detected (growth: _____MB)

**Notes**: _______________________________________________________________

### Test 6.5: Response Time

**Manual Verification** (requires hub):
- [ ] Commands execute within 100ms
- [ ] Screenshot returned within 500ms
- [ ] Input injection responsive (< 50ms)
- [ ] No lag or delays

**Notes**: _______________________________________________________________

---

## 🐛 SECTION 7: Error Handling Tests

### Test 7.1: Hub Unreachable

```bash
# Start bridge with invalid hub URL
./target/release/axonbridge --hub-url http://invalid-hub:9999 &

# Check logs and behavior
tail -f ~/.axonbridge/logs/bridge.log
```

**Expected**:
- [ ] Clear error message shown
- [ ] Bridge doesn't crash
- [ ] Notification shows connection failed
- [ ] Bridge retries connection

**Notes**: _______________________________________________________________

### Test 7.2: Screenshot Tool Missing

```bash
# Temporarily rename scrot
sudo mv /usr/bin/scrot /usr/bin/scrot.bak

# Restart bridge and trigger screenshot
./target/release/axonbridge &

# Restore scrot after test
sudo mv /usr/bin/scrot.bak /usr/bin/scrot
```

**Expected**:
- [ ] Bridge falls back to gnome-screenshot or imagemagick
- [ ] No crash
- [ ] Error logged but handled gracefully

**Notes**: _______________________________________________________________

### Test 7.3: Invalid Commands

**Manual Verification** (requires hub):
- [ ] Bridge handles invalid gRPC commands
- [ ] Returns proper error messages
- [ ] Doesn't crash on malformed input

**Notes**: _______________________________________________________________

---

## 📋 OVERALL TEST SUMMARY

### Test Results by Section

| Section | Tests | Passed | Failed | Deferred | Notes |
|---------|-------|--------|--------|----------|-------|
| 1. Screenshots | 3 | ___ | ___ | ___ | |
| 2. System Tray | 3 | ___ | ___ | ___ | |
| 3. Input Locking | 3 | ___ | ___ | ___ | |
| 4. Notifications | 3 | ___ | ___ | ___ | |
| 5. Connection | 4 | ___ | ___ | ___ | |
| 6. Performance | 5 | ___ | ___ | ___ | |
| 7. Error Handling | 3 | ___ | ___ | ___ | |
| **TOTAL** | **24** | **___** | **___** | **___** | |

### Pass Criteria

- [ ] **PASS**: ≥ 90% tests passed (≥22/24)
- [ ] **CONDITIONAL PASS**: 75-89% tests passed (18-21/24) - Minor issues documented
- [ ] **FAIL**: < 75% tests passed (<18/24) - Major issues found

### Overall Status

- [ ] ✅ **PASS** - All critical tests passed, ready for production
- [ ] ⚠️ **CONDITIONAL PASS** - Minor issues, needs fixes before production
- [ ] ❌ **FAIL** - Major issues, significant work needed

---

## 🐛 Issues Found

### Critical Issues (Blockers)

1. ________________________________________________________________
2. ________________________________________________________________
3. ________________________________________________________________

### Medium Issues (Should Fix)

1. ________________________________________________________________
2. ________________________________________________________________
3. ________________________________________________________________

### Minor Issues (Nice to Fix)

1. ________________________________________________________________
2. ________________________________________________________________
3. ________________________________________________________________

---

## 📸 Screenshot Evidence

**Attach screenshots showing**:
- [ ] System tray icon visible
- [ ] Menu opened
- [ ] Notification example
- [ ] Successful screenshot capture
- [ ] Performance metrics

**Screenshots saved to**: ________________________________

---

## 🎯 Next Steps

### If PASS ✅
- [ ] Tag commit as `bridge-v3.0-tested`
- [ ] Deploy to production bridge instances
- [ ] Monitor for 24 hours
- [ ] Document any production issues

### If CONDITIONAL PASS ⚠️
- [ ] Document all issues in GitHub issues
- [ ] Fix medium-priority issues
- [ ] Re-test affected areas
- [ ] Then deploy to production

### If FAIL ❌
- [ ] Create detailed issue report
- [ ] Prioritize fixes
- [ ] Fix critical issues
- [ ] Run full test suite again
- [ ] Do NOT deploy until PASS

---

## 📝 Tester Sign-Off

**Tester Name**: ________________________________  
**Date Completed**: ________________________________  
**Time Spent**: ________________________________  
**Overall Status**: ________________________________  

**Signature**: ________________________________

---

## 📧 Reporting Results

**Send results to**: tyler@axonhub.ai  
**Include**:
1. This completed checklist
2. Screenshots/evidence
3. Log files: `~/.axonbridge/logs/bridge.log`
4. Any error messages
5. Performance metrics

**Slack Channel**: #bridge-testing

---

**Document Version**: 1.0  
**Last Updated**: October 26, 2025  
**Prepared By**: AxonHub Development Team
