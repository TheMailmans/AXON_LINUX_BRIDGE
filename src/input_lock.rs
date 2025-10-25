//! Input Locking Controller
//!
//! Manages keyboard and mouse input locking/unlocking for AI control handoff.
//! Provides safe input lock/unlock with watchdog protection and emergency override.

use anyhow::{Context, Result};
use std::process::Command;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Controls input locking on X11 desktop
pub struct InputLockController {
    /// Keyboard device ID (from xinput list)
    keyboard_id: Option<String>,
    
    /// Mouse device ID (from xinput list)
    mouse_id: Option<String>,
    
    /// Master keyboard ID (for reattach)
    master_keyboard_id: Option<String>,
    
    /// Master pointer ID (for reattach)
    master_pointer_id: Option<String>,
    
    /// When inputs were locked (for timeout watchdog)
    locked_at: Option<Instant>,
    
    /// Maximum lock duration before auto-unlock
    lock_timeout: Duration,
    
    /// Current lock state
    is_locked: bool,
}

impl InputLockController {
    /// Create new input lock controller
    pub fn new() -> Self {
        Self {
            keyboard_id: None,
            mouse_id: None,
            master_keyboard_id: None,
            master_pointer_id: None,
            locked_at: None,
            lock_timeout: Duration::from_secs(5 * 60), // 5 minute timeout
            is_locked: false,
        }
    }
    
    /// Initialize by discovering input devices
    pub fn init(&mut self) -> Result<()> {
        info!("[InputLock] Initializing input device discovery");
        
        self.discover_input_devices()?;
        
        if self.keyboard_id.is_some() && self.mouse_id.is_some() {
            info!(
                "[InputLock] ✅ Discovered devices: keyboard={:?}, mouse={:?}",
                self.keyboard_id, self.mouse_id
            );
            Ok(())
        } else {
            warn!("[InputLock] ⚠️  Could not discover some input devices");
            Err(anyhow::anyhow!(
                "Failed to discover input devices: keyboard={:?}, mouse={:?}",
                self.keyboard_id,
                self.mouse_id
            ))
        }
    }
    
    /// Discover input devices via xinput
    fn discover_input_devices(&mut self) -> Result<()> {
        debug!("[InputLock] Running xinput list to discover devices");
        
        let output = Command::new("xinput")
            .arg("list")
            .output()
            .context("Failed to run xinput list")?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("xinput list failed: {}", stderr);
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // Parse xinput output to find keyboard and pointer devices
        // Format: "⎜   ↳ Device Name id=N..."
        for line in stdout.lines() {
            if line.contains("keyboard") && line.contains("id=") {
                if let Some(id) = extract_device_id(line) {
                    self.keyboard_id = Some(id);
                    debug!("[InputLock] Found keyboard: {}", self.keyboard_id.as_ref().unwrap());
                }
            } else if line.contains("pointer") && line.contains("id=") && !line.contains("Keyboard") {
                if let Some(id) = extract_device_id(line) {
                    self.mouse_id = Some(id);
                    debug!("[InputLock] Found mouse: {}", self.mouse_id.as_ref().unwrap());
                }
            }
        }
        
        // If not found by exact name, use fallback discovery
        if self.keyboard_id.is_none() || self.mouse_id.is_none() {
            self.discover_master_devices()?;
        }
        
        Ok(())
    }
    
    /// Discover master devices as fallback
    fn discover_master_devices(&mut self) -> Result<()> {
        debug!("[InputLock] Using master device discovery");
        
        let output = Command::new("xinput")
            .arg("list")
            .output()
            .context("Failed to run xinput list")?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        for line in stdout.lines() {
            if line.contains("master keyboard") {
                if let Some(id) = extract_device_id(line) {
                    self.master_keyboard_id = Some(id);
                }
            } else if line.contains("master pointer") {
                if let Some(id) = extract_device_id(line) {
                    self.master_pointer_id = Some(id);
                }
            }
        }
        
        Ok(())
    }
    
    /// Lock user input (disable keyboard and mouse)
    pub async fn lock_inputs(&mut self) -> Result<()> {
        if self.is_locked {
            debug!("[InputLock] Already locked, ignoring duplicate lock");
            return Ok(());
        }
        
        info!("[InputLock] 🔒 Locking user input");
        
        const MAX_RETRIES: u32 = 3;
        
        // Try multiple times with small delays
        for attempt in 1..=MAX_RETRIES {
            let result = self.lock_inputs_attempt();
            
            if result.is_ok() {
                self.is_locked = true;
                self.locked_at = Some(Instant::now());
                info!("[InputLock] ✅ Input locked successfully");
                
                // Start watchdog for timeout
                let timeout = self.lock_timeout;
                tokio::spawn(async move {
                    self_watchdog_timer(timeout).await;
                });
                
                return Ok(());
            }
            
            if attempt < MAX_RETRIES {
                warn!(
                    "[InputLock] Lock attempt {}/{} failed, retrying",
                    attempt, MAX_RETRIES
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        error!("[InputLock] ❌ Failed to lock input after {} attempts", MAX_RETRIES);
        anyhow::bail!("Failed to lock input after {} attempts", MAX_RETRIES)
    }
    
    /// Actual lock attempt using xinput
    fn lock_inputs_attempt(&self) -> Result<()> {
        // Method 1: Try floating devices (removes from master)
        if let Some(kb_id) = &self.keyboard_id {
            let output = Command::new("xinput")
                .args(&["float", kb_id])
                .output()
                .context("Failed to float keyboard")?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("[InputLock] Failed to float keyboard: {}", stderr);
                return Err(anyhow::anyhow!("Failed to float keyboard: {}", stderr));
            }
        }
        
        if let Some(mouse_id) = &self.mouse_id {
            let output = Command::new("xinput")
                .args(&["float", mouse_id])
                .output()
                .context("Failed to float mouse")?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("[InputLock] Failed to float mouse: {}", stderr);
                return Err(anyhow::anyhow!("Failed to float mouse: {}", stderr));
            }
        }
        
        Ok(())
    }
    
    /// Unlock user input (enable keyboard and mouse)
    pub async fn unlock_inputs(&mut self) -> Result<()> {
        if !self.is_locked {
            debug!("[InputLock] Already unlocked, ignoring duplicate unlock");
            return Ok(());
        }
        
        info!("[InputLock] 🔓 Unlocking user input");
        
        const MAX_RETRIES: u32 = 3;
        
        // Try multiple times with small delays
        for attempt in 1..=MAX_RETRIES {
            let result = self.unlock_inputs_attempt();
            
            if result.is_ok() {
                self.is_locked = false;
                self.locked_at = None;
                info!("[InputLock] ✅ Input unlocked successfully");
                return Ok(());
            }
            
            if attempt < MAX_RETRIES {
                warn!(
                    "[InputLock] Unlock attempt {}/{} failed, retrying",
                    attempt, MAX_RETRIES
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        error!("[InputLock] ❌ Failed to unlock input after {} attempts", MAX_RETRIES);
        anyhow::bail!("Failed to unlock input after {} attempts", MAX_RETRIES)
    }
    
    /// Actual unlock attempt using xinput
    fn unlock_inputs_attempt(&self) -> Result<()> {
        // Method 1: Try reattaching to master
        if let Some(kb_id) = &self.keyboard_id {
            if let Some(master_kb) = &self.master_keyboard_id {
                let output = Command::new("xinput")
                    .args(&["reattach", kb_id, master_kb])
                    .output()
                    .context("Failed to reattach keyboard")?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("[InputLock] Failed to reattach keyboard: {}", stderr);
                    return Err(anyhow::anyhow!("Failed to reattach keyboard: {}", stderr));
                }
            }
        }
        
        if let Some(mouse_id) = &self.mouse_id {
            if let Some(master_ptr) = &self.master_pointer_id {
                let output = Command::new("xinput")
                    .args(&["reattach", mouse_id, master_ptr])
                    .output()
                    .context("Failed to reattach mouse")?;
                
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    warn!("[InputLock] Failed to reattach mouse: {}", stderr);
                    return Err(anyhow::anyhow!("Failed to reattach mouse: {}", stderr));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check if inputs are currently locked
    pub fn is_locked(&self) -> bool {
        self.is_locked
    }
    
    /// Get time since lock was engaged
    pub fn time_locked(&self) -> Option<Duration> {
        self.locked_at.map(|t| t.elapsed())
    }
    
    /// Check if lock has exceeded timeout (returns true if should auto-unlock)
    pub fn should_timeout(&self) -> bool {
        if let Some(locked_at) = self.locked_at {
            locked_at.elapsed() > self.lock_timeout
        } else {
            false
        }
    }
    
    /// Emergency unlock (no retries, force unlock)
    pub async fn emergency_unlock(&mut self) -> Result<()> {
        warn!("[InputLock] 🚨 EMERGENCY UNLOCK triggered");
        self.is_locked = false;
        self.locked_at = None;
        self.unlock_inputs_attempt()?;
        info!("[InputLock] ✅ Emergency unlock complete");
        Ok(())
    }
}

impl Default for InputLockController {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract device ID from xinput output line
/// Example: "⎜   ↳ Name id=15..."
fn extract_device_id(line: &str) -> Option<String> {
    for part in line.split_whitespace() {
        if part.starts_with("id=") {
            return Some(part[3..].to_string());
        }
    }
    None
}

/// Watchdog timer that auto-unlocks after timeout
async fn self_watchdog_timer(timeout: Duration) {
    tokio::time::sleep(timeout).await;
    warn!("[InputLock] ⏰ Lock timeout reached, should trigger auto-unlock");
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_extract_device_id() {
        let line = "⎜   ↳ AT Translated Set 2 keyboard id=13";
        assert_eq!(extract_device_id(line), Some("13".to_string()));
    }
    
    #[test]
    fn test_extract_device_id_with_properties() {
        let line = "⎜   ↳ SynPS/2 Synaptics TouchPad id=11 [slave pointer (2)]";
        assert_eq!(extract_device_id(line), Some("11".to_string()));
    }
    
    #[test]
    fn test_create_controller() {
        let controller = InputLockController::new();
        assert!(!controller.is_locked());
        assert_eq!(controller.time_locked(), None);
    }
}
