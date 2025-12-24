//! Repository Implementation

use crate::StorageError;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;
use tracing::{debug, info};

/// Sensor log record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorRecord {
    pub timestamp_ms: i64,
    pub rpm: i32,
    pub speed: i32,
    pub coolant_temp: i32,
    pub engine_load: i32,
    pub maf: f64,
    pub fuel_trim_short: f64,
    pub fuel_trim_long: f64,
}

/// Prediction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRecord {
    pub id: i64,
    pub timestamp_ms: i64,
    pub fault_class: String,
    pub confidence: f64,
    pub severity: String,
}

/// Repository for data access (in-memory implementation for now)
pub struct Repository {
    /// Sensor records (in-memory)
    sensor_log: Mutex<VecDeque<SensorRecord>>,
    /// Prediction records (in-memory)
    predictions: Mutex<Vec<PredictionRecord>>,
    /// Max sensor records (7 days at 5Hz = ~3M, but we limit for memory)
    max_sensor_records: usize,
    /// Max prediction records
    max_prediction_records: usize,
    /// Next prediction ID
    next_prediction_id: Mutex<i64>,
}

impl Repository {
    /// Create a new in-memory repository
    pub fn new() -> Self {
        info!("Creating in-memory repository");
        Self {
            sensor_log: Mutex::new(VecDeque::with_capacity(10000)),
            predictions: Mutex::new(Vec::with_capacity(1000)),
            max_sensor_records: 100_000, // ~5.5 hours at 5Hz
            max_prediction_records: 10_000,
            next_prediction_id: Mutex::new(1),
        }
    }

    /// Create a new repository with SQLite (placeholder)
    pub async fn with_sqlite(_db_path: &str) -> Result<Self, StorageError> {
        // In real implementation, we would use sqlx here:
        // let pool = SqlitePool::connect(db_path).await?;
        // Run migrations, setup WAL mode, etc.
        
        Ok(Self::new())
    }

    /// Insert a sensor record
    pub fn insert_sensor(&self, record: SensorRecord) -> Result<(), StorageError> {
        let mut log = self.sensor_log.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;

        // Enforce retention
        while log.len() >= self.max_sensor_records {
            log.pop_front();
        }

        log.push_back(record);
        Ok(())
    }

    /// Insert a prediction record
    pub fn insert_prediction(&self, mut record: PredictionRecord) -> Result<i64, StorageError> {
        let mut predictions = self.predictions.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;

        // Get next ID
        let mut id = self.next_prediction_id.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;
        
        record.id = *id;
        *id += 1;

        // Enforce retention
        if predictions.len() >= self.max_prediction_records {
            predictions.remove(0);
        }

        let returned_id = record.id;
        predictions.push(record);
        debug!("Inserted prediction with ID {}", returned_id);
        
        Ok(returned_id)
    }

    /// Get recent sensor records
    pub fn get_sensors(&self, limit: usize) -> Result<Vec<SensorRecord>, StorageError> {
        let log = self.sensor_log.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;

        Ok(log.iter().rev().take(limit).cloned().collect())
    }

    /// Get sensor records since a timestamp
    pub fn get_sensors_since(&self, since_ms: i64) -> Result<Vec<SensorRecord>, StorageError> {
        let log = self.sensor_log.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;

        Ok(log.iter().filter(|r| r.timestamp_ms >= since_ms).cloned().collect())
    }

    /// Get predictions with optional filters
    pub fn get_predictions(
        &self,
        severity: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PredictionRecord>, StorageError> {
        let predictions = self.predictions.lock().map_err(|e| {
            StorageError::DatabaseError(format!("Lock error: {}", e))
        })?;

        let filtered: Vec<_> = predictions
            .iter()
            .rev()
            .filter(|p| severity.map_or(true, |s| p.severity == s))
            .take(limit)
            .cloned()
            .collect();

        Ok(filtered)
    }

    /// Get total sensor count
    pub fn sensor_count(&self) -> usize {
        self.sensor_log.lock().map(|l| l.len()).unwrap_or(0)
    }

    /// Get total prediction count
    pub fn prediction_count(&self) -> usize {
        self.predictions.lock().map(|p| p.len()).unwrap_or(0)
    }

    /// Clear all data (for testing)
    pub fn clear(&self) {
        if let Ok(mut log) = self.sensor_log.lock() {
            log.clear();
        }
        if let Ok(mut preds) = self.predictions.lock() {
            preds.clear();
        }
    }
}

impl Default for Repository {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_insert_and_retrieve() {
        let repo = Repository::new();
        
        let record = SensorRecord {
            timestamp_ms: 1234567890,
            rpm: 3000,
            speed: 60,
            coolant_temp: 85,
            engine_load: 50,
            maf: 12.5,
            fuel_trim_short: 2.0,
            fuel_trim_long: 1.5,
        };
        
        repo.insert_sensor(record.clone()).unwrap();
        
        let sensors = repo.get_sensors(10).unwrap();
        assert_eq!(sensors.len(), 1);
        assert_eq!(sensors[0].rpm, 3000);
    }

    #[test]
    fn test_prediction_insert() {
        let repo = Repository::new();
        
        let record = PredictionRecord {
            id: 0,
            timestamp_ms: 1234567890,
            fault_class: "overheating".to_string(),
            confidence: 0.85,
            severity: "high".to_string(),
        };
        
        let id = repo.insert_prediction(record).unwrap();
        assert_eq!(id, 1);
        
        let preds = repo.get_predictions(None, 10).unwrap();
        assert_eq!(preds.len(), 1);
        assert_eq!(preds[0].fault_class, "overheating");
    }

    #[test]
    fn test_retention_limit() {
        let mut repo = Repository::new();
        repo.max_sensor_records = 5;
        
        for i in 0..10 {
            repo.insert_sensor(SensorRecord {
                timestamp_ms: i,
                rpm: i as i32 * 100,
                ..Default::default()
            }).unwrap();
        }
        
        assert_eq!(repo.sensor_count(), 5);
    }
}

impl Default for SensorRecord {
    fn default() -> Self {
        Self {
            timestamp_ms: 0,
            rpm: 0,
            speed: 0,
            coolant_temp: 0,
            engine_load: 0,
            maf: 0.0,
            fuel_trim_short: 0.0,
            fuel_trim_long: 0.0,
        }
    }
}
