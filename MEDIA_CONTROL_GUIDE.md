# Media Control Guide

## Overview

The media control module provides unified APIs for controlling media playback (play, pause, next, previous, stop) across all major platforms and media players.

## Quick Start

### Play/Pause

```bash
# Toggle play/pause
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaPlayPause
```

### Next Track

```bash
# Skip to next track
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaNext
```

### Previous Track

```bash
# Go to previous track
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaPrevious
```

### Stop Playback

```bash
# Stop current playback
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaStop
```

## API Reference

### MediaPlayPause RPC

**Request:**
```protobuf
message MediaPlayPauseRequest {
  string agent_id = 1;
}
```

**Response:**
```protobuf
message MediaPlayPauseResponse {
  bool success = 1;
  string method_used = 2;       // "command" or "keyboard"
  optional string error = 3;
  int64 timestamp = 4;
}
```

**Behavior:**
- If media is paused → plays
- If media is playing → pauses
- If no media running → attempts to start

### MediaNext RPC

**Request:**
```protobuf
message MediaNextRequest {
  string agent_id = 1;
}
```

**Response:**
```protobuf
message MediaNextResponse {
  bool success = 1;
  string method_used = 2;
  optional string error = 3;
  int64 timestamp = 4;
}
```

**Behavior:**
- Skips to next track in current media player
- Works with playlists and queues
- Graceful failure if no media player running

### MediaPrevious RPC

**Request:**
```protobuf
message MediaPreviousRequest {
  string agent_id = 1;
}
```

**Response:**
```protobuf
message MediaPreviousResponse {
  bool success = 1;
  string method_used = 2;
  optional string error = 3;
  int64 timestamp = 4;
}
```

**Behavior:**
- Goes to previous track
- Seeks to start if early in current track
- Graceful failure if no media player running

### MediaStop RPC

**Request:**
```protobuf
message MediaStopRequest {
  string agent_id = 1;
}
```

**Response:**
```protobuf
message MediaStopResponse {
  bool success = 1;
  string method_used = 2;
  optional string error = 3;
  int64 timestamp = 4;
}
```

**Behavior:**
- Stops current playback
- Resets position to beginning
- Works with all media players

## Supported Media Players

### Linux

#### Primary: playerctl

**Supported Applications:**
- Spotify
- VLC
- Audacious
- cmus
- mpd
- Rhythmbox
- GNOME Music
- KDE Elisa
- Any application using MPRIS (Media Player Remote Interfacing Specification)

**Installation:**
```bash
# Ubuntu/Debian
sudo apt-get install playerctl

# Fedora/RHEL
sudo dnf install playerctl

# Arch
sudo pacman -S playerctl
```

**How it works:**
```bash
# List available players
playerctl -l

# Play/pause
playerctl play-pause

# Next/previous
playerctl next
playerctl previous

# Stop
playerctl stop

# Get status
playerctl status
```

**Features:**
- Works with multiple players simultaneously
- Selects "active" player automatically
- Can target specific player with `-p player-name`

#### Fallback: XF86Audio Keys

Uses keyboard simulation with special media keys:
- `XF86AudioPlay` - Play
- `XF86AudioPause` - Pause
- `XF86AudioNext` - Next track
- `XF86AudioPrev` - Previous track
- `XF86AudioStop` - Stop

**Requires:**
- xdotool installed
- Media key bindings configured
- Window manager supporting XF86 keys

### macOS

#### Primary: osascript (Apple Music)

**Supported Applications:**
- Apple Music
- iTunes
- Any application scripting via AppleScript

**How it works:**
```bash
# Play/pause (through Music app)
osascript -e 'tell application "Music" to playpause'

# Next track
osascript -e 'tell application "Music" to next track'

# Previous track
osascript -e 'tell application "Music" to previous track'

# Stop
osascript -e 'tell application "Music" to stop'

# Get current track
osascript -e 'tell application "Music" to get current track'
```

**Key Codes (Alternative):**
- 104 - Play/Pause
- 123 - Previous
- 124 - Next
- 101 - Stop

#### Supported Players
- Apple Music (default)
- iTunes
- Spotify (with AppleScript support)
- Vox
- Scrobbles for Last.fm

### Windows

#### Primary: nircmd

**Supported Applications:**
- Windows Media Player
- Groove Music
- Spotify
- VLC
- Any application responding to media key events

**Installation:**
Download from [nircmd](https://www.nirsoft.net/utils/nircmd.html)

**How it works:**
```bash
# Play/pause
nircmd.exe mediaplaypause

# Next track
nircmd.exe medianext

# Previous track
nircmd.exe mediaprev

# Stop
nircmd.exe mediastop
```

#### Fallback: Virtual Key Codes

Uses Windows media key constants:
- `VK_MEDIA_PLAY_PAUSE` (0xB3)
- `VK_MEDIA_NEXT_TRACK` (0xB0)
- `VK_MEDIA_PREV_TRACK` (0xB1)
- `VK_MEDIA_STOP` (0xB2)

**Requires:**
- Driver support for media keys
- Proper window focus

## Platform-Specific Configuration

### Linux Configuration

#### playerctl Configuration

Create `~/.config/playerctl/playerctl.conf`:
```ini
[playerctl]
# Specify default player
player = spotify

# Daemon mode settings
daemon = true
```

#### Checking Available Players

```bash
# List all available MPRIS players
playerctl -l

# Check specific player status
playerctl -p spotify status

# Get currently playing track
playerctl -p spotify metadata --format "{{artist}} - {{title}}"
```

### macOS Configuration

#### AppleScript Permissions

If you get permission errors:
1. System Preferences > Security & Privacy
2. Automation tab
3. Grant Terminal (or your app) access to "Music"

#### Testing AppleScript

```bash
# Test music control
osascript -e 'tell application "Music" to activate'

# Get current playing status
osascript -e 'tell application "Music" to player state'
```

### Windows Configuration

#### nircmd Permissions

1. Download nircmd.exe
2. Place in System32 folder or add to PATH
3. May need admin privileges for some operations
4. Test with: `nircmd.exe mediaplaypause`

#### Windows 10/11 Media Control

Modern Windows uses "media control" built into OS:
- Works with any app using Media Control API
- Works even when app is in background
- Consistent across all players

## Advanced Usage

### Playlist Navigation

```rust
// Skip through playlist programmatically
async fn skip_tracks(player: &MediaControl, count: u32) -> Result<()> {
    for _ in 0..count {
        player.next()?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}
```

### Smart Playback Control

```rust
// Pause when idle, resume when active
async fn smart_control() -> Result<()> {
    loop {
        // Check if system is idle
        if is_system_idle()? {
            player.pause()?;
        } else {
            player.play_pause()?;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
```

### Track Change Detection

```rust
// Monitor for track changes
async fn monitor_playback() -> Result<()> {
    let mut last_track = String::new();
    
    loop {
        let current_track = get_current_track()?;
        if current_track != last_track {
            println!("Now playing: {}", current_track);
            last_track = current_track;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
```

### Multi-Player Control

```rust
// Control specific player (Linux with playerctl)
async fn control_specific_player(player_name: &str) -> Result<()> {
    // Linux: playerctl -p <player_name> play-pause
    // macOS: Switch to app and control
    // Windows: Works with all media key responders
    Ok(())
}
```

## Troubleshooting

### "Media control not available"

**Linux Solutions:**
1. Install playerctl:
   ```bash
   sudo apt-get install playerctl
   ```

2. Check if a media player is running:
   ```bash
   playerctl -l
   ```

3. If no players, start one:
   ```bash
   spotify &  # or your favorite player
   ```

4. Test playerctl directly:
   ```bash
   playerctl play-pause
   ```

**macOS Solutions:**
1. Ensure Music app is available
2. Check AppleScript permissions:
   ```bash
   osascript -e 'tell application "Music" to player state'
   ```

3. Try granting access:
   - System Preferences > Security & Privacy > Automation
   - Grant Terminal/app access to Music

**Windows Solutions:**
1. Install nircmd if using Windows
2. Start a media player (Spotify, VLC, etc.)
3. Check if media keys work:
   ```powershell
   nircmd.exe mediaplaypause
   ```

### Media player not responding

**Linux:**
1. Check if player supports MPRIS:
   ```bash
   playerctl -l
   ```

2. If not listed, player may not support media control
3. Use keyboard fallback (XF86Audio keys)

**macOS:**
1. Ensure Music app is in focus
2. Check if AppleScript is enabled
3. Try forcing play state:
   ```bash
   osascript -e 'tell application "Music" to activate'
   osascript -e 'tell application "Music" to play'
   ```

**Windows:**
1. Verify media player is running
2. Test nircmd directly
3. Check driver support

### Keyboard Fallback Issues

If media keys don't work:

**Linux:**
1. Verify xdotool is installed:
   ```bash
   which xdotool
   ```

2. Test media key binding:
   ```bash
   xdotool key XF86AudioPlay
   ```

3. Check if window manager supports XF86 keys

**macOS:**
1. Verify osascript access granted
2. Try key codes directly:
   ```bash
   osascript -e 'tell application "System Events" to key code 104'
   ```

**Windows:**
1. Verify media key driver support
2. Test with your media player's settings
3. May need to update drivers

## Performance Notes

### Latency

- **playerctl:** ~50-100ms
- **osascript:** ~100-150ms
- **nircmd:** ~100-200ms
- **Keyboard fallback:** <50ms

### Resource Usage

- **Memory:** <1MB per operation
- **CPU:** Minimal (mostly waiting for player response)
- **Disk I/O:** None

## Integration Examples

### Python Client

```python
import grpc
from axonbridge.agent_pb2 import MediaPlayPauseRequest
from axonbridge.agent_pb2_grpc import DesktopAgentStub

channel = grpc.insecure_channel('localhost:50051')
stub = DesktopAgentStub(channel)

# Play/pause
request = MediaPlayPauseRequest(agent_id='desktop-1')
response = stub.MediaPlayPause(request)
print(f"Success: {response.success}")

# Next track
from axonbridge.agent_pb2 import MediaNextRequest
request = MediaNextRequest(agent_id='desktop-1')
response = stub.MediaNext(request)

# Previous track
from axonbridge.agent_pb2 import MediaPreviousRequest
request = MediaPreviousRequest(agent_id='desktop-1')
response = stub.MediaPrevious(request)

# Stop
from axonbridge.agent_pb2 import MediaStopRequest
request = MediaStopRequest(agent_id='desktop-1')
response = stub.MediaStop(request)
```

### Command Line Usage

```bash
# Play current track
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaPlayPause

# Next track
grpcurl -plaintext \
  -d '{"agent_id": "desktop-1"}' \
  localhost:50051 axonbridge.DesktopAgent/MediaNext

# Skip to next in rapid succession
for i in {1..5}; do
  grpcurl -plaintext \
    -d '{"agent_id": "desktop-1"}' \
    localhost:50051 axonbridge.DesktopAgent/MediaNext
  sleep 0.5
done
```

## Related Documentation

- [Volume Control Guide](VOLUME_CONTROL_GUIDE.md)
- [Brightness Control Guide](BRIGHTNESS_CONTROL_GUIDE.md)
- [System Control Architecture](SYSTEM_CONTROL_ARCHITECTURE.md)
- [Bridge Connection Info](BRIDGE_CONNECTION_INFO.txt)

## Version History

**v3.1.0** (Current)
- Cross-platform media control
- 6 media actions (play, pause, play_pause, next, previous, stop)
- 4 gRPC RPCs (MediaPlayPause, MediaNext, MediaPrevious, MediaStop)
- 11 unit tests
- Linux: playerctl + XF86Audio keys
- macOS: osascript + Apple Music integration
- Windows: nircmd + virtual media keys
- Hybrid execution (command + keyboard fallback)
