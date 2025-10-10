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
use tracing::{info, error, debug};

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
            
            let mut capturer = LinuxCapturer::new()
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
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(Status::unimplemented("GetWindowList only supported on macOS"))
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
                crate::proto_gen::agent::ShortcutInfo {
                    name: s.name,
                    raw_shortcut: s.raw_shortcut,
                    normalized_keys: s.normalized_keys,
                    command: s.command,
                    is_single_key: s.normalized_keys.len() == 1,
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
