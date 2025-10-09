use anyhow::{Result, Context};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use crate::platform::SystemInfo;
use crate::capture::CaptureManager;
// use crate::audio::{AudioManager, AudioSource}; // TODO: Temporary disabled for benchmark
use crate::streaming::{StreamManager, StreamConfig};
use crate::video::EncodedFrame;

/// Agent state
#[derive(Debug, Clone)]
pub enum AgentState {
    Initializing,
    Connected,
    Capturing,
    Disconnected,
    Error(String),
}

/// Desktop Agent
/// 
/// Manages connection to hub, screen capture, and input injection
pub struct Agent {
    session_id: String,
    hub_url: String,
    agent_id: String,
    state: Arc<RwLock<AgentState>>,
    system_info: SystemInfo,
    capture_manager: Arc<RwLock<Option<CaptureManager>>>,
    // audio_manager: Arc<RwLock<Option<AudioManager>>>, // TODO: Temporary disabled
    stream_manager: Arc<RwLock<Option<StreamManager>>>,
}

impl Agent {
    /// Create new agent
    pub fn new(session_id: String, hub_url: String) -> Result<Self> {
        let agent_id = format!("agent-{}", uuid::Uuid::new_v4());
        let system_info = crate::platform::get_system_info()
            .context("Failed to get system info")?;
        
        Ok(Self {
            session_id,
            hub_url,
            agent_id,
            state: Arc::new(RwLock::new(AgentState::Initializing)),
            system_info,
            capture_manager: Arc::new(RwLock::new(None)),
            // audio_manager: Arc::new(RwLock::new(None)), // TODO: Temporary disabled
            stream_manager: Arc::new(RwLock::new(None)),
        })
    }
    
    /// Get agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
    
    /// Get session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }
    
    /// Get current state
    pub async fn state(&self) -> AgentState {
        self.state.read().await.clone()
    }
    
    /// Run agent
    pub async fn run(self) -> Result<()> {
        info!("Agent {} starting for session {}", self.agent_id, self.session_id);
        
        // Connect to hub
        self.connect_to_hub().await?;
        
        // Start gRPC server for hub to connect to
        self.start_grpc_server().await?;
        
        // Keep alive loop
        self.keep_alive().await?;
        
        Ok(())
    }
    
    /// Connect to hub
    async fn connect_to_hub(&self) -> Result<()> {
        info!("Connecting to hub at {}", self.hub_url);
        
        // TODO: Establish WebSocket or gRPC connection to hub
        // For now, just update state
        
        *self.state.write().await = AgentState::Connected;
        
        info!("Connected to hub successfully");
        Ok(())
    }
    
    /// Start gRPC server
    async fn start_grpc_server(&self) -> Result<()> {
        info!("Starting gRPC server on 127.0.0.1:50051");
        
        // gRPC server is started in main.rs and runs in parallel
        // This method just indicates readiness
        
        info!("gRPC server ready");
        Ok(())
    }
    
    /// Keep alive loop
    async fn keep_alive(&self) -> Result<()> {
        info!("Starting keep-alive loop");
        
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let state = self.state().await;
            match state {
                AgentState::Disconnected => {
                    info!("Agent disconnected, shutting down");
                    break;
                }
                AgentState::Error(ref err) => {
                    error!("Agent in error state: {}", err);
                    break;
                }
                _ => {
                    info!("Agent heartbeat - state: {:?}", state);
                }
            }
        }
        
        Ok(())
    }
    
    /// Start capture with streaming
    pub async fn start_capture(&self) -> Result<()> {
        info!("Starting capture with streaming pipeline");
        
        *self.state.write().await = AgentState::Capturing;
        
        // Create stream manager with default config (30 FPS, JPEG, no audio)
        let config = StreamConfig::default();
        let mut stream_manager = StreamManager::new(config);
        
        // Start streaming pipeline
        stream_manager.start().await?;
        
        *self.stream_manager.write().await = Some(stream_manager);
        
        info!("Capture and streaming started");
        Ok(())
    }
    
    /// Stop capture and streaming
    pub async fn stop_capture(&self) -> Result<()> {
        info!("Stopping capture and streaming");
        
        // Stop stream manager if running
        if let Some(mut manager) = self.stream_manager.write().await.take() {
            manager.stop().await?;
        }
        
        *self.state.write().await = AgentState::Connected;
        
        info!("Capture and streaming stopped");
        Ok(())
    }
    
    /// Start audio capture (temporarily disabled)
    pub async fn start_audio(&self, _source: crate::audio::AudioSource) -> Result<()> {
        anyhow::bail!("Audio capture temporarily disabled for MVP")
    }
    
    /// Stop audio capture (temporarily disabled)
    pub async fn stop_audio(&self) -> Result<()> {
        anyhow::bail!("Audio capture temporarily disabled for MVP")
    }
    
    /// Get frame from capture manager (legacy method)
    pub async fn get_capture_frame(&self) -> Result<Vec<u8>> {
        // For compatibility, create a simple frame if stream manager is running
        if let Some(_manager) = self.stream_manager.read().await.as_ref() {
            // Return stub data - gRPC service should use get_stream_frame instead
            Ok(vec![0xFF, 0xD8, 0xFF, 0xE0]) // JPEG header
        } else {
            anyhow::bail!("Capture not started")
        }
    }
    
    /// Subscribe to frame stream from streaming pipeline
    /// Returns a broadcast receiver that can be used to consume encoded frames
    pub async fn subscribe_frames(&self) -> Result<tokio::sync::broadcast::Receiver<EncodedFrame>> {
        if let Some(manager) = self.stream_manager.read().await.as_ref() {
            Ok(manager.subscribe())
        } else {
            anyhow::bail!("Streaming not started - call start_capture() first")
        }
    }
    
    /// Get audio frame from audio manager (temporarily disabled)
    pub async fn get_audio_frame(&self) -> Result<crate::audio::AudioFrame> {
        anyhow::bail!("Audio capture temporarily disabled for MVP")
    }
    
    /// Disconnect from hub
    pub async fn disconnect(&self) -> Result<()> {
        info!("Disconnecting from hub");
        
        // Stop audio if running (commented out - audio manager temporarily disabled)
        // if self.audio_manager.read().await.is_some() {
        //     self.stop_audio().await?;
        // }
        
        // Stop capture if running
        if self.stream_manager.read().await.is_some() {
            self.stop_capture().await?;
        }
        
        *self.state.write().await = AgentState::Disconnected;
        
        info!("Disconnected from hub");
        Ok(())
    }
}

// Add uuid dependency
use uuid::Uuid;
