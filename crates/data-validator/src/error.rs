//! Validation Error Types

use thiserror::Error;

/// Errors during data validation
#[derive(Debug, Clone, Error)]
pub enum ValidationError {
    /// Value out of allowed range
    #[error("{field} value {value} is out of range [{min}, {max}]")]
    OutOfRange {
        field: &'static str,
        value: f64,
        min: f64,
        max: f64,
    },

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected:02X}, got {actual:02X}")]
    ChecksumMismatch { expected: u8, actual: u8 },

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
}
