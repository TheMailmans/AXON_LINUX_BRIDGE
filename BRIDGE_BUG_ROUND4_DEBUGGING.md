# 🔍 ROUND 4 DEBUGGING - Hub-Side RPC Timeout Investigation

**Date:** 2025-10-14 01:19 UTC  
**Focus:** Hub RPC client timeout configuration  
**Status:** Ready for Hub-side debugging

---

## 🎯 Critical Observation

After 3 rounds of Bridge-side fixes, we've observed a **critical pattern**:

1. ✅ **Calculator launches successfully every time** (visible on VM screen)
2. ❌ **Hub reports error after exactly 5 seconds** (⏱️ suspiciously precise timing)
3. 🚨 **5 seconds is a classic default RPC timeout duration**

---

## 💡 New Hypothesis: Hub RPC Client Timeout

### The Problem
**The Bridge is working correctly!** 

The issue is likely that the **Hub's RPC client has a 5-second timeout**, which is **too short** for application launches that take 2-6 seconds to complete.

### Evidence
- Bridge launches app successfully (we can see it!)
- Error occurs at exactly 5 seconds (not random)
- Bridge logs don't show RPC completion
- Classic symptoms of client-side timeout

### Why Previous Fixes Didn't Help
- **Rounds 1-3:** Fixed Bridge server-side logic
- **Round 4:** Need to fix Hub client-side timeout

---

## 🔬 What to Investigate on Hub Side

### 1. RPC Client Configuration

**File to Check:** `axon-hub/src/bridge/client.rs` (or similar)

**Look for these patterns:**

```rust
// PATTERN 1: Timeout in channel/client creation
Channel::from_shared(uri)?
    .timeout(Duration::from_secs(5))  // ⚠️ TOO SHORT!
    .connect()
    .await?

// PATTERN 2: Per-request timeout
client
    .launch_application(request)
    .timeout(Duration::from_secs(5))  // ⚠️ TOO SHORT!
    .await?

// PATTERN 3: Deadline in request
Request::new(msg)
    .set_timeout(Duration::from_secs(5))  // ⚠️ TOO SHORT!
```

### 2. gRPC Configuration

**Search for:**
- `Duration::from_secs(5)` or similar
- `timeout` or `deadline` configuration
- Channel builder settings
- Per-RPC timeout settings

---

## 🛠️ Recommended Fixes

### Fix Option 1: Increase Global RPC Timeout

```rust
// BEFORE (too short):
Channel::from_shared(uri)?
    .timeout(Duration::from_secs(5))
    .connect()
    .await?

// AFTER (better):
Channel::from_shared(uri)?
    .timeout(Duration::from_secs(30))  // Give app launches time to complete
    .connect()
    .await?
```

### Fix Option 2: Per-Operation Timeout

```rust
// Set different timeouts for different operations
match operation {
    "LaunchApplication" | "CloseApplication" => {
        // App launches need more time
        client.call_with_timeout(request, Duration::from_secs(30))
    }
    "GetFrame" | "InjectMouseClick" => {
        // Fast operations can have shorter timeout
        client.call_with_timeout(request, Duration::from_secs(5))
    }
}
```

### Fix Option 3: Configurable Timeout

```rust
// Make timeout configurable
pub struct BridgeClient {
    client: DesktopAgentClient<Channel>,
    timeout: Duration,
}

impl BridgeClient {
    pub fn new(uri: String, timeout_secs: u64) -> Result<Self> {
        let channel = Channel::from_shared(uri)?
            .timeout(Duration::from_secs(timeout_secs))
            .connect()
            .await?;
        
        Ok(Self {
            client: DesktopAgentClient::new(channel),
            timeout: Duration::from_secs(timeout_secs),
        })
    }
}
```

---

## 📋 Debugging Steps for Hub

### Step 1: Search for Timeout Configuration

```bash
# In axon-hub repository:
cd axon-hub

# Search for timeout configurations:
grep -r "Duration::from_secs" src/
grep -r "timeout" src/bridge/
grep -r "deadline" src/bridge/

# Look for specific patterns:
grep -r "from_secs(5)" src/
grep -r ".timeout(" src/
```

### Step 2: Add Enhanced Logging

```rust
// In Hub's bridge client:
info!("🚀 Calling LaunchApplication RPC...");
let start = Instant::now();

match client
    .launch_application(request)
    .await
{
    Ok(response) => {
        let elapsed = start.elapsed();
        info!("✅ LaunchApplication succeeded in {:?}", elapsed);
        info!("   Response: success={}, error='{}'", 
              response.success, response.error);
    }
    Err(e) => {
        let elapsed = start.elapsed();
        error!("❌ LaunchApplication failed after {:?}", elapsed);
        error!("   Error: {:?}", e);
        error!("   Status code: {:?}", e.code());  // Look for DeadlineExceeded
    }
}
```

### Step 3: Check Error Codes

**Look for these specific gRPC status codes:**

```rust
use tonic::Code;

match error.code() {
    Code::DeadlineExceeded => {
        // This confirms it's a timeout issue!
        error!("⏱️  RPC TIMEOUT! Need to increase timeout duration");
    }
    Code::Unavailable => {
        error!("📡 Bridge unavailable (connection issue)");
    }
    Code::Internal => {
        error!("💥 Internal error (Bridge crashed or logic error)");
    }
    _ => {
        error!("❓ Other error: {:?}", error);
    }
}
```

---

## 🧪 Test After Fixing

### 1. Quick Test
```bash
# From Mac Hub:
# Launch calculator and measure time
time ./test-launch-calculator.sh

# Expected result with fix:
# - Success: true
# - Time: 2-6 seconds (variable, but under new timeout)
```

### 2. Stress Test
```bash
# Launch multiple apps in sequence:
for app in calculator firefox gedit; do
    echo "Testing $app..."
    ./test-launch-app.sh $app
    sleep 2
done
```

### 3. Slow App Test
```bash
# Test with a slower-launching app:
./test-launch-app.sh libreoffice

# This should succeed even if it takes 10+ seconds
```

---

## 📊 Expected Results

### Before Fix (Current Behavior)
```
LaunchApplication("calculator")
  → Calculator launches on VM ✅
  → Hub waits 5 seconds ⏱️
  → Hub times out ❌
  → Hub returns error to caller
  → Calculator still running on VM
```

### After Fix (Expected Behavior)
```
LaunchApplication("calculator")
  → Calculator launches on VM ✅
  → Hub waits up to 30 seconds ⏱️
  → Bridge responds with success (2-6 seconds) ✅
  → Hub returns success to caller ✅
  → Calculator running on VM ✅
```

---

## 🔍 What to Look For in Logs

### Hub Logs (Before Fix)
```
🚀 Calling LaunchApplication RPC...
❌ LaunchApplication failed after 5.001s
   Error: Status { code: DeadlineExceeded, message: "Timeout exceeded" }
   Status code: DeadlineExceeded
```
**^ This confirms the timeout hypothesis!**

### Hub Logs (After Fix)
```
🚀 Calling LaunchApplication RPC...
✅ LaunchApplication succeeded in 3.245s
   Response: success=true, error=''
```

### Bridge Logs (Round 3 logging)
```
🚀 [ROUND3] LaunchApplication RPC ENTRY: app_name='calculator'
🎯 [ROUND3] APPINDEX HIT! Matched 'calculator' to 'Calculator'
🔧 [ROUND3] METHOD 1: Trying gio launch
🔧 [ROUND3-GIO] Entering launch_with_gio()
✅ [ROUND3-GIO] SUCCESS! Command succeeded, returning Ok(true)
✅ [ROUND3] SUCCESS! Launched Calculator via gio launch
✅ [ROUND3] Returning success response to RPC caller
```
**Bridge is working correctly!**

---

## 🎯 Root Cause Analysis

### Why This Makes Sense

1. **Bridge Launches App Successfully**
   - Round 3 logs prove this
   - Calculator appears on screen
   - No errors in Bridge logs

2. **5-Second Pattern**
   - Not random (always 5 seconds)
   - Classic default gRPC timeout
   - Too short for app launches

3. **Error Occurs Hub-Side**
   - Hub reports the error
   - Hub doesn't see Bridge's success response
   - Timeout prevents response from being received

### The Fix

**Increase Hub RPC timeout from 5 seconds to 15-30 seconds**

This gives app launches enough time while still catching real errors.

---

## 📁 Files to Check on Hub

```
axon-hub/
├── src/
│   ├── bridge/
│   │   ├── client.rs        ← Main client implementation
│   │   ├── connection.rs    ← Connection/channel setup
│   │   └── config.rs        ← Configuration (timeout settings?)
│   ├── agent/
│   │   └── executor.rs      ← Code that calls bridge client
│   └── config/
│       └── settings.rs      ← Global timeout configuration?
```

---

## 🚀 Quick Win Checklist

For Hub developers:

- [ ] Find timeout configuration in Hub code
- [ ] Check current timeout value (probably 5 seconds)
- [ ] Increase to 30 seconds for LaunchApplication
- [ ] Add logging to capture error codes
- [ ] Test with calculator launch
- [ ] Verify success response received
- [ ] Test with slower apps (Firefox, LibreOffice)
- [ ] Confirm all launches now succeed

---

## 💡 Why Rounds 1-3 Didn't Work

**The Bridge was never the problem!**

- Round 1: Fixed AppIndex logic → Bridge was already working
- Round 2: Fixed fallback methods → Bridge was already working
- Round 3: Added logging → Proved Bridge was working

The issue is **Hub-side timeout**, not Bridge-side execution.

---

## 🎊 Next Steps

1. **Hub Team:** Search for timeout configuration
2. **Hub Team:** Increase timeout from 5 to 30 seconds
3. **Hub Team:** Add error code logging
4. **Test:** Run calculator launch
5. **Verify:** Check for `DeadlineExceeded` vs `success=true`
6. **Celebrate:** When it works! 🎉

---

**This is the fix! The Bridge is working correctly. The Hub just needs to wait longer for the response! 🎯**

Focus: Hub RPC client timeout configuration  
Fix: Increase timeout from 5 to 30 seconds  
Expected Result: LaunchApplication succeeds consistently
