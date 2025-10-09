/*!
 * Video Encoding Module
 * 
 * Handles video encoding for desktop capture streams.
 * Supports H.264 encoding with hardware acceleration when available.
 */

pub mod encoder;
pub mod frame;

#[cfg(target_os = "macos")]
pub mod videotoolbox_ffi;

pub use encoder::{VideoEncoder, EncoderConfig, EncodedFrame, create_encoder};
pub use frame::{RawFrame, PixelFormat};

/// Video encoding quality preset
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// Low quality, high compression (good for slow networks)
    Low,
    /// Medium quality, balanced (default)
    Medium,
    /// High quality, low compression (good for fast networks)
    High,
    /// Custom bitrate in kbps
    Custom(u32),
}

impl Quality {
    /// Get bitrate in kbps for this quality level
    pub fn bitrate_kbps(&self, width: u32, height: u32) -> u32 {
        let pixels = width * height;
        match self {
            Quality::Low => {
                // ~1 Mbps for 1080p, scales with resolution
                (pixels / 2000).max(500)
            }
            Quality::Medium => {
                // ~2.5 Mbps for 1080p
                (pixels * 3 / 2000).max(1000)
            }
            Quality::High => {
                // ~5 Mbps for 1080p
                (pixels * 5 / 2000).max(2000)
            }
            Quality::Custom(bitrate) => *bitrate,
        }
    }
}

/// Video codec
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// H.264/AVC codec (widely supported)
    H264,
    /// H.265/HEVC codec (better compression, less support)
    H265,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_bitrate() {
        // 1920x1080 (1080p) = 2,073,600 pixels
        // Low: 2073600 / 2000 = 1036 kbps
        assert_eq!(Quality::Low.bitrate_kbps(1920, 1080), 1036);
        // Medium: 2073600 * 3 / 2000 = 3110 kbps
        assert_eq!(Quality::Medium.bitrate_kbps(1920, 1080), 3110);
        // High: 2073600 * 5 / 2000 = 5184 kbps
        assert_eq!(Quality::High.bitrate_kbps(1920, 1080), 5184);
        
        // Custom
        assert_eq!(Quality::Custom(4000).bitrate_kbps(1920, 1080), 4000);
    }
}
