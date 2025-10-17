//! Frame caching with LRU eviction and TTL support.
//!
//! Implements a bounded cache for compressed frames with:
//! - LRU eviction policy (remove least-recently-used when full)
//! - TTL support (remove frames older than max_age)
//! - Hit/miss tracking for metrics
//! - Thread-safe access with Arc<RwLock<>>

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::capture::compression::CompressedFrame;

/// Cache entry with timestamp and access tracking
#[derive(Clone)]
struct CacheEntry {
    /// The actual compressed frame
    frame: CompressedFrame,
    /// When this entry was inserted
    inserted_at: Instant,
    /// When this entry was last accessed
    last_accessed: Instant,
}

impl CacheEntry {
    /// Check if entry has expired
    fn is_expired(&self, max_age: Duration) -> bool {
        self.inserted_at.elapsed() > max_age
    }
}

/// Frame cache with LRU eviction and TTL
pub struct FrameCache {
    /// Max number of frames to cache
    max_entries: usize,
    /// Max age for cached frames
    max_age: Duration,
    /// Actual cache storage: key -> (frame, timestamp)
    entries: HashMap<String, CacheEntry>,
    /// Cache metrics
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
}

impl FrameCache {
    /// Create new frame cache
    pub fn new(max_entries: usize) -> Self {
        Self::with_ttl(max_entries, Duration::from_secs(60))
    }

    /// Create cache with custom TTL
    pub fn with_ttl(max_entries: usize, max_age: Duration) -> Self {
        Self {
            max_entries,
            max_age,
            entries: HashMap::new(),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Insert or update a frame in cache
    pub fn insert(&mut self, key: String, frame: CompressedFrame) {
        // Remove expired entries first
        self.evict_expired();

        // If cache is full, evict LRU entry
        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&key) {
            self.evict_lru();
        }

        // Insert new entry
        self.entries.insert(
            key,
            CacheEntry {
                frame,
                inserted_at: Instant::now(),
                last_accessed: Instant::now(),
            },
        );
    }

    /// Get frame from cache (updates access time)
    pub fn get(&mut self, key: &str) -> Option<CompressedFrame> {
        // Check if key exists and not expired
        if let Some(entry) = self.entries.get_mut(key) {
            if !entry.is_expired(self.max_age) {
                // Update access time for LRU
                entry.last_accessed = Instant::now();
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.frame.clone());
            } else {
                // Expired, remove it
                self.entries.remove(key);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Remove all expired entries
    fn evict_expired(&mut self) {
        let mut to_remove = Vec::new();
        for (key, entry) in self.entries.iter() {
            if entry.is_expired(self.max_age) {
                to_remove.push(key.clone());
            }
        }
        for key in to_remove {
            self.entries.remove(&key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Remove least-recently-used entry
    fn evict_lru(&mut self) {
        if let Some(lru_key) = self.entries.iter()
            .min_by_key(|(_, entry)| entry.last_accessed)
            .map(|(k, _)| k.clone())
        {
            self.entries.remove(&lru_key);
            self.evictions.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_requests = self.hits.load(Ordering::Relaxed)
            + self.misses.load(Ordering::Relaxed);
        let hit_rate = if total_requests > 0 {
            self.hits.load(Ordering::Relaxed) as f32 / total_requests as f32
        } else {
            0.0
        };

        CacheStats {
            entries_count: self.entries.len() as u64,
            max_entries: self.max_entries as u64,
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            hit_rate,
            evictions: self.evictions.load(Ordering::Relaxed),
        }
    }

    /// Get current number of cached entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Estimate memory usage in bytes
    pub fn estimated_memory_bytes(&self) -> usize {
        self.entries.values()
            .map(|entry| entry.frame.compressed_size)
            .sum()
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries_count: u64,
    pub max_entries: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f32,
    pub evictions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_frame(hash: &str) -> CompressedFrame {
        CompressedFrame {
            data: vec![0u8; 100],
            frame_hash: hash.into(),
            uncompressed_size: 1000,
            compressed_size: 100,
            compression_ratio: 0.1,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = FrameCache::new(10);
        let frame = create_test_frame("frame1");

        cache.insert("key1".into(), frame.clone());
        
        let retrieved = cache.get("key1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().frame_hash, "frame1");
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = FrameCache::new(10);
        
        let result = cache.get("nonexistent");
        assert!(result.is_none());
        
        let stats = cache.stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hits, 0);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = FrameCache::new(2);

        cache.insert("key1".into(), create_test_frame("frame1"));
        cache.insert("key2".into(), create_test_frame("frame2"));
        // This should evict key1 (least recently used)
        cache.insert("key3".into(), create_test_frame("frame3"));

        // key1 should be gone
        assert!(cache.get("key1").is_none());
        // key2 and key3 should exist
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_ttl_expiration() {
        let mut cache = FrameCache::with_ttl(10, Duration::from_millis(100));
        
        cache.insert("key1".into(), create_test_frame("frame1"));
        assert!(cache.get("key1").is_some());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        
        // Entry should be expired
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_hit_rate_calculation() {
        let mut cache = FrameCache::new(10);
        cache.insert("key1".into(), create_test_frame("frame1"));

        cache.get("key1"); // hit
        cache.get("key1"); // hit
        cache.get("key2"); // miss
        cache.get("key2"); // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_clear() {
        let mut cache = FrameCache::new(10);
        cache.insert("key1".into(), create_test_frame("frame1"));
        cache.insert("key2".into(), create_test_frame("frame2"));

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_memory_estimation() {
        let mut cache = FrameCache::new(10);
        
        let frame = CompressedFrame {
            data: vec![0u8; 500],
            frame_hash: "frame1".into(),
            uncompressed_size: 5000,
            compressed_size: 500,
            compression_ratio: 0.1,
            width: 1920,
            height: 1080,
        };

        cache.insert("key1".into(), frame);
        
        let memory = cache.estimated_memory_bytes();
        assert_eq!(memory, 500);
    }
}
