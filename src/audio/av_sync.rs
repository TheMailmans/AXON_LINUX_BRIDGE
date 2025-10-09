/*!
 * Audio/Video Synchronization
 * 
 * Manages timestamp synchronization between audio and video streams.
 */

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn, debug};

/// A/V synchronization manager
pub struct AVSyncManager {
    /// Base timestamp for synchronization (milliseconds)
    base_timestamp: Arc<AtomicU64>,
    /// Audio timestamp offset
    audio_offset_ms: Arc<AtomicU64>,
    /// Video timestamp offset  
    video_offset_ms: Arc<AtomicU64>,
    /// Maximum allowed A/V drift in milliseconds
    max_drift_ms: u64,
}

impl AVSyncManager {
    /// Create new A/V sync manager
    pub fn new(max_drift_ms: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        info!("Creating A/V sync manager with {}ms max drift", max_drift_ms);
        
        Self {
            base_timestamp: Arc::new(AtomicU64::new(now)),
            audio_offset_ms: Arc::new(AtomicU64::new(0)),
            video_offset_ms: Arc::new(AtomicU64::new(0)),
            max_drift_ms,
        }
    }
    
    /// Reset synchronization base
    pub fn reset(&self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        self.base_timestamp.store(now, Ordering::Relaxed);
        self.audio_offset_ms.store(0, Ordering::Relaxed);
        self.video_offset_ms.store(0, Ordering::Relaxed);
        
        info!("A/V sync reset to base timestamp: {}", now);
    }
    
    /// Get synchronized audio timestamp
    pub fn get_audio_timestamp(&self) -> u64 {
        let base = self.base_timestamp.load(Ordering::Relaxed);
        let offset = self.audio_offset_ms.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // Calculate relative timestamp
        let elapsed = now.saturating_sub(base);
        elapsed.saturating_add(offset)
    }
    
    /// Get synchronized video timestamp
    pub fn get_video_timestamp(&self) -> u64 {
        let base = self.base_timestamp.load(Ordering::Relaxed);
        let offset = self.video_offset_ms.load(Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        // Calculate relative timestamp
        let elapsed = now.saturating_sub(base);
        elapsed.saturating_add(offset)
    }
    
    /// Adjust audio timing offset
    pub fn adjust_audio_offset(&self, offset_ms: i64) {
        let current = self.audio_offset_ms.load(Ordering::Relaxed) as i64;
        let new_offset = (current + offset_ms).max(0) as u64;
        
        self.audio_offset_ms.store(new_offset, Ordering::Relaxed);
        debug!("Adjusted audio offset: {} → {}ms", current, new_offset);
    }
    
    /// Adjust video timing offset
    pub fn adjust_video_offset(&self, offset_ms: i64) {
        let current = self.video_offset_ms.load(Ordering::Relaxed) as i64;
        let new_offset = (current + offset_ms).max(0) as u64;
        
        self.video_offset_ms.store(new_offset, Ordering::Relaxed);
        debug!("Adjusted video offset: {} → {}ms", current, new_offset);
    }
    
    /// Calculate current A/V drift
    pub fn get_av_drift(&self) -> i64 {
        let audio_ts = self.get_audio_timestamp();
        let video_ts = self.get_video_timestamp();
        
        (audio_ts as i64) - (video_ts as i64)
    }
    
    /// Check if A/V drift is within acceptable range
    pub fn is_synced(&self) -> bool {
        let drift = self.get_av_drift().abs() as u64;
        drift <= self.max_drift_ms
    }
    
    /// Get synchronization statistics
    pub fn get_stats(&self) -> AVSyncStats {
        let drift = self.get_av_drift();
        let is_synced = self.is_synced();
        
        AVSyncStats {
            audio_timestamp: self.get_audio_timestamp(),
            video_timestamp: self.get_video_timestamp(),
            drift_ms: drift,
            is_synced,
            max_drift_ms: self.max_drift_ms,
        }
    }
    
    /// Auto-correct drift if it exceeds threshold
    pub fn auto_correct(&self) -> bool {
        let drift = self.get_av_drift();
        
        if drift.abs() as u64 > self.max_drift_ms {
            // Audio is ahead - slow it down
            if drift > 0 {
                let correction = -(drift / 2); // Correct half the drift
                self.adjust_audio_offset(correction);
                warn!("Auto-correcting audio drift: {}ms ahead, adjusting by {}ms", 
                      drift, correction);
                return true;
            }
            // Video is ahead - slow it down
            else {
                let correction = drift / 2; // Correct half the drift
                self.adjust_video_offset(correction);
                warn!("Auto-correcting video drift: {}ms ahead, adjusting by {}ms", 
                      -drift, -correction);
                return true;
            }
        }
        
        false
    }
}

/// A/V synchronization statistics
#[derive(Debug, Clone, Copy)]
pub struct AVSyncStats {
    /// Current audio timestamp (ms)
    pub audio_timestamp: u64,
    /// Current video timestamp (ms)
    pub video_timestamp: u64,
    /// A/V drift in milliseconds (positive = audio ahead)
    pub drift_ms: i64,
    /// Whether A/V is synchronized
    pub is_synced: bool,
    /// Maximum allowed drift
    pub max_drift_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_av_sync_creation() {
        let sync = AVSyncManager::new(100);
        assert_eq!(sync.max_drift_ms, 100);
        
        // Initial state should be synced (no drift)
        assert!(sync.is_synced());
        assert_eq!(sync.get_av_drift(), 0);
    }

    #[test]
    fn test_timestamp_generation() {
        let sync = AVSyncManager::new(100);
        
        let audio_ts = sync.get_audio_timestamp();
        let video_ts = sync.get_video_timestamp();
        
        // Should be very close (within a few ms)
        let drift = (audio_ts as i64 - video_ts as i64).abs();
        assert!(drift < 10, "Initial drift too large: {}ms", drift);
    }

    #[test]
    fn test_audio_offset_adjustment() {
        let sync = AVSyncManager::new(100);
        
        // Adjust audio forward
        sync.adjust_audio_offset(50);
        
        let drift = sync.get_av_drift();
        assert!(drift > 0, "Audio should be ahead");
        assert!(drift >= 45 && drift <= 55, "Drift should be ~50ms, got {}ms", drift);
    }

    #[test]
    fn test_video_offset_adjustment() {
        let sync = AVSyncManager::new(100);
        
        // Adjust video forward
        sync.adjust_video_offset(50);
        
        let drift = sync.get_av_drift();
        assert!(drift < 0, "Video should be ahead");
        assert!(drift >= -55 && drift <= -45, "Drift should be ~-50ms, got {}ms", drift);
    }

    #[test]
    fn test_sync_detection() {
        let sync = AVSyncManager::new(100);
        
        // Within threshold
        sync.adjust_audio_offset(50);
        assert!(sync.is_synced(), "Should be synced with 50ms drift");
        
        // Beyond threshold
        sync.adjust_audio_offset(100);
        assert!(!sync.is_synced(), "Should not be synced with 150ms drift");
    }

    #[test]
    fn test_auto_correction() {
        let sync = AVSyncManager::new(100);
        
        // Create large drift
        sync.adjust_audio_offset(200);
        assert!(!sync.is_synced());
        
        // Auto-correct
        let corrected = sync.auto_correct();
        assert!(corrected, "Should have performed correction");
        
        // Should be closer to sync
        let new_drift = sync.get_av_drift().abs();
        assert!(new_drift < 200, "Drift should be reduced");
    }

    #[test]
    fn test_reset() {
        let sync = AVSyncManager::new(100);
        
        // Introduce offsets
        sync.adjust_audio_offset(100);
        sync.adjust_video_offset(50);
        
        // Reset
        sync.reset();
        
        // Should be back to zero drift
        assert_eq!(sync.get_av_drift(), 0);
        assert!(sync.is_synced());
    }

    #[test]
    fn test_stats() {
        let sync = AVSyncManager::new(100);
        
        let stats = sync.get_stats();
        assert_eq!(stats.max_drift_ms, 100);
        assert!(stats.is_synced);
        assert_eq!(stats.drift_ms, 0);
    }
}
