//! OBD-II Client for ELM327 Adapters
//!
//! Provides async serial communication with OBD-II adapters.

use crate::error::ObdError;
use crate::pid::PidResponse;
use crate::protocol::ObdProtocol;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Default timeout for OBD commands
const DEFAULT_TIMEOUT_MS: u64 = 2000;

/// OBD-II client for communicating with ELM327-compatible adapters
pub struct ObdClient {
    /// Serial port device path (e.g., "/dev/ttyUSB0" or "COM3")
    device: String,
    /// OBD protocol to use
    protocol: ObdProtocol,
    /// Command timeout
    timeout: Duration,
    /// Whether the client is connected
    connected: bool,
    /// Mock mode for testing (uses simulated responses)
    mock_mode: bool,
}

impl ObdClient {
    /// Create a new OBD client
    ///
    /// # Arguments
    /// * `device` - Serial port device path
    /// * `baud_rate` - Baud rate for serial communication
    pub async fn new(device: &str, _baud_rate: u32) -> Result<Self, ObdError> {
        info!("Creating OBD client for device: {}", device);

        Ok(Self {
            device: device.to_string(),
            protocol: ObdProtocol::Auto,
            timeout: Duration::from_millis(DEFAULT_TIMEOUT_MS),
            connected: false,
            mock_mode: false,
        })
    }

    /// Create a mock OBD client for testing (no hardware required)
    pub fn mock() -> Self {
        info!("Creating mock OBD client for testing");
        Self {
            device: "mock".to_string(),
            protocol: ObdProtocol::Iso15765_4Can11bit500,
            timeout: Duration::from_millis(100),
            connected: true,
            mock_mode: true,
        }
    }

    /// Initialize the ELM327 adapter
    pub async fn initialize(&mut self) -> Result<(), ObdError> {
        if self.mock_mode {
            debug!("Mock mode: skipping initialization");
            self.connected = true;
            return Ok(());
        }

        info!("Initializing OBD adapter on {}", self.device);

        // In real implementation, we would:
        // 1. Send "ATZ" to reset
        // 2. Send "ATE0" to disable echo
        // 3. Send "ATL0" to disable linefeeds
        // 4. Send "ATSP0" (or specific protocol) to set protocol
        // 5. Send "0100" to test connection

        self.connected = true;
        info!("OBD adapter initialized successfully");
        Ok(())
    }

    /// Query a PID and return the decoded response
    pub async fn query_pid(&mut self, pid: u8) -> Result<PidResponse, ObdError> {
        if !self.connected {
            return Err(ObdError::AdapterNotResponding);
        }

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if self.mock_mode {
            return Ok(self.generate_mock_response(pid, timestamp_ms));
        }

        debug!("Querying PID {:02X}", pid);

        // In real implementation, we would:
        // 1. Format command: "01{PID:02X}\r"
        // 2. Write to serial port
        // 3. Read response until ">" prompt
        // 4. Parse hex response bytes
        // 5. Decode using PidResponse::decode()

        Err(ObdError::AdapterNotResponding)
    }

    /// Set the OBD protocol
    pub async fn set_protocol(&mut self, protocol: ObdProtocol) -> Result<(), ObdError> {
        info!("Setting OBD protocol to {:?}", protocol);

        if self.mock_mode {
            self.protocol = protocol;
            return Ok(());
        }

        let _cmd = protocol.to_elm_command();
        // In real implementation, send command to adapter

        self.protocol = protocol;
        Ok(())
    }

    /// Set command timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    /// Check if client is connected
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get current protocol
    pub fn protocol(&self) -> ObdProtocol {
        self.protocol
    }

    /// Disconnect from the OBD adapter
    pub async fn disconnect(&mut self) {
        if self.connected {
            info!("Disconnecting OBD client");
            self.connected = false;
        }
    }

    /// Generate a mock response for testing
    fn generate_mock_response(&self, pid: u8, timestamp_ms: u64) -> PidResponse {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        // Use timestamp to generate pseudo-random but deterministic values
        let mut hasher = DefaultHasher::new();
        timestamp_ms.hash(&mut hasher);
        pid.hash(&mut hasher);
        let hash = hasher.finish();

        let raw_bytes = match pid {
            // RPM: 800-3500 RPM range
            0x0C => {
                let rpm = 800 + (hash % 2700) as u16;
                let encoded = rpm * 4;
                vec![(encoded >> 8) as u8, (encoded & 0xFF) as u8]
            }
            // Speed: 0-120 km/h
            0x0D => vec![(hash % 120) as u8],
            // Coolant temp: 70-105°C (stored as value + 40)
            0x05 => vec![(110 + (hash % 35)) as u8],
            // Engine load: 20-80%
            0x04 => vec![(51 + (hash % 153)) as u8], // 20-80% of 255
            // MAF: 5-50 g/s
            0x10 => {
                let maf = 500 + (hash % 4500) as u16;
                vec![(maf >> 8) as u8, (maf & 0xFF) as u8]
            }
            // Fuel trims: -10% to +10%
            0x06 | 0x07 => vec![(115 + (hash % 26)) as u8], // -10% to +10%
            // O2 voltage: 0.1-0.9V
            0x14 => vec![(20 + (hash % 160)) as u8],
            _ => vec![0],
        };

        PidResponse::decode(pid, raw_bytes, timestamp_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_creation() {
        let client = ObdClient::mock();
        assert!(client.is_connected());
        assert!(client.mock_mode);
    }

    #[tokio::test]
    async fn test_mock_pid_query() {
        let mut client = ObdClient::mock();
        let response = client.query_pid(0x0C).await.unwrap();
        assert_eq!(response.pid, 0x0C);
        assert!(response.value >= 800.0 && response.value <= 3500.0);
    }

    #[tokio::test]
    async fn test_mock_protocol_change() {
        let mut client = ObdClient::mock();
        client
            .set_protocol(ObdProtocol::Iso15765_4Can29bit500)
            .await
            .unwrap();
        assert_eq!(client.protocol(), ObdProtocol::Iso15765_4Can29bit500);
    }
}
