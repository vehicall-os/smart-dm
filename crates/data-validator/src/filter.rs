//! Median Filter for Noise Reduction

/// Sliding window median filter for noise reduction
pub struct MedianFilter {
    window: Vec<f64>,
    size: usize,
    position: usize,
    filled: bool,
}

impl MedianFilter {
    /// Create a new median filter with given window size
    pub fn new(size: usize) -> Self {
        assert!(size > 0 && size % 2 == 1, "Window size must be odd and > 0");
        Self {
            window: vec![0.0; size],
            size,
            position: 0,
            filled: false,
        }
    }

    /// Add a value and get the filtered output
    pub fn filter(&mut self, value: f64) -> f64 {
        self.window[self.position] = value;
        self.position = (self.position + 1) % self.size;
        
        if self.position == 0 {
            self.filled = true;
        }

        if !self.filled {
            // Return input until window is filled
            return value;
        }

        // Sort a copy and return median
        let mut sorted = self.window.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        sorted[self.size / 2]
    }

    /// Reset the filter
    pub fn reset(&mut self) {
        self.window.fill(0.0);
        self.position = 0;
        self.filled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_filter_basic() {
        let mut filter = MedianFilter::new(5);
        
        // Fill window
        filter.filter(10.0);
        filter.filter(12.0);
        filter.filter(11.0);
        filter.filter(100.0); // outlier
        filter.filter(13.0);
        
        // Next value should be filtered
        let result = filter.filter(12.0);
        // Median of [12, 11, 100, 13, 12] = 12
        assert!((result - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_median_filter_removes_spike() {
        let mut filter = MedianFilter::new(5);
        
        // Normal values with one spike
        for val in [10.0, 11.0, 10.0, 100.0, 10.0] {
            filter.filter(val);
        }
        
        let result = filter.filter(11.0);
        // Should filter out the spike
        assert!(result < 20.0);
    }
}
