# AXONBRIDGE-Linux Installation Guide

Complete step-by-step installation guide for running official OSWorld benchmarks.

---

## 🎯 Overview

This guide will help you:
1. Set up Ubuntu 22.04 VM on UTM
2. Install all dependencies
3. Build AXONBRIDGE-Linux
4. Install OSWorld
5. Run official 369-task benchmark

**Total Time:** 2-3 hours  
**Platform:** macOS (UTM) + Ubuntu 22.04 LTS

---

## STEP 1: Download Ubuntu ISO (5 minutes)

### 1.1 Download Ubuntu

```bash
# Visit: https://ubuntu.com/download/desktop
# Download: ubuntu-22.04.3-desktop-amd64.iso
# Size: ~4.7 GB
```

Or use command line:

```bash
cd ~/Downloads
curl -O https://releases.ubuntu.com/22.04/ubuntu-22.04.3-desktop-amd64.iso
```

---

## STEP 2: Create Ubuntu VM in UTM (15 minutes)

### 2.1 Install UTM

```bash
# Download from: https://mac.getutm.app/
# Or install via Homebrew:
brew install --cask utm
```

### 2.2 Create New VM

1. Open UTM
2. Click **"+"** → **"Virtualize"** (NOT Emulate!)
3. Select **"Linux"**
4. Click **"Browse"** → Select `ubuntu-22.04.3-desktop-amd64.iso`

### 2.3 VM Configuration

**Recommended Settings:**

| Setting | Value | Notes |
|---------|-------|-------|
| **RAM** | 8 GB | Minimum 4 GB, 8 GB recommended |
| **CPU Cores** | 4 cores | Minimum 2, 4 recommended |
| **Storage** | 60 GB | Minimum 50 GB |
| **Network** | Shared Network | Enables Mac ↔ VM communication |
| **Display** | Default | 1920x1080 |

5. Click **"Save"**

### 2.4 Start VM and Install Ubuntu

1. Click **"Play"** button on VM
2. Select **"Install Ubuntu"**
3. **Language:** English
4. **Keyboard:** Your layout
5. **Installation type:** Normal installation
6. **Install third-party software:** ✅ Yes
7. **Erase disk:** ✅ Yes (VM only, safe)
8. **Create user:**
   - Name: `osworld`
   - Password: `osworld`
9. Wait for installation (~15 minutes)
10. Click **"Restart Now"**

---

## STEP 3: Initial Ubuntu Setup (10 minutes)

### 3.1 Login and Update System

```bash
# Open Terminal (Ctrl+Alt+T)
sudo apt update
sudo apt upgrade -y
```

### 3.2 Install Basic Tools

```bash
sudo apt install -y \
    curl \
    wget \
    git \
    vim \
    build-essential \
    net-tools
```

### 3.3 Get VM IP Address

```bash
# Get IP address (save this!)
hostname -I
# Example output: 192.168.64.5

# Test network from Mac:
# Open Mac terminal:
ping 192.168.64.5
```

**Save this IP address!** You'll need it for Hub configuration.

---

## STEP 4: Install Bridge Dependencies (10 minutes)

### 4.1 Install X11 Automation Tools

```bash
sudo apt install -y \
    xdotool \
    wmctrl \
    scrot \
    x11-utils \
    xdpyinfo \
    imagemagick
```

### 4.2 Verify Tools Work

```bash
# Test xdotool
xdotool key space
# (Should open application launcher)

# Test wmctrl
wmctrl -l
# (Should list open windows)

# Test scrot
scrot /tmp/test.png
ls -lh /tmp/test.png
# (Should create screenshot)
```

---

## STEP 5: Install Rust (10 minutes)

### 5.1 Install Rust Toolchain

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Follow prompts:
# 1. Proceed with installation (default)

# Add to PATH
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

Expected output:
```
rustc 1.75.0 (or newer)
cargo 1.75.0 (or newer)
```

---

## STEP 6: Install OSWorld Applications (20 minutes)

### 6.1 Install LibreOffice

```bash
sudo apt install -y \
    libreoffice-writer \
    libreoffice-calc \
    libreoffice-impress \
    libreoffice-draw
```

### 6.2 Install GIMP

```bash
sudo apt install -y gimp
```

### 6.3 Install Google Chrome

```bash
# Download Chrome
cd ~/Downloads
wget https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb

# Install
sudo dpkg -i google-chrome-stable_current_amd64.deb

# Fix dependencies
sudo apt --fix-broken install -y
```

### 6.4 Install Other Apps

```bash
# Thunderbird
sudo apt install -y thunderbird

# VLC
sudo apt install -y vlc

# VS Code
sudo snap install code --classic

# File manager (should be pre-installed)
sudo apt install -y nautilus
```

### 6.5 Verify All Apps Launch

```bash
# Test each app
libreoffice --writer &
sleep 2
killall soffice.bin

gimp &
sleep 2
killall gimp

google-chrome &
sleep 2
killall chrome

thunderbird &
sleep 2
killall thunderbird

vlc &
sleep 2
killall vlc

code &
sleep 2
killall code
```

---

## STEP 7: Clone and Build AXONBRIDGE-Linux (15 minutes)

### 7.1 Clone Repository

```bash
cd ~
git clone https://github.com/TheMailmans/AXONBRIDGE
cd AXONBRIDGE/linux
```

(Or create from the files we just generated):

```bash
# If repo not ready yet, copy files from Mac:
mkdir -p ~/AXONBRIDGE-Linux
cd ~/AXONBRIDGE-Linux

# Copy the .rs files we created to this directory
# (Use UTM shared folder or scp from Mac)
```

### 7.2 Build Bridge

```bash
# Build release version
cargo build --release

# This takes ~5-10 minutes first time
# Subsequent builds are much faster
```

### 7.3 Test Bridge

```bash
# Run Bridge
./target/release/axonbridge

# Expected output:
# [INFO] AXONBRIDGE-Linux v1.0.0
# [INFO] Starting gRPC server on 0.0.0.0:50051
# [INFO] Ready to receive commands from AxonHub
```

**Leave this running!**

---

## STEP 8: Test from Mac (10 minutes)

### 8.1 Update Hub Configuration

```bash
# On your Mac:
cd ~/Documents/Projects/ThinkBackHub

# Update Bridge address in test files
# Change from: 192.168.64.2:50051
# Change to: 192.168.64.5:50051 (your Ubuntu VM IP)
```

### 8.2 Test Connection

```python
# Create test file: test_linux_bridge.py
import sys
sys.path.insert(0, 'apps/hub/proto')

import grpc
import agent_pb2
import agent_pb2_grpc

# Use your Ubuntu VM IP
BRIDGE_IP = '192.168.64.5:50051'

channel = grpc.insecure_channel(BRIDGE_IP)
stub = agent_pb2_grpc.DesktopAgentStub(channel)

# Test connection
response = stub.RegisterAgent(agent_pb2.ConnectRequest())
print(f'✅ Connected! Agent: {response.agent_id}')

# Test keyboard
stub.InjectKeyPress(agent_pb2.KeyPressRequest(
    agent_id=response.agent_id,
    key='space',
    modifiers=['cmd']
))
print('✅ Keyboard works!')

# Test window list
windows = list(stub.GetWindowList(agent_pb2.GetWindowListRequest(
    agent_id=response.agent_id
)).windows)
print(f'✅ Window list works! Found {len(windows)} windows')

# Test screenshot
screenshot = stub.CaptureScreenshot(agent_pb2.ScreenshotRequest(
    agent_id=response.agent_id
))
print(f'✅ Screenshot works! {len(screenshot.image_data)} bytes')

print('\n🎉 LINUX BRIDGE FULLY WORKING!')
```

Run it:

```bash
python3 test_linux_bridge.py
```

Expected output:
```
✅ Connected! Agent: agent-xxxxx
✅ Keyboard works!
✅ Window list works! Found 3 windows
✅ Screenshot works! 234567 bytes

🎉 LINUX BRIDGE FULLY WORKING!
```

---

## STEP 9: Install OSWorld (15 minutes)

### 9.1 Install Python Dependencies

```bash
# On Ubuntu VM:
sudo apt install -y \
    python3-pip \
    python3-venv \
    python3-dev
```

### 9.2 Clone OSWorld

```bash
cd ~
git clone https://github.com/xlang-ai/OSWorld
cd OSWorld
```

### 9.3 Create Virtual Environment

```bash
python3 -m venv venv
source venv/bin/activate
```

### 9.4 Install OSWorld

```bash
pip install --upgrade pip
pip install -r requirements.txt
```

### 9.5 Test OSWorld Evaluators

```bash
# Test evaluator import
python3 -c "
from desktop_env.evaluators.metrics import general

# Test check_include_exclude
result = general.check_include_exclude('Calculator Terminal', {
    'include': ['Calculator'],
    'exclude': []
})
print(f'✅ OSWorld evaluator works! Result: {result}')
"
```

Expected output:
```
✅ OSWorld evaluator works! Result: 1.0
```

---

## STEP 10: Run Full System Test (10 minutes)

### 10.1 Simple Task Test

On Mac, create `test_full_system_linux.py`:

```python
#!/usr/bin/env python3
"""
Test complete system: Hub (Mac) + Bridge (Ubuntu) + OSWorld
"""

import sys
sys.path.insert(0, 'apps/hub/proto')
sys.path.insert(0, os.path.expanduser('~/OSWorld'))

import grpc
import agent_pb2
import agent_pb2_grpc
from desktop_env.evaluators.metrics import general
import time

BRIDGE_IP = '192.168.64.5:50051'  # Your Ubuntu VM IP

print('🧪 Testing Complete System: Hub + Bridge + OSWorld')
print('='*70)

# Connect to Bridge
channel = grpc.insecure_channel(BRIDGE_IP)
stub = agent_pb2_grpc.DesktopAgentStub(channel)
response = stub.RegisterAgent(agent_pb2.ConnectRequest())
agent_id = response.agent_id
print(f'✅ Connected to Linux Bridge: {agent_id}')

# Get BEFORE state
windows_before = list(stub.GetWindowList(agent_pb2.GetWindowListRequest(
    agent_id=agent_id
)).windows)
print(f'✅ BEFORE: {len(windows_before)} windows')

# Evaluate BEFORE (Calculator should NOT be present)
reward_before = general.check_include_exclude(', '.join(windows_before), {
    'include': [],
    'exclude': ['Calculator']
})
print(f'✅ OSWorld BEFORE score: {reward_before}')

# Execute task: Open Calculator
print('🚀 Executing: Open Calculator')
stub.InjectKeyPress(agent_pb2.KeyPressRequest(
    agent_id=agent_id, key='space', modifiers=['cmd']
))
time.sleep(1)

for char in "Calculator":
    stub.InjectKeyPress(agent_pb2.KeyPressRequest(
        agent_id=agent_id,
        key=char.lower(),
        modifiers=['shift'] if char.isupper() else []
    ))
    time.sleep(0.05)

stub.InjectKeyPress(agent_pb2.KeyPressRequest(
    agent_id=agent_id, key='return', modifiers=[]
))
time.sleep(2)

# Get AFTER state
windows_after = list(stub.GetWindowList(agent_pb2.GetWindowListRequest(
    agent_id=agent_id
)).windows)
print(f'✅ AFTER: {len(windows_after)} windows')

# Evaluate AFTER (Calculator SHOULD be present)
reward_after = general.check_include_exclude(', '.join(windows_after), {
    'include': ['Calculator'],
    'exclude': []
})
print(f'✅ OSWorld AFTER score: {reward_after}')

# Final score
final_score = 1.0 if reward_before == 1.0 and reward_after == 1.0 else 0.0
print('='*70)
print(f'🏆 FINAL SCORE: {final_score} / 1.0')

if final_score == 1.0:
    print('✅ SUCCESS! Full system working!')
else:
    print('❌ FAILURE')

print('='*70)
```

Run it:

```bash
python3 test_full_system_linux.py
```

Expected output:
```
🧪 Testing Complete System: Hub + Bridge + OSWorld
======================================================================
✅ Connected to Linux Bridge: agent-xxxxx
✅ BEFORE: 3 windows
✅ OSWorld BEFORE score: 1.0
🚀 Executing: Open Calculator
✅ AFTER: 4 windows
✅ OSWorld AFTER score: 1.0
======================================================================
🏆 FINAL SCORE: 1.0 / 1.0
✅ SUCCESS! Full system working!
======================================================================
```

---

## ✅ INSTALLATION COMPLETE!

**You now have:**
- ✅ Ubuntu 22.04 VM running
- ✅ AXONBRIDGE-Linux compiled and running
- ✅ All OSWorld apps installed
- ✅ OSWorld installed and working
- ✅ Full system tested (1.0 score)

---

## NEXT STEPS

### Run Official OSWorld 369-Task Benchmark

```bash
# On Mac:
cd ~/Documents/Projects/ThinkBackHub
python3 RUN_OSWORLD_BENCHMARK_369.py
```

This will:
1. Load all 369 official OSWorld tasks
2. Run each through full AxonHub system
3. Use Bridge to execute on Ubuntu
4. Evaluate with official OSWorld evaluators
5. Save results for submission

---

## Troubleshooting

### Bridge won't start
```bash
# Check port not in use
sudo lsof -i :50051

# Check logs
./target/release/axonbridge
```

### Can't connect from Mac
```bash
# On Ubuntu, check firewall
sudo ufw status

# Allow port if needed
sudo ufw allow 50051

# Verify IP address
hostname -I
```

### Screenshot fails
```bash
# Install missing tools
sudo apt install scrot imagemagick
```

### Apps won't launch
```bash
# Verify installations
which libreoffice
which gimp
which google-chrome
```

---

## Support

- **Issues:** https://github.com/TheMailmans/AXONBRIDGE/issues
- **OSWorld:** https://github.com/xlang-ai/OSWorld

---

**Ready for Official OSWorld Benchmarking!** 🚀
