//! ONNX Inference Engine
//!
//! Provides high-performance ML inference using tract-onnx.

mod batcher;
mod engine;

pub use batcher::InferenceBatcher;
pub use engine::{InferenceEngine, InferenceResult, Prediction};

use thiserror::Error;

/// Errors during inference
#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("Model load failed: {0}")]
    ModelLoadError(String),
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
    #[error("Invalid input shape: expected {expected}, got {actual}")]
    InvalidInputShape { expected: String, actual: String },
    #[error("Inference timeout after {0}ms")]
    Timeout(u64),
}
