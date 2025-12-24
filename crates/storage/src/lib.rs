//! Storage Layer
//!
//! Provides SQLite persistence with repository pattern.

mod repository;

pub use repository::{Repository, SensorRecord, PredictionRecord};

use thiserror::Error;

/// Storage errors
#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Record not found")]
    NotFound,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}
