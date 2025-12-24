//! OBD-II PID Definitions and Response Parsing
//!
//! Defines the standard OBD-II Parameter IDs (PIDs) and their decoding formulas.

use serde::{Deserialize, Serialize};

/// Standard OBD-II PIDs for Mode 01 (current data)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Pid {
    /// Engine RPM (0x0C)
    Rpm = 0x0C,
    /// Vehicle speed (0x0D)
    Speed = 0x0D,
    /// Engine coolant temperature (0x05)
    CoolantTemp = 0x05,
    /// Calculated engine load (0x04)
    EngineLoad = 0x04,
    /// Mass air flow rate (0x10)
    Maf = 0x10,
    /// Short-term fuel trim bank 1 (0x06)
    ShortFuelTrim = 0x06,
    /// Long-term fuel trim bank 1 (0x07)
    LongFuelTrim = 0x07,
    /// Oxygen sensor voltage bank 1, sensor 1 (0x14)
    O2Voltage = 0x14,
    /// Intake manifold absolute pressure (0x0B)
    IntakeManifoldPressure = 0x0B,
    /// Throttle position (0x11)
    ThrottlePosition = 0x11,
}

impl Pid {
    /// Get the PID hex value
    pub fn as_hex(&self) -> u8 {
        *self as u8
    }

    /// Get the number of response bytes for this PID
    pub fn response_bytes(&self) -> usize {
        match self {
            Pid::Rpm | Pid::Maf | Pid::O2Voltage => 2,
            _ => 1,
        }
    }

    /// Get the sampling priority (higher = more frequent)
    pub fn sampling_priority(&self) -> u8 {
        match self {
            Pid::Rpm | Pid::Speed | Pid::CoolantTemp | Pid::EngineLoad => 10, // 5Hz
            Pid::Maf => 5, // 1Hz
            _ => 2, // 0.5Hz
        }
    }
}

/// Response from a PID query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PidResponse {
    /// The PID that was queried
    pub pid: u8,
    /// Timestamp when the response was received (Unix ms)
    pub timestamp_ms: u64,
    /// Decoded value
    pub value: f64,
    /// Raw bytes from the response
    pub raw_bytes: Vec<u8>,
}

impl PidResponse {
    /// Create a new PID response by decoding raw bytes
    pub fn decode(pid: u8, raw_bytes: Vec<u8>, timestamp_ms: u64) -> Self {
        let value = Self::decode_value(pid, &raw_bytes);
        Self {
            pid,
            timestamp_ms,
            value,
            raw_bytes,
        }
    }

    /// Decode the raw bytes to a value based on the PID formula
    fn decode_value(pid: u8, bytes: &[u8]) -> f64 {
        match pid {
            // RPM: ((A*256)+B)/4
            0x0C if bytes.len() >= 2 => {
                ((bytes[0] as f64 * 256.0) + bytes[1] as f64) / 4.0
            }
            // Speed: A (km/h)
            0x0D if !bytes.is_empty() => bytes[0] as f64,
            // Coolant Temp: A - 40 (°C)
            0x05 if !bytes.is_empty() => bytes[0] as f64 - 40.0,
            // Engine Load: A * 100 / 255 (%)
            0x04 if !bytes.is_empty() => bytes[0] as f64 * 100.0 / 255.0,
            // MAF: ((A*256)+B) / 100 (g/s)
            0x10 if bytes.len() >= 2 => {
                ((bytes[0] as f64 * 256.0) + bytes[1] as f64) / 100.0
            }
            // Short/Long fuel trim: (A - 128) * 100 / 128 (%)
            0x06 | 0x07 if !bytes.is_empty() => {
                (bytes[0] as f64 - 128.0) * 100.0 / 128.0
            }
            // O2 Voltage: A / 200 (V)
            0x14 if !bytes.is_empty() => bytes[0] as f64 / 200.0,
            // Intake manifold pressure: A (kPa)
            0x0B if !bytes.is_empty() => bytes[0] as f64,
            // Throttle position: A * 100 / 255 (%)
            0x11 if !bytes.is_empty() => bytes[0] as f64 * 100.0 / 255.0,
            _ => 0.0,
        }
    }
}

/// A complete sensor frame containing all collected PIDs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SensorFrame {
    /// Timestamp (Unix ms)
    pub timestamp_ms: u64,
    /// Engine RPM
    pub rpm: u16,
    /// Vehicle speed (km/h)
    pub speed: u8,
    /// Coolant temperature (°C, offset by +40 for storage as u8)
    pub coolant_temp: i16,
    /// Engine load (0-100%)
    pub engine_load: u8,
    /// Mass air flow rate (g/s * 100 for precision)
    pub maf: u16,
    /// Short-term fuel trim (% * 100, signed)
    pub fuel_trim_short: i16,
    /// Long-term fuel trim (% * 100, signed)
    pub fuel_trim_long: i16,
    /// O2 sensor voltage (V * 1000)
    pub o2_voltage: u16,
}

impl SensorFrame {
    /// Size of this struct in bytes (for ring buffer allocation)
    pub const SIZE_BYTES: usize = 48;

    /// Create a new empty frame with the given timestamp
    pub fn new(timestamp_ms: u64) -> Self {
        Self {
            timestamp_ms,
            ..Default::default()
        }
    }

    /// Update a field from a PID response
    pub fn update_from_response(&mut self, response: &PidResponse) {
        match response.pid {
            0x0C => self.rpm = response.value as u16,
            0x0D => self.speed = response.value as u8,
            0x05 => self.coolant_temp = response.value as i16,
            0x04 => self.engine_load = response.value as u8,
            0x10 => self.maf = (response.value * 100.0) as u16,
            0x06 => self.fuel_trim_short = (response.value * 100.0) as i16,
            0x07 => self.fuel_trim_long = (response.value * 100.0) as i16,
            0x14 => self.o2_voltage = (response.value * 1000.0) as u16,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpm_decode() {
        // 1A 2B => ((0x1A * 256) + 0x2B) / 4 = (26*256 + 43) / 4 = 6699/4 = 1674.75
        let response = PidResponse::decode(0x0C, vec![0x1A, 0x2B], 0);
        assert!((response.value - 1674.75).abs() < 0.01);
    }

    #[test]
    fn test_coolant_temp_decode() {
        // 0x73 = 115, so temp = 115 - 40 = 75°C
        let response = PidResponse::decode(0x05, vec![0x73], 0);
        assert!((response.value - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_speed_decode() {
        // 0x55 = 85 km/h
        let response = PidResponse::decode(0x0D, vec![0x55], 0);
        assert!((response.value - 85.0).abs() < 0.01);
    }

    #[test]
    fn test_fuel_trim_decode() {
        // 0x80 = 128, so trim = (128-128)*100/128 = 0%
        let response = PidResponse::decode(0x06, vec![0x80], 0);
        assert!((response.value - 0.0).abs() < 0.01);

        // 0x90 = 144, so trim = (144-128)*100/128 = 12.5%
        let response = PidResponse::decode(0x06, vec![0x90], 0);
        assert!((response.value - 12.5).abs() < 0.01);
    }
}
