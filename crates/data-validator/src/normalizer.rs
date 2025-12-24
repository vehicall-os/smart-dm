//! Data Normalization using EWMA

use serde::{Deserialize, Serialize};

/// Normalization method
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NormalizationMethod {
    /// Z-score normalization using EWMA
    ZScore,
    /// Min-max normalization to [0, 1]
    MinMax,
    /// No normalization
    None,
}

/// Normalizer using Exponentially Weighted Moving Average
pub struct Normalizer {
    /// Current mean estimate
    mean: f64,
    /// Current variance estimate
    variance: f64,
    /// EWMA smoothing factor (0-1, higher = more weight on recent)
    alpha: f64,
    /// Whether initialized with first value
    initialized: bool,
    /// Method to use
    method: NormalizationMethod,
    /// Min value seen (for MinMax)
    min: f64,
    /// Max value seen (for MinMax)
    max: f64,
}

impl Normalizer {
    /// Create a new normalizer
    pub fn new(method: NormalizationMethod, alpha: f64) -> Self {
        Self {
            mean: 0.0,
            variance: 1.0,
            alpha: alpha.clamp(0.0, 1.0),
            initialized: false,
            method,
            min: f64::MAX,
            max: f64::MIN,
        }
    }

    /// Normalize a value and update statistics
    pub fn normalize(&mut self, value: f64) -> f64 {
        // Update min/max
        self.min = self.min.min(value);
        self.max = self.max.max(value);

        if !self.initialized {
            self.mean = value;
            self.variance = 1.0;
            self.initialized = true;
            return 0.0; // First value normalizes to 0
        }

        // Update EWMA mean
        let delta = value - self.mean;
        self.mean += self.alpha * delta;

        // Update EWMA variance
        self.variance = (1.0 - self.alpha) * (self.variance + self.alpha * delta * delta);

        match self.method {
            NormalizationMethod::ZScore => {
                let std_dev = self.variance.sqrt().max(0.0001);
                (value - self.mean) / std_dev
            }
            NormalizationMethod::MinMax => {
                let range = (self.max - self.min).max(0.0001);
                (value - self.min) / range
            }
            NormalizationMethod::None => value,
        }
    }

    /// Get current mean
    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// Get current standard deviation
    pub fn std_dev(&self) -> f64 {
        self.variance.sqrt()
    }

    /// Reset the normalizer
    pub fn reset(&mut self) {
        self.mean = 0.0;
        self.variance = 1.0;
        self.initialized = false;
        self.min = f64::MAX;
        self.max = f64::MIN;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zscore_normalization() {
        let mut norm = Normalizer::new(NormalizationMethod::ZScore, 0.1);
        
        // Feed stable values
        for _ in 0..100 {
            norm.normalize(100.0);
        }
        
        // Value at mean should normalize to ~0
        let result = norm.normalize(100.0);
        assert!(result.abs() < 0.1);
        
        // Value above mean should be positive
        let result = norm.normalize(110.0);
        assert!(result > 0.0);
    }

    #[test]
    fn test_minmax_normalization() {
        let mut norm = Normalizer::new(NormalizationMethod::MinMax, 0.1);
        
        // Feed range of values
        norm.normalize(0.0);
        norm.normalize(100.0);
        
        // Mid value should be ~0.5
        let result = norm.normalize(50.0);
        assert!((result - 0.5).abs() < 0.1);
    }
}
