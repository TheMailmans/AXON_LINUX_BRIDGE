/*!
 * Linux PulseAudio Capture Implementation
 * 
 * Low-level PulseAudio audio capture using Simple API.
 */

use anyhow::Result;
use std::ptr;
use std::sync::{Arc, Mutex};
use std::ffi::CString;
use tracing::{info, warn, error, debug};

use super::pulseaudio_ffi::*;
use super::ring_buffer::RingBuffer;

/// PulseAudio capture context
pub struct PulseAudioCapture {
    pa_simple: Option<*mut pa_simple>,
    sample_rate: u32,
    channels: u8,
    ring_buffer: Arc<RingBuffer>,
    is_running: bool,
    frames_captured: Arc<Mutex<u64>>,
}

impl PulseAudioCapture {
    /// Create new PulseAudio capture instance
    pub fn new(sample_rate: u32, channels: u8) -> Result<Self> {
        info!("Initializing PulseAudio capture: {}Hz, {}ch", sample_rate, channels);
        
        // Ring buffer size: 2 seconds of audio
        let buffer_size = (sample_rate * channels as u32 * 2) as usize;
        
        Ok(Self {
            pa_simple: None,
            sample_rate,
            channels,
            ring_buffer: Arc::new(RingBuffer::new(buffer_size)),
            is_running: false,
            frames_captured: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Start system audio capture (monitor mode)
    pub fn start_system_capture(&mut self) -> Result<()> {
        info!("Starting PulseAudio system monitor capture");
        
        unsafe {
            // Create sample spec
            let sample_spec = create_float32_sample_spec(self.sample_rate, self.channels);
            
            // Create channel map
            let channel_map = if self.channels == 2 {
                create_stereo_channel_map()
            } else {
                let mut map = pa_channel_map {
                    channels: self.channels,
                    map: [0; 32],
                };
                map.map[0] = PA_CHANNEL_POSITION_MONO;
                map
            };
            
            // Application name
            let app_name = CString::new("AxonHub Audio Capture").unwrap();
            
            // Stream name
            let stream_name = CString::new("System Audio Monitor").unwrap();
            
            // Monitor source (captures what's being played)
            // Using NULL device gets default monitor source
            let device = ptr::null();
            
            let mut error: i32 = 0;
            
            // Create PulseAudio simple connection
            let pa = pa_simple_new(
                ptr::null(), // Default server
                app_name.as_ptr() as *const i8,
                PA_STREAM_RECORD, // Recording
                device,
                stream_name.as_ptr() as *const i8,
                &sample_spec,
                &channel_map,
                ptr::null(), // Default buffer attributes
                &mut error,
            );
            
            if pa.is_null() {
                let error_str = pa_strerror(error);
                let error_msg = std::ffi::CStr::from_ptr(error_str as *const u8);
                anyhow::bail!("Failed to create PulseAudio connection: {}", 
                    error_msg.to_string_lossy());
            }
            
            self.pa_simple = Some(pa);
            self.is_running = true;
            
            info!("PulseAudio capture started successfully");
        }
        
        Ok(())
    }
    
    /// Start microphone capture
    pub fn start_microphone_capture(&mut self) -> Result<()> {
        info!("Starting PulseAudio microphone capture");
        
        // Similar to system capture but with default input device
        unsafe {
            let sample_spec = create_float32_sample_spec(self.sample_rate, self.channels);
            let channel_map = if self.channels == 2 {
                create_stereo_channel_map()
            } else {
                let mut map = pa_channel_map {
                    channels: self.channels,
                    map: [0; 32],
                };
                map.map[0] = PA_CHANNEL_POSITION_MONO;
                map
            };
            
            let app_name = CString::new("AxonHub Audio Capture").unwrap();
            let stream_name = CString::new("Microphone Input").unwrap();
            
            let mut error: i32 = 0;
            
            let pa = pa_simple_new(
                ptr::null(),
                app_name.as_ptr() as *const i8,
                PA_STREAM_RECORD,
                ptr::null(), // Default input device
                stream_name.as_ptr() as *const i8,
                &sample_spec,
                &channel_map,
                ptr::null(),
                &mut error,
            );
            
            if pa.is_null() {
                let error_str = pa_strerror(error);
                let error_msg = std::ffi::CStr::from_ptr(error_str as *const u8);
                anyhow::bail!("Failed to create PulseAudio connection: {}", 
                    error_msg.to_string_lossy());
            }
            
            self.pa_simple = Some(pa);
            self.is_running = true;
            
            info!("PulseAudio microphone capture started");
        }
        
        Ok(())
    }
    
    /// Read captured audio data
    pub fn read_samples(&self, num_samples: usize) -> Result<Option<Vec<f32>>> {
        if !self.is_running {
            return Ok(None);
        }
        
        unsafe {
            let pa = self.pa_simple.unwrap();
            
            // Allocate buffer
            let mut buffer = vec![0.0f32; num_samples];
            let bytes = num_samples * std::mem::size_of::<f32>();
            
            let mut error: i32 = 0;
            
            // Read audio data
            let result = pa_simple_read(
                pa,
                buffer.as_mut_ptr() as *mut std::ffi::c_void,
                bytes,
                &mut error,
            );
            
            if result < 0 {
                let error_str = pa_strerror(error);
                let error_msg = std::ffi::CStr::from_ptr(error_str as *const u8);
                return Err(anyhow::anyhow!("Failed to read audio: {}", 
                    error_msg.to_string_lossy()));
            }
            
            // Write to ring buffer
            let written = self.ring_buffer.write(&buffer);
            if written < num_samples {
                debug!("Ring buffer overflow: wrote {}/{} samples", written, num_samples);
            }
            
            // Update counter
            if let Ok(mut count) = self.frames_captured.lock() {
                *count += 1;
            }
            
            Ok(Some(buffer))
        }
    }
    
    /// Get captured audio frame from ring buffer
    pub fn get_frame(&self, frame_size: usize) -> Result<Vec<f32>> {
        let mut output = vec![0.0f32; frame_size];
        let read = self.ring_buffer.read(&mut output);
        
        if read < frame_size {
            debug!("Ring buffer underrun: got {}/{} samples", read, frame_size);
        }
        
        Ok(output)
    }
    
    /// Get number of samples available in buffer
    pub fn available_samples(&self) -> usize {
        self.ring_buffer.available()
    }
    
    /// Get number of frames captured
    pub fn get_frames_captured(&self) -> u64 {
        *self.frames_captured.lock().unwrap()
    }
    
    /// Get latency
    pub fn get_latency(&self) -> Result<u64> {
        if !self.is_running {
            return Ok(0);
        }
        
        unsafe {
            let pa = self.pa_simple.unwrap();
            let mut error: i32 = 0;
            
            let latency = pa_simple_get_latency(pa, &mut error);
            
            if error != 0 {
                let error_str = pa_strerror(error);
                let error_msg = std::ffi::CStr::from_ptr(error_str as *const u8);
                return Err(anyhow::anyhow!("Failed to get latency: {}", 
                    error_msg.to_string_lossy()));
            }
            
            Ok(latency)
        }
    }
    
    /// Stop capture
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }
        
        info!("Stopping PulseAudio capture");
        
        unsafe {
            if let Some(pa) = self.pa_simple.take() {
                pa_simple_free(pa);
            }
        }
        
        self.is_running = false;
        
        info!("PulseAudio capture stopped");
        
        Ok(())
    }
    
    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
}

impl Drop for PulseAudioCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

unsafe impl Send for PulseAudioCapture {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pulseaudio_capture_creation() {
        let capture = PulseAudioCapture::new(48000, 2);
        assert!(capture.is_ok());
        
        let cap = capture.unwrap();
        assert_eq!(cap.sample_rate, 48000);
        assert_eq!(cap.channels, 2);
        assert!(!cap.is_running);
    }

    #[test]
    fn test_pulseaudio_capture_buffer() {
        let capture = PulseAudioCapture::new(48000, 2).unwrap();
        
        // Get frame (should return silence when not running)
        let frame = capture.get_frame(1920).unwrap();
        assert_eq!(frame.len(), 1920);
    }

    #[test]
    fn test_pulseaudio_not_running() {
        let capture = PulseAudioCapture::new(48000, 2).unwrap();
        assert!(!capture.is_running());
        
        let result = capture.read_samples(1920).unwrap();
        assert!(result.is_none());
    }
}
