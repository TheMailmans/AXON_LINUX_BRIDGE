# AxonHub OSWorld Test Setup

Complete setup guide for running OSWorld tests through your Mac/Ubuntu bridge architecture.

---

## Architecture Overview

```
┌─────────────────────────────┐     ┌──────────────────┐
│  Ubuntu VM                  │     │  Mac             │
│  ┌───────────────────────┐  │     │  ┌────────────┐  │
│  │ Test Runner           │  │     │  │ AxonHub    │  │
│  │ run_axonhub_official  │  │     │  │ Core       │  │
│  └──────────┬────────────┘  │     │  │ (Node.js)  │  │
│             ↓                │     │  └─────▲──────┘  │
│  ┌───────────────────────┐  │     │        │         │
│  │ AxonHub Agent         │  │     │        │         │
│  │ (Python)              │  │     │        │         │
│  └──────────┬────────────┘  │     │        │         │
│             ↓                │     │        │         │
│  ┌───────────────────────┐  │     │        │         │
│  │ Bridge Client         │──┼─────┼────────┘         │
│  │ (gRPC localhost:50051)│  │     │                  │
│  └───────────────────────┘  │     │                  │
│             ↓                │     │                  │
│  ┌───────────────────────┐  │     │                  │
│  │ Ubuntu Desktop        │  │     │                  │
│  │ (xdotool, wmctrl)     │  │     │                  │
│  └───────────────────────┘  │     │                  │
└─────────────────────────────┘     └──────────────────┘
```

---

## Part 1: Start AxonHub Core on Mac

The AxonHub Core runs on your Mac and provides the "brain" for decision-making.

### 1.1 Navigate to AxonHub directory

```bash
cd /Users/tylermailman/Documents/Projects/AxonHu
```

### 1.2 Install dependencies (first time only)

```bash
npm install
```

### 1.3 Build the hub

```bash
cd apps/hub
npm run build
```

### 1.4 Set up environment variables

```bash
# Generate JWT secret (first time only)
export JWT_SECRET=$(node -e "console.log(require('crypto').randomBytes(64).toString('hex'))")

# Set production mode
export NODE_ENV=production

# Optional: Set port (default is 4545)
export PORT=8787
```

### 1.5 Start the AxonHub Core

```bash
node dist/server.js
```

**Expected output:**
```
[INFO] AXONHUB v0.2.0 starting...
[INFO] Server listening on port 8787
[INFO] Health probe: http://localhost:8787/health
[INFO] Desktop sessions enabled
```

**Keep this terminal open** - the core must stay running.

### 1.6 Verify it's running

Open a new terminal:
```bash
curl http://localhost:8787/health
```

Should return:
```json
{"ok":true,"sessionCount":0,"uptimeSec":10}
```

---

## Part 2: Setup on Ubuntu VM

Copy the test files to your Ubuntu VM.

### 2.1 Copy files to Ubuntu

**Option A: If using shared folder:**
```bash
# Files are already at:
# /home/tylermailman/Documents/Projects/AXONBRIDGE-Linux/osworld_test_setup/
```

**Option B: Manual copy via scp:**
```bash
# From Mac, run:
scp -r /Users/tylermailman/Documents/Projects/AXONBRIDGE-Linux/osworld_test_setup \
    tylermailman@<ubuntu-ip>:/home/tylermailman/
```

### 2.2 SSH into Ubuntu VM

```bash
ssh tylermailman@<ubuntu-ip>
```

### 2.3 Navigate to test directory

```bash
cd /home/tylermailman/osworld_test_setup
# or
cd /home/tylermailman/Documents/Projects/AXONBRIDGE-Linux/osworld_test_setup
```

### 2.4 Install system dependencies

```bash
sudo apt update
sudo apt install -y \
    python3 \
    python3-pip \
    python3-venv \
    xdotool \
    wmctrl \
    scrot
```

### 2.5 Create Python virtual environment

```bash
python3 -m venv venv
source venv/bin/activate
```

### 2.6 Install Python dependencies

```bash
pip install --upgrade pip
pip install grpcio grpcio-tools
```

### 2.7 Generate gRPC Python files

You need the `agent.proto` file from AxonHub. Copy it from Mac:

```bash
# From Mac terminal:
scp /Users/tylermailman/Documents/Projects/AxonHu/vm-shared/desktop-agent/proto/agent.proto \
    tylermailman@<ubuntu-ip>:/home/tylermailman/osworld_test_setup/
```

Then on Ubuntu:
```bash
cd /home/tylermailman/osworld_test_setup/mm_agents
python3 -m grpc_tools.protoc \
    -I.. \
    --python_out=. \
    --grpc_python_out=. \
    ../agent.proto
```

This generates:
- `agent_pb2.py`
- `agent_pb2_grpc.py`

---

## Part 3: Start the Bridge on Ubuntu

The bridge connects to the AxonHub Core on Mac and executes commands on Ubuntu.

### 3.1 Find the bridge binary

```bash
cd /home/tylermailman/Documents/Projects/AXONBRIDGE-Linux
ls target/release/axonbridge
```

If not built yet:
```bash
cargo build --release
```

### 3.2 Start the bridge

```bash
./target/release/axonbridge
```

**Expected output:**
```
[INFO] AXONBRIDGE-Linux v1.0.0
[INFO] Starting gRPC server on 0.0.0.0:50051
[INFO] Ready to receive commands
```

**Keep this terminal open** - the bridge must stay running.

### 3.3 Test the bridge connection

Open a new terminal on Ubuntu:
```bash
cd /home/tylermailman/osworld_test_setup/mm_agents
source ../venv/bin/activate
python3 axonhub_text_first_agent.py
```

Should output:
```
✅ Connected to bridge, agent ID: <id>
Open windows: [...]
Screenshot size: XXXX bytes
✅ Test complete
```

---

## Part 4: Run OSWorld Tests

Now you're ready to run tests!

### 4.1 Set API key

```bash
export ANTHROPIC_API_KEY="your-api-key-here"
```

### 4.2 Run a single test

```bash
cd /home/tylermailman/osworld_test_setup
source venv/bin/activate

python3 run_axonhub_official.py \
    --test osworld_012 \
    --bridge localhost:50051
```

### 4.3 Run all hard tests

```bash
python3 run_axonhub_official.py \
    --difficulty hard \
    --bridge localhost:50051
```

### 4.4 Run full benchmark

```bash
# WARNING: Takes 4-6 hours, costs $50-150
python3 run_axonhub_official.py \
    --full \
    --bridge localhost:50051
```

---

## Troubleshooting

### Mac: AxonHub Core won't start

```bash
# Check if port is in use
lsof -i :8787

# Try different port
PORT=8888 node dist/server.js
```

### Ubuntu: Can't connect to bridge

```bash
# Check bridge is running
ps aux | grep axonbridge

# Check port is open
netstat -tulpn | grep 50051

# Restart bridge
killall axonbridge
./target/release/axonbridge
```

### Ubuntu: gRPC import errors

```bash
# Make sure you generated the proto files
cd mm_agents
ls agent_pb2.py agent_pb2_grpc.py

# If missing, regenerate
python3 -m grpc_tools.protoc -I.. --python_out=. --grpc_python_out=. ../agent.proto
```

### Can't find OSWorld tests

The test runner looks for OSWorld test configurations in:
1. `./config/default_test.json`
2. `/home/tylermailman/Documents/Projects/OSWorld/config/default_test.json`
3. `./OSWorld/config/default_test.json`

Make sure OSWorld is installed on Ubuntu.

---

## Quick Reference

### Mac - Start AxonHub Core
```bash
cd /Users/tylermailman/Documents/Projects/AxonHu/apps/hub
export JWT_SECRET=$(node -e "console.log(require('crypto').randomBytes(64).toString('hex'))")
export NODE_ENV=production
export PORT=8787
node dist/server.js
```

### Ubuntu - Start Bridge
```bash
cd /home/tylermailman/Documents/Projects/AXONBRIDGE-Linux
./target/release/axonbridge
```

### Ubuntu - Run Test
```bash
cd /home/tylermailman/osworld_test_setup
source venv/bin/activate
export ANTHROPIC_API_KEY="your-key"
python3 run_axonhub_official.py --test osworld_012 --bridge localhost:50051
```

---

## Next Steps

1. ✅ Start AxonHub Core on Mac
2. ✅ Start Bridge on Ubuntu
3. ✅ Run single test to verify setup
4. 📊 Review results in `axonhub_official_results/`
5. 🚀 Run larger batches when ready
6. 📤 Submit results to OSWorld maintainers

---

## Results Location

All results are saved to:
```
/home/tylermailman/osworld_test_setup/axonhub_official_results/
├── results_TIMESTAMP.json
└── SUBMISSION_REPORT_TIMESTAMP.md
```

Use the submission report to send to OSWorld maintainers for leaderboard entry.

---

## Support

For issues:
- Bridge: Check AXONBRIDGE-Linux README
- Core: Check AxonHub README  
- Tests: Check OSWorld documentation

Good luck with your testing! 🚀
