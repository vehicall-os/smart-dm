//! Alerting System
//!
//! Provides confidence calibration, alert deduplication, and severity mapping.

mod manager;

pub use manager::{AlertManager, AlertConfig, AlertState};
