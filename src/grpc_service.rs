/*!
 * gRPC Service Implementation
 * 
 * Implements the DesktopAgent gRPC service for hub communication.
 */

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tonic::{Request, Response, Status};
use tracing::{info, error, debug, warn};

use crate::proto_gen::agent::desktop_agent_server::{DesktopAgent, DesktopAgentServer};
use crate::proto_gen::agent::*;
use crate::agent::Agent;
use tokio_stream::Stream;
use std::pin::Pin;

/// gRPC service wrapper around Agent
pub struct DesktopAgentService {
    agent: Arc<RwLock<Option<Agent>>>,
}

impl DesktopAgentService {
    pub fn new() -> Self {
        Self {
            agent: Arc::new(RwLock::new(None)),
        }
    }
    
    pub fn server() -> DesktopAgentServer<Self> {
        DesktopAgentServer::new(Self::new())
    }
}

#[tonic::async_trait]
impl DesktopAgent for DesktopAgentService {
    type StreamFramesStream = Pin<Box<dyn Stream<Item = Result<FrameData, Status>> + Send>>;
    type StreamAudioStream = Pin<Box<dyn Stream<Item = Result<AudioData, Status>> + Send>>;
    
    async fn register_agent(
        &self,
        request: Request<ConnectRequest>,
    ) -> Result<Response<ConnectResponse>, Status> {
        let req = request.into_inner();
        info!("RegisterAgent called for session: {}", req.session_id);
        
        // Create agent instance
        let agent = Agent::new(req.session_id.clone(), req.hub_url.clone())
            .map_err(|e| Status::internal(e.to_string()))?;
        
        let agent_id = agent.agent_id().to_string();
        let system_info = crate::platform::get_system_info()
            .map_err(|e| Status::internal(e.to_string()))?;
        
        // Store agent
        *self.agent.write().await = Some(agent);
        
        let response = ConnectResponse {
            agent_id,
            status: "connected".to_string(),
            system_info: Some(SystemInfo {
                os: system_info.os,
                os_version: system_info.os_version,
                arch: system_info.arch,
                hostname: system_info.hostname,
                screen_width: system_info.screen_width as i32,
                screen_height: system_info.screen_height as i32,
                displays: vec![], // TODO: Populate displays
            }),
        };
        
        Ok(Response::new(response))
    }
    
    async fn unregister_agent(
        &self,
        request: Request<DisconnectRequest>,
    ) -> Result<Response<DisconnectResponse>, Status> {
        let req = request.into_inner();
        info!("UnregisterAgent called for agent: {}", req.agent_id);
        
        // Disconnect agent
        if let Some(agent) = self.agent.read().await.as_ref() {
            agent.disconnect().await
                .map_err(|e| Status::internal(e.to_string()))?;
        }
        
        *self.agent.write().await = None;
        
        Ok(Response::new(DisconnectResponse { success: true }))
    }
    
    async fn heartbeat(
        &self,
        request: Request<HeartbeatRequest>,
    ) -> Result<Response<HeartbeatResponse>, Status> {
        let _req = request.into_inner();
        
        let response = HeartbeatResponse {
            server_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            status: "ok".to_string(),
        };
        
        Ok(Response::new(response))
    }
    
    async fn start_capture(
        &self,
        request: Request<StartCaptureRequest>,
    ) -> Result<Response<StartCaptureResponse>, Status> {
        let req = request.into_inner();
        info!("StartCapture called for agent: {}", req.agent_id);
        
        if let Some(agent) = self.agent.read().await.as_ref() {
            agent.start_capture().await
                .map_err(|e| Status::internal(e.to_string()))?;
            
            Ok(Response::new(StartCaptureResponse {
                success: true,
                capture_id: format!("capture-{}", req.agent_id),
            }))
        } else {
            Err(Status::not_found("Agent not registered"))
        }
    }
    
    async fn stop_capture(
        &self,
        request: Request<StopCaptureRequest>,
    ) -> Result<Response<StopCaptureResponse>, Status> {
        let req = request.into_inner();
        info!("StopCapture called");
        
        if let Some(agent) = self.agent.read().await.as_ref() {
            agent.stop_capture().await
                .map_err(|e| Status::internal(e.to_string()))?;
            
            Ok(Response::new(StopCaptureResponse { success: true }))
        } else {
            Err(Status::not_found("Agent not registered"))
        }
    }
    
    async fn get_frame(
        &self,
        request: Request<GetFrameRequest>,
    ) -> Result<Response<GetFrameResponse>, Status> {
        let _req = request.into_inner();
        
        debug!("GetFrame RPC called - capturing on-demand screenshot");
        
        // Capture a single frame directly using platform capturer
        // This is independent of the streaming pipeline
        #[cfg(target_os = "linux")]
        {
            use crate::capture::linux::LinuxCapturer;
            use crate::capture::CaptureConfig;
            
            // CRITICAL FIX: Use spawn_blocking to avoid blocking the async runtime
            // The scrot command is a synchronous blocking operation
            let raw_frame = tokio::task::spawn_blocking(move || {
                let mut capturer = LinuxCapturer::new()?;
                let config = CaptureConfig::default();
                capturer.start(&config)?;
                let raw_frame = capturer.get_raw_frame()?;
                capturer.stop()?;
                Ok::<_, anyhow::Error>(raw_frame)
            })
            .await
            .map_err(|e| Status::internal(format!("Task join error: {}", e)))?
            .map_err(|e| Status::internal(format!("Failed to capture frame: {}", e)))?;
            
            info!("Captured frame: {}x{}, {} bytes", 
                raw_frame.width, raw_frame.height, raw_frame.data.len());
            
            // No a11y on macOS (yet)
            Ok(Response::new(GetFrameResponse {
                frame: Some(FrameData {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    width: raw_frame.width as i32,
                    height: raw_frame.height as i32,
                    data: raw_frame.data,
                    format: 2, // PNG format
                    sequence_number: raw_frame.sequence as i32,
                    accessibility_tree: None,
                    discovered_shortcuts: vec![],
                }),
            }))
        }
        
        #[cfg(target_os = "macos")]
        {
            use crate::capture::macos::MacOSCapturer;
            use crate::capture::CaptureConfig;
            
            let mut capturer = MacOSCapturer::new()
                .map_err(|e| Status::internal(format!("Failed to create capturer: {}", e)))?;
            
            let config = CaptureConfig::default();
            capturer.start(&config)
                .map_err(|e| Status::internal(format!("Failed to start capture: {}", e)))?;
            
            let raw_frame = capturer.get_raw_frame()
                .map_err(|e| Status::internal(format!("Failed to get frame: {}", e)))?;
            
            capturer.stop()
                .map_err(|e| Status::internal(format!("Failed to stop capture: {}", e)))?;
            
            info!("Captured frame: {}x{}, {} bytes", 
                raw_frame.width, raw_frame.height, raw_frame.data.len());
            
            // Capture a11y tree (Linux only)
            let (a11y_tree, shortcuts) = capture_a11y_data();
            
            Ok(Response::new(GetFrameResponse {
                frame: Some(FrameData {
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as i64,
                    width: raw_frame.width as i32,
                    height: raw_frame.height as i32,
                    data: raw_frame.data,
                    format: 0, // RGBA/BGRA
                    sequence_number: raw_frame.sequence as i32,
                    accessibility_tree: a11y_tree,
                    discovered_shortcuts: shortcuts,
                }),
            }))
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(Status::unimplemented("GetFrame only implemented for Linux and macOS"))
        }
    }
    
    async fn stream_frames(
        &self,
        request: Request<StreamFramesRequest>,
    ) -> Result<Response<Self::StreamFramesStream>, Status> {
        let _req = request.into_inner();
        info!("StreamFrames called");
        
        let agent_arc = self.agent.clone();
        
        // Get broadcast receiver from stream manager
        let mut frame_rx = {
            if let Some(agent) = agent_arc.read().await.as_ref() {
                // Subscribe to frame stream from StreamManager
                match agent.subscribe_frames().await {
                    Ok(rx) => rx,
                    Err(e) => {
                        return Err(Status::internal(format!("Failed to subscribe to frame stream: {}", e)));
                    }
                }
            } else {
                return Err(Status::not_found("Agent not registered"));
            }
        };
        
        let stream = async_stream::stream! {
            info!("Frame streaming started (consuming from broadcast channel)");
            
            // Consume frames from broadcast channel
            loop {
                match frame_rx.recv().await {
                    Ok(encoded_frame) => {
                        // Convert EncodedFrame to FrameData for gRPC
                        // Only capture a11y on keyframes to reduce overhead
                        let (a11y_tree, shortcuts) = if encoded_frame.is_keyframe {
                            capture_a11y_data()
                        } else {
                            (None, vec![])
                        };
                        
                        let frame = FrameData {
                            timestamp: encoded_frame.timestamp_ms as i64,
                            width: encoded_frame.width as i32,
                            height: encoded_frame.height as i32,
                            data: encoded_frame.data,
                            format: 3, // H264 (matches FrameFormat enum in proto)
                            sequence_number: encoded_frame.sequence as i32,
                            accessibility_tree: a11y_tree,
                            discovered_shortcuts: shortcuts,
                        };
                        
                        debug!("Streaming frame {} ({} bytes, keyframe: {})", 
                               frame.sequence_number, frame.data.len(), encoded_frame.is_keyframe);
                        yield Ok(frame);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Client is too slow, some frames were skipped
                        debug!("Frame receiver lagged, skipped {} frames", n);
                        continue;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // Stream manager stopped, close stream
                        info!("Frame broadcast channel closed, ending stream");
                        break;
                    }
                }
            }
            
            info!("Frame streaming stopped");
        };
        
        Ok(Response::new(Box::pin(stream)))
    }
    
    async fn start_audio(
        &self,
        request: Request<StartAudioRequest>,
    ) -> Result<Response<StartAudioResponse>, Status> {
        let req = request.into_inner();
        info!("StartAudio called");
        
        if let Some(agent) = self.agent.read().await.as_ref() {
            let source = match req.source() {
                AudioSource::System => crate::audio::AudioSource::System,
                AudioSource::Application => {
                    crate::audio::AudioSource::Application(
                        req.application_id.unwrap_or_default()
                    )
                }
                AudioSource::Microphone => crate::audio::AudioSource::Microphone,
            };
            
            agent.start_audio(source).await
                .map_err(|e| Status::internal(e.to_string()))?;
            
            Ok(Response::new(StartAudioResponse {
                success: true,
                audio_id: format!("audio-{}", req.agent_id),
            }))
        } else {
            Err(Status::not_found("Agent not registered"))
        }
    }
    
    async fn stop_audio(
        &self,
        request: Request<StopAudioRequest>,
    ) -> Result<Response<StopAudioResponse>, Status> {
        let _req = request.into_inner();
        info!("StopAudio called");
        
        if let Some(agent) = self.agent.read().await.as_ref() {
            agent.stop_audio().await
                .map_err(|e| Status::internal(e.to_string()))?;
            
            Ok(Response::new(StopAudioResponse { success: true }))
        } else {
            Err(Status::not_found("Agent not registered"))
        }
    }
    
    async fn stream_audio(
        &self,
        request: Request<StreamAudioRequest>,
    ) -> Result<Response<Self::StreamAudioStream>, Status> {
        let _req = request.into_inner();
        info!("StreamAudio called");
        
        let agent_arc = self.agent.clone();
        
        // Create audio stream - default 100ms buffers
        let buffer_interval = Duration::from_millis(100);
        
        let stream = async_stream::stream! {
            let mut interval_timer = interval(buffer_interval);
            let mut sequence = 0u64;
            
            info!("Audio streaming started with 100ms buffers");
            
            loop {
                interval_timer.tick().await;
                
                // Get audio frame
                let audio_result = {
                    if let Some(agent) = agent_arc.read().await.as_ref() {
                        match agent.get_audio_frame().await {
                            Ok(audio_frame) => {
                                Some(audio_frame)
                            }
                            Err(e) => {
                                debug!("Audio capture error: {}", e);
                                None
                            }
                        }
                    } else {
                        None
                    }
                };
                
                if let Some(audio_frame) = audio_result {
                    sequence += 1;
                    
                    // Convert f32 samples to u8 bytes (16-bit PCM)
                    let mut byte_data = Vec::with_capacity(audio_frame.data.len() * 2);
                    for sample in audio_frame.data {
                        let sample_i16 = (sample * 32767.0) as i16;
                        byte_data.extend_from_slice(&sample_i16.to_le_bytes());
                    }
                    
                    let audio_data = AudioData {
                        timestamp: audio_frame.timestamp as i64,
                        data: byte_data,
                        format: 0, // PCM
                        sample_rate: audio_frame.sample_rate as i32,
                        channels: audio_frame.channels as i32,
                    };
                    
                    debug!("Streaming audio frame {}", sequence);
                    yield Ok(audio_data);
                } else {
                    // No audio available, skip
                    debug!("No audio available, skipping");
                }
            }
        };
        
        Ok(Response::new(Box::pin(stream)))
    }
    
    async fn inject_mouse_move(
        &self,
        request: Request<MouseMoveRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        let req = request.into_inner();
        info!("inject_mouse_move: x={}, y={}", req.x, req.y);
        
        match crate::input::inject_mouse_move(req.x, req.y) {
            Ok(()) => {
                info!("Mouse move successful");
                Ok(Response::new(InputResponse {
                    success: true,
                    error: None,
                }))
            }
            Err(e) => {
                error!("Mouse move failed: {}", e);
                Ok(Response::new(InputResponse {
                    success: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    async fn inject_mouse_click(
        &self,
        request: Request<MouseClickRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        let req = request.into_inner();
        let button = match req.button() {
            crate::proto_gen::agent::MouseButton::Left => "left",
            crate::proto_gen::agent::MouseButton::Right => "right",
            crate::proto_gen::agent::MouseButton::Middle => "middle",
        };
        
        info!("inject_mouse_click: x={}, y={}, button={}", req.x, req.y, button);
        
        match crate::input::inject_mouse_click(req.x, req.y, button) {
            Ok(()) => {
                info!("Mouse click successful");
                Ok(Response::new(InputResponse {
                    success: true,
                    error: None,
                }))
            }
            Err(e) => {
                error!("Mouse click failed: {}", e);
                Ok(Response::new(InputResponse {
                    success: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    async fn inject_key_press(
        &self,
        request: Request<KeyPressRequest>,
    ) -> Result<Response<InputResponse>, Status> {
        let req = request.into_inner();
        info!("inject_key_press: key={}, modifiers={:?}", req.key, req.modifiers);
        
        match crate::input::inject_key_press(&req.key, &req.modifiers) {
            Ok(()) => {
                info!("Key press successful");
                Ok(Response::new(InputResponse {
                    success: true,
                    error: None,
                }))
            }
            Err(e) => {
                error!("Key press failed: {}", e);
                Ok(Response::new(InputResponse {
                    success: false,
                    error: Some(e.to_string()),
                }))
            }
        }
    }
    
    async fn get_system_info(
        &self,
        request: Request<SystemInfoRequest>,
    ) -> Result<Response<SystemInfoResponse>, Status> {
        let _req = request.into_inner();
        
        let system_info = crate::platform::get_system_info()
            .map_err(|e| Status::internal(e.to_string()))?;
        
        Ok(Response::new(SystemInfoResponse {
            info: Some(SystemInfo {
                os: system_info.os,
                os_version: system_info.os_version,
                arch: system_info.arch,
                hostname: system_info.hostname,
                screen_width: system_info.screen_width as i32,
                screen_height: system_info.screen_height as i32,
                displays: vec![],
            }),
        }))
    }
    
    // OSWorld evaluator support methods
    
    async fn get_window_list(
        &self,
        request: Request<GetWindowListRequest>,
    ) -> Result<Response<GetWindowListResponse>, Status> {
        let _req = request.into_inner();
        info!("GetWindowList called");
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            let output = Command::new("osascript")
                .arg("-e")
                .arg(r#"tell application "System Events"
                    set windowList to {}
                    repeat with proc in (every process whose visible is true)
                        try
                            repeat with win in (every window of proc)
                                set end of windowList to name of win
                            end repeat
                        end try
                    end repeat
                    return windowList
                end tell"#)
                .output()
                .map_err(|e| Status::internal(format!("Failed to get windows: {}", e)))?;
            
            let windows_str = String::from_utf8_lossy(&output.stdout);
            let windows: Vec<String> = windows_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            info!("Found {} windows", windows.len());
            Ok(Response::new(GetWindowListResponse { windows }))
        }
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            // Use wmctrl to get window list on Linux
            let output = Command::new("wmctrl")
                .arg("-l")
                .output()
                .map_err(|e| Status::internal(format!("Failed to run wmctrl: {}", e)))?;
            
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(Status::internal(format!("wmctrl failed: {}", stderr)));
            }
            
            let stdout = String::from_utf8_lossy(&output.stdout);
            let windows: Vec<String> = stdout
                .lines()
                .filter(|line| !line.is_empty())
                .map(|line| {
                    // wmctrl format: 0x01234567  0 hostname Title of window
                    let parts: Vec<&str> = line.splitn(4, ' ').collect();
                    parts.get(3).unwrap_or(&"").to_string()
                })
                .collect();
            
            info!("Found {} windows", windows.len());
            Ok(Response::new(GetWindowListResponse { windows }))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(Status::unimplemented("GetWindowList only supported on macOS and Linux"))
        }
    }
    
    async fn get_process_list(
        &self,
        request: Request<GetProcessListRequest>,
    ) -> Result<Response<GetProcessListResponse>, Status> {
        let _req = request.into_inner();
        info!("GetProcessList called");
        
        use std::process::Command;
        
        let output = Command::new("ps")
            .arg("-eo")
            .arg("comm")
            .output()
            .map_err(|e| Status::internal(format!("Failed to get processes: {}", e)))?;
        
        let processes_str = String::from_utf8_lossy(&output.stdout);
        let processes: Vec<String> = processes_str
            .lines()
            .skip(1) // Skip header
            .map(|line| {
                // Get just the process name (last component of path)
                std::path::Path::new(line)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(line)
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect();
        
        info!("Found {} processes", processes.len());
        Ok(Response::new(GetProcessListResponse { processes }))
    }
    
    async fn get_browser_tabs(
        &self,
        request: Request<GetBrowserTabsRequest>,
    ) -> Result<Response<GetBrowserTabsResponse>, Status> {
        let req = request.into_inner();
        info!("GetBrowserTabs called for browser: {}", req.browser);
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            let script = match req.browser.as_str() {
                "chrome" => r#"
                    tell application "Google Chrome"
                        set tabList to {}
                        repeat with w in windows
                            repeat with t in tabs of w
                                set end of tabList to URL of t
                            end repeat
                        end repeat
                        return tabList
                    end tell
                "#,
                "safari" => r#"
                    tell application "Safari"
                        set tabList to {}
                        repeat with w in windows
                            repeat with t in tabs of w
                                set end of tabList to URL of t
                            end repeat
                        end repeat
                        return tabList
                    end tell
                "#,
                _ => return Err(Status::invalid_argument(format!("Unsupported browser: {}", req.browser))),
            };
            
            let output = Command::new("osascript")
                .arg("-e")
                .arg(script)
                .output()
                .map_err(|e| Status::internal(format!("Failed to get browser tabs: {}", e)))?;
            
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                return Err(Status::internal(format!("Browser not running or error: {}", err)));
            }
            
            let tabs_str = String::from_utf8_lossy(&output.stdout);
            let tabs: Vec<String> = tabs_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            
            info!("Found {} tabs in {}", tabs.len(), req.browser);
            Ok(Response::new(GetBrowserTabsResponse { tabs }))
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(Status::unimplemented("GetBrowserTabs only supported on macOS"))
        }
    }
    
    async fn list_files(
        &self,
        request: Request<ListFilesRequest>,
    ) -> Result<Response<ListFilesResponse>, Status> {
        let req = request.into_inner();
        info!("ListFiles called for directory: {}", req.directory);
        
        use std::process::Command;
        
        let output = Command::new("ls")
            .arg("-1")
            .arg(&req.directory)
            .output()
            .map_err(|e| Status::internal(format!("Failed to list files: {}", e)))?;
        
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(Status::internal(format!("Directory not found or error: {}", err)));
        }
        
        let files_str = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = files_str
            .lines()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect();
        
        info!("Found {} files in {}", files.len(), req.directory);
        Ok(Response::new(ListFilesResponse { files }))
    }
    
    async fn get_clipboard(
        &self,
        request: Request<GetClipboardRequest>,
    ) -> Result<Response<GetClipboardResponse>, Status> {
        let _req = request.into_inner();
        info!("GetClipboard called");
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            let output = Command::new("pbpaste")
                .output()
                .map_err(|e| Status::internal(format!("Failed to get clipboard: {}", e)))?;
            
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            
            info!("Clipboard content: {} bytes", content.len());
            Ok(Response::new(GetClipboardResponse { content }))
        }
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            let output = Command::new("xclip")
                .arg("-selection")
                .arg("clipboard")
                .arg("-o")
                .output()
                .map_err(|e| Status::internal(format!("Failed to get clipboard: {}", e)))?;
            
            let content = String::from_utf8_lossy(&output.stdout).to_string();
            
            info!("Clipboard content: {} bytes", content.len());
            Ok(Response::new(GetClipboardResponse { content }))
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Err(Status::unimplemented("GetClipboard not supported on this platform"))
        }
    }
    
    async fn launch_application(
        &self,
        request: Request<LaunchApplicationRequest>,
    ) -> Result<Response<LaunchApplicationResponse>, Status> {
        let req = request.into_inner();
        info!("LaunchApplication called: app_name={}", req.app_name);
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            // Map friendly names to actual Linux binaries
            let binary_name = match req.app_name.to_lowercase().as_str() {
                "calculator" | "calc" => "gnome-calculator",
                "terminal" | "term" => "gnome-terminal",
                "files" | "file-manager" | "nautilus" => "nautilus",
                "firefox" => "firefox",
                "chrome" | "google-chrome" => "google-chrome",
                "text-editor" | "gedit" => "gedit",
                "settings" => "gnome-control-center",
                _ => {
                    let err_msg = format!("Unknown application: {}", req.app_name);
                    error!("{}", err_msg);
                    return Ok(Response::new(LaunchApplicationResponse {
                        success: false,
                        error: err_msg,
                    }));
                }
            };
            
            info!("Launching binary: {}", binary_name);
            
            match Command::new(binary_name).spawn() {
                Ok(child) => {
                    info!("Successfully launched {} (PID: {})", binary_name, child.id());
                    Ok(Response::new(LaunchApplicationResponse {
                        success: true,
                        error: String::new(),
                    }))
                }
                Err(e) => {
                    let error_msg = format!("Failed to launch {}: {}", binary_name, e);
                    error!("{}", error_msg);
                    Ok(Response::new(LaunchApplicationResponse {
                        success: false,
                        error: error_msg,
                    }))
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            // Map friendly names to macOS application names
            let app_name = match req.app_name.to_lowercase().as_str() {
                "calculator" | "calc" => "Calculator",
                "terminal" | "term" => "Terminal",
                "safari" => "Safari",
                "firefox" => "Firefox",
                "chrome" | "google-chrome" => "Google Chrome",
                "text-editor" | "textedit" => "TextEdit",
                "finder" | "files" => "Finder",
                "settings" | "system-preferences" => "System Settings",
                _ => {
                    let err_msg = format!("Unknown application: {}", req.app_name);
                    error!("{}", err_msg);
                    return Ok(Response::new(LaunchApplicationResponse {
                        success: false,
                        error: err_msg,
                    }));
                }
            };
            
            info!("Launching application: {}", app_name);
            
            match Command::new("open").arg("-a").arg(app_name).spawn() {
                Ok(_) => {
                    info!("Successfully launched {}", app_name);
                    Ok(Response::new(LaunchApplicationResponse {
                        success: true,
                        error: String::new(),
                    }))
                }
                Err(e) => {
                    let error_msg = format!("Failed to launch {}: {}", app_name, e);
                    error!("{}", error_msg);
                    Ok(Response::new(LaunchApplicationResponse {
                        success: false,
                        error: error_msg,
                    }))
                }
            }
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(Status::unimplemented("LaunchApplication only supported on Linux and macOS"))
        }
    }
    
    async fn close_application(
        &self,
        request: Request<CloseApplicationRequest>,
    ) -> Result<Response<CloseApplicationResponse>, Status> {
        let req = request.into_inner();
        info!("CloseApplication called: app_name={}", req.app_name);
        
        #[cfg(target_os = "linux")]
        {
            use std::process::Command;
            
            info!("Closing window matching: {}", req.app_name);
            
            // Step 1: List all windows and find matching window ID
            match Command::new("wmctrl").arg("-l").output() {
                Ok(list_output) => {
                    let windows = String::from_utf8_lossy(&list_output.stdout);
                    
                    // Find window ID for matching title (case-insensitive)
                    let app_lower = req.app_name.to_lowercase();
                    let window_id = windows
                        .lines()
                        .find(|line| {
                            let line_lower = line.to_lowercase();
                            
                            // Special handling for terminal windows
                            if app_lower.contains("terminal") || app_lower == "gnome-terminal" {
                                // Terminal windows show as "user@host: ~" or similar patterns
                                line_lower.contains("@") && (line_lower.contains(": ~") || line_lower.contains(": /")) ||
                                line_lower.contains("terminal") ||
                                line_lower.contains("bash") ||
                                line_lower.contains("shell")
                            }
                            // Special handling for file manager
                            else if app_lower.contains("nautilus") || app_lower.contains("files") {
                                line_lower.contains("home") ||
                                line_lower.contains("documents") ||
                                line_lower.contains("downloads") ||
                                line_lower.contains("files") ||
                                line_lower.contains("nautilus")
                            }
                            // Special handling for settings
                            else if app_lower.contains("control-center") || app_lower.contains("settings") {
                                line_lower.contains("settings") ||
                                line_lower.contains("control center") ||
                                line_lower.contains("preferences")
                            }
                            // Default: look for app name in window title
                            else {
                                line_lower.contains(&app_lower)
                            }
                        })
                        .and_then(|line| line.split_whitespace().next())
                        .map(|s| s.to_string());
                    
                    if let Some(id) = window_id {
                        info!("Found window ID: {} for {}", id, req.app_name);
                        
                        // Step 2: Close by window ID using -ic (close immediately)
                        match Command::new("wmctrl").arg("-ic").arg(&id).output() {
                            Ok(close_output) => {
                                if close_output.status.success() {
                                    info!("Successfully closed window {} (ID: {})", req.app_name, id);
                                    Ok(Response::new(CloseApplicationResponse {
                                        success: true,
                                        error: String::new(),
                                    }))
                                } else {
                                    // Fallback: Try pkill if wmctrl fails
                                    info!("wmctrl failed, trying pkill for {}", req.app_name);
                                    
                                    // Use the actual process name for terminals
                                    let process_name = if req.app_name.contains("terminal") {
                                        "gnome-terminal"
                                    } else {
                                        &req.app_name
                                    };
                                    
                                    let _ = Command::new("pkill")
                                        .arg("-f")
                                        .arg(process_name)
                                        .output();
                                    
                                    Ok(Response::new(CloseApplicationResponse {
                                        success: true,
                                        error: String::new(),
                                    }))
                                }
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to close window: {}", e);
                                error!("{}", error_msg);
                                Ok(Response::new(CloseApplicationResponse {
                                    success: false,
                                    error: error_msg,
                                }))
                            }
                        }
                    } else {
                        // No window found, try pkill as fallback
                        info!("No window found for {}, trying pkill", req.app_name);
                        
                        // Use the actual process name for terminals
                        let process_name = if req.app_name.contains("terminal") {
                            "gnome-terminal"
                        } else {
                            &req.app_name
                        };
                        
                        match Command::new("pkill").arg("-f").arg(process_name).output() {
                            Ok(_) => {
                                info!("Sent kill signal to {}", req.app_name);
                                Ok(Response::new(CloseApplicationResponse {
                                    success: true,
                                    error: String::new(),
                                }))
                            }
                            Err(e) => {
                                let error_msg = format!("No window found and pkill failed: {}", e);
                                error!("{}", error_msg);
                                Ok(Response::new(CloseApplicationResponse {
                                    success: false,
                                    error: error_msg,
                                }))
                            }
                        }
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to list windows: {}", e);
                    error!("{}", error_msg);
                    Ok(Response::new(CloseApplicationResponse {
                        success: false,
                        error: error_msg,
                    }))
                }
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            
            info!("Closing application by name: {}", req.app_name);
            
            // Use AppleScript to quit the application gracefully
            let script = format!("tell application \"{}\" to quit", req.app_name);
            
            match Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .output()
            {
                Ok(output) => {
                    if output.status.success() {
                        info!("Successfully closed application: {}", req.app_name);
                        Ok(Response::new(CloseApplicationResponse {
                            success: true,
                            error: String::new(),
                        }))
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        let error_msg = format!("Failed to quit application: {}", stderr);
                        error!("{}", error_msg);
                        Ok(Response::new(CloseApplicationResponse {
                            success: false,
                            error: error_msg,
                        }))
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to execute osascript: {}", e);
                    error!("{}", error_msg);
                    Ok(Response::new(CloseApplicationResponse {
                        success: false,
                        error: error_msg,
                    }))
                }
            }
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            Err(Status::unimplemented("CloseApplication only supported on Linux and macOS"))
        }
    }
    
    async fn take_screenshot(
        &self,
        request: Request<TakeScreenshotRequest>,
    ) -> Result<Response<TakeScreenshotResponse>, Status> {
        use std::process::Command;
        
        let req = request.into_inner();
        info!("Taking screenshot for agent: {}", req.agent_id);
        
        // Generate default path if not provided
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        
        // Use actual user's home directory, fallback to /tmp
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let pictures_dir = format!("{}/Pictures", home_dir);
        let default_path = format!("{}/screenshot_{}.png", pictures_dir, timestamp);
        
        let save_path = if req.save_path.is_empty() { 
            default_path.clone()
        } else { 
            req.save_path.clone()
        };
        
        // Ensure Pictures directory exists
        if let Err(e) = std::fs::create_dir_all(&pictures_dir) {
            warn!("Failed to create Pictures directory: {}, using /tmp", e);
        }
        
        // Method 1: Try gnome-screenshot first (most reliable for GNOME desktop)
        match Command::new("gnome-screenshot")
            .arg("-f")  // File output
            .arg(&save_path)
            .env("DISPLAY", ":0")
            .output() {
            Ok(output) if output.status.success() => {
                info!("Screenshot saved using gnome-screenshot: {}", save_path);
                
                // Read the image data to include in response (optional)
                let image_data = std::fs::read(&save_path).unwrap_or_default();
                
                return Ok(Response::new(TakeScreenshotResponse {
                    success: true,
                    file_path: save_path,
                    error: String::new(),
                    image_data,
                }));
            }
            Ok(output) => {
                debug!("gnome-screenshot failed: {:?}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                debug!("gnome-screenshot not available: {}", e);
            }
        }
        
        // Method 2: Fallback to scrot (lightweight screenshot tool)
        match Command::new("scrot")
            .arg(&save_path)
            .env("DISPLAY", ":0")
            .output() {
            Ok(output) if output.status.success() => {
                info!("Screenshot saved using scrot: {}", save_path);
                
                // Read the image data to include in response (optional)
                let image_data = std::fs::read(&save_path).unwrap_or_default();
                
                return Ok(Response::new(TakeScreenshotResponse {
                    success: true,
                    file_path: save_path,
                    error: String::new(),
                    image_data,
                }));
            }
            Ok(output) => {
                debug!("scrot failed: {:?}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => {
                debug!("scrot not available: {}", e);
            }
        }
        
        // Method 3: Use existing GetFrame method to capture and save
        info!("Falling back to GetFrame method for screenshot");
        
        // Call our existing GetFrame implementation
        let frame_request = GetFrameRequest {
            agent_id: req.agent_id.clone(),
            capture_id: String::new(),
        };
        
        match self.get_frame(Request::new(frame_request)).await {
            Ok(frame_response) => {
                let frame_data = frame_response.into_inner().frame.map(|f| f.data).unwrap_or_default();
                
                // Save the screenshot data to file
                match std::fs::write(&save_path, &frame_data) {
                    Ok(_) => {
                        info!("Screenshot saved using GetFrame: {}", save_path);
                        Ok(Response::new(TakeScreenshotResponse {
                            success: true,
                            file_path: save_path,
                            error: String::new(),
                            image_data: frame_data,
                        }))
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to save screenshot: {}", e);
                        error!("{}", error_msg);
                        Ok(Response::new(TakeScreenshotResponse {
                            success: false,
                            file_path: String::new(),
                            error: error_msg,
                            image_data: vec![],
                        }))
                    }
                }
            }
            Err(e) => {
                let error_msg = format!("Failed to capture screenshot: {}", e);
                error!("{}", error_msg);
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

/// Capture accessibility tree and extract shortcuts
#[cfg(target_os = "linux")]
fn capture_a11y_data() -> (Option<String>, Vec<crate::proto_gen::agent::ShortcutInfo>) {
    use crate::a11y::A11yModule;
    
    let a11y = A11yModule::new();
    
    match a11y.discover_shortcuts() {
        Ok(shortcuts) => {
            let tree = crate::a11y::capture::capture_accessibility_tree().ok();
            let proto_shortcuts = shortcuts.into_iter().map(|s| {
                let is_single_key = s.normalized_keys.len() == 1;
                crate::proto_gen::agent::ShortcutInfo {
                    name: s.name,
                    raw_shortcut: s.raw_shortcut,
                    normalized_keys: s.normalized_keys,
                    command: s.command,
                    is_single_key,
                }
            }).collect();
            
            (tree, proto_shortcuts)
        }
        Err(e) => {
            tracing::warn!("A11y capture failed: {}", e);
            (None, vec![])
        }
    }
}

#[cfg(not(target_os = "linux"))]
fn capture_a11y_data() -> (Option<String>, Vec<crate::proto_gen::agent::ShortcutInfo>) {
    (None, vec![])
}
