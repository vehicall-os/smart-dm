//! FFI bindings for C++ camera capture

use std::ffi::CString;
use std::os::raw::c_char;

use crate::{CameraConfig, CameraError, CameraType};

/// C pixel format enum
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CPixelFormat {
    Rgb24 = 0,
    Mjpeg = 1,
    H264 = 2,
    Yuyv = 3,
    Nv12 = 4,
}

/// C camera type enum
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub enum CCameraType {
    CabinIr = 0,
    Road = 1,
    External = 2,
}

/// C video frame structure (matches camera_capture.h)
#[repr(C)]
pub struct CVideoFrame {
    pub data: *mut u8,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub stride: u32,
    pub format: CPixelFormat,
    pub timestamp_ns: u64,
    pub sequence: u32,
    pub buffer_id: i32,
}

/// C camera configuration
#[repr(C)]
pub struct CCameraConfig {
    pub device: *const c_char,
    pub camera_type: CCameraType,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub format: CPixelFormat,
    pub enable_ir: i32,
    pub buffer_count: i32,
}

// Cabin camera FFI functions
extern "C" {
    fn cabin_camera_init(config: *const CCameraConfig) -> i32;
    fn cabin_camera_start() -> i32;
    fn cabin_camera_stop();
    fn cabin_camera_shutdown();
    fn cabin_camera_read_frame(timeout_ms: i32) -> *mut CVideoFrame;
    fn cabin_camera_release_frame(frame: *mut CVideoFrame);
    fn cabin_camera_is_streaming() -> i32;
    fn cabin_camera_last_error() -> *const c_char;
}

// Road camera FFI functions
extern "C" {
    fn road_camera_init(config: *const CCameraConfig) -> i32;
    fn road_camera_start() -> i32;
    fn road_camera_stop();
    fn road_camera_shutdown();
    fn road_camera_read_frame(timeout_ms: i32) -> *mut CVideoFrame;
    fn road_camera_release_frame(frame: *mut CVideoFrame);
    fn road_camera_is_streaming() -> i32;
    fn road_camera_last_error() -> *const c_char;
}

/// Camera driver wrapper
pub struct CameraDriver {
    camera_type: CameraType,
    device: CString,
}

impl CameraDriver {
    /// Initialize a camera
    pub fn new(config: &CameraConfig) -> Result<Self, CameraError> {
        let device = CString::new(config.device.as_str())
            .map_err(|e| CameraError::Open(e.to_string()))?;
        
        let c_config = CCameraConfig {
            device: device.as_ptr(),
            camera_type: match config.camera_type {
                CameraType::Cabin => CCameraType::CabinIr,
                CameraType::Road => CCameraType::Road,
            },
            width: config.width,
            height: config.height,
            fps: config.fps,
            format: if config.camera_type == CameraType::Cabin {
                CPixelFormat::Mjpeg
            } else {
                CPixelFormat::H264
            },
            enable_ir: if config.enable_ir { 1 } else { 0 },
            buffer_count: 4,
        };

        let ret = match config.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_init(&c_config) },
            CameraType::Road => unsafe { road_camera_init(&c_config) },
        };

        if ret != 0 {
            return Err(CameraError::Open(format!("Init failed: {}", ret)));
        }

        Ok(Self {
            camera_type: config.camera_type,
            device,
        })
    }

    /// Start streaming
    pub fn start(&self) -> Result<(), CameraError> {
        let ret = match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_start() },
            CameraType::Road => unsafe { road_camera_start() },
        };

        if ret != 0 {
            Err(CameraError::Stream(format!("Start failed: {}", ret)))
        } else {
            Ok(())
        }
    }

    /// Stop streaming
    pub fn stop(&self) {
        match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_stop() },
            CameraType::Road => unsafe { road_camera_stop() },
        }
    }

    /// Check if streaming
    pub fn is_streaming(&self) -> bool {
        match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_is_streaming() == 1 },
            CameraType::Road => unsafe { road_camera_is_streaming() == 1 },
        }
    }

    /// Read next frame (blocking with timeout)
    pub fn read_frame(&self, timeout_ms: i32) -> Option<CapturedFrame> {
        let frame_ptr = match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_read_frame(timeout_ms) },
            CameraType::Road => unsafe { road_camera_read_frame(timeout_ms) },
        };

        if frame_ptr.is_null() {
            return None;
        }

        Some(CapturedFrame {
            ptr: frame_ptr,
            camera_type: self.camera_type,
        })
    }
}

impl Drop for CameraDriver {
    fn drop(&mut self) {
        match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_shutdown() },
            CameraType::Road => unsafe { road_camera_shutdown() },
        }
    }
}

/// Captured frame with RAII cleanup
pub struct CapturedFrame {
    ptr: *mut CVideoFrame,
    camera_type: CameraType,
}

impl CapturedFrame {
    /// Get frame data as slice
    pub fn data(&self) -> &[u8] {
        unsafe {
            let frame = &*self.ptr;
            std::slice::from_raw_parts(frame.data, frame.size)
        }
    }

    /// Get frame width
    pub fn width(&self) -> u32 {
        unsafe { (*self.ptr).width }
    }

    /// Get frame height
    pub fn height(&self) -> u32 {
        unsafe { (*self.ptr).height }
    }

    /// Get timestamp in nanoseconds
    pub fn timestamp_ns(&self) -> u64 {
        unsafe { (*self.ptr).timestamp_ns }
    }

    /// Get frame sequence number
    pub fn sequence(&self) -> u32 {
        unsafe { (*self.ptr).sequence }
    }

    /// Get pixel format
    pub fn format(&self) -> CPixelFormat {
        unsafe { (*self.ptr).format }
    }
}

impl Drop for CapturedFrame {
    fn drop(&mut self) {
        match self.camera_type {
            CameraType::Cabin => unsafe { cabin_camera_release_frame(self.ptr) },
            CameraType::Road => unsafe { road_camera_release_frame(self.ptr) },
        }
    }
}

// Make CapturedFrame Send + Sync for async usage
unsafe impl Send for CapturedFrame {}
unsafe impl Sync for CapturedFrame {}
