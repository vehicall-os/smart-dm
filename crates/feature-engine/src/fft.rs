//! FFT-based Frequency Analysis

use rustfft::{FftPlanner, num_complex::Complex};

/// Frequency band definitions (Hz)
#[derive(Debug, Clone, Copy)]
pub struct FrequencyBands {
    /// Low frequency band (0-2 Hz)
    pub low: (f64, f64),
    /// Medium frequency band (2-5 Hz)
    pub medium: (f64, f64),
    /// High frequency band (5-10 Hz)
    pub high: (f64, f64),
}

impl Default for FrequencyBands {
    fn default() -> Self {
        Self {
            low: (0.0, 2.0),
            medium: (2.0, 5.0),
            high: (5.0, 10.0),
        }
    }
}

/// FFT Analyzer for frequency domain features
pub struct FftAnalyzer {
    /// FFT planner for efficient computation
    planner: FftPlanner<f64>,
    /// Frequency bands to analyze
    bands: FrequencyBands,
    /// Sampling frequency (Hz)
    sample_rate: f64,
}

/// Power spectral density in frequency bands
#[derive(Debug, Clone, Default)]
pub struct SpectralFeatures {
    /// Power in low frequency band
    pub power_low: f64,
    /// Power in medium frequency band
    pub power_medium: f64,
    /// Power in high frequency band
    pub power_high: f64,
    /// Dominant frequency
    pub dominant_frequency: f64,
    /// Total spectral power
    pub total_power: f64,
}

impl FftAnalyzer {
    /// Create a new FFT analyzer
    pub fn new(sample_rate: f64) -> Self {
        Self {
            planner: FftPlanner::new(),
            bands: FrequencyBands::default(),
            sample_rate,
        }
    }

    /// Apply Hamming window to reduce spectral leakage
    fn apply_hamming_window(signal: &mut [f64]) {
        let n = signal.len();
        for i in 0..n {
            let window = 0.54 - 0.46 * (2.0 * std::f64::consts::PI * i as f64 / (n - 1) as f64).cos();
            signal[i] *= window;
        }
    }

    /// Compute spectral features from a signal
    pub fn analyze(&mut self, signal: &[f64]) -> SpectralFeatures {
        if signal.is_empty() {
            return SpectralFeatures::default();
        }

        let n = signal.len();
        
        // Apply window
        let mut windowed: Vec<f64> = signal.to_vec();
        Self::apply_hamming_window(&mut windowed);
        
        // Convert to complex
        let mut buffer: Vec<Complex<f64>> = windowed
            .iter()
            .map(|&v| Complex::new(v, 0.0))
            .collect();
        
        // Perform FFT
        let fft = self.planner.plan_fft_forward(n);
        fft.process(&mut buffer);
        
        // Compute power spectrum (magnitude squared, normalized)
        let power_spectrum: Vec<f64> = buffer
            .iter()
            .take(n / 2) // Only positive frequencies
            .map(|c| (c.norm_sqr()) / (n as f64))
            .collect();
        
        // Frequency resolution
        let freq_resolution = self.sample_rate / n as f64;
        
        // Compute band powers
        let mut power_low = 0.0;
        let mut power_medium = 0.0;
        let mut power_high = 0.0;
        let mut max_power = 0.0;
        let mut dominant_freq_idx = 0;
        
        for (i, &power) in power_spectrum.iter().enumerate() {
            let freq = i as f64 * freq_resolution;
            
            if freq >= self.bands.low.0 && freq < self.bands.low.1 {
                power_low += power;
            } else if freq >= self.bands.medium.0 && freq < self.bands.medium.1 {
                power_medium += power;
            } else if freq >= self.bands.high.0 && freq < self.bands.high.1 {
                power_high += power;
            }
            
            if power > max_power {
                max_power = power;
                dominant_freq_idx = i;
            }
        }
        
        let total_power = power_spectrum.iter().sum();
        let dominant_frequency = dominant_freq_idx as f64 * freq_resolution;
        
        SpectralFeatures {
            power_low,
            power_medium,
            power_high,
            dominant_frequency,
            total_power,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_sine_wave() {
        let mut analyzer = FftAnalyzer::new(100.0); // 100 Hz sample rate
        
        // Generate 2 Hz sine wave
        let signal: Vec<f64> = (0..256)
            .map(|i| (2.0 * std::f64::consts::PI * 2.0 * i as f64 / 100.0).sin())
            .collect();
        
        let features = analyzer.analyze(&signal);
        
        // Dominant frequency should be around 2 Hz
        assert!((features.dominant_frequency - 2.0).abs() < 1.0);
        // Most power should be in low band
        assert!(features.power_low > features.power_high);
    }

    #[test]
    fn test_empty_signal() {
        let mut analyzer = FftAnalyzer::new(100.0);
        let features = analyzer.analyze(&[]);
        assert_eq!(features.total_power, 0.0);
    }
}
