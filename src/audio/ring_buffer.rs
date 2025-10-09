/*!
 * Lock-free ring buffer for audio samples
 * 
 * Optimized for single-producer single-consumer (SPSC) scenarios.
 */

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Lock-free ring buffer for f32 audio samples
pub struct RingBuffer {
    buffer: Vec<f32>,
    capacity: usize,
    write_pos: AtomicUsize,
    read_pos: AtomicUsize,
}

impl RingBuffer {
    /// Create new ring buffer with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0.0; capacity],
            capacity,
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
        }
    }
    
    /// Write samples to the buffer
    /// Returns number of samples actually written
    pub fn write(&self, samples: &[f32]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        let available = self.available_write_space(write_pos, read_pos);
        let to_write = samples.len().min(available);
        
        if to_write == 0 {
            return 0;
        }
        
        // Write in two parts if wrapping around
        let end = write_pos + to_write;
        if end <= self.capacity {
            // Single contiguous write
            unsafe {
                let dst = self.buffer.as_ptr().add(write_pos) as *mut f32;
                std::ptr::copy_nonoverlapping(samples.as_ptr(), dst, to_write);
            }
        } else {
            // Split write
            let first_part = self.capacity - write_pos;
            let second_part = to_write - first_part;
            
            unsafe {
                let dst1 = self.buffer.as_ptr().add(write_pos) as *mut f32;
                std::ptr::copy_nonoverlapping(samples.as_ptr(), dst1, first_part);
                
                let dst2 = self.buffer.as_ptr() as *mut f32;
                std::ptr::copy_nonoverlapping(samples.as_ptr().add(first_part), dst2, second_part);
            }
        }
        
        // Update write position
        self.write_pos.store((write_pos + to_write) % self.capacity, Ordering::Release);
        
        to_write
    }
    
    /// Read samples from the buffer
    /// Returns number of samples actually read
    pub fn read(&self, output: &mut [f32]) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        
        let available = self.available_read_samples(write_pos, read_pos);
        let to_read = output.len().min(available);
        
        if to_read == 0 {
            return 0;
        }
        
        // Read in two parts if wrapping around
        let end = read_pos + to_read;
        if end <= self.capacity {
            // Single contiguous read
            unsafe {
                let src = self.buffer.as_ptr().add(read_pos);
                std::ptr::copy_nonoverlapping(src, output.as_mut_ptr(), to_read);
            }
        } else {
            // Split read
            let first_part = self.capacity - read_pos;
            let second_part = to_read - first_part;
            
            unsafe {
                let src1 = self.buffer.as_ptr().add(read_pos);
                std::ptr::copy_nonoverlapping(src1, output.as_mut_ptr(), first_part);
                
                let src2 = self.buffer.as_ptr();
                std::ptr::copy_nonoverlapping(src2, output.as_mut_ptr().add(first_part), second_part);
            }
        }
        
        // Update read position
        self.read_pos.store((read_pos + to_read) % self.capacity, Ordering::Release);
        
        to_read
    }
    
    /// Get number of samples available for reading
    pub fn available(&self) -> usize {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        self.available_read_samples(write_pos, read_pos)
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.available() == 0
    }
    
    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let read_pos = self.read_pos.load(Ordering::Acquire);
        self.available_write_space(write_pos, read_pos) == 0
    }
    
    /// Clear the buffer
    pub fn clear(&self) {
        let write_pos = self.write_pos.load(Ordering::Acquire);
        self.read_pos.store(write_pos, Ordering::Release);
    }
    
    // Helper: calculate available read samples
    fn available_read_samples(&self, write_pos: usize, read_pos: usize) -> usize {
        if write_pos >= read_pos {
            write_pos - read_pos
        } else {
            self.capacity - read_pos + write_pos
        }
    }
    
    // Helper: calculate available write space
    fn available_write_space(&self, write_pos: usize, read_pos: usize) -> usize {
        // Reserve one slot to distinguish full from empty
        self.capacity - self.available_read_samples(write_pos, read_pos) - 1
    }
}

unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_creation() {
        let buffer = RingBuffer::new(1024);
        assert_eq!(buffer.capacity, 1024);
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_ring_buffer_write_read() {
        let buffer = RingBuffer::new(1024);
        
        // Write some samples
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let written = buffer.write(&input);
        assert_eq!(written, 5);
        assert_eq!(buffer.available(), 5);
        
        // Read them back
        let mut output = vec![0.0; 5];
        let read = buffer.read(&mut output);
        assert_eq!(read, 5);
        assert_eq!(output, input);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_ring_buffer_wraparound() {
        let buffer = RingBuffer::new(10);
        
        // Fill buffer
        let input1 = vec![1.0; 8];
        buffer.write(&input1);
        
        // Read some
        let mut output = vec![0.0; 5];
        buffer.read(&mut output);
        
        // Write more (should wrap around)
        let input2 = vec![2.0; 5];
        let written = buffer.write(&input2);
        assert_eq!(written, 5);
        
        // Read all
        let mut output2 = vec![0.0; 8];
        let read = buffer.read(&mut output2);
        assert_eq!(read, 8);
        
        // First 3 should be 1.0, last 5 should be 2.0
        assert_eq!(output2[0..3], vec![1.0; 3]);
        assert_eq!(output2[3..8], vec![2.0; 5]);
    }

    #[test]
    fn test_ring_buffer_overflow() {
        let buffer = RingBuffer::new(10);
        
        // Try to write more than capacity
        let input = vec![1.0; 20];
        let written = buffer.write(&input);
        assert_eq!(written, 9); // One less than capacity (reserved slot)
        assert!(buffer.is_full());
        
        // Writing more should fail
        let written2 = buffer.write(&[2.0]);
        assert_eq!(written2, 0);
    }

    #[test]
    fn test_ring_buffer_clear() {
        let buffer = RingBuffer::new(1024);
        
        let input = vec![1.0; 100];
        buffer.write(&input);
        assert_eq!(buffer.available(), 100);
        
        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.available(), 0);
    }
}
