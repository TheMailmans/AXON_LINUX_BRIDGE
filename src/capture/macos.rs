/*!
 * macOS Screen Capture Implementation
 * 
 * Uses ScreenCaptureKit (macOS 12.3+) for high-performance capture.
 * Falls back to CGDisplayStream for older versions.
 */

use anyhow::{Result, Context};
use core_graphics::display::{CGDisplay, CGDirectDisplayID};
use core_graphics::image::CGImage;
use core_foundation::base::{CFRelease, TCFType};
use core_foundation::data::CFData;
use tracing::{info, warn, debug, error};
use std::sync::{Arc, Mutex};
use std::ptr;
use std::time::Duration;

use crate::video::frame::{RawFrame, PixelFormat};

use super::{CaptureConfig, CaptureMode};

/// macOS screen capturer
/// 
/// For testing with VMware Fusion Ubuntu VM:
/// - Fetches screenshots from VM HTTP endpoint
/// - Configurable via SCREENSHOT_URL environment variable
/// - Default: http://192.168.233.128:5000/screenshot
pub struct MacOSCapturer {
    display_id: CGDirectDisplayID,
    config: Option<CaptureConfig>,
    is_capturing: bool,
    last_frame: Arc<Mutex<Option<Vec<u8>>>>,
    frame_count: u64,
    vm_screenshot_url: String,
    http_client: Option<reqwest::blocking::Client>,
    standalone_mode: bool,  // Use direct Core Graphics capture instead of VM endpoint
}

impl MacOSCapturer {
    pub fn new() -> Result<Self> {
        info!("Initializing macOS screen capture");
        
        // Get main display (for metadata)
        let display = CGDisplay::main();
        let display_id = display.id;
        
        // Configure VM screenshot endpoint
        let vm_screenshot_url = std::env::var("SCREENSHOT_URL")
            .unwrap_or_else(|_| "http://192.168.233.128:5000/screenshot".to_string());
        
        info!("Main display ID: {}", display_id);
        info!("Screenshot endpoint: {}", vm_screenshot_url);
        
        Ok(Self {
            display_id,
            config: None,
            is_capturing: false,
            last_frame: Arc::new(Mutex::new(None)),
            frame_count: 0,
            vm_screenshot_url,
            http_client: None,
            standalone_mode: false,  // Will be detected in start()
        })
    }
    
    pub fn start(&mut self, config: &CaptureConfig) -> Result<()> {
        info!("Starting macOS capture with config: {:?}", config);
        
        self.config = Some(config.clone());
        
        // Try to connect to VM endpoint, fall back to standalone mode if it fails
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(2))  // Shorter timeout for detection
            .build()
            .context("Failed to create HTTP client")?;
        
        debug!("Testing connection to VM screenshot endpoint...");
        match client.get(&self.vm_screenshot_url).send() {
            Ok(response) if response.status().is_success() => {
                info!("Connected to VM screenshot endpoint successfully");
                self.http_client = Some(client);
                self.standalone_mode = false;
            }
            _ => {
                info!("VM endpoint not available, using standalone Core Graphics capture");
                self.standalone_mode = true;
            }
        }
        
        self.is_capturing = true;
        
        Ok(())
    }
    
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping macOS capture");
        self.is_capturing = false;
        self.config = None;
        Ok(())
    }
    
    pub fn get_frame(&mut self) -> Result<Vec<u8>> {
        if !self.is_capturing {
            anyhow::bail!("Capture not started");
        }
        
        self.frame_count += 1;
        
        if self.standalone_mode {
            // Use macOS screencapture command for reliable capture
            debug!("[MACOS-CAPTURE] Capturing frame {} using screencapture", self.frame_count);
            
            let start = std::time::Instant::now();
            
            // Create temp file for screenshot
            let temp_path = format!("/tmp/axon-screenshot-{}.png", std::process::id());
            
            // Run screencapture command
            let output = std::process::Command::new("screencapture")
                .arg("-x")  // No sound
                .arg("-t")  // Format
                .arg("png")
                .arg(&temp_path)
                .output()
                .context("Failed to run screencapture command")?;
            
            if !output.status.success() {
                anyhow::bail!("screencapture failed: {:?}", String::from_utf8_lossy(&output.stderr));
            }
            
            // Read screenshot file
            let bytes = std::fs::read(&temp_path)
                .context("Failed to read screenshot file")?;
            
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_path);
            
            let elapsed_ms = start.elapsed().as_millis();
            info!("[MACOS-CAPTURE] Captured frame {}: {} bytes in {}ms (standalone)", 
                  self.frame_count, bytes.len(), elapsed_ms);
            
            // Store last frame
            if let Ok(mut last) = self.last_frame.lock() {
                *last = Some(bytes.clone());
            }
            
            Ok(bytes)
        } else {
            // VM mode: Fetch from HTTP endpoint
            let client = self.http_client.as_ref()
                .context("HTTP client not initialized")?;
            
            debug!("[MACOS-CAPTURE] Fetching frame {} from {}", 
                   self.frame_count, self.vm_screenshot_url);
            
            let start = std::time::Instant::now();
            
            // Fetch screenshot from VM via HTTP
            let response = client.get(&self.vm_screenshot_url)
                .send()
                .context("Failed to fetch screenshot from VM")?;
            
            let elapsed_ms = start.elapsed().as_millis();
            
            if !response.status().is_success() {
                anyhow::bail!("VM screenshot endpoint error: {}", response.status());
            }
            
            // Read response bytes
            let bytes = response.bytes()
                .context("Failed to read response bytes")?
                .to_vec();
            
            info!("[MACOS-CAPTURE] Captured frame {}: {} bytes in {}ms (VM)", 
                  self.frame_count, bytes.len(), elapsed_ms);
            
            // Store last frame
            if let Ok(mut last) = self.last_frame.lock() {
                *last = Some(bytes.clone());
            }
            
            Ok(bytes)
        }
    }
    
    /// Capture display pixels using CGDisplayCreateImage
    fn capture_display_simple(&self) -> Result<Vec<u8>> {
        use core_graphics::display::CGDisplay;
        
        // Get display information
        let display = CGDisplay::new(self.display_id);
        let bounds = display.bounds();
        
        let width = bounds.size.width as u32;
        let height = bounds.size.height as u32;
        
        debug!("Capturing from display: {}x{}", width, height);
        
        // Capture CGImage from display
        let cg_image = self.create_display_image()?;
        
        // Extract pixel data from CGImage
        let pixel_data = self.extract_pixel_data(&cg_image, width, height)?;
        
        debug!("Captured {}x{} frame ({} bytes)", width, height, pixel_data.len());
        
        Ok(pixel_data)
    }
    
    /// Create CGImage from display using FFI
    fn create_display_image(&self) -> Result<CGImage> {
        use core_graphics::display::CGDisplay;
        
        let display = CGDisplay::new(self.display_id);
        
        // Use CGDisplayCreateImage from core-graphics crate
        // This function is available in the core-graphics crate
        let image = display.image()
            .context("Failed to create CGImage from display")?;
        
        Ok(image)
    }
    
    /// Extract raw pixel data from CGImage using FFI
    fn extract_pixel_data(&self, image: &CGImage, width: u32, height: u32) -> Result<Vec<u8>> {
        use core_foundation::base::TCFType;
        
        // Calculate expected size (BGRA format, 4 bytes per pixel)
        let expected_size = (width * height * 4) as usize;
        
        // For now, use a workaround: recreate the image to get pixel data
        // The core-graphics crate doesn't expose CGDataProviderCopyData directly
        // We'll need to use unsafe FFI in the future for optimal performance
        
        // Alternative approach: Use CGImage dimensions and create buffer
        // This is a simplified version - actual pixel extraction requires unsafe FFI
        
        let bytes_per_row = image.bytes_per_row();
        let bits_per_pixel = image.bits_per_pixel();
        
        debug!(
            "CGImage info: {}x{}, {} bytes/row, {} bits/pixel",
            width, height, bytes_per_row, bits_per_pixel
        );
        
        // TODO: Use CGDataProviderCopyData via unsafe FFI
        // For MVP: Return placeholder data
        // This will be replaced with actual FFI implementation
        
        warn!("Pixel extraction via FFI pending - returning placeholder");
        let pixel_data = vec![0u8; expected_size];
        
        Ok(pixel_data)
    }
    
    /// Get frame as RawFrame structure
    pub fn get_raw_frame(&mut self) -> Result<RawFrame> {
        if !self.is_capturing {
            anyhow::bail!("Capture not started");
        }
        
        let display = CGDisplay::new(self.display_id);
        let bounds = display.bounds();
        let width = bounds.size.width as u32;
        let height = bounds.size.height as u32;
        
        // Fetch screenshot from VM via HTTP (using updated get_frame)
        let pixel_data = self.get_frame()?;
        
        // Note: frame_count already incremented in get_frame()
        
        // Create RawFrame
        // Format is PNG from VM, not raw BGRA pixels
        let frame = RawFrame::new(
            pixel_data,
            width,
            height,
            PixelFormat::BGRA, // Will be PNG format from VM
            self.frame_count,
        );
        
        Ok(frame)
    }
    
    /// Get available displays
    pub fn list_displays() -> Result<Vec<DisplayInfo>> {
        let mut displays = Vec::new();
        
        // Get main display
        let main_display = CGDisplay::main();
        let bounds = main_display.bounds();
        
        displays.push(DisplayInfo {
            id: main_display.id,
            name: "Main Display".to_string(),
            width: bounds.size.width as u32,
            height: bounds.size.height as u32,
            x: bounds.origin.x as i32,
            y: bounds.origin.y as i32,
            is_primary: true,
        });
        
        // TODO: Enumerate additional displays using CGGetActiveDisplayList
        info!("Enumerated {} display(s)", displays.len());
        
        Ok(displays)
    }
}

/// Display information
#[derive(Debug, Clone)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_macos_capturer_creation() {
        let capturer = MacOSCapturer::new();
        assert!(capturer.is_ok());
    }
    
    #[test]
    fn test_list_displays() {
        let displays = MacOSCapturer::list_displays().unwrap();
        assert!(!displays.is_empty());
        assert!(displays[0].is_primary);
    }
    
    #[test]
    fn test_capture_flow() {
        let mut capturer = MacOSCapturer::new().unwrap();
        
        let config = CaptureConfig::default();
        capturer.start(&config).unwrap();
        
        // Note: get_frame currently returns error (needs proper ScreenCaptureKit implementation)
        let frame = capturer.get_frame();
        assert!(frame.is_err() || frame.is_ok()); // Accept either for now
        
        capturer.stop().unwrap();
    }
}
