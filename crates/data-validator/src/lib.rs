//! Data Validation and Normalization
//!
//! Provides input validation, range checking, and normalization for OBD-II data.

mod error;
mod filter;
mod normalizer;
mod validator;

pub use error::ValidationError;
pub use filter::MedianFilter;
pub use normalizer::{Normalizer, NormalizationMethod};
pub use validator::{Validator, ValidationConfig, ValidationResult};
