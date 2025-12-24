//! Statistical Features Computation

use ring_buffer::SensorFrame;

/// Statistical features for a signal
#[derive(Debug, Clone, Default)]
pub struct StatisticalFeatures {
    /// Mean value
    pub mean: f64,
    /// Standard deviation
    pub std_dev: f64,
    /// Skewness (asymmetry)
    pub skewness: f64,
    /// Kurtosis (tailedness)
    pub kurtosis: f64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Rate of change (derivative)
    pub rate_of_change: f64,
    /// Number of zero crossings
    pub zero_crossings: usize,
}

impl StatisticalFeatures {
    /// Compute statistical features from a slice of values
    pub fn compute(values: &[f64]) -> Self {
        if values.is_empty() {
            return Self::default();
        }

        let n = values.len() as f64;
        
        // Mean
        let mean = values.iter().sum::<f64>() / n;
        
        // Min/Max
        let min = values.iter().cloned().fold(f64::MAX, f64::min);
        let max = values.iter().cloned().fold(f64::MIN, f64::max);
        
        // Variance and higher moments
        let mut m2 = 0.0;
        let mut m3 = 0.0;
        let mut m4 = 0.0;
        
        for &v in values {
            let d = v - mean;
            m2 += d * d;
            m3 += d * d * d;
            m4 += d * d * d * d;
        }
        
        let variance = m2 / n;
        let std_dev = variance.sqrt();
        
        // Skewness: E[(X-μ)³] / σ³
        let skewness = if std_dev > 0.0 {
            (m3 / n) / (std_dev * std_dev * std_dev)
        } else {
            0.0
        };
        
        // Kurtosis: E[(X-μ)⁴] / σ⁴ - 3 (excess kurtosis)
        let kurtosis = if std_dev > 0.0 {
            (m4 / n) / (variance * variance) - 3.0
        } else {
            0.0
        };
        
        // Rate of change (average derivative)
        let rate_of_change = if values.len() >= 2 {
            let mut total_change = 0.0;
            for i in 1..values.len() {
                total_change += (values[i] - values[i - 1]).abs();
            }
            total_change / (values.len() - 1) as f64
        } else {
            0.0
        };
        
        // Zero crossings (relative to mean)
        let mut zero_crossings = 0;
        for i in 1..values.len() {
            let prev = values[i - 1] - mean;
            let curr = values[i] - mean;
            if prev.signum() != curr.signum() && prev != 0.0 && curr != 0.0 {
                zero_crossings += 1;
            }
        }
        
        Self {
            mean,
            std_dev,
            skewness,
            kurtosis,
            min,
            max,
            rate_of_change,
            zero_crossings,
        }
    }

    /// Extract RPM values from sensor frames
    pub fn extract_rpm(frames: &[SensorFrame]) -> Vec<f64> {
        frames.iter().map(|f| f.rpm as f64).collect()
    }

    /// Extract coolant temp values from sensor frames
    pub fn extract_coolant_temp(frames: &[SensorFrame]) -> Vec<f64> {
        frames.iter().map(|f| f.coolant_temp as f64).collect()
    }

    /// Extract speed values from sensor frames
    pub fn extract_speed(frames: &[SensorFrame]) -> Vec<f64> {
        frames.iter().map(|f| f.speed as f64).collect()
    }

    /// Extract engine load values from sensor frames
    pub fn extract_engine_load(frames: &[SensorFrame]) -> Vec<f64> {
        frames.iter().map(|f| f.engine_load as f64).collect()
    }

    /// Extract MAF values from sensor frames
    pub fn extract_maf(frames: &[SensorFrame]) -> Vec<f64> {
        frames.iter().map(|f| f.maf as f64 / 100.0).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mean_computation() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let stats = StatisticalFeatures::compute(&values);
        assert!((stats.mean - 3.0).abs() < 0.001);
    }

    #[test]
    fn test_std_dev_computation() {
        let values = vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let stats = StatisticalFeatures::compute(&values);
        // Std dev should be ~2.0 for this dataset
        assert!((stats.std_dev - 2.0).abs() < 0.1);
    }

    #[test]
    fn test_zero_crossings() {
        // Values oscillating around mean of 5
        let values = vec![3.0, 7.0, 4.0, 8.0, 2.0, 6.0];
        let stats = StatisticalFeatures::compute(&values);
        assert!(stats.zero_crossings >= 2);
    }

    #[test]
    fn test_empty_values() {
        let values: Vec<f64> = vec![];
        let stats = StatisticalFeatures::compute(&values);
        assert_eq!(stats.mean, 0.0);
    }
}
