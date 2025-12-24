//! Lock-Free Ring Buffer
//!
//! Provides a high-performance SPSC ring buffer for sensor frame storage.

mod buffer;

pub use buffer::RingBuffer;

use serde::{Deserialize, Serialize};

/// Sensor frame stored in the ring buffer (from obd-protocol, duplicated to avoid circular dep)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensorFrame {
    pub timestamp_ms: u64,
    pub rpm: u16,
    pub speed: u8,
    pub coolant_temp: i16,
    pub engine_load: u8,
    pub maf: u16,
    pub fuel_trim_short: i16,
    pub fuel_trim_long: i16,
    pub o2_voltage: u16,
}
