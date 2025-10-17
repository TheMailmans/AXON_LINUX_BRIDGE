# Mac Team Quick Reference - v3.1 System Control

## 🎯 Everything You Need

### Files to Copy

1. **Proto File:** `MAC_TEAM_agent_v3.1.proto` (19KB)
2. **Integration Guide:** `MAC_TEAM_V3.1_INTEGRATION_GUIDE.md` (21KB)
3. **This Reference:** `MAC_TEAM_QUICK_REFERENCE.md`

### Bridge Connection

```
Endpoint:  192.168.64.3:50051
Protocol:  gRPC (HTTP/2)
Status:    ✅ RUNNING NOW
Package:   axonbridge / axon.agent
Service:   DesktopAgent
```

---

## 📡 New RPCs (9 Total)

### Volume (3 RPCs)

```python
# Get volume
GetVolume(GetVolumeRequest) → GetVolumeResponse
  Returns: float volume (0.0-1.0)

# Set volume  
SetVolume(SetVolumeRequest) → SetVolumeResponse
  Input: float volume (0.0-1.0)
  Returns: bool success

# Mute
MuteVolume(MuteVolumeRequest) → MuteVolumeResponse
  Input: bool mute
  Returns: bool is_muted
```

### Brightness (2 RPCs)

```python
# Get brightness
GetBrightness(GetBrightnessRequest) → GetBrightnessResponse
  Returns: float level (0.0-1.0)

# Set brightness
SetBrightness(SetBrightnessRequest) → SetBrightnessResponse
  Input: float level (0.0-1.0)
  Returns: bool success
```

### Media (4 RPCs)

```python
# Play/Pause toggle
MediaPlayPause(MediaPlayPauseRequest) → MediaPlayPauseResponse
  Returns: bool success

# Next track
MediaNext(MediaNextRequest) → MediaNextResponse
  Returns: bool success

# Previous track  
MediaPrevious(MediaPreviousRequest) → MediaPreviousResponse
  Returns: bool success

# Stop playback
MediaStop(MediaStopRequest) → MediaStopResponse
  Returns: bool success
```

---

## 🚀 Quick Test (Python)

```python
import grpc
from generated import agent_pb2_grpc, agent_pb2

# Connect
channel = grpc.insecure_channel('192.168.64.3:50051')
stub = agent_pb2_grpc.DesktopAgentStub(channel)

# Test volume
req = agent_pb2.GetVolumeRequest(agent_id='test')
resp = stub.GetVolume(req)
print(f"Volume: {resp.volume}")  # Should work!

# Set volume
req = agent_pb2.SetVolumeRequest(agent_id='test', volume=0.5)
resp = stub.SetVolume(req)
print(f"Success: {resp.success}")
```

---

## ✅ Status Check

**Current Bridge Status:**
- Volume Control: ✅ WORKING (50% volume right now)
- Brightness Control: ⚠️ VM limitation (returns 0)
- Media Control: ⚠️ No player running

**All RPCs are accessible and responding!**

---

## 📦 Proto Generation

**Swift:**
```bash
protoc --swift_out=. --grpc-swift_out=. MAC_TEAM_agent_v3.1.proto
```

**Python:**
```bash
python -m grpc_tools.protoc -I. \
  --python_out=. --grpc_python_out=. \
  MAC_TEAM_agent_v3.1.proto
```

**Go:**
```bash
protoc --go_out=. --go-grpc_out=. MAC_TEAM_agent_v3.1.proto
```

---

## 🔧 Common Request Patterns

### Volume Control
```python
# Read → Modify → Write pattern
current = stub.GetVolume(GetVolumeRequest(agent_id='x')).volume
new_volume = min(1.0, current + 0.1)  # Increase 10%
stub.SetVolume(SetVolumeRequest(agent_id='x', volume=new_volume))
```

### Brightness Control
```python
# Set to 75%
stub.SetBrightness(SetBrightnessRequest(agent_id='x', level=0.75))
```

### Media Control
```python
# Play/pause toggle (most common)
stub.MediaPlayPause(MediaPlayPauseRequest(agent_id='x'))

# Skip track
stub.MediaNext(MediaNextRequest(agent_id='x'))
```

---

## ⚡ Key Points

1. **All volumes/brightness are 0.0-1.0** (NOT 0-100)
2. **Validation happens server-side** (returns INVALID_ARGUMENT)
3. **All RPCs are backwards compatible** (v3.0 still works)
4. **Each response includes timestamp** (milliseconds since epoch)
5. **Method tracking** (`method_used`: "command" or "keyboard")

---

## 📊 Error Codes

```
OK (0)                  - Success
INVALID_ARGUMENT (3)    - Volume/brightness out of range (0.0-1.0)
UNAVAILABLE (14)        - Bridge not running
INTERNAL (13)           - System tool failed
```

---

## 📚 Full Documentation

See `MAC_TEAM_V3.1_INTEGRATION_GUIDE.md` for:
- Complete message definitions
- Translation layer examples
- Error handling patterns
- Platform-specific details
- Troubleshooting guide

---

## 🎯 Implementation Steps

1. ✅ Copy `MAC_TEAM_agent_v3.1.proto`
2. ✅ Generate client code
3. ✅ Test connection to `192.168.64.3:50051`
4. ✅ Add 9 RPCs to translation layer
5. ✅ Handle validation errors
6. ✅ Update UI
7. ✅ Deploy

---

## 🆘 Need Help?

**Bridge is running now at:** `192.168.64.3:50051`

**Test it works:**
```bash
nc -zv 192.168.64.3 50051
# Should show: Connection successful
```

**Check documentation:**
- Integration guide: 21KB complete guide
- Proto file: 19KB with all definitions
- Bridge status: Running and ready

---

**Ready to integrate! All tools and docs provided.** 🚀
