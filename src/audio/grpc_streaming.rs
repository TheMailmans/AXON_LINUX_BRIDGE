/*!
 * gRPC Audio Streaming Service
 * 
 * Implements the audio streaming gRPC service for the desktop agent.
 */

use anyhow::Result;
use tokio::sync::broadcast;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use tracing::{info, warn, debug};

use super::{EncodedAudioFrame};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum AudioSource {
    System,
    Microphone,
    Application,
}

// Import generated proto types
// use crate::proto::agent::{
//     AudioStreamRequest, AudioFrame, AudioSource as ProtoAudioSource,
//     AudioLevelRequest, AudioLevelResponse, MuteRequest, MuteResponse,
// };

/// Audio streaming service implementation
pub struct AudioStreamingService {
    // Audio frame broadcaster
    audio_tx: broadcast::Sender<EncodedAudioFrame>,
}

impl AudioStreamingService {
    /// Create new audio streaming service
    pub fn new(audio_tx: broadcast::Sender<EncodedAudioFrame>) -> Self {
        info!("Creating audio streaming service");
        Self { audio_tx }
    }
    
    /// Start audio streaming
    pub async fn stream_audio(
        &self,
        _request: Request<()>, // AudioStreamRequest when proto is compiled
    ) -> Result<Response<ReceiverStream<EncodedAudioFrame>>, Status> {
        info!("Starting audio stream");
        
        // Subscribe to audio frames and convert to channel
        let mut rx = self.audio_tx.subscribe();
        let (tx, receiver) = tokio::sync::mpsc::channel(16);
        
        // Spawn task to forward broadcast to channel
        tokio::spawn(async move {
            while let Ok(frame) = rx.recv().await {
                if tx.send(frame).await.is_err() {
                    break;
                }
            }
        });
        
        let stream = ReceiverStream::new(receiver);
        Ok(Response::new(stream))
    }
    
    /// Set audio level
    pub async fn set_audio_level(
        &self,
        level: f32,
    ) -> Result<(), Status> {
        info!("Setting audio level to {}", level);
        
        if level < 0.0 || level > 1.0 {
            return Err(Status::invalid_argument("Level must be between 0.0 and 1.0"));
        }
        
        // TODO: Implement actual volume control
        warn!("Audio level control not yet implemented");
        
        Ok(())
    }
    
    /// Mute/unmute audio
    pub async fn mute_audio(
        &self,
        muted: bool,
    ) -> Result<bool, Status> {
        info!("Setting audio mute to {}", muted);
        
        // TODO: Implement actual mute control
        warn!("Audio mute control not yet implemented");
        
        Ok(muted)
    }
}

/// Audio streaming statistics
#[derive(Debug, Clone)]
pub struct AudioStreamStats {
    pub active_streams: u32,
    pub frames_sent: u64,
    pub bytes_sent: u64,
    pub dropped_frames: u64,
}

impl Default for AudioStreamStats {
    fn default() -> Self {
        Self {
            active_streams: 0,
            frames_sent: 0,
            bytes_sent: 0,
            dropped_frames: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_streaming_service_creation() {
        let (tx, _rx) = broadcast::channel(16);
        let service = AudioStreamingService::new(tx);
        
        // Just verify it was created
        assert!(std::ptr::addr_of!(service) as usize != 0);
    }

    #[tokio::test]
    async fn test_set_audio_level_validation() {
        let (tx, _rx) = broadcast::channel(16);
        let service = AudioStreamingService::new(tx);
        
        // Valid level
        assert!(service.set_audio_level(0.5).await.is_ok());
        
        // Invalid levels
        assert!(service.set_audio_level(-0.1).await.is_err());
        assert!(service.set_audio_level(1.5).await.is_err());
    }

    #[tokio::test]
    async fn test_mute_audio() {
        let (tx, _rx) = broadcast::channel(16);
        let service = AudioStreamingService::new(tx);
        
        // Mute
        let result = service.mute_audio(true).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);
        
        // Unmute
        let result = service.mute_audio(false).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }
}
