//! Feature Engineering Engine
//!
//! Provides statistical and frequency domain feature extraction for ML inference.

mod features;
mod fft;
mod statistics;

pub use features::{FeatureVector, FeatureExtractor};
pub use fft::FftAnalyzer;
pub use statistics::StatisticalFeatures;
