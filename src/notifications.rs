//! Desktop Notifications Module
//!
//! Sends desktop notifications to inform users of control mode changes,
//! errors, and important status updates.

use anyhow::{Context, Result};
use notify_rust::{Notification, Timeout, Urgency};
use tracing::{error, info};

/// Notification urgency/importance level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationLevel {
    /// Informational message (low urgency)
    Info,
    
    /// Success message (normal urgency)
    Success,
    
    /// Warning message (normal urgency)
    Warning,
    
    /// Error message (critical urgency)
    Error,
}

impl NotificationLevel {
    /// Convert to notify-rust Urgency
    fn to_urgency(&self) -> Urgency {
        match self {
            NotificationLevel::Info => Urgency::Low,
            NotificationLevel::Success => Urgency::Normal,
            NotificationLevel::Warning => Urgency::Normal,
            NotificationLevel::Error => Urgency::Critical,
        }
    }
    
    /// Get notification icon name
    fn icon(&self) -> &'static str {
        match self {
            NotificationLevel::Info => "dialog-information",
            NotificationLevel::Success => "dialog-ok",
            NotificationLevel::Warning => "dialog-warning",
            NotificationLevel::Error => "dialog-error",
        }
    }
    
    /// Get notification timeout
    fn timeout(&self) -> Timeout {
        match self {
            NotificationLevel::Info => Timeout::Milliseconds(5000),
            NotificationLevel::Success => Timeout::Milliseconds(5000),
            NotificationLevel::Warning => Timeout::Milliseconds(8000),
            NotificationLevel::Error => Timeout::Milliseconds(10000),
        }
    }
}

/// Show a desktop notification
pub fn show_notification(
    title: &str,
    body: &str,
    level: NotificationLevel,
) -> Result<()> {
    info!("[Notifications] Showing notification: {} - {}", title, body);
    
    Notification::new()
        .appname("AxonBridge")
        .summary(title)
        .body(body)
        .icon(level.icon())
        .urgency(level.to_urgency())
        .timeout(level.timeout())
        .show()
        .context("Failed to show desktop notification")?;
    
    Ok(())
}

/// Show notification when AI takes control
pub fn notify_ai_control_active() -> Result<()> {
    show_notification(
        "🤖 AI Control Active",
        "Desktop is now controlled by AI. User inputs are locked.\nClick the system tray icon to request control.",
        NotificationLevel::Info,
    )
}

/// Show notification when user gains control for training
pub fn notify_training_mode_active() -> Result<()> {
    show_notification(
        "👤 Training Mode Active",
        "You now have control of the desktop.\nDemonstrate the correct actions.\nClick 'Stop Training' in the system tray when done.",
        NotificationLevel::Success,
    )
}

/// Show notification when training is complete
pub fn notify_training_complete() -> Result<()> {
    show_notification(
        "✅ Training Complete",
        "AI has regained control of the desktop.\nYour demonstration has been recorded.",
        NotificationLevel::Success,
    )
}

/// Show notification for emergency unlock
pub fn notify_emergency_unlock() -> Result<()> {
    show_notification(
        "🚨 Emergency Unlock",
        "Inputs have been unlocked immediately.\nYou can now use keyboard and mouse.",
        NotificationLevel::Warning,
    )
}

/// Show notification when orchestrator connects
pub fn notify_orchestrator_connected() -> Result<()> {
    show_notification(
        "✅ Orchestrator Connected",
        "AxonBridge is now connected to the orchestrator.\nReady to receive commands.",
        NotificationLevel::Success,
    )
}

/// Show notification when orchestrator disconnects
pub fn notify_orchestrator_disconnected() -> Result<()> {
    show_notification(
        "⚠️  Orchestrator Disconnected",
        "Connection to orchestrator lost.\nAttempting to reconnect...",
        NotificationLevel::Warning,
    )
}

/// Show notification for input lock timeout
pub fn notify_lock_timeout() -> Result<()> {
    show_notification(
        "⏰ Lock Timeout Exceeded",
        "Input lock has been active for over 5 minutes.\nAutomatically unlocking for safety.",
        NotificationLevel::Warning,
    )
}

/// Show notification for error conditions
pub fn notify_error(message: &str) -> Result<()> {
    show_notification(
        "❌ Error",
        message,
        NotificationLevel::Error,
    )
}

/// Show welcome notification on bridge startup
pub fn notify_bridge_started() -> Result<()> {
    show_notification(
        "🚀 AxonBridge Started",
        "Desktop automation bridge is running.\nCheck the system tray for controls.",
        NotificationLevel::Info,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_notification_level_urgency() {
        assert_eq!(NotificationLevel::Info.to_urgency(), Urgency::Low);
        assert_eq!(NotificationLevel::Success.to_urgency(), Urgency::Normal);
        assert_eq!(NotificationLevel::Warning.to_urgency(), Urgency::Normal);
        assert_eq!(NotificationLevel::Error.to_urgency(), Urgency::Critical);
    }
    
    #[test]
    fn test_notification_level_icon() {
        assert_eq!(NotificationLevel::Info.icon(), "dialog-information");
        assert_eq!(NotificationLevel::Success.icon(), "dialog-ok");
        assert_eq!(NotificationLevel::Warning.icon(), "dialog-warning");
        assert_eq!(NotificationLevel::Error.icon(), "dialog-error");
    }
    
    // Note: Cannot test actual notification display without desktop environment
    // Integration testing will verify notifications work on Ubuntu VM
}
