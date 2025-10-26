# Bridge Team Testing Instructions
## Quick Start Guide for Ubuntu Testing

**Target**: Bridge Team Engineers  
**Time Required**: 2-3 hours  
**Ubuntu Version**: 22.04 LTS or later

---

## 🎯 Your Mission

Test the new screenshot implementation and verify all bridge functionality on Ubuntu.

---

## ⚡ Quick Start (5 minutes)

```bash
# 1. Pull latest code
cd ~/AXONBRIDGE-Linux
git pull origin main

# 2. Install dependencies
sudo apt-get update
sudo apt-get install -y scrot imagemagick gnome-screenshot

# 3. Build
cargo build --release

# 4. Run quick screenshot test
./test_screenshot.sh

# 5. Start the full test checklist
# Open: UBUNTU_TEST_CHECKLIST.md
```

---

## 📋 What to Test

### Priority 1: Critical (Must Pass) ⚠️

1. **Screenshot Capture** - Test all 3 methods work
2. **System Tray** - Icon appears and menu works
3. **Performance** - CPU < 5%, Memory < 100MB

### Priority 2: Important (Should Pass) 📊

4. **Input Locking** - Keyboard/mouse lock during AI control
5. **Notifications** - Desktop notifications appear
6. **Connection** - Connects to hub successfully

### Priority 3: Nice to Have ✨

7. **Error Handling** - Graceful failures
8. **Memory Leaks** - No leaks after 100 screenshots

---

## 🔧 Setup Instructions

### Step 1: Environment

```bash
# Check Ubuntu version
lsb_release -a
# Need: 22.04 or later

# Check you have a display
echo $DISPLAY
# Should show: :0 or :1

# Check desktop environment
echo $DESKTOP_SESSION
```

### Step 2: Dependencies

```bash
sudo apt-get update
sudo apt-get install -y \
  scrot \
  imagemagick \
  gnome-screenshot \
  build-essential \
  pkg-config \
  libdbus-1-dev

# Verify Rust installed
rustc --version
# If not: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Step 3: Build Bridge

```bash
cd ~/AXONBRIDGE-Linux
git pull origin main

# Build release version
cargo build --release

# Verify binary
ls -lh target/release/axonbridge
```

---

## ✅ Running Tests

### Quick Smoke Test (2 minutes)

```bash
# Test screenshot tools
./test_screenshot.sh

# Expected: At least 1 method shows ✓
```

### Full Test Suite (2 hours)

```bash
# Open the checklist
cat UBUNTU_TEST_CHECKLIST.md

# Follow each section:
# 1. Screenshot Tests
# 2. System Tray Tests  
# 3. Input Locking Tests
# 4. Notification Tests
# 5. Connection Tests
# 6. Performance Tests
# 7. Error Handling Tests

# Check boxes as you complete each test
```

### Performance Quick Check (1 minute)

```bash
# Start bridge
./target/release/axonbridge &

# Wait 10 seconds
sleep 10

# Check resource usage
top -b -n 1 | grep axonbridge
# CPU should be < 5%
# Memory should be < 100MB
```

---

## 📊 What to Document

### For Each Test

✅ **If PASS**:
- Mark checkbox in UBUNTU_TEST_CHECKLIST.md
- Note any observations

❌ **If FAIL**:
- Mark checkbox as failed
- Copy error message
- Save log file: `~/.axonbridge/logs/bridge.log`
- Take screenshot of issue
- Describe steps to reproduce

---

## 🐛 Common Issues & Solutions

### Issue: "cargo: command not found"

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Issue: "libdbus-1-dev not found"

```bash
sudo apt-get update
sudo apt-get install -y libdbus-1-dev pkg-config
```

### Issue: "scrot: command not found"

```bash
sudo apt-get install -y scrot imagemagick gnome-screenshot
```

### Issue: "System tray icon doesn't appear"

```bash
# Check desktop environment supports tray
# Try restarting bridge
pkill axonbridge
./target/release/axonbridge &
```

### Issue: "Screenshot test fails"

```bash
# Verify display available
echo $DISPLAY

# Try manual screenshot
scrot /tmp/test.png
ls -la /tmp/test.png
```

---

## 📸 Screenshot Evidence Needed

Take screenshots showing:

1. ✅ **Screenshot test results** - `./test_screenshot.sh` output
2. ✅ **System tray icon** - Show it in system tray
3. ✅ **Menu opened** - Right-click menu visible
4. ✅ **Performance metrics** - `top` output showing < 5% CPU
5. ✅ **Successful build** - `cargo build --release` success message

Save to: `~/AXONBRIDGE-Linux/test-evidence/`

---

## 📝 Reporting Results

### Quick Report (If all pass)

```bash
# In Slack #bridge-testing channel:
"✅ Ubuntu testing COMPLETE
- All 24 tests PASSED
- Screenshot: [attach screenshot]
- Performance: CPU 2%, Memory 45MB
- System: Ubuntu 22.04, GNOME
- Ready for production"
```

### Detailed Report (If issues found)

```bash
# 1. Fill out UBUNTU_TEST_CHECKLIST.md completely

# 2. Collect logs
cp ~/.axonbridge/logs/bridge.log ~/AXONBRIDGE-Linux/test-evidence/

# 3. Email to: tyler@axonhub.ai
Subject: Bridge Testing Results - [PASS/FAIL]
Attach:
- UBUNTU_TEST_CHECKLIST.md (completed)
- Screenshots
- bridge.log
- Any error messages
```

---

## 🚦 Decision Tree

```
Run Quick Smoke Test (./test_screenshot.sh)
    ↓
    ├─ All 3 methods work? ──→ ✅ Continue to Full Tests
    ├─ 1-2 methods work? ───→ ⚠️  Continue (partial pass OK)
    └─ 0 methods work? ─────→ ❌ STOP - Report issue immediately
                                  Check: DISPLAY set? Packages installed?

Run Full Test Suite (UBUNTU_TEST_CHECKLIST.md)
    ↓
    ├─ ≥90% pass (22+/24)? ──→ ✅ APPROVE for production
    ├─ 75-89% pass (18-21)? ─→ ⚠️  CONDITIONAL - Fix minor issues
    └─ <75% pass (<18)? ────→ ❌ FAIL - Major issues, needs dev work
```

---

## ⏱️ Time Estimates

| Task | Time |
|------|------|
| Setup & Dependencies | 10 min |
| Build Bridge | 5 min |
| Screenshot Tests | 10 min |
| System Tray Tests | 15 min |
| Input Locking Tests | 20 min |
| Notification Tests | 10 min |
| Connection Tests | 30 min |
| Performance Tests | 20 min |
| Error Handling Tests | 15 min |
| Documentation | 15 min |
| **TOTAL** | **2-3 hours** |

---

## 🆘 Need Help?

### During Testing

1. **Check logs**: `tail -f ~/.axonbridge/logs/bridge.log`
2. **Restart bridge**: `pkill axonbridge && ./target/release/axonbridge &`
3. **Re-read test**: Make sure following instructions exactly

### Stuck on a Test?

1. **Document what you see** - Screenshot + logs
2. **Mark as "Blocked"** - Continue to next test
3. **Report at end** - Don't let one issue stop all testing

### Report Issues To

- **Slack**: #bridge-testing
- **Email**: tyler@axonhub.ai  
- **GitHub**: Open issue with `[Ubuntu Testing]` label

---

## ✅ Final Checklist for Testers

Before submitting results:

- [ ] Completed UBUNTU_TEST_CHECKLIST.md (all sections)
- [ ] Took screenshots of key tests
- [ ] Saved bridge.log file
- [ ] Documented any issues found
- [ ] Filled out Overall Test Summary section
- [ ] Signed off at bottom of checklist
- [ ] Sent results via email/Slack

---

## 🎉 What Happens Next

### If PASS ✅
- Dev team tags release as `bridge-v3.0-production`
- Deployed to all production bridges
- You get kudos! 🎊

### If Issues Found ⚠️
- Dev team reviews issues
- Fixes prioritized and implemented
- You'll be asked to re-test specific areas
- Then deploy after fixes

---

**Thank you for testing!** 🙏

Your testing helps ensure our bridge works perfectly on Ubuntu for all users.

---

**Document Version**: 1.0  
**Prepared**: October 26, 2025  
**Contact**: tyler@axonhub.ai
