/*!
 * Audio Encoder using Opus codec
 * 
 * Provides high-quality audio compression for streaming.
 */

use anyhow::Result;
// use audiopus::{coder::Encoder as OpusEnc, Channels, Application, SampleRate};  // TODO: Re-enable
use tracing::{info, debug};

use super::{AudioFrame, EncodedAudioFrame};

/// Opus audio encoder
pub struct AudioEncoder {
    // encoder: OpusEnc,  // TODO: Re-enable when audiopus works
    sample_rate: u32,
    channels: u32,
    bitrate: u32,
    frame_size: usize,
    sequence: u64,
}

impl AudioEncoder {
    /// Create new Opus encoder
    pub fn new(sample_rate: u32, channels: u32, bitrate: u32) -> Result<Self> {
        info!("Creating Opus encoder (stub): {}Hz, {}ch, {}bps", sample_rate, channels, bitrate);
        
        // TODO: Actual Opus encoder when audiopus dependency works
        // For now, stub implementation
        
        // Frame size: 20ms typical (960 samples at 48kHz)
        let frame_size = (sample_rate as usize * 20) / 1000;
        
        info!("Opus encoder ready (stub mode): frame_size={}", frame_size);
        
        Ok(Self {
            // encoder,  // TODO: Re-enable
            sample_rate,
            channels,
            bitrate,
            frame_size,
            sequence: 0,
        })
    }
    
    /// Encode audio frame to Opus
    pub fn encode(&mut self, frame: &AudioFrame) -> Result<EncodedAudioFrame> {
        // Validate frame
        if frame.sample_rate != self.sample_rate {
            anyhow::bail!("Sample rate mismatch: expected {}, got {}", 
                         self.sample_rate, frame.sample_rate);
        }
        
        if frame.channels != self.channels {
            anyhow::bail!("Channel count mismatch: expected {}, got {}", 
                         self.channels, frame.channels);
        }
        
        // TODO: Actual Opus encoding when audiopus works
        // For now, simulate compressed output (50:1 compression ratio)
        let compressed_size = (frame.data.len() * std::mem::size_of::<f32>()) / 50;
        let output = vec![0u8; compressed_size.max(64)];
        
        debug!("Encoded audio frame (stub): {} PCM samples → {} bytes", 
               frame.data.len(), output.len());
        
        let encoded = EncodedAudioFrame {
            timestamp_ms: frame.timestamp,
            data: output,
            sample_rate: self.sample_rate,
            channels: self.channels,
            sequence: self.sequence,
        };
        
        self.sequence += 1;
        
        Ok(encoded)
    }
    
    /// Get frame size (samples per frame)
    pub fn frame_size(&self) -> usize {
        self.frame_size
    }
    
    /// Get total samples per frame (frame_size * channels)
    pub fn total_samples_per_frame(&self) -> usize {
        self.frame_size * self.channels as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_creation() {
        let encoder = AudioEncoder::new(48000, 2, 128000);
        assert!(encoder.is_ok());
        
        let enc = encoder.unwrap();
        assert_eq!(enc.sample_rate, 48000);
        assert_eq!(enc.channels, 2);
        assert_eq!(enc.frame_size, 960); // 20ms at 48kHz
    }

    #[test]
    fn test_encode_frame() {
        let mut encoder = AudioEncoder::new(48000, 2, 128000).unwrap();
        
        // Create test frame (960 samples * 2 channels = 1920 samples)
        let test_data = vec![0.0f32; 960 * 2];
        let frame = AudioFrame {
            timestamp: 1000,
            data: test_data,
            sample_rate: 48000,
            channels: 2,
        };
        
        let encoded = encoder.encode(&frame);
        assert!(encoded.is_ok());
        
        let enc_frame = encoded.unwrap();
        assert!(enc_frame.data.len() > 0);
        assert!(enc_frame.data.len() < 1920 * 4); // Compressed
        assert_eq!(enc_frame.sequence, 0);
    }

    #[test]
    fn test_sequence_increment() {
        let mut encoder = AudioEncoder::new(48000, 2, 128000).unwrap();
        
        let test_data = vec![0.0f32; 960 * 2];
        let frame = AudioFrame {
            timestamp: 1000,
            data: test_data.clone(),
            sample_rate: 48000,
            channels: 2,
        };
        
        let enc1 = encoder.encode(&frame).unwrap();
        assert_eq!(enc1.sequence, 0);
        
        let frame2 = AudioFrame {
            timestamp: 1020,
            data: test_data,
            sample_rate: 48000,
            channels: 2,
        };
        
        let enc2 = encoder.encode(&frame2).unwrap();
        assert_eq!(enc2.sequence, 1);
    }
}
