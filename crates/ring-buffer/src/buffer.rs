//! Lock-Free Ring Buffer Implementation

use crate::SensorFrame;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Default buffer capacity (3000 frames = ~10 min at 5Hz)
pub const DEFAULT_CAPACITY: usize = 3000;

/// Lock-free SPSC ring buffer for sensor frames
pub struct RingBuffer {
    /// Pre-allocated storage
    storage: Box<[SensorFrame]>,
    /// Capacity of the buffer
    capacity: usize,
    /// Head position (write pointer)
    head: AtomicUsize,
    /// Tail position (read pointer) 
    tail: AtomicUsize,
    /// Total frames written (for statistics)
    total_written: AtomicUsize,
}

impl RingBuffer {
    /// Create a new ring buffer with given capacity
    pub fn new(capacity: usize) -> Self {
        let storage: Vec<SensorFrame> = (0..capacity).map(|_| SensorFrame::default()).collect();
        Self {
            storage: storage.into_boxed_slice(),
            capacity,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            total_written: AtomicUsize::new(0),
        }
    }

    /// Create a buffer with default capacity (3000 frames)
    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }

    /// Push a frame into the buffer (overwrites oldest if full)
    pub fn push(&self, frame: SensorFrame) {
        let head = self.head.load(Ordering::Relaxed);
        let next_head = (head + 1) % self.capacity;
        
        // SAFETY: We're the only writer, storage is pre-allocated
        unsafe {
            let ptr = self.storage.as_ptr() as *mut SensorFrame;
            std::ptr::write(ptr.add(head), frame);
        }
        
        self.head.store(next_head, Ordering::Release);
        self.total_written.fetch_add(1, Ordering::Relaxed);
        
        // If buffer is full, advance tail
        let tail = self.tail.load(Ordering::Relaxed);
        if next_head == tail {
            self.tail.store((tail + 1) % self.capacity, Ordering::Release);
        }
    }

    /// Get the number of frames currently in the buffer
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head >= tail {
            head - tail
        } else {
            self.capacity - tail + head
        }
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity - 1
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get fill ratio (0.0 to 1.0)
    pub fn fill_ratio(&self) -> f64 {
        self.len() as f64 / self.capacity as f64
    }

    /// Read the last N frames (most recent first)
    pub fn read_last(&self, count: usize) -> Vec<SensorFrame> {
        let len = self.len();
        let count = count.min(len);
        let head = self.head.load(Ordering::Acquire);
        
        let mut frames = Vec::with_capacity(count);
        for i in 0..count {
            let idx = if head >= i + 1 {
                head - i - 1
            } else {
                self.capacity - (i + 1 - head)
            };
            frames.push(self.storage[idx].clone());
        }
        frames
    }

    /// Read frames within a time window (duration in milliseconds)
    pub fn read_window(&self, duration_ms: u64) -> Vec<SensorFrame> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        let cutoff = now.saturating_sub(duration_ms);
        
        self.read_last(self.len())
            .into_iter()
            .take_while(|f| f.timestamp_ms >= cutoff)
            .collect()
    }

    /// Get total frames written (for statistics)
    pub fn total_written(&self) -> usize {
        self.total_written.load(Ordering::Relaxed)
    }

    /// Clear the buffer
    pub fn clear(&self) {
        self.tail.store(self.head.load(Ordering::Relaxed), Ordering::Release);
    }
}

// SAFETY: RingBuffer is designed for SPSC use, but we mark it Send+Sync
// for flexibility in async contexts where the runtime may move it between threads.
unsafe impl Send for RingBuffer {}
unsafe impl Sync for RingBuffer {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_read() {
        let buffer = RingBuffer::new(10);
        
        for i in 0..5 {
            buffer.push(SensorFrame {
                timestamp_ms: i as u64 * 1000,
                rpm: (i * 100) as u16,
                ..Default::default()
            });
        }
        
        assert_eq!(buffer.len(), 5);
        
        let frames = buffer.read_last(3);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0].rpm, 400); // Most recent
        assert_eq!(frames[2].rpm, 200);
    }

    #[test]
    fn test_overwrite_oldest() {
        let buffer = RingBuffer::new(5);
        
        // Fill beyond capacity
        for i in 0..10 {
            buffer.push(SensorFrame {
                rpm: (i * 100) as u16,
                ..Default::default()
            });
        }
        
        // Should only have capacity-1 frames
        assert_eq!(buffer.len(), 4);
        
        // Oldest should be overwritten
        let frames = buffer.read_last(4);
        assert!(frames[0].rpm >= 500); // Recent frames
    }

    #[test]
    fn test_fill_ratio() {
        let buffer = RingBuffer::new(100);
        assert_eq!(buffer.fill_ratio(), 0.0);
        
        for i in 0..50 {
            buffer.push(SensorFrame::default());
        }
        
        assert!((buffer.fill_ratio() - 0.5).abs() < 0.01);
    }
}
