//! IMU (Inertial Measurement Unit) service

use std::ffi::CString;
use std::os::raw::c_char;
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

/// IMU error types
#[derive(Error, Debug)]
pub enum ImuError {
    #[error("Failed to open IMU: {0}")]
    Open(String),
    
    #[error("IMU read failed")]
    Read,
    
    #[error("IMU not initialized")]
    NotInitialized,
}

/// Raw IMU data (matches C struct)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CImuData {
    pub accel_x: i16,
    pub accel_y: i16,
    pub accel_z: i16,
    pub gyro_x: i16,
    pub gyro_y: i16,
    pub gyro_z: i16,
    pub temperature: i16,
    pub timestamp_ns: u64,
}

/// Processed IMU data with physical units
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CImuProcessed {
    pub accel_x_g: f32,
    pub accel_y_g: f32,
    pub accel_z_g: f32,
    pub gyro_x_dps: f32,
    pub gyro_y_dps: f32,
    pub gyro_z_dps: f32,
    pub temperature_c: f32,
    pub g_force: f32,
    pub timestamp_ns: u64,
}

/// IMU configuration
#[repr(C)]
pub struct CImuConfig {
    pub i2c_device: *const c_char,
    pub i2c_address: u8,
    pub sample_rate_hz: i32,
}

// FFI functions
extern "C" {
    fn imu_init(config: *const CImuConfig) -> i32;
    fn imu_shutdown();
    fn imu_read_raw(data: *mut CImuData) -> i32;
    fn imu_read_processed(data: *mut CImuProcessed) -> i32;
    fn imu_is_initialized() -> i32;
    fn imu_last_error() -> *const c_char;
}

/// Processed IMU data
#[derive(Debug, Clone, Copy)]
pub struct ImuData {
    /// Acceleration in X (g)
    pub accel_x: f32,
    /// Acceleration in Y (g)
    pub accel_y: f32,
    /// Acceleration in Z (g)
    pub accel_z: f32,
    /// Angular velocity X (deg/s)
    pub gyro_x: f32,
    /// Angular velocity Y (deg/s)
    pub gyro_y: f32,
    /// Angular velocity Z (deg/s)
    pub gyro_z: f32,
    /// Temperature (Celsius)
    pub temperature: f32,
    /// Total G-force magnitude
    pub g_force: f32,
    /// Timestamp (nanoseconds)
    pub timestamp_ns: u64,
}

impl From<CImuProcessed> for ImuData {
    fn from(c: CImuProcessed) -> Self {
        Self {
            accel_x: c.accel_x_g,
            accel_y: c.accel_y_g,
            accel_z: c.accel_z_g,
            gyro_x: c.gyro_x_dps,
            gyro_y: c.gyro_y_dps,
            gyro_z: c.gyro_z_dps,
            temperature: c.temperature_c,
            g_force: c.g_force,
            timestamp_ns: c.timestamp_ns,
        }
    }
}

/// IMU configuration
#[derive(Debug, Clone)]
pub struct ImuConfig {
    /// I2C device path
    pub device: String,
    /// I2C address (default: 0x68)
    pub address: u8,
    /// Sample rate in Hz
    pub sample_rate: u32,
}

impl Default for ImuConfig {
    fn default() -> Self {
        Self {
            device: "/dev/i2c-1".to_string(),
            address: 0x68,
            sample_rate: 100,
        }
    }
}

/// IMU driver wrapper
pub struct ImuDriver {
    _device: CString,
}

impl ImuDriver {
    /// Initialize IMU
    pub fn new(config: &ImuConfig) -> Result<Self, ImuError> {
        let device = CString::new(config.device.as_str())
            .map_err(|e| ImuError::Open(e.to_string()))?;
        
        let c_config = CImuConfig {
            i2c_device: device.as_ptr(),
            i2c_address: config.address,
            sample_rate_hz: config.sample_rate as i32,
        };

        let ret = unsafe { imu_init(&c_config) };
        if ret != 0 {
            return Err(ImuError::Open(format!("Init failed: {}", ret)));
        }

        Ok(Self { _device: device })
    }

    /// Read processed IMU data
    pub fn read(&self) -> Result<ImuData, ImuError> {
        let mut data = CImuProcessed {
            accel_x_g: 0.0,
            accel_y_g: 0.0,
            accel_z_g: 0.0,
            gyro_x_dps: 0.0,
            gyro_y_dps: 0.0,
            gyro_z_dps: 0.0,
            temperature_c: 0.0,
            g_force: 0.0,
            timestamp_ns: 0,
        };

        let ret = unsafe { imu_read_processed(&mut data) };
        if ret != 0 {
            return Err(ImuError::Read);
        }

        Ok(ImuData::from(data))
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        unsafe { imu_is_initialized() == 1 }
    }
}

impl Drop for ImuDriver {
    fn drop(&mut self) {
        unsafe { imu_shutdown() };
    }
}

/// Async IMU service for tokio
pub struct ImuService {
    receiver: mpsc::Receiver<ImuData>,
    _shutdown: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl ImuService {
    /// Spawn IMU service with configurable sample rate
    pub fn spawn(config: ImuConfig) -> Result<Self, ImuError> {
        let sample_rate = config.sample_rate;
        let (tx, rx) = mpsc::channel::<ImuData>(100);
        let shutdown = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        std::thread::spawn(move || {
            let driver = match ImuDriver::new(&config) {
                Ok(d) => d,
                Err(e) => {
                    error!("Failed to initialize IMU: {}", e);
                    return;
                }
            };

            let interval = std::time::Duration::from_micros(1_000_000 / sample_rate as u64);
            
            while !shutdown_clone.load(std::sync::atomic::Ordering::SeqCst) {
                match driver.read() {
                    Ok(data) => {
                        if tx.blocking_send(data).is_err() {
                            debug!("IMU receiver dropped");
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("IMU read error: {}", e);
                    }
                }
                std::thread::sleep(interval);
            }
        });

        Ok(Self {
            receiver: rx,
            _shutdown: shutdown,
        })
    }

    /// Receive next IMU sample
    pub async fn next(&mut self) -> Option<ImuData> {
        self.receiver.recv().await
    }
}
