//! System Tray Module
//!
//! Provides visual status indicator and control menu in the Ubuntu system tray.
//! Enables users to request control, stop training, and see current status.

use anyhow::{Context, Result};
use ksni::{menu::*, Icon, Tray, TrayService};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::input_lock::InputLockController;

/// System tray service for AxonBridge
pub struct AxonBridgeTray {
    /// Input lock controller reference
    input_lock: Arc<RwLock<InputLockController>>,
    
    /// Current control mode (AI or Human)
    control_mode: Arc<RwLock<ControlMode>>,
    
    /// Orchestrator connection status
    orchestrator_connected: Arc<RwLock<bool>>,
    
    /// Base URL for orchestrator API
    orchestrator_url: String,
}

/// Control mode state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlMode {
    /// AI is controlling the desktop (user locked out)
    AiControl,
    
    /// User is demonstrating (training mode)
    TrainingMode,
    
    /// No active control session
    Idle,
}

impl ControlMode {
    /// Get display name for mode
    pub fn as_str(&self) -> &'static str {
        match self {
            ControlMode::AiControl => "AI Controlling",
            ControlMode::TrainingMode => "Training Mode",
            ControlMode::Idle => "Idle",
        }
    }
    
    /// Get emoji icon for mode
    pub fn emoji(&self) -> &'static str {
        match self {
            ControlMode::AiControl => "🤖",
            ControlMode::TrainingMode => "👤",
            ControlMode::Idle => "⚪",
        }
    }
}

impl AxonBridgeTray {
    /// Create new system tray service
    pub fn new(
        input_lock: Arc<RwLock<InputLockController>>,
        orchestrator_url: String,
    ) -> Self {
        Self {
            input_lock,
            control_mode: Arc::new(RwLock::new(ControlMode::Idle)),
            orchestrator_connected: Arc::new(RwLock::new(false)),
            orchestrator_url,
        }
    }
    
    /// Update control mode and refresh tray
    pub async fn set_control_mode(&self, mode: ControlMode) {
        let mut current_mode = self.control_mode.write().await;
        *current_mode = mode;
        info!("[SystemTray] Control mode changed to: {:?}", mode);
    }
    
    /// Update orchestrator connection status
    pub async fn set_orchestrator_connected(&self, connected: bool) {
        let mut status = self.orchestrator_connected.write().await;
        *status = connected;
        info!("[SystemTray] Orchestrator connection: {}", connected);
    }
    
    /// Handle "Request Control" menu action
    async fn request_control(&self) -> Result<()> {
        info!("[SystemTray] User clicked 'Request Control'");
        
        // Check if already in training mode
        let mode = *self.control_mode.read().await;
        if mode == ControlMode::TrainingMode {
            warn!("[SystemTray] Already in training mode, ignoring request");
            
            // Show notification
            crate::notifications::show_notification(
                "Already in Training Mode",
                "You already have control of the desktop.",
                crate::notifications::NotificationLevel::Info,
            )?;
            
            return Ok(());
        }
        
        // Check orchestrator connection
        let connected = *self.orchestrator_connected.read().await;
        if !connected {
            error!("[SystemTray] Cannot request control: orchestrator not connected");
            
            crate::notifications::show_notification(
                "Orchestrator Not Connected",
                "Cannot request control. Bridge is not connected to orchestrator.",
                crate::notifications::NotificationLevel::Error,
            )?;
            
            return Ok(());
        }
        
        // Make API call to orchestrator
        // Note: In production, this would be done via the gRPC client
        // For now, we just unlock inputs locally and update state
        
        let mut lock_controller = self.input_lock.write().await;
        lock_controller.unlock_inputs().await
            .context("Failed to unlock inputs")?;
        
        // Update mode
        self.set_control_mode(ControlMode::TrainingMode).await;
        
        // Show notification
        crate::notifications::show_notification(
            "Training Mode Active",
            "You now have control. Demonstrate the correct actions. Click 'Stop Training' when done.",
            crate::notifications::NotificationLevel::Success,
        )?;
        
        info!("[SystemTray] ✅ Switched to training mode");
        
        Ok(())
    }
    
    /// Handle "Stop Training" menu action
    async fn stop_training(&self) -> Result<()> {
        info!("[SystemTray] User clicked 'Stop Training'");
        
        // Check if in training mode
        let mode = *self.control_mode.read().await;
        if mode != ControlMode::TrainingMode {
            warn!("[SystemTray] Not in training mode, ignoring stop");
            
            crate::notifications::show_notification(
                "Not in Training Mode",
                "You are not currently training.",
                crate::notifications::NotificationLevel::Info,
            )?;
            
            return Ok(());
        }
        
        // Lock inputs (return to AI control)
        let mut lock_controller = self.input_lock.write().await;
        lock_controller.lock_inputs().await
            .context("Failed to lock inputs")?;
        
        // Update mode
        self.set_control_mode(ControlMode::AiControl).await;
        
        // Show notification
        crate::notifications::show_notification(
            "AI Control Restored",
            "Training complete. AI has regained control of the desktop.",
            crate::notifications::NotificationLevel::Success,
        )?;
        
        info!("[SystemTray] ✅ Returned to AI control");
        
        Ok(())
    }
    
    /// Handle "Emergency Unlock" menu action
    async fn emergency_unlock(&self) -> Result<()> {
        warn!("[SystemTray] 🚨 Emergency unlock triggered by user");
        
        let mut lock_controller = self.input_lock.write().await;
        lock_controller.emergency_unlock().await
            .context("Failed to emergency unlock")?;
        
        // Update mode to idle
        self.set_control_mode(ControlMode::Idle).await;
        
        // Show notification
        crate::notifications::show_notification(
            "Emergency Unlock Complete",
            "Inputs have been unlocked. You can now use keyboard and mouse.",
            crate::notifications::NotificationLevel::Warning,
        )?;
        
        info!("[SystemTray] ✅ Emergency unlock complete");
        
        Ok(())
    }
}

impl Tray for AxonBridgeTray {
    /// Get tray icon
    fn icon_name(&self) -> String {
        // Use different icons based on mode
        // In production, we'd use actual icon files
        "applications-system".to_string()
    }
    
    /// Get tray icon pixmap (custom icon)
    fn icon_pixmap(&self) -> Vec<Icon> {
        // TODO: Add custom icon based on control mode
        // For now, using system default
        vec![]
    }
    
    /// Get tray title (shown in some DEs)
    fn title(&self) -> String {
        let mode = tokio::runtime::Handle::current()
            .block_on(self.control_mode.read());
        
        format!("AxonBridge - {}", mode.as_str())
    }
    
    /// Get tooltip text
    fn tool_tip(&self) -> ToolTip {
        let mode = tokio::runtime::Handle::current()
            .block_on(self.control_mode.read());
        
        let connected = tokio::runtime::Handle::current()
            .block_on(self.orchestrator_connected.read());
        
        let status = if *connected {
            "Connected to orchestrator"
        } else {
            "Not connected"
        };
        
        ToolTip {
            title: format!("{} AxonBridge", mode.emoji()),
            description: format!("{}\n{}", mode.as_str(), status),
            icon_name: String::new(),
            icon_pixmap: vec![],
        }
    }
    
    /// Build tray menu
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mode = tokio::runtime::Handle::current()
            .block_on(self.control_mode.read());
        
        let connected = tokio::runtime::Handle::current()
            .block_on(self.orchestrator_connected.read());
        
        let mut menu = vec![
            // Status header (disabled)
            MenuItem::Separator,
            StandardItem {
                label: format!("{} Status: {}", mode.emoji(), mode.as_str()),
                enabled: false,
                ..Default::default()
            }.into(),
            MenuItem::Separator,
        ];
        
        // Add mode-specific actions
        match *mode {
            ControlMode::AiControl => {
                // AI is controlling, user can request control
                menu.push(StandardItem {
                    label: "🎓 Request Control (Train AI)".to_string(),
                    enabled: *connected,
                    activate: Box::new(|this: &mut Self| {
                        tokio::runtime::Handle::current()
                            .block_on(async {
                                if let Err(e) = this.request_control().await {
                                    error!("[SystemTray] Failed to request control: {}", e);
                                }
                            });
                    }),
                    ..Default::default()
                }.into());
            }
            ControlMode::TrainingMode => {
                // User is training, show stop button
                menu.push(StandardItem {
                    label: "⏹️  Stop Training (Return to AI)".to_string(),
                    enabled: true,
                    activate: Box::new(|this: &mut Self| {
                        tokio::runtime::Handle::current()
                            .block_on(async {
                                if let Err(e) = this.stop_training().await {
                                    error!("[SystemTray] Failed to stop training: {}", e);
                                }
                            });
                    }),
                    ..Default::default()
                }.into());
            }
            ControlMode::Idle => {
                // No active session
                menu.push(StandardItem {
                    label: "No active session".to_string(),
                    enabled: false,
                    ..Default::default()
                }.into());
            }
        }
        
        // Emergency unlock (always available)
        menu.push(MenuItem::Separator);
        menu.push(StandardItem {
            label: "🚨 Emergency Unlock".to_string(),
            enabled: true,
            activate: Box::new(|this: &mut Self| {
                tokio::runtime::Handle::current()
                    .block_on(async {
                        if let Err(e) = this.emergency_unlock().await {
                            error!("[SystemTray] Emergency unlock failed: {}", e);
                        }
                    });
            }),
            ..Default::default()
        }.into());
        
        // Separator before quit
        menu.push(MenuItem::Separator);
        
        // Connection status
        let conn_status = if *connected {
            "✅ Connected to Orchestrator"
        } else {
            "❌ Disconnected"
        };
        menu.push(StandardItem {
            label: conn_status.to_string(),
            enabled: false,
            ..Default::default()
        }.into());
        
        // Quit option
        menu.push(MenuItem::Separator);
        menu.push(StandardItem {
            label: "Quit AxonBridge".to_string(),
            enabled: true,
            activate: Box::new(|_this: &mut Self| {
                info!("[SystemTray] User clicked quit");
                std::process::exit(0);
            }),
            ..Default::default()
        }.into());
        
        menu
    }
    
    /// Handle tray icon activation (click)
    fn activate(&mut self, _x: i32, _y: i32) {
        // Show menu on click
        // ksni handles this automatically
    }
}

/// Start system tray service
pub async fn start_system_tray(
    input_lock: Arc<RwLock<InputLockController>>,
    orchestrator_url: String,
) -> Result<(TrayService<AxonBridgeTray>, Arc<AxonBridgeTray>)> {
    info!("[SystemTray] Initializing system tray icon");
    
    let tray = Arc::new(AxonBridgeTray::new(input_lock, orchestrator_url));
    
    let service = TrayService::new(tray.clone());
    
    info!("[SystemTray] ✅ System tray icon initialized");
    
    Ok((service, tray))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_control_mode_display() {
        assert_eq!(ControlMode::AiControl.as_str(), "AI Controlling");
        assert_eq!(ControlMode::TrainingMode.as_str(), "Training Mode");
        assert_eq!(ControlMode::Idle.as_str(), "Idle");
    }
    
    #[test]
    fn test_control_mode_emoji() {
        assert_eq!(ControlMode::AiControl.emoji(), "🤖");
        assert_eq!(ControlMode::TrainingMode.emoji(), "👤");
        assert_eq!(ControlMode::Idle.emoji(), "⚪");
    }
}
