/*!
 * Video Frame Representation
 * 
 * Raw and encoded frame structures for video processing.
 */

use std::time::{SystemTime, UNIX_EPOCH};

/// Pixel format for raw frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// BGRA 8-bit per channel (common on macOS)
    BGRA,
    /// RGBA 8-bit per channel
    RGBA,
    /// RGB 24-bit
    RGB24,
    /// YUV 4:2:0 planar (common for video encoding)
    YUV420,
}

impl PixelFormat {
    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelFormat::BGRA | PixelFormat::RGBA => 4,
            PixelFormat::RGB24 => 3,
            PixelFormat::YUV420 => 1, // Y plane only
        }
    }
    
    /// Check if this format has an alpha channel
    pub fn has_alpha(&self) -> bool {
        matches!(self, PixelFormat::BGRA | PixelFormat::RGBA)
    }
}

/// Raw uncompressed video frame
#[derive(Debug, Clone)]
pub struct RawFrame {
    /// Frame pixel data
    pub data: Vec<u8>,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Pixel format
    pub format: PixelFormat,
    /// Timestamp in milliseconds since epoch
    pub timestamp_ms: u64,
    /// Frame sequence number
    pub sequence: u64,
}

impl RawFrame {
    /// Create a new raw frame
    pub fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        format: PixelFormat,
        sequence: u64,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            data,
            width,
            height,
            format,
            timestamp_ms,
            sequence,
        }
    }
    
    /// Get expected data size for this frame
    pub fn expected_size(&self) -> usize {
        match self.format {
            PixelFormat::YUV420 => {
                // Y plane + U plane + V plane
                let y_size = (self.width * self.height) as usize;
                let uv_size = (self.width * self.height / 4) as usize;
                y_size + uv_size * 2
            }
            _ => {
                (self.width * self.height) as usize * self.format.bytes_per_pixel()
            }
        }
    }
    
    /// Validate frame data size
    pub fn is_valid(&self) -> bool {
        self.data.len() == self.expected_size()
    }
    
    /// Convert BGRA to RGBA
    pub fn bgra_to_rgba(&self) -> Option<RawFrame> {
        if self.format != PixelFormat::BGRA {
            return None;
        }
        
        let mut rgba_data = Vec::with_capacity(self.data.len());
        for chunk in self.data.chunks_exact(4) {
            rgba_data.push(chunk[2]); // R = B
            rgba_data.push(chunk[1]); // G = G
            rgba_data.push(chunk[0]); // B = R
            rgba_data.push(chunk[3]); // A = A
        }
        
        Some(RawFrame {
            data: rgba_data,
            width: self.width,
            height: self.height,
            format: PixelFormat::RGBA,
            timestamp_ms: self.timestamp_ms,
            sequence: self.sequence,
        })
    }
    
    /// Convert to YUV420 (simple conversion, not optimized)
    pub fn to_yuv420(&self) -> Option<RawFrame> {
        if !matches!(self.format, PixelFormat::BGRA | PixelFormat::RGBA | PixelFormat::RGB24) {
            return None;
        }
        
        let pixel_size = self.format.bytes_per_pixel();
        let pixels = self.width * self.height;
        
        // Calculate YUV420 plane sizes
        let y_size = pixels as usize;
        let uv_size = (pixels / 4) as usize;
        let total_size = y_size + uv_size * 2;
        
        let mut yuv_data = vec![0u8; total_size];
        
        // Convert RGB to YUV using BT.601 coefficients
        for y in 0..self.height {
            for x in 0..self.width {
                let idx = ((y * self.width + x) * pixel_size as u32) as usize;
                
                let (r, g, b) = match self.format {
                    PixelFormat::BGRA | PixelFormat::RGBA => {
                        let r = if self.format == PixelFormat::RGBA { self.data[idx] } else { self.data[idx + 2] };
                        let g = self.data[idx + 1];
                        let b = if self.format == PixelFormat::RGBA { self.data[idx + 2] } else { self.data[idx] };
                        (r, g, b)
                    }
                    PixelFormat::RGB24 => {
                        (self.data[idx], self.data[idx + 1], self.data[idx + 2])
                    }
                    _ => unreachable!(),
                };
                
                // Y = 0.299*R + 0.587*G + 0.114*B
                let y_val = ((66 * r as u32 + 129 * g as u32 + 25 * b as u32 + 128) >> 8) + 16;
                yuv_data[(y * self.width + x) as usize] = y_val.min(255) as u8;
                
                // Subsample UV (every 2x2 block)
                if x % 2 == 0 && y % 2 == 0 {
                    let uv_idx = (y / 2 * self.width / 2 + x / 2) as usize;
                    
                    // U = -0.169*R - 0.331*G + 0.500*B + 128
                    let u_val = ((-38 * r as i32 - 74 * g as i32 + 112 * b as i32 + 128) >> 8) + 128;
                    yuv_data[y_size + uv_idx] = u_val.clamp(0, 255) as u8;
                    
                    // V = 0.500*R - 0.419*G - 0.081*B + 128
                    let v_val = ((112 * r as i32 - 94 * g as i32 - 18 * b as i32 + 128) >> 8) + 128;
                    yuv_data[y_size + uv_size + uv_idx] = v_val.clamp(0, 255) as u8;
                }
            }
        }
        
        Some(RawFrame {
            data: yuv_data,
            width: self.width,
            height: self.height,
            format: PixelFormat::YUV420,
            timestamp_ms: self.timestamp_ms,
            sequence: self.sequence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pixel_format() {
        assert_eq!(PixelFormat::BGRA.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::RGBA.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::RGB24.bytes_per_pixel(), 3);
        assert!(PixelFormat::BGRA.has_alpha());
        assert!(!PixelFormat::RGB24.has_alpha());
    }

    #[test]
    fn test_raw_frame_creation() {
        let data = vec![0u8; 1920 * 1080 * 4];
        let frame = RawFrame::new(data, 1920, 1080, PixelFormat::BGRA, 0);
        
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.format, PixelFormat::BGRA);
        assert!(frame.is_valid());
    }

    #[test]
    fn test_bgra_to_rgba() {
        let data = vec![10, 20, 30, 40, 255, 128, 64, 255]; // 2 BGRA pixels (B,G,R,A)
        let frame = RawFrame::new(data, 2, 1, PixelFormat::BGRA, 0);
        
        let rgba = frame.bgra_to_rgba().unwrap();
        assert_eq!(rgba.format, PixelFormat::RGBA);
        assert_eq!(rgba.data[0], 30); // R (was B)
        assert_eq!(rgba.data[1], 20); // G (was G)
        assert_eq!(rgba.data[2], 10); // B (was R)
        assert_eq!(rgba.data[3], 40); // A (was A)
    }

    #[test]
    fn test_frame_validation() {
        let data = vec![0u8; 100]; // Too small
        let frame = RawFrame::new(data, 1920, 1080, PixelFormat::BGRA, 0);
        assert!(!frame.is_valid());
    }
}
