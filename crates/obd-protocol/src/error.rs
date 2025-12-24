//! OBD-II Error Types

use thiserror::Error;

/// Errors that can occur during OBD-II communication
#[derive(Debug, Error)]
pub enum ObdError {
    /// Serial port connection error
    #[error("Serial port error: {0}")]
    SerialError(String),

    /// Timeout waiting for response
    #[error("Timeout waiting for OBD response after {0}ms")]
    Timeout(u64),

    /// Invalid response from adapter
    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    /// Checksum mismatch
    #[error("Checksum mismatch: expected {expected:02X}, got {actual:02X}")]
    ChecksumError { expected: u8, actual: u8 },

    /// Protocol not supported
    #[error("Protocol not supported: {0}")]
    UnsupportedProtocol(String),

    /// PID not supported by vehicle
    #[error("PID {0:02X} not supported by vehicle")]
    PidNotSupported(u8),

    /// Adapter not responding
    #[error("OBD adapter not responding")]
    AdapterNotResponding,

    /// CAN bus error
    #[error("CAN bus error: {0}")]
    CanBusError(String),

    /// Vehicle not connected
    #[error("Vehicle ignition is off or not connected")]
    VehicleNotConnected,
}

impl From<std::io::Error> for ObdError {
    fn from(err: std::io::Error) -> Self {
        ObdError::SerialError(err.to_string())
    }
}
