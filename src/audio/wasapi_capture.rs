/*!
 * Windows WASAPI Audio Capture Implementation
 * 
 * Low-level WASAPI audio capture using COM interfaces.
 */

use anyhow::Result;
use std::ptr;
use std::sync::{Arc, Mutex};
use tracing::{info, warn, error, debug};

use super::wasapi_ffi::*;
use super::ring_buffer::RingBuffer;

/// WASAPI capture context
pub struct WasapiCapture {
    device_enumerator: Option<*mut IMMDeviceEnumerator>,
    device: Option<*mut IMMDevice>,
    audio_client: Option<*mut IAudioClient>,
    capture_client: Option<*mut IAudioCaptureClient>,
    sample_rate: u32,
    channels: u32,
    ring_buffer: Arc<RingBuffer>,
    is_running: bool,
    frames_captured: Arc<Mutex<u64>>,
}

impl WasapiCapture {
    /// Create new WASAPI capture instance
    pub fn new(sample_rate: u32, channels: u32) -> Result<Self> {
        info!("Initializing WASAPI capture: {}Hz, {}ch", sample_rate, channels);
        
        // Ring buffer size: 2 seconds of audio
        let buffer_size = (sample_rate * channels * 2) as usize;
        
        Ok(Self {
            device_enumerator: None,
            device: None,
            audio_client: None,
            capture_client: None,
            sample_rate,
            channels,
            ring_buffer: Arc::new(RingBuffer::new(buffer_size)),
            is_running: false,
            frames_captured: Arc::new(Mutex::new(0)),
        })
    }
    
    /// Start system audio capture (loopback mode)
    pub fn start_system_capture(&mut self) -> Result<()> {
        info!("Starting WASAPI system loopback capture");
        
        unsafe {
            // Initialize COM
            let hr = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
            if hr != S_OK && hr != 0x00000001 { // S_FALSE = already initialized
                anyhow::bail!("Failed to initialize COM: {:x}", hr);
            }
            
            // Create device enumerator
            let mut enumerator: *mut IMMDeviceEnumerator = ptr::null_mut();
            let hr = CoCreateInstance(
                &CLSID_MMDeviceEnumerator,
                ptr::null_mut(),
                CLSCTX_ALL,
                &IID_IMMDeviceEnumerator,
                &mut enumerator as *mut *mut IMMDeviceEnumerator as *mut *mut std::ffi::c_void,
            );
            
            if hr != S_OK {
                CoUninitialize();
                anyhow::bail!("Failed to create device enumerator: {:x}", hr);
            }
            
            self.device_enumerator = Some(enumerator);
            info!("Created device enumerator");
            
            // Get default audio output device
            let mut device: *mut IMMDevice = ptr::null_mut();
            let hr = ((*(*enumerator).lpVtbl).GetDefaultAudioEndpoint)(
                enumerator,
                eRender, // Output device for loopback
                eConsole,
                &mut device,
            );
            
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to get default audio device: {:x}", hr);
            }
            
            self.device = Some(device);
            info!("Got default audio device");
            
            // Activate IAudioClient
            let mut audio_client: *mut IAudioClient = ptr::null_mut();
            let hr = ((*(*device).lpVtbl).Activate)(
                device,
                &IID_IAudioClient,
                CLSCTX_ALL,
                ptr::null_mut(),
                &mut audio_client as *mut *mut IAudioClient as *mut *mut std::ffi::c_void,
            );
            
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to activate audio client: {:x}", hr);
            }
            
            self.audio_client = Some(audio_client);
            info!("Activated audio client");
            
            // Get mix format
            let mut format_ptr: *mut WAVEFORMATEX = ptr::null_mut();
            let hr = ((*(*audio_client).lpVtbl).GetMixFormat)(
                audio_client,
                &mut format_ptr,
            );
            
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to get mix format: {:x}", hr);
            }
            
            let format = *format_ptr;
            info!("Mix format: {}Hz, {} channels", format.nSamplesPerSec, format.nChannels);
            
            // Initialize audio client in loopback mode
            let buffer_duration: i64 = 10000000; // 1 second in 100-nanosecond units
            let hr = ((*(*audio_client).lpVtbl).Initialize)(
                audio_client,
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                buffer_duration,
                0,
                format_ptr,
                ptr::null(),
            );
            
            CoTaskMemFree(format_ptr as *mut std::ffi::c_void);
            
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to initialize audio client: {:x}", hr);
            }
            
            info!("Initialized audio client in loopback mode");
            
            // Get capture client
            let mut capture_client: *mut IAudioCaptureClient = ptr::null_mut();
            let hr = ((*(*audio_client).lpVtbl).GetService)(
                audio_client,
                &IID_IAudioCaptureClient,
                &mut capture_client as *mut *mut IAudioCaptureClient as *mut *mut std::ffi::c_void,
            );
            
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to get capture client: {:x}", hr);
            }
            
            self.capture_client = Some(capture_client);
            info!("Got capture client");
            
            // Start audio client
            let hr = ((*(*audio_client).lpVtbl).Start)(audio_client);
            if hr != S_OK {
                self.cleanup();
                anyhow::bail!("Failed to start audio client: {:x}", hr);
            }
            
            self.is_running = true;
            info!("WASAPI capture started successfully");
        }
        
        Ok(())
    }
    
    /// Read captured audio data
    pub fn read_samples(&self) -> Result<Option<Vec<f32>>> {
        if !self.is_running {
            return Ok(None);
        }
        
        unsafe {
            let capture_client = self.capture_client.unwrap();
            
            // Get next packet size
            let mut packet_size: u32 = 0;
            let hr = ((*(*capture_client).lpVtbl).GetNextPacketSize)(
                capture_client,
                &mut packet_size,
            );
            
            if hr != S_OK {
                return Err(anyhow::anyhow!("Failed to get packet size: {:x}", hr));
            }
            
            if packet_size == 0 {
                return Ok(None);
            }
            
            // Get buffer
            let mut data: *mut u8 = ptr::null_mut();
            let mut num_frames: u32 = 0;
            let mut flags: u32 = 0;
            
            let hr = ((*(*capture_client).lpVtbl).GetBuffer)(
                capture_client,
                &mut data,
                &mut num_frames,
                &mut flags,
                ptr::null_mut(),
                ptr::null_mut(),
            );
            
            if hr != S_OK {
                return Err(anyhow::anyhow!("Failed to get buffer: {:x}", hr));
            }
            
            // Convert to f32 samples
            let num_samples = (num_frames * self.channels) as usize;
            let samples = std::slice::from_raw_parts(data as *const f32, num_samples);
            
            // Write to ring buffer
            let written = self.ring_buffer.write(samples);
            if written < num_samples {
                debug!("Ring buffer overflow: wrote {}/{} samples", written, num_samples);
            }
            
            // Update counter
            if let Ok(mut count) = self.frames_captured.lock() {
                *count += 1;
            }
            
            // Release buffer
            let hr = ((*(*capture_client).lpVtbl).ReleaseBuffer)(
                capture_client,
                num_frames,
            );
            
            if hr != S_OK {
                warn!("Failed to release buffer: {:x}", hr);
            }
            
            Ok(Some(samples.to_vec()))
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
    
    /// Stop capture
    pub fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }
        
        info!("Stopping WASAPI capture");
        
        unsafe {
            if let Some(audio_client) = self.audio_client {
                let hr = ((*(*audio_client).lpVtbl).Stop)(audio_client);
                if hr != S_OK {
                    warn!("Failed to stop audio client: {:x}", hr);
                }
            }
        }
        
        self.cleanup();
        self.is_running = false;
        
        info!("WASAPI capture stopped");
        
        Ok(())
    }
    
    /// Check if capture is running
    pub fn is_running(&self) -> bool {
        self.is_running
    }
    
    /// Cleanup COM resources
    fn cleanup(&mut self) {
        unsafe {
            if let Some(capture_client) = self.capture_client.take() {
                ((*(*capture_client).lpVtbl).parent.Release)(capture_client as *mut IUnknown);
            }
            
            if let Some(audio_client) = self.audio_client.take() {
                ((*(*audio_client).lpVtbl).parent.Release)(audio_client as *mut IUnknown);
            }
            
            if let Some(device) = self.device.take() {
                ((*(*device).lpVtbl).parent.Release)(device as *mut IUnknown);
            }
            
            if let Some(enumerator) = self.device_enumerator.take() {
                ((*(*enumerator).lpVtbl).parent.Release)(enumerator as *mut IUnknown);
            }
            
            CoUninitialize();
        }
    }
}

impl Drop for WasapiCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

unsafe impl Send for WasapiCapture {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasapi_capture_creation() {
        let capture = WasapiCapture::new(48000, 2);
        assert!(capture.is_ok());
        
        let cap = capture.unwrap();
        assert_eq!(cap.sample_rate, 48000);
        assert_eq!(cap.channels, 2);
        assert!(!cap.is_running);
    }

    #[test]
    fn test_wasapi_capture_buffer() {
        let capture = WasapiCapture::new(48000, 2).unwrap();
        
        // Get frame (should return silence when not running)
        let frame = capture.get_frame(1920).unwrap();
        assert_eq!(frame.len(), 1920);
    }
}
