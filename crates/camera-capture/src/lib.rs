//! Camera Capture Library for Vehicle Diagnostics
//!
//! Provides V4L2 camera capture with FFI bindings to C++ drivers.
//! Supports:
//! - Cabin IR camera (640x480 @ 15fps) for DMS
//! - Road dashcam (1080p @ 30fps) for ADAS
//! - IMU sensor for crash detection

pub mod ffi;
pub mod frame;
pub mod imu;

pub use frame::{VideoFrame, PixelFormat};
pub use imu::{ImuData, ImuService};

use thiserror::Error;

/// Camera error types
#[derive(Error, Debug)]
pub enum CameraError {
    #[error("Failed to open camera: {0}")]
    Open(String),
    
    #[error("Invalid format: {0}")]
    Format(String),
    
    #[error("Buffer allocation failed")]
    Buffer,
    
    #[error("Streaming error: {0}")]
    Stream(String),
    
    #[error("Capture timeout")]
    Timeout,
    
    #[error("Camera not initialized")]
    NotInitialized,
}

/// Camera type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraType {
    /// Cabin-facing IR camera for driver monitoring
    Cabin,
    /// Road-facing dashcam for ADAS
    Road,
}

/// Camera configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// Device path (e.g., "/dev/video0")
    pub device: String,
    /// Camera type
    pub camera_type: CameraType,
    /// Capture width
    pub width: u32,
    /// Capture height
    pub height: u32,
    /// Target FPS
    pub fps: u32,
    /// Enable IR mode (cabin only)
    pub enable_ir: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device: "/dev/video0".to_string(),
            camera_type: CameraType::Cabin,
            width: 640,
            height: 480,
            fps: 15,
            enable_ir: true,
        }
    }
}

impl CameraConfig {
    /// Create cabin camera config (DMS)
    pub fn cabin() -> Self {
        Self {
            device: "/dev/video0".to_string(),
            camera_type: CameraType::Cabin,
            width: 640,
            height: 480,
            fps: 15,
            enable_ir: true,
        }
    }
    
    /// Create road camera config (ADAS)
    pub fn road() -> Self {
        Self {
            device: "/dev/video1".to_string(),
            camera_type: CameraType::Road,
            width: 1920,
            height: 1080,
            fps: 30,
            enable_ir: false,
        }
    }
}
