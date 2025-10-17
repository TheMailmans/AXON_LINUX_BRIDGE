//! Frame diffing manager for bandwidth optimization.
//!
//! Manages frame caching, hash computation, and diff generation
//! to reduce bandwidth when streaming frames.

use super::diff::FrameDiffer;
use crate::proto_gen::agent::{DiffFrame, DiffRegion, FrameData};
use anyhow::{Result, bail};
use std::collections::HashMap;
use blake3::Hasher;

/// Cached frame data for diffing
#[derive(Debug, Clone)]
pub struct CachedFrame {
    pub hash: String,
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
    pub timestamp: i64,
}

/// Configuration for frame diffing
#[derive(Debug, Clone, Copy)]
pub struct DiffConfig {
    pub enable_diffing: bool,
    pub min_changed_percent: i32,  // Don't send if less than this changed
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            enable_diffing: true,
            min_changed_percent: 5,  // Skip if < 5% changed
        }
    }
}

/// Manages frame caching and diff generation
pub struct FrameDiffManager {
    reference_frame: Option<CachedFrame>,
    differ: FrameDiffer,
    config: DiffConfig,
}

impl FrameDiffManager {
    /// Create new frame diff manager
    pub fn new(width: usize, height: usize, config: DiffConfig) -> Self {
        Self {
            reference_frame: None,
            differ: FrameDiffer::new(width as u32, height as u32),
            config,
        }
    }

    /// Compute hash of frame data
    pub fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(data);
        let hash = hasher.finalize();
        hash.to_hex().to_string()
    }

    /// Process a frame and optionally generate diff
    pub fn process_frame(
        &mut self,
        frame_data: FrameData,
        full_frame_override: bool,
    ) -> Result<ProcessedFrame> {
        let current_hash = Self::compute_hash(&frame_data.data);

        // If no reference frame or full frame requested, return full frame
        if self.reference_frame.is_none() || full_frame_override {
            let cached = CachedFrame {
                hash: current_hash.clone(),
                width: frame_data.width as usize,
                height: frame_data.height as usize,
                data: frame_data.data.clone(),
                timestamp: frame_data.timestamp,
            };

            self.reference_frame = Some(cached);

            return Ok(ProcessedFrame {
                frame_type: FrameType::Full(frame_data),
                hash: current_hash,
                is_diff: false,
                changed_percent: 100,
            });
        }

        let reference = self.reference_frame.clone().unwrap();

        // If hash matches, skip frame
        if reference.hash == current_hash {
            return Ok(ProcessedFrame {
                frame_type: FrameType::Skip,
                hash: current_hash,
                is_diff: false,
                changed_percent: 0,
            });
        }

        // Compute diff if enabled
        if !self.config.enable_diffing {
            self.reference_frame = Some(CachedFrame {
                hash: current_hash.clone(),
                width: frame_data.width as usize,
                height: frame_data.height as usize,
                data: frame_data.data.clone(),
                timestamp: frame_data.timestamp,
            });

            return Ok(ProcessedFrame {
                frame_type: FrameType::Full(frame_data),
                hash: current_hash,
                is_diff: false,
                changed_percent: 100,
            });
        }

        // Generate diff regions
        let diff_regions = self.generate_diff_regions(
            &reference,
            &frame_data.data,
            frame_data.width as usize,
            frame_data.height as usize,
        )?;

        // Calculate changed percentage
        let total_pixels = (frame_data.width as i64) * (frame_data.height as i64);
        let changed_pixels: i64 = diff_regions
            .iter()
            .map(|r| (r.width as i64) * (r.height as i64))
            .sum();

        let changed_percent = if total_pixels > 0 {
            ((changed_pixels * 100) / total_pixels) as i32
        } else {
            0
        };

        // Skip frame if change below threshold
        if changed_percent < self.config.min_changed_percent {
            return Ok(ProcessedFrame {
                frame_type: FrameType::Skip,
                hash: current_hash,
                is_diff: false,
                changed_percent,
            });
        }

        // Update reference frame
        self.reference_frame = Some(CachedFrame {
            hash: current_hash.clone(),
            width: frame_data.width as usize,
            height: frame_data.height as usize,
            data: frame_data.data,
            timestamp: frame_data.timestamp,
        });

        let diff_frame = DiffFrame {
            base_frame_hash: reference.hash.clone(),
            regions: diff_regions,
            total_changed_pixels: changed_pixels,
            changed_percent,
        };

        Ok(ProcessedFrame {
            frame_type: FrameType::Diff(diff_frame),
            hash: current_hash,
            is_diff: true,
            changed_percent,
        })
    }

    /// Generate diff regions between reference and current frame
    fn generate_diff_regions(
        &self,
        reference: &CachedFrame,
        current_data: &[u8],
        width: usize,
        height: usize,
    ) -> Result<Vec<DiffRegion>> {
        if reference.width != width || reference.height != height {
            bail!("Frame dimensions mismatch: reference {}x{}, current {}x{}", 
                reference.width, reference.height, width, height);
        }

        let mut regions = Vec::new();
        let bytes_per_pixel = 4; // RGBA

        // Simple block-based diffing (16x16 pixel blocks)
        let block_size = 16;

        for block_y in (0..height).step_by(block_size) {
            for block_x in (0..width).step_by(block_size) {
                let block_width = std::cmp::min(block_size, width - block_x);
                let block_height = std::cmp::min(block_size, height - block_y);

                // Check if block changed
                let mut changed = false;
                'check_block: for y in 0..block_height {
                    for x in 0..block_width {
                        let px = block_x + x;
                        let py = block_y + y;
                        let offset = (py * width + px) * bytes_per_pixel;

                        if offset + bytes_per_pixel <= reference.data.len()
                            && offset + bytes_per_pixel <= current_data.len()
                        {
                            if reference.data[offset..offset + bytes_per_pixel]
                                != current_data[offset..offset + bytes_per_pixel]
                            {
                                changed = true;
                                break 'check_block;
                            }
                        }
                    }
                }

                // If block changed, include it
                if changed {
                    let start_offset = (block_y * width + block_x) * bytes_per_pixel;
                    let end_offset = start_offset + (block_width * block_height * bytes_per_pixel);

                    let pixel_data = if end_offset <= current_data.len() {
                        current_data[start_offset..end_offset].to_vec()
                    } else {
                        // Partial block at edge
                        let safe_end = std::cmp::min(end_offset, current_data.len());
                        current_data[start_offset..safe_end].to_vec()
                    };

                    regions.push(DiffRegion {
                        x: block_x as i32,
                        y: block_y as i32,
                        width: block_width as i32,
                        height: block_height as i32,
                        pixel_data,
                    });
                }
            }
        }

        Ok(regions)
    }

    /// Reset frame reference (forces full frame on next process)
    pub fn reset(&mut self) {
        self.reference_frame = None;
    }

    /// Get current config
    pub fn config(&self) -> DiffConfig {
        self.config
    }

    /// Set new config
    pub fn set_config(&mut self, config: DiffConfig) {
        self.config = config;
    }
}

/// Result of frame processing
#[derive(Debug)]
pub struct ProcessedFrame {
    pub frame_type: FrameType,
    pub hash: String,
    pub is_diff: bool,
    pub changed_percent: i32,
}

/// Frame type after processing
#[derive(Debug)]
pub enum FrameType {
    Full(FrameData),
    Diff(DiffFrame),
    Skip,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_manager_creation() {
        let manager = FrameDiffManager::new(1920, 1080, DiffConfig::default());
        assert!(manager.reference_frame.is_none());
    }

    #[test]
    fn test_diff_manager_default_config() {
        let config = DiffConfig::default();
        assert!(config.enable_diffing);
        assert_eq!(config.min_changed_percent, 5);
    }

    #[test]
    fn test_compute_hash_consistent() {
        let data = b"test frame data";
        let hash1 = FrameDiffManager::compute_hash(data);
        let hash2 = FrameDiffManager::compute_hash(data);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_compute_hash_different() {
        let data1 = b"test frame data 1";
        let data2 = b"test frame data 2";
        let hash1 = FrameDiffManager::compute_hash(data1);
        let hash2 = FrameDiffManager::compute_hash(data2);
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_reset_clears_reference() {
        let mut manager = FrameDiffManager::new(100, 100, DiffConfig::default());
        manager.reference_frame = Some(CachedFrame {
            hash: "test".to_string(),
            width: 100,
            height: 100,
            data: vec![0; 1000],
            timestamp: 0,
        });
        manager.reset();
        assert!(manager.reference_frame.is_none());
    }

    #[test]
    fn test_config_get_set() {
        let mut manager = FrameDiffManager::new(1920, 1080, DiffConfig::default());
        let new_config = DiffConfig {
            enable_diffing: false,
            min_changed_percent: 10,
        };
        manager.set_config(new_config);
        let retrieved = manager.config();
        assert!(!retrieved.enable_diffing);
        assert_eq!(retrieved.min_changed_percent, 10);
    }
}
