//! OBD-II Protocol Implementation
//!
//! This crate provides async serial communication with ELM327-compatible
//! OBD-II adapters. It supports ISO 15765-4 (CAN) and legacy protocols.
//!
//! ## FFI Layer
//!
//! The `ffi` module provides safe Rust bindings to the C++ CAN driver for
//! low-latency hardware interaction.

mod client;
mod error;
pub mod ffi;
mod pid;
mod protocol;

pub use client::ObdClient;
pub use error::ObdError;
pub use ffi::{AsyncCanDriver, CanDriver, CSensorFrame, DriverConfig, DriverError};
pub use pid::{Pid, PidResponse, SensorFrame};
pub use protocol::ObdProtocol;

/// OBD-II mode constants
pub mod mode {
    /// Current data
    pub const CURRENT_DATA: u8 = 0x01;
    /// Freeze frame data
    pub const FREEZE_FRAME: u8 = 0x02;
    /// Diagnostic trouble codes
    pub const READ_DTC: u8 = 0x03;
    /// Clear trouble codes
    pub const CLEAR_DTC: u8 = 0x04;
    /// Vehicle information
    pub const VEHICLE_INFO: u8 = 0x09;
}
