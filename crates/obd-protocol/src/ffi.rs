//! FFI Bindings for C++ CAN/OBD-II Driver
//!
//! This module provides safe Rust wrappers around the C++ driver layer.
//! The C++ layer handles low-level CAN bus communication via SocketCAN
//! or ELM327 serial protocol, while Rust handles validation, feature
//! extraction, and application logic.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// FFI type aliases matching C structures
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CCanFrame {
    pub can_id: u32,
    pub dlc: u8,
    pub data: [u8; 8],
    pub timestamp_ns: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CSensorFrame {
    pub timestamp_ns: u64,
    pub rpm: u16,
    pub coolant_temp: u8,
    pub speed: u8,
    pub engine_load: u8,
    pub maf: u16,
    pub throttle_pos: u8,
    pub fuel_trim_short: i8,
    pub fuel_trim_long: i8,
    pub valid_mask: u8,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CDriverConfig {
    pub can_interface: *const c_char,
    pub serial_device: *const c_char,
    pub serial_baud_rate: i32,
    pub use_elm327: i32,
}

/// Error codes from C driver
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanErrorCode {
    Ok = 0,
    ErrorInit = -1,
    NotInitialized = -2,
    BusOff = -10,
    NoAck = -11,
    Timeout = -12,
    SerialOpen = -20,
    SerialTimeout = -21,
    ProtocolMismatch = -30,
    InvalidResponse = -31,
    NoData = -40,
    Unknown = -99,
}

impl From<i32> for CanErrorCode {
    fn from(code: i32) -> Self {
        match code {
            0 => Self::Ok,
            -1 => Self::ErrorInit,
            -2 => Self::NotInitialized,
            -10 => Self::BusOff,
            -11 => Self::NoAck,
            -12 => Self::Timeout,
            -20 => Self::SerialOpen,
            -21 => Self::SerialTimeout,
            -30 => Self::ProtocolMismatch,
            -31 => Self::InvalidResponse,
            -40 => Self::NoData,
            _ => Self::Unknown,
        }
    }
}

/// Driver errors
#[derive(Error, Debug)]
pub enum DriverError {
    #[error("Driver initialization failed: {0}")]
    Init(String),

    #[error("Driver not initialized")]
    NotInitialized,

    #[error("CAN bus off")]
    BusOff,

    #[error("Communication timeout")]
    Timeout,

    #[error("Serial port error: {0}")]
    Serial(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("No data available")]
    NoData,

    #[error("Unknown driver error: {0}")]
    Unknown(String),
}

impl From<CanErrorCode> for DriverError {
    fn from(code: CanErrorCode) -> Self {
        match code {
            CanErrorCode::Ok => unreachable!("Ok is not an error"),
            CanErrorCode::ErrorInit => Self::Init("initialization failed".to_string()),
            CanErrorCode::NotInitialized => Self::NotInitialized,
            CanErrorCode::BusOff => Self::BusOff,
            CanErrorCode::NoAck | CanErrorCode::Timeout => Self::Timeout,
            CanErrorCode::SerialOpen => Self::Serial("failed to open port".to_string()),
            CanErrorCode::SerialTimeout => Self::Serial("timeout".to_string()),
            CanErrorCode::ProtocolMismatch => Self::Protocol("mismatch".to_string()),
            CanErrorCode::InvalidResponse => Self::Protocol("invalid response".to_string()),
            CanErrorCode::NoData => Self::NoData,
            CanErrorCode::Unknown => Self::Unknown("unknown error".to_string()),
        }
    }
}

// Extern C functions from the C++ driver
// These are linked at compile time via build.rs
#[cfg(feature = "ffi")]
extern "C" {
    fn can_driver_init(config: *const CDriverConfig) -> i32;
    fn can_driver_shutdown();
    fn can_driver_is_initialized() -> i32;
    fn can_driver_read_frame(frame_out: *mut CCanFrame) -> i32;
    fn can_driver_read_sensor_frame(frame_out: *mut CSensorFrame) -> i32;
    fn can_driver_last_error() -> *const c_char;
    fn can_driver_error_str(code: i32) -> *const c_char;
}

// Mock implementations for when FFI is not available
#[cfg(not(feature = "ffi"))]
mod mock_ffi {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    static MOCK_INITIALIZED: AtomicBool = AtomicBool::new(false);
    static MOCK_FRAME_COUNT: AtomicU64 = AtomicU64::new(0);
    
    pub unsafe fn can_driver_init(_config: *const CDriverConfig) -> i32 {
        MOCK_INITIALIZED.store(true, Ordering::SeqCst);
        0
    }
    
    pub unsafe fn can_driver_shutdown() {
        MOCK_INITIALIZED.store(false, Ordering::SeqCst);
    }
    
    pub unsafe fn can_driver_is_initialized() -> i32 {
        if MOCK_INITIALIZED.load(Ordering::SeqCst) { 1 } else { 0 }
    }
    
    pub unsafe fn can_driver_read_frame(frame_out: *mut CCanFrame) -> i32 {
        if !MOCK_INITIALIZED.load(Ordering::SeqCst) {
            return -2;
        }
        
        let count = MOCK_FRAME_COUNT.fetch_add(1, Ordering::SeqCst);
        let frame = &mut *frame_out;
        
        frame.can_id = 0x7E8;
        frame.dlc = 8;
        frame.timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        
        // Generate mock RPM data
        frame.data[0] = 0x04;
        frame.data[1] = 0x41;
        frame.data[2] = 0x0C;
        let rpm = 2500 + (count % 500) as u16;
        frame.data[3] = ((rpm * 4) >> 8) as u8;
        frame.data[4] = ((rpm * 4) & 0xFF) as u8;
        
        1
    }
    
    pub unsafe fn can_driver_read_sensor_frame(frame_out: *mut CSensorFrame) -> i32 {
        if !MOCK_INITIALIZED.load(Ordering::SeqCst) {
            return -2;
        }
        
        let count = MOCK_FRAME_COUNT.fetch_add(1, Ordering::SeqCst);
        let frame = &mut *frame_out;
        
        frame.timestamp_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        
        frame.rpm = 2500 + (count % 500) as u16;
        frame.coolant_temp = 85;
        frame.speed = 60 + (count % 20) as u8;
        frame.engine_load = 40 + (count % 30) as u8;
        frame.maf = 1500;
        frame.throttle_pos = 25;
        frame.fuel_trim_short = 0;
        frame.fuel_trim_long = 2;
        frame.valid_mask = 0xFF;
        
        1
    }
    
    pub unsafe fn can_driver_last_error() -> *const c_char {
        static MSG: &[u8] = b"No error\0";
        MSG.as_ptr() as *const c_char
    }
    
    pub unsafe fn can_driver_error_str(_code: i32) -> *const c_char {
        static MSG: &[u8] = b"OK\0";
        MSG.as_ptr() as *const c_char
    }
}

#[cfg(not(feature = "ffi"))]
use mock_ffi::*;

/// Configuration for the CAN driver
#[derive(Debug, Clone)]
pub struct DriverConfig {
    /// CAN interface name (e.g., "can0", "vcan0")
    pub can_interface: String,
    /// Serial device for ELM327 (e.g., "/dev/ttyUSB0")
    pub serial_device: String,
    /// ELM327 baud rate (default: 38400)
    pub baud_rate: i32,
    /// Use ELM327 mode
    pub use_elm327: bool,
}

impl Default for DriverConfig {
    fn default() -> Self {
        Self {
            can_interface: "vcan0".to_string(),
            serial_device: "/dev/ttyUSB0".to_string(),
            baud_rate: 38400,
            use_elm327: false,
        }
    }
}

/// Safe wrapper around the C++ CAN driver
pub struct CanDriver {
    _initialized: AtomicBool,
}

impl CanDriver {
    /// Create and initialize a new CAN driver
    pub fn new(config: &DriverConfig) -> Result<Self, DriverError> {
        let can_iface = CString::new(config.can_interface.as_str())
            .map_err(|e| DriverError::Init(e.to_string()))?;
        let serial_dev = CString::new(config.serial_device.as_str())
            .map_err(|e| DriverError::Init(e.to_string()))?;

        let c_config = CDriverConfig {
            can_interface: can_iface.as_ptr(),
            serial_device: serial_dev.as_ptr(),
            serial_baud_rate: config.baud_rate,
            use_elm327: if config.use_elm327 { 1 } else { 0 },
        };

        let ret = unsafe { can_driver_init(&c_config) };
        
        if ret != 0 {
            let error_msg = unsafe {
                let ptr = can_driver_last_error();
                if ptr.is_null() {
                    "unknown error".to_string()
                } else {
                    CStr::from_ptr(ptr).to_string_lossy().into_owned()
                }
            };
            return Err(DriverError::Init(error_msg));
        }

        info!("CAN driver initialized: interface={}, serial={}", 
              config.can_interface, config.serial_device);

        Ok(Self {
            _initialized: AtomicBool::new(true),
        })
    }

    /// Read a raw CAN frame (non-blocking)
    pub fn read_frame(&self) -> Result<Option<CCanFrame>, DriverError> {
        let mut frame = CCanFrame {
            can_id: 0,
            dlc: 0,
            data: [0; 8],
            timestamp_ns: 0,
        };

        let ret = unsafe { can_driver_read_frame(&mut frame) };

        match ret {
            r if r > 0 => Ok(Some(frame)),
            0 => Ok(None),
            code => Err(CanErrorCode::from(code).into()),
        }
    }

    /// Read a decoded sensor frame (non-blocking)
    pub fn read_sensor_frame(&self) -> Result<Option<CSensorFrame>, DriverError> {
        let mut frame = CSensorFrame {
            timestamp_ns: 0,
            rpm: 0,
            coolant_temp: 0,
            speed: 0,
            engine_load: 0,
            maf: 0,
            throttle_pos: 0,
            fuel_trim_short: 0,
            fuel_trim_long: 0,
            valid_mask: 0,
        };

        let ret = unsafe { can_driver_read_sensor_frame(&mut frame) };

        match ret {
            r if r > 0 => Ok(Some(frame)),
            0 => Ok(None),
            code => Err(CanErrorCode::from(code).into()),
        }
    }

    /// Check if the driver is initialized
    pub fn is_initialized(&self) -> bool {
        unsafe { can_driver_is_initialized() == 1 }
    }
}

impl Drop for CanDriver {
    fn drop(&mut self) {
        info!("Shutting down CAN driver");
        unsafe { can_driver_shutdown() };
    }
}

/// Async wrapper around CanDriver for use with Tokio
pub struct AsyncCanDriver {
    receiver: mpsc::Receiver<CSensorFrame>,
    _shutdown: std::sync::Arc<AtomicBool>,
}

impl AsyncCanDriver {
    /// Spawn a new async CAN driver with a background polling thread
    pub fn spawn(config: DriverConfig) -> Result<Self, DriverError> {
        let (tx, rx) = mpsc::channel::<CSensorFrame>(1000);
        let shutdown = std::sync::Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Spawn polling thread
        std::thread::spawn(move || {
            let driver = match CanDriver::new(&config) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to initialize CAN driver: {}", e);
                    return;
                }
            };

            while !shutdown_clone.load(Ordering::SeqCst) {
                match driver.read_sensor_frame() {
                    Ok(Some(frame)) => {
                        if tx.blocking_send(frame).is_err() {
                            debug!("Receiver dropped, stopping CAN polling");
                            break;
                        }
                    }
                    Ok(None) => {
                        // No data, brief sleep
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => {
                        warn!("CAN read error: {}", e);
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                }
            }
        });

        Ok(Self {
            receiver: rx,
            _shutdown: shutdown,
        })
    }

    /// Receive the next sensor frame
    pub async fn next_frame(&mut self) -> Option<CSensorFrame> {
        self.receiver.recv().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_config_default() {
        let config = DriverConfig::default();
        assert_eq!(config.can_interface, "vcan0");
        assert_eq!(config.baud_rate, 38400);
        assert!(!config.use_elm327);
    }

    #[test]
    fn test_mock_driver() {
        let config = DriverConfig::default();
        let driver = CanDriver::new(&config).unwrap();
        
        // Read some mock frames
        for _ in 0..10 {
            let frame = driver.read_sensor_frame().unwrap();
            assert!(frame.is_some());
        }
    }

    #[test]
    fn test_error_code_conversion() {
        assert_eq!(CanErrorCode::from(0), CanErrorCode::Ok);
        assert_eq!(CanErrorCode::from(-10), CanErrorCode::BusOff);
        assert_eq!(CanErrorCode::from(-999), CanErrorCode::Unknown);
    }
}
