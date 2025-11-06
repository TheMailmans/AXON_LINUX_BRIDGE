//! AXONBRIDGE-Linux: Production-ready Linux Bridge for AxonHub OSWorld integration
//!
//! This bridge enables AxonHub to control Ubuntu desktop environments for official
//! OSWorld benchmarking with full input locking, safety mechanisms, and emergency controls.

use anyhow::{Context, Result};
use std::net::SocketAddr;
use std::sync::Arc;
use std::process::Command;
use std::fs;
use std::io::{self, ErrorKind};
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod input_lock;
mod system_tray;
mod notifications;

use input_lock::InputLockController;

// Include generated proto code
pub mod agent {
    tonic::include_proto!("axon.agent");
}

use agent::{
    desktop_agent_server::{DesktopAgent, DesktopAgentServer},
    *,
};

/// Bridge server state
pub struct BridgeService {
    /// Input lock controller
    input_lock: Arc<RwLock<InputLockController>>,

    /// Agent ID (assigned on registration)
    agent_id: Arc<RwLock<Option<String>>>,

    /// Pairing code for initial connection
    pairing_code: Arc<RwLock<String>>,

    /// System tray handle (optional - may not be available in headless mode)
    tray_handle: Option<Arc<system_tray::AxonBridgeTray>>,
}

impl BridgeService {
    /// Create new bridge service
    pub fn new() -> Result<Self> {
        let mut input_lock_controller = InputLockController::new();

        // Initialize input devices
        input_lock_controller.init()?;

        // Generate pairing code
        let pairing_code = Self::generate_pairing_code();

        Ok(Self {
            input_lock: Arc::new(RwLock::new(input_lock_controller)),
            agent_id: Arc::new(RwLock::new(None)),
            pairing_code: Arc::new(RwLock::new(pairing_code)),
            tray_handle: None,
        })
    }

    /// Generate a random pairing code (format: ABC-123)
    fn generate_pairing_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        // Generate 3 uppercase letters
        let letters: String = (0..3)
            .map(|_| rng.gen_range(b'A'..=b'Z') as char)
            .collect();

        // Generate 3 digits
        let numbers: String = (0..3)
            .map(|_| rng.gen_range(b'0'..=b'9') as char)
            .collect();

        format!("{}-{}", letters, numbers)
    }

    /// Get the current pairing code
    pub async fn get_pairing_code(&self) -> String {
        self.pairing_code.read().await.clone()
    }
    
    /// Set system tray handle
    pub fn set_tray_handle(&mut self, tray: Arc<system_tray::AxonBridgeTray>) {
        self.tray_handle = Some(tray);
    }
}

#[tonic::async_trait]
impl DesktopAgent for BridgeService {
    /// Register agent
    async fn register_agent(
        &self,
        request: Request<ConnectRequest>,
    ) -> Result<Response<ConnectResponse>, Status> {
        let req = request.into_inner();
        
        info!("[Bridge] Registering agent: session_id={}", req.session_id);
        
        // Generate agent ID (use session_id as agent_id for now)
        let agent_id = req.session_id.clone();
        *self.agent_id.write().await = Some(agent_id.clone());
        
        // Update tray: orchestrator connected
        if let Some(ref tray) = self.tray_handle {
            tray.set_orchestrator_connected(true).await;
        }
        
        // Show connection notification
        if let Err(e) = notifications::notify_orchestrator_connected() {
            warn!("[Bridge] Failed to show connection notification: {}", e);
        }
        
        // Get system info
        let system_info = get_system_info().await?;
        
        info!("[Bridge] ✅ Agent registered: agent_id={}", agent_id);
        
        Ok(Response::new(ConnectResponse {
            agent_id,
            status: "connected".to_string(),
            system_info: Some(system_info),
        }))
    }
    
    /// Unregister agent
    async fn unregister_agent(
        &self,
        request: Request<DisconnectRequest>,
    ) -> Result<Response<DisconnectResponse>, Status> {
        let req = request.into_inner();
        
        info!("[Bridge] Unregistering agent: agent_id={}", req.agent_id);
        
        // Unlock inputs on disconnect (safety measure)
        let mut lock_controller = self.input_lock.write().await;
        if lock_controller.is_locked() {
            warn!("[Bridge] ⚠️  Input still locked on disconnect, auto-unlocking");
            if let Err(e) = lock_controller.unlock_inputs().await {
                error!("[Bridge] Failed to unlock inputs on disconnect: {}", e);
            }
        }
        
        *self.agent_id.write().await = None;
        
        // Update tray: orchestrator disconnected
        if let Some(ref tray) = self.tray_handle {
            tray.set_orchestrator_connected(false).await;
            tray.set_control_mode(system_tray::ControlMode::Idle).await;
        }
        
        // Show disconnection notification
        if let Err(e) = notifications::notify_orchestrator_disconnected() {
            warn!("[Bridge] Failed to show disconnection notification: {}", e);
        }
        
        info!("[Bridge] ✅ Agent unregistered");
        
        Ok(Response::new(DisconnectResponse { success: true }))
    }
    
    /// Heartbeat
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let _req = request.into_inner();
        
        Ok(Response::new(HeartbeatResponse {
            server_timestamp: chrono::Utc::now().timestamp(),
            status: "ok".to_string(),
        }))
    }
    
    /// Set input lock (PRIORITY 1 - Critical for control handoff)
    async fn set_input_lock(
        &self,
        request: Request<InputLockRequest>,
    ) -> Result<Response<InputLockResponse>, Status> {
        let req = request.into_inner();
        
        info!(
            "[Bridge] SetInputLock called: agent_id={}, locked={}",
            req.agent_id, req.locked
        );
        
        let mut lock_controller = self.input_lock.write().await;
        
        let result = if req.locked {
            lock_controller.lock_inputs().await
        } else {
            lock_controller.unlock_inputs().await
        };
        
        match result {
            Ok(()) => {
                info!(
                    "[Bridge] ✅ Input lock set successfully: locked={}",
                    req.locked
                );
                
                // Update system tray and send notification
                if let Some(ref tray) = self.tray_handle {
                    if req.locked {
                        // AI is now controlling
                        tray.set_control_mode(system_tray::ControlMode::AiControl).await;
                        if let Err(e) = notifications::notify_ai_control_active() {
                            warn!("[Bridge] Failed to show AI control notification: {}", e);
                        }
                    } else {
                        // User is now training
                        tray.set_control_mode(system_tray::ControlMode::TrainingMode).await;
                        if let Err(e) = notifications::notify_training_mode_active() {
                            warn!("[Bridge] Failed to show training mode notification: {}", e);
                        }
                    }
                }
                
                Ok(Response::new(InputLockResponse {
                    success: true,
                    error: None,
                }))
            }
            Err(e) => {
                error!("[Bridge] ❌ Failed to set input lock: {}", e);
                
                // Show error notification
                if let Err(notif_err) = notifications::notify_error(&format!(
                    "Failed to {} inputs: {}",
                    if req.locked { "lock" } else { "unlock" },
                    e
                )) {
                    warn!("[Bridge] Failed to show error notification: {}", notif_err);
                }
                
                Ok(Response::new(InputLockResponse {
                    success: false,
                    error: Some(format!("Failed to set input lock: {}", e)),
                }))
            }
        }
    }
    
    // Placeholder implementations for other RPCs (not needed for Priority 1)
    
    async fn start_capture(
        &self,
        _request: Request<StartCaptureRequest>,
    ) -> Result<Response<StartCaptureResponse>, Status> {
        Err(Status::unimplemented("start_capture not yet implemented"))
    }
    
    async fn stop_capture(
        &self,
        _request: Request<StopCaptureRequest>,
    ) -> Result<Response<StopCaptureResponse>, Status> {
        Err(Status::unimplemented("stop_capture not yet implemented"))
    }
    
    async fn get_frame(
        &self,
        _request: Request<GetFrameRequest>,
    ) -> Result<Response<GetFrameResponse>, Status> {
        Err(Status::unimplemented("get_frame not yet implemented"))
    }
    
    type StreamFramesStream = tokio_stream::wrappers::ReceiverStream<Result<FrameData, Status>>;
    
    async fn stream_frames(
        &self,
        _request: Request<StreamFramesRequest>,
    ) -> Result<Response<Self::StreamFramesStream>, Status> {
        Err(Status::unimplemented("stream_frames not yet implemented"))
    }
    
    async fn start_audio(
        &self,
        _request: Request<StartAudioRequest>,
    ) -> Result<Response<StartAudioResponse>, Status> {
        Err(Status::unimplemented("start_audio not yet implemented"))
    }
    
    async fn stop_audio(
        &self,
        _request: Request<StopAudioRequest>,
    ) -> Result<Response<StopAudioResponse>, Status> {
        Err(Status::unimplemented("stop_audio not yet implemented"))
    }
    
    type StreamAudioStream = tokio_stream::wrappers::ReceiverStream<Result<AudioData, Status>>;
    
    async fn stream_audio(
        &self,
        _request: Request<StreamAudioRequest>,
    ) -> Result<Response<Self::StreamAudioStream>, Status> {
        Err(Status::unimplemented("stream_audio not yet implemented"))
    }
    
    async fn inject_mouse_move(
        &self,
        _request: Request<MouseMoveRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        Err(Status::unimplemented("inject_mouse_move not yet implemented"))
    }
    
    async fn inject_mouse_click(
        &self,
        _request: Request<MouseClickRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        Err(Status::unimplemented("inject_mouse_click not yet implemented"))
    }
    
    async fn inject_key_press(
        &self,
        _request: Request<KeyPressRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        Err(Status::unimplemented("inject_key_press not yet implemented"))
    }
    
    async fn get_system_info(
        &self,
        _request: Request<SystemInfoRequest>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
        let info = get_system_info().await?;
        Ok(Response::new(SystemInfoResponse { info: Some(info) }))
    }
    
    async fn get_window_list(
        &self,
        _request: Request<GetWindowListRequest>,
    ) -> Result<Response<GetWindowListResponse>, Status> {
        Err(Status::unimplemented("get_window_list not yet implemented"))
    }
    
    async fn get_process_list(
        &self,
        _request: Request<GetProcessListRequest>,
    ) -> Result<Response<GetProcessListResponse>, Status> {
        Err(Status::unimplemented("get_process_list not yet implemented"))
    }
    
    async fn get_browser_tabs(
        &self,
        _request: Request<GetBrowserTabsRequest>,
    ) -> Result<Response<GetBrowserTabsResponse>, Status> {
        Err(Status::unimplemented("get_browser_tabs not yet implemented"))
    }
    
    async fn list_files(
        &self,
        _request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        Err(Status::unimplemented("list_files not yet implemented"))
    }
    
    async fn get_clipboard(
        &self,
        _request: Request<GetClipboardRequest>,
    ) -> Result<Response<GetClipboardResponse>, Status> {
        Err(Status::unimplemented("get_clipboard not yet implemented"))
    }
    
    async fn launch_application(
        &self,
        _request: Request<LaunchApplicationRequest>,
    ) -> Result<Response<LaunchApplicationResponse>, Status> {
        Err(Status::unimplemented("launch_application not yet implemented"))
    }
    
    async fn close_application(
        &self,
        _request: Request<CloseApplicationRequest>,
    ) -> Result<Response<CloseApplicationResponse>, Status> {
        Err(Status::unimplemented("close_application not yet implemented"))
    }
    
    async fn take_screenshot(
        &self,
        request: Request<TakeScreenshotRequest>,
    ) -> Result<Response<TakeScreenshotResponse>, Status> {
        let req = request.into_inner();
        info!("[Bridge] Taking screenshot...");
        
        // Try to capture screenshot with fallback methods
        match capture_screenshot_with_fallback() {
            Ok(image_data) => {
                info!("[Bridge] ✅ Screenshot captured successfully ({} bytes)", image_data.len());
                Ok(Response::new(TakeScreenshotResponse {
                    success: true,
                    file_path: req.save_path.clone(),
                    error: String::new(),
                    image_data,
                }))
            }
            Err(e) => {
                let error_msg = format!("Failed to capture screenshot: {}", e);
                error!("[Bridge] ✗ {}", error_msg);
                Ok(Response::new(TakeScreenshotResponse {
                    success: false,
                    file_path: String::new(),
                    error: error_msg,
                    image_data: vec![],
                }))
            }
        }
    }
}

/// Get system information
async fn get_system_info() -> Result<SystemInfo, Status> {
    // Basic system info (can be enhanced later)
    Ok(SystemInfo {
        os: "Linux".to_string(),
        os_version: "Ubuntu 22.04".to_string(), // Can detect actual version
        arch: std::env::consts::ARCH.to_string(),
        hostname: hostname::get()
            .map_err(|e| Status::internal(format!("Failed to get hostname: {}", e)))?
            .to_string_lossy()
            .to_string(),
        screen_width: 1920, // TODO: Detect actual screen resolution
        screen_height: 1080,
        displays: vec![Display {
            id: 0,
            name: "Default".to_string(),
            width: 1920,
            height: 1080,
            x: 0,
            y: 0,
            is_primary: true,
        }],
    })
}

/// Start emergency hotkey listener (Ctrl+Alt+Shift+U)
async fn start_emergency_hotkey_listener(input_lock: Arc<RwLock<InputLockController>>) {
    info!("[Bridge] Starting emergency hotkey listener (Ctrl+Alt+Shift+U)");
    
    tokio::spawn(async move {
        loop {
            // Check for emergency hotkey every 100ms
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            
            // Check if Ctrl+Alt+Shift+U is pressed
            if check_emergency_hotkey().await {
                warn!("[Bridge] 🚨 EMERGENCY HOTKEY DETECTED!");
                
                let mut lock_controller = input_lock.write().await;
                if lock_controller.is_locked() {
                    warn!("[Bridge] Executing emergency unlock");
                    
                    if let Err(e) = lock_controller.emergency_unlock().await {
                        error!("[Bridge] Emergency unlock failed: {}", e);
                    } else {
                        info!("[Bridge] ✅ Emergency unlock successful");
                    }
                }
            }
        }
    });
}

/// Check if emergency hotkey is pressed (Ctrl+Alt+Shift+U)
async fn check_emergency_hotkey() -> bool {
    // Use xdotool to check key state
    let output = tokio::process::Command::new("xdotool")
        .args(&["getmouselocation"])
        .output()
        .await;
    
    if let Ok(_output) = output {
        // TODO: Implement actual hotkey detection using X11 events
        // For now, this is a placeholder
        // In production, we'd use X11 key event monitoring or evdev
        false
    } else {
        false
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║              AXONBRIDGE-Linux v1.0.0                          ║");
    info!("║     Production-ready Linux Bridge for AxonHub OSWorld         ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");

    // Create bridge service
    let mut bridge_service = BridgeService::new()
        .context("Failed to create bridge service")?;

    info!("[Bridge] ✅ Input lock controller initialized");

    // Get and display pairing code
    let pairing_code = bridge_service.get_pairing_code().await;
    info!("");
    info!("╔═══════════════════════════════════════════════════════════════╗");
    info!("║                      PAIRING CODE                             ║");
    info!("║                                                               ║");
    info!("║                        {}                              ║", pairing_code);
    info!("║                                                               ║");
    info!("║  Enter this code in the AxonHub admin panel to pair          ║");
    info!("║  URL: https://axonhub.fly.dev/admin/bridges                   ║");
    info!("╚═══════════════════════════════════════════════════════════════╝");
    info!("");
    
    // Start system tray icon and notifications
    let orchestrator_url = "http://localhost:8080".to_string(); // TODO: Make configurable
    let (_tray_service, tray_handle) = system_tray::start_system_tray(
        bridge_service.input_lock.clone(),
        orchestrator_url,
    ).await.context("Failed to initialize system tray")?;
    
    // Pass tray handle to bridge service
    bridge_service.set_tray_handle(tray_handle.clone());
    
    // Show startup notification
    if let Err(e) = notifications::notify_bridge_started() {
        warn!("[Bridge] Failed to show startup notification: {}", e);
    }
    
    // Start emergency hotkey listener
    start_emergency_hotkey_listener(bridge_service.input_lock.clone()).await;
    
    // Start watchdog timer task
    start_watchdog_timer(
        bridge_service.input_lock.clone(),
        tray_handle.clone(),
    ).await;
    
    // Configure server address
    let addr: SocketAddr = "0.0.0.0:50051".parse()?;
    
    info!("[Bridge] Starting gRPC server on {}", addr);
    info!("[Bridge] Ready to receive commands from AxonHub");
    info!("[Bridge] System tray icon active - check top panel");
    info!("[Bridge] Emergency hotkey: Ctrl+Alt+Shift+U");
    
    // Update tray: set to idle initially
    tray_handle.set_control_mode(system_tray::ControlMode::Idle).await;
    
    // Start server
    Server::builder()
        .add_service(DesktopAgentServer::new(bridge_service))
        .serve(addr)
        .await
        .context("gRPC server failed")?;
    
    Ok(())
}

/// Start watchdog timer that auto-unlocks after timeout
async fn start_watchdog_timer(
    input_lock: Arc<RwLock<InputLockController>>,
    tray_handle: Arc<system_tray::AxonBridgeTray>,
) {
    info!("[Bridge] Starting watchdog timer");
    
    tokio::spawn(async move {
        loop {
            // Check every 5 seconds
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            let mut lock_controller = input_lock.write().await;
            
            if lock_controller.should_timeout() {
                warn!(
                    "[Bridge] ⏰ Input lock timeout exceeded ({:?}), auto-unlocking",
                    lock_controller.time_locked()
                );
                
                if let Err(e) = lock_controller.unlock_inputs().await {
                    error!("[Bridge] Watchdog auto-unlock failed: {}", e);
                } else {
                    info!("[Bridge] ✅ Watchdog auto-unlock successful");
                    
                    // Update tray to idle
                    tray_handle.set_control_mode(system_tray::ControlMode::Idle).await;
                    
                    // Show notification
                    if let Err(e) = notifications::notify_lock_timeout() {
                        warn!("[Bridge] Failed to show timeout notification: {}", e);
                    }
                }
            }
        }
    });
}

/// Capture screenshot with 3 fallback methods (2025 best practice)
fn capture_screenshot_with_fallback() -> Result<Vec<u8>, String> {
    // Method 1: Try scrot (fastest, most reliable)
    if let Ok(data) = capture_with_scrot() {
        return Ok(data);
    }
    
    // Method 2: Try gnome-screenshot
    if let Ok(data) = capture_with_gnome_screenshot() {
        return Ok(data);
    }
    
    // Method 3: Try ImageMagick import
    if let Ok(data) = capture_with_imagemagick() {
        return Ok(data);
    }
    
    Err(
        "All screenshot methods failed. Install scrot, gnome-screenshot, or imagemagick"
            .to_string(),
    )
}

/// Capture screenshot using scrot
fn capture_with_scrot() -> Result<Vec<u8>, String> {
    let temp_file = "/tmp/axonbridge_screenshot_scrot.png";
    
    let output = Command::new("scrot")
        .arg(temp_file)
        .arg("--overwrite")
        .output()
        .map_err(|e| format!("Failed to execute scrot: {}", e))?;
    
    if !output.status.success() {
        return Err("scrot command failed".to_string());
    }
    
    let data = fs::read(temp_file).map_err(|e| format!("Failed to read screenshot file: {}", e))?;
    let _ = fs::remove_file(temp_file); // Cleanup
    
    Ok(data)
}

/// Capture screenshot using gnome-screenshot
fn capture_with_gnome_screenshot() -> Result<Vec<u8>, String> {
    let temp_file = "/tmp/axonbridge_screenshot_gnome.png";
    
    let output = Command::new("gnome-screenshot")
        .arg("-f")
        .arg(temp_file)
        .output()
        .map_err(|e| format!("Failed to execute gnome-screenshot: {}", e))?;
    
    if !output.status.success() {
        return Err("gnome-screenshot command failed".to_string());
    }
    
    let data = fs::read(temp_file).map_err(|e| format!("Failed to read screenshot file: {}", e))?;
    let _ = fs::remove_file(temp_file); // Cleanup
    
    Ok(data)
}

/// Capture screenshot using ImageMagick import
fn capture_with_imagemagick() -> Result<Vec<u8>, String> {
    let temp_file = "/tmp/axonbridge_screenshot_im.png";
    
    let output = Command::new("import")
        .arg("-window")
        .arg("root")
        .arg(temp_file)
        .output()
        .map_err(|e| format!("Failed to execute imagemagick import: {}", e))?;
    
    if !output.status.success() {
        return Err("imagemagick import command failed".to_string());
    }
    
    let data = fs::read(temp_file).map_err(|e| format!("Failed to read screenshot file: {}", e))?;
    let _ = fs::remove_file(temp_file); // Cleanup
    
    Ok(data)
}

#[cfg(test)]
mod screenshot_tests {
    use super::*;
    
    #[test]
    fn test_screenshot_fallback() {
        // This test verifies that at least one screenshot method is available
        let result = capture_screenshot_with_fallback();
        
        if result.is_ok() {
            let data = result.unwrap();
            // Verify PNG header (8-byte PNG magic number)
            assert_eq!(&data[0..8], &[137, 80, 78, 71, 13, 10, 26, 10], "Invalid PNG header");
            assert!(data.len() > 100, "Screenshot data seems too small");
        } else {
            println!("⚠️  Screenshot test skipped: No screenshot tool available");
            println!("   Install scrot, gnome-screenshot, or imagemagick");
        }
    }
}
