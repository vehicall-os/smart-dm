//! Feature Vector Assembly

use crate::fft::FftAnalyzer;
use crate::statistics::StatisticalFeatures;
use ring_buffer::{RingBuffer, SensorFrame};
use serde::{Deserialize, Serialize};
use tracing::debug;

/// Number of features in the vector (45 as per blueprint)
pub const FEATURE_DIMENSION: usize = 45;

/// Feature vector for ML inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureVector {
    /// Raw feature values (45 dimensions)
    pub values: Vec<f64>,
    /// Timestamp when features were computed
    pub timestamp_ms: u64,
    
    // Named features for easy access
    /// Coolant temp mean (30s window)
    pub coolant_temp_mean_30s: f64,
    /// Coolant temp rate of change
    pub coolant_temp_rate: f64,
    /// RPM mean
    pub rpm_mean: f64,
    /// RPM std dev
    pub rpm_std_dev: f64,
}

impl Default for FeatureVector {
    fn default() -> Self {
        Self {
            values: vec![0.0; FEATURE_DIMENSION],
            timestamp_ms: 0,
            coolant_temp_mean_30s: 0.0,
            coolant_temp_rate: 0.0,
            rpm_mean: 0.0,
            rpm_std_dev: 0.0,
        }
    }
}


/// Feature extractor that processes sensor frames
pub struct FeatureExtractor {
    /// FFT analyzer
    fft_analyzer: FftAnalyzer,
    /// Sample rate (Hz)
    sample_rate: f64,
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new(sample_rate: f64) -> Self {
        Self {
            fft_analyzer: FftAnalyzer::new(sample_rate),
            sample_rate,
        }
    }

    /// Extract features from the ring buffer
    pub fn extract(&mut self, buffer: &RingBuffer) -> FeatureVector {
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        // Get frames for different windows
        let frames_30s = buffer.read_window(30_000);
        let frames_60s = buffer.read_window(60_000);
        let frames_300s = buffer.read_window(300_000);

        debug!(
            "Extracting features: 30s={}, 60s={}, 300s={} frames",
            frames_30s.len(),
            frames_60s.len(),
            frames_300s.len()
        );

        let mut values = vec![0.0; FEATURE_DIMENSION];
        let mut idx = 0;

        // Statistical features for each signal and window
        // 5 signals × 4 stats × 3 windows = 60, but we select 20
        
        // RPM features (30s window)
        let rpm_30s = StatisticalFeatures::extract_rpm(&frames_30s);
        let rpm_stats_30s = StatisticalFeatures::compute(&rpm_30s);
        values[idx] = rpm_stats_30s.mean; idx += 1;
        values[idx] = rpm_stats_30s.std_dev; idx += 1;
        values[idx] = rpm_stats_30s.skewness; idx += 1;
        values[idx] = rpm_stats_30s.kurtosis; idx += 1;

        // Coolant temp features (30s window)
        let coolant_30s = StatisticalFeatures::extract_coolant_temp(&frames_30s);
        let coolant_stats_30s = StatisticalFeatures::compute(&coolant_30s);
        values[idx] = coolant_stats_30s.mean; idx += 1;
        values[idx] = coolant_stats_30s.std_dev; idx += 1;
        values[idx] = coolant_stats_30s.skewness; idx += 1;
        values[idx] = coolant_stats_30s.kurtosis; idx += 1;

        // Speed features (30s window)
        let speed_30s = StatisticalFeatures::extract_speed(&frames_30s);
        let speed_stats_30s = StatisticalFeatures::compute(&speed_30s);
        values[idx] = speed_stats_30s.mean; idx += 1;
        values[idx] = speed_stats_30s.std_dev; idx += 1;
        values[idx] = speed_stats_30s.skewness; idx += 1;
        values[idx] = speed_stats_30s.kurtosis; idx += 1;

        // Engine load features (30s window)
        let load_30s = StatisticalFeatures::extract_engine_load(&frames_30s);
        let load_stats_30s = StatisticalFeatures::compute(&load_30s);
        values[idx] = load_stats_30s.mean; idx += 1;
        values[idx] = load_stats_30s.std_dev; idx += 1;
        values[idx] = load_stats_30s.skewness; idx += 1;
        values[idx] = load_stats_30s.kurtosis; idx += 1;

        // MAF features (30s window)
        let maf_30s = StatisticalFeatures::extract_maf(&frames_30s);
        let maf_stats_30s = StatisticalFeatures::compute(&maf_30s);
        values[idx] = maf_stats_30s.mean; idx += 1;
        values[idx] = maf_stats_30s.std_dev; idx += 1;
        values[idx] = maf_stats_30s.skewness; idx += 1;
        values[idx] = maf_stats_30s.kurtosis; idx += 1;

        // Frequency features (15 total: 3 bands × 5 signals)
        let rpm_fft = self.fft_analyzer.analyze(&rpm_30s);
        values[idx] = rpm_fft.power_low; idx += 1;
        values[idx] = rpm_fft.power_medium; idx += 1;
        values[idx] = rpm_fft.power_high; idx += 1;

        let coolant_fft = self.fft_analyzer.analyze(&coolant_30s);
        values[idx] = coolant_fft.power_low; idx += 1;
        values[idx] = coolant_fft.power_medium; idx += 1;
        values[idx] = coolant_fft.power_high; idx += 1;

        let speed_fft = self.fft_analyzer.analyze(&speed_30s);
        values[idx] = speed_fft.power_low; idx += 1;
        values[idx] = speed_fft.power_medium; idx += 1;
        values[idx] = speed_fft.power_high; idx += 1;

        let load_fft = self.fft_analyzer.analyze(&load_30s);
        values[idx] = load_fft.power_low; idx += 1;
        values[idx] = load_fft.power_medium; idx += 1;
        values[idx] = load_fft.power_high; idx += 1;

        let maf_fft = self.fft_analyzer.analyze(&maf_30s);
        values[idx] = maf_fft.power_low; idx += 1;
        values[idx] = maf_fft.power_medium; idx += 1;
        values[idx] = maf_fft.power_high; idx += 1;

        // Temporal features (10 total)
        values[idx] = rpm_stats_30s.rate_of_change; idx += 1;
        values[idx] = rpm_stats_30s.zero_crossings as f64; idx += 1;
        values[idx] = coolant_stats_30s.rate_of_change; idx += 1;
        values[idx] = coolant_stats_30s.zero_crossings as f64; idx += 1;
        values[idx] = speed_stats_30s.rate_of_change; idx += 1;
        values[idx] = speed_stats_30s.zero_crossings as f64; idx += 1;
        values[idx] = load_stats_30s.rate_of_change; idx += 1;
        values[idx] = load_stats_30s.zero_crossings as f64; idx += 1;
        values[idx] = maf_stats_30s.rate_of_change; idx += 1;
        values[idx] = maf_stats_30s.zero_crossings as f64;

        FeatureVector {
            values,
            timestamp_ms,
            coolant_temp_mean_30s: coolant_stats_30s.mean,
            coolant_temp_rate: coolant_stats_30s.rate_of_change,
            rpm_mean: rpm_stats_30s.mean,
            rpm_std_dev: rpm_stats_30s.std_dev,
        }
    }

    /// Extract features from a slice of frames directly
    pub fn extract_from_frames(&mut self, frames: &[SensorFrame]) -> FeatureVector {
        let buffer = RingBuffer::new(frames.len().max(1));
        for frame in frames {
            buffer.push(frame.clone());
        }
        self.extract(&buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_extraction() {
        let mut extractor = FeatureExtractor::new(5.0);
        let buffer = RingBuffer::new(100);
        
        // Add some test frames
        for i in 0..50 {
            buffer.push(SensorFrame {
                timestamp_ms: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
                rpm: 2000 + (i * 10) as u16,
                coolant_temp: 85,
                speed: 60,
                engine_load: 40,
                maf: 1500,
                ..Default::default()
            });
        }
        
        let features = extractor.extract(&buffer);
        
        // Check that features are populated
        assert!(features.rpm_mean > 0.0);
        assert!(features.coolant_temp_mean_30s > 0.0);
        assert_eq!(features.values.len(), FEATURE_DIMENSION);
    }
}
