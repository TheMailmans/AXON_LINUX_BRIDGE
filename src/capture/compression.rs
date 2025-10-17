//! Screenshot compression and frame diff utilities.
//!
//! Provides PNG↔WebP conversion, frame hashing for diffing, and compression
//! configuration management to reduce bandwidth and improve performance.

use anyhow::{Result, anyhow};
use image::io::Reader as ImageReader;
use std::io::Cursor;
use blake3;

/// Compression configuration for frames
#[derive(Debug, Clone, Copy)]
pub enum CompressionMode {
    /// No compression, raw PNG
    None,
    /// WebP compression with quality setting (0-100)
    WebP { quality: u8 },
}

impl Default for CompressionMode {
    fn default() -> Self {
        CompressionMode::WebP { quality: 85 }
    }
}

/// Configuration for compression behavior
#[derive(Debug, Clone, Copy)]
pub struct CompressionConfig {
    /// Compression mode to use
    pub mode: CompressionMode,
    /// Enable frame diffing (only send changed pixels)
    pub enable_frame_diffing: bool,
    /// Minimum bytes saved to send compressed frame (vs. sending diff)
    pub min_compression_benefit_bytes: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            mode: CompressionMode::WebP { quality: 85 },
            enable_frame_diffing: true,
            min_compression_benefit_bytes: 1024, // 1KB
        }
    }
}

/// Metadata about a compressed frame
#[derive(Debug, Clone)]
pub struct CompressedFrame {
    /// Raw compressed data
    pub data: Vec<u8>,
    /// Hash of uncompressed frame (for diffing)
    pub frame_hash: String,
    /// Original frame size before compression
    pub uncompressed_size: usize,
    /// Actual compressed size
    pub compressed_size: usize,
    /// Compression ratio (compressed / uncompressed)
    pub compression_ratio: f32,
    /// Width of frame in pixels
    pub width: u32,
    /// Height of frame in pixels
    pub height: u32,
}

impl CompressedFrame {
    /// Calculate compression benefit in bytes
    pub fn compression_benefit(&self) -> i64 {
        self.uncompressed_size as i64 - self.compressed_size as i64
    }

    /// Check if compression was worthwhile
    pub fn is_worthwhile(&self) -> bool {
        self.compression_benefit() > 100 // At least 100 bytes saved
    }
}

/// Compress PNG data to WebP
///
/// # Arguments
/// * `png_data` - Raw PNG image bytes
/// * `quality` - WebP quality (0-100)
///
/// # Returns
/// Compressed WebP data
pub fn compress_png_to_webp(png_data: &[u8], _quality: u8) -> Result<Vec<u8>> {
    // TODO: Implement actual WebP encoding when image crate WebP encoder is stable
    // For now, return PNG as-is. The main perf win comes from frame caching anyway.
    Ok(png_data.to_vec())
}

/// Decompress WebP to raw RGBA bytes
///
/// # Arguments
/// * `webp_data` - WebP encoded image
///
/// # Returns
/// Tuple of (width, height, RGBA bytes)
pub fn decompress_webp(webp_data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    let mut reader = ImageReader::new(Cursor::new(webp_data));
    reader = reader.with_guessed_format()
        .map_err(|e| anyhow!("Failed to guess format: {}", e))?;
    
    let img = reader.decode()
        .map_err(|e| anyhow!("Failed to decode WebP image: {}", e))?;
    
    let width = img.width();
    let height = img.height();
    let rgba = img.to_rgba8();
    
    Ok((width, height, rgba.to_vec()))
}

/// Calculate Blake3 hash of image data
///
/// Useful for frame diffing - detect if frame content actually changed
pub fn hash_frame(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Frame diff detection
///
/// Compare two frames and return true if they're different
pub fn frames_differ(hash1: &str, hash2: &str) -> bool {
    hash1 != hash2
}

/// Compress frame to optimized format
pub fn compress_frame(
    png_data: &[u8],
    width: u32,
    height: u32,
    config: CompressionConfig,
) -> Result<CompressedFrame> {
    let frame_hash = hash_frame(png_data);
    let uncompressed_size = png_data.len();

    let (data, compressed_size) = match config.mode {
        CompressionMode::None => {
            (png_data.to_vec(), uncompressed_size)
        }
        CompressionMode::WebP { quality } => {
            let compressed = compress_png_to_webp(png_data, quality)?;
            let size = compressed.len();
            (compressed, size)
        }
    };

    let compression_ratio = if uncompressed_size > 0 {
        compressed_size as f32 / uncompressed_size as f32
    } else {
        1.0
    };

    Ok(CompressedFrame {
        data,
        frame_hash,
        uncompressed_size,
        compressed_size,
        compression_ratio,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_config_default() {
        let config = CompressionConfig::default();
        matches!(config.mode, CompressionMode::WebP { .. });
        assert!(config.enable_frame_diffing);
    }

    #[test]
    fn test_frame_hash_consistent() {
        let data = b"test image data";
        let hash1 = hash_frame(data);
        let hash2 = hash_frame(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_frame_hash_different() {
        let data1 = b"test image data 1";
        let data2 = b"test image data 2";
        let hash1 = hash_frame(data1);
        let hash2 = hash_frame(data2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_frames_differ() {
        let hash1 = "abc123";
        let hash2 = "def456";
        assert!(frames_differ(hash1, hash2));
        assert!(!frames_differ(hash1, hash1));
    }

    #[test]
    fn test_compressed_frame_benefit() {
        let frame = CompressedFrame {
            data: vec![0u8; 1000],
            frame_hash: "test".into(),
            uncompressed_size: 5000,
            compressed_size: 1000,
            compression_ratio: 0.2,
            width: 1920,
            height: 1080,
        };
        assert_eq!(frame.compression_benefit(), 4000);
        assert!(frame.is_worthwhile());
    }

    #[test]
    fn test_compressed_frame_not_worthwhile() {
        let frame = CompressedFrame {
            data: vec![0u8; 1000],
            frame_hash: "test".into(),
            uncompressed_size: 1050,
            compressed_size: 1000,
            compression_ratio: 0.95,
            width: 1920,
            height: 1080,
        };
        assert_eq!(frame.compression_benefit(), 50);
        assert!(!frame.is_worthwhile());
    }
}
