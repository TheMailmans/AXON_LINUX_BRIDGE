//! Linux Screen Capture
//!
//! Uses OSWorld's Python controller for reliable screenshot capture.

use anyhow::{Result, bail};
use tracing::{info, debug, warn};
use super::{CaptureConfig, CaptureMode};
use crate::video::{RawFrame, PixelFormat};
use std::process::Command;

pub struct LinuxCapturer {
    is_capturing: bool,
    width: u32,
    height: u32,
    frame_count: u64,
}

impl LinuxCapturer {
    pub fn new() -> Result<Self> {
        info!("Creating Linux capturer");
        
        // Detect screen resolution
        let (width, height) = Self::detect_screen_size().unwrap_or((1920, 1080));
        info!("Detected screen size: {}x{}", width, height);
        
        Ok(Self {
            is_capturing: false,
            width,
            height,
            frame_count: 0,
        })
    }

    pub fn start(&mut self, _config: &CaptureConfig) -> Result<()> {
        info!("Starting Linux screen capture");
        self.is_capturing = true;
        Ok(())
    }

    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping Linux screen capture");
        self.is_capturing = false;
        Ok(())
    }

    pub fn capture_frame(&mut self) -> Result<RawFrame> {
        if !self.is_capturing {
            bail!("Not capturing - call start() first");
        }
        
        self.frame_count += 1;
        
        // Capture using OSWorld Python controller (proven to work)
        let data = Self::capture_via_python_controller()?;
        
        debug!("Captured frame {}: {} bytes", self.frame_count, data.len());
        
        Ok(RawFrame {
            data,
            width: self.width,
            height: self.height,
            format: PixelFormat::BGRA, // Python controller returns PNG
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            sequence: self.frame_count,
        })
    }

    pub fn get_raw_frame(&mut self) -> Result<RawFrame> {
        // For GetFrame RPC, we don't need is_capturing check
        // Just capture on-demand
        self.frame_count += 1;
        
        let data = Self::capture_via_python_controller()?;
        
        debug!("Captured on-demand frame: {} bytes", data.len());
        
        Ok(RawFrame {
            data,
            width: self.width,
            height: self.height,
            format: PixelFormat::BGRA,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            sequence: self.frame_count,
        })
    }

    pub fn is_capturing(&self) -> bool {
        self.is_capturing
    }
    
    /// Capture screenshot using OSWorld Python controller
    /// 
    /// This is the most reliable method as it reuses OSWorld's proven capture code.
    fn capture_via_python_controller() -> Result<Vec<u8>> {
        debug!("Capturing screenshot via Python controller");
        
        let output = Command::new("python3")
            .arg("-c")
            .arg(r#"
import sys
from desktop_env.controllers.python import PythonController
controller = PythonController()
screenshot = controller.get_screenshot()
sys.stdout.buffer.write(screenshot)
"#)
            .output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Python screenshot capture failed: {}", stderr);
        }
        
        if output.stdout.is_empty() {
            bail!("Python returned empty screenshot");
        }
        
        debug!("Python capture successful: {} bytes", output.stdout.len());
        Ok(output.stdout)
    }
    
    /// Detect screen size using xdpyinfo
    fn detect_screen_size() -> Result<(u32, u32)> {
        let output = Command::new("xdpyinfo")
            .output();
        
        if let Ok(output) = output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // Parse: dimensions: 1920x1080 pixels
            for line in stdout.lines() {
                if line.contains("dimensions:") {
                    if let Some(dims) = line.split_whitespace().nth(1) {
                        let parts: Vec<&str> = dims.split('x').collect();
                        if parts.len() == 2 {
                            if let (Ok(w), Ok(h)) = (parts[0].parse(), parts[1].parse()) {
                                return Ok((w, h));
                            }
                        }
                    }
                }
            }
        }
        
        warn!("Could not detect screen size, using default 1920x1080");
        Ok((1920, 1080))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    #[ignore] // Only run on Linux with OSWorld environment
    fn test_python_capture() {
        let result = LinuxCapturer::capture_via_python_controller();
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.len() > 10000); // Should be at least 10KB for a screenshot
    }
}
