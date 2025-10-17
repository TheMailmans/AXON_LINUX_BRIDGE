//! Frame differential detection for v2.4.
//!
//! Computes frame differences to reduce bandwidth by only transmitting
//! changed regions instead of full frames.

use anyhow::Result;

/// Rectangular region definition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Region {
    /// Create new region
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// Get size in bytes for this region (assuming 4 bytes per pixel RGBA)
    pub fn size_bytes(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 4
    }

    /// Check if region is valid (positive dimensions)
    pub fn is_valid(&self) -> bool {
        self.width > 0 && self.height > 0
    }
}

/// Frame difference analyzer
pub struct FrameDiffer {
    frame_width: u32,
    frame_height: u32,
    /// Block size for region detection (16x16 pixels default)
    block_size: u32,
}

impl FrameDiffer {
    /// Create new frame differ
    pub fn new(frame_width: u32, frame_height: u32) -> Self {
        Self {
            frame_width,
            frame_height,
            block_size: 16,
        }
    }

    /// Detect changed regions between two frames
    ///
    /// Assumes both frames are RGBA (4 bytes per pixel)
    pub fn detect_changes(
        &self,
        _frame1: &[u8],
        _frame2: &[u8],
    ) -> Result<Vec<Region>> {
        // TODO: Implement pixel-by-pixel comparison
        // For now, return empty regions (no changes detected)
        Ok(Vec::new())
    }

    /// Calculate changed pixel percentage (0-100)
    pub fn calculate_changed_percent(
        total_pixels: usize,
        changed_pixels: usize,
    ) -> u32 {
        if total_pixels == 0 {
            return 0;
        }
        ((changed_pixels as f32 / total_pixels as f32) * 100.0) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_creation() {
        let region = Region::new(10, 20, 100, 200);
        assert_eq!(region.x, 10);
        assert_eq!(region.y, 20);
        assert_eq!(region.width, 100);
        assert_eq!(region.height, 200);
    }

    #[test]
    fn test_region_size_bytes() {
        let region = Region::new(0, 0, 16, 16);
        // 16*16*4 = 1024
        assert_eq!(region.size_bytes(), 1024);
    }

    #[test]
    fn test_region_validity() {
        assert!(Region::new(0, 0, 10, 10).is_valid());
        assert!(!Region::new(0, 0, 0, 10).is_valid());
        assert!(!Region::new(0, 0, 10, 0).is_valid());
    }

    #[test]
    fn test_frame_differ_creation() {
        let differ = FrameDiffer::new(1920, 1080);
        assert_eq!(differ.frame_width, 1920);
        assert_eq!(differ.frame_height, 1080);
    }

    #[test]
    fn test_changed_percent_calculation() {
        assert_eq!(FrameDiffer::calculate_changed_percent(1000, 0), 0);
        assert_eq!(FrameDiffer::calculate_changed_percent(1000, 500), 50);
        assert_eq!(FrameDiffer::calculate_changed_percent(1000, 1000), 100);
        assert_eq!(FrameDiffer::calculate_changed_percent(0, 0), 0);
    }
}
