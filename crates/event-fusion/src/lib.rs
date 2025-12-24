//! Event Fusion Engine
//!
//! Correlates data from multiple sources:
//! - OBD (engine, speed, braking)
//! - DMS (driver state, drowsiness)
//! - ADAS (lane, objects, signs)
//! - IMU (G-forces, crash detection)
//!
//! Generates unified events for storage and alerting.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use dms::DmsAnalysis;
use adas::AdasAnalysis;
use camera_capture::imu::ImuData;

/// Fusion error types
#[derive(Error, Debug)]
pub enum FusionError {
    #[error("Missing data source: {0}")]
    MissingSource(String),
    
    #[error("Timestamp mismatch")]
    TimestampMismatch,
}

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

/// Fused event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FusedEvent {
    /// Normal driving (periodic)
    Normal,
    
    /// Hard braking detected
    HardBraking {
        severity: Severity,
        decel_g: f32,
        speed_before_kmh: f32,
    },
    
    /// Forward collision warning triggered with braking
    EmergencyBraking {
        severity: Severity,
        object_distance_m: f32,
        reaction_time_ms: u64,
    },
    
    /// Drowsiness while lane departing
    DrowsinessLaneDeparture {
        severity: Severity,
        eyes_closed_ms: u64,
    },
    
    /// Crash detected (high G-force + airbag)
    Crash {
        severity: Severity,
        g_force: f32,
        airbag_deployed: bool,
    },
    
    /// Distraction sustained
    SustainedDistraction {
        severity: Severity,
        duration_ms: u64,
    },
    
    /// Speeding detected
    Speeding {
        current_kmh: u32,
        limit_kmh: u32,
    },
}

/// OBD frame for fusion
#[derive(Debug, Clone)]
pub struct ObdFrame {
    pub timestamp_ns: u64,
    pub rpm: u16,
    pub speed_kmh: u8,
    pub brake_pedal: u8,
    pub throttle: u8,
}

/// Sliding window for any data type
struct SlidingWindow<T> {
    data: VecDeque<T>,
    capacity: usize,
}

impl<T> SlidingWindow<T> {
    fn new(capacity: usize) -> Self {
        Self {
            data: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    fn push(&mut self, item: T) {
        if self.data.len() >= self.capacity {
            self.data.pop_front();
        }
        self.data.push_back(item);
    }

    fn back(&self) -> Option<&T> {
        self.data.back()
    }

    #[allow(dead_code)]
    fn iter(&self) -> impl Iterator<Item = &T> {
        self.data.iter()
    }
}

/// Event fusion engine
pub struct EventFusion {
    /// OBD data window (60s @ 5Hz)
    obd_window: SlidingWindow<ObdFrame>,
    
    /// DMS analysis window (10s @ 15fps)
    dms_window: SlidingWindow<DmsAnalysis>,
    
    /// ADAS analysis window (10s @ 6fps)
    adas_window: SlidingWindow<AdasAnalysis>,
    
    /// IMU data window (10s @ 100Hz)
    imu_window: SlidingWindow<ImuData>,
    
    /// Configuration
    config: FusionConfig,
    
    /// Current driver ID
    driver_id: Option<String>,
}

/// Fusion configuration
#[derive(Debug, Clone)]
pub struct FusionConfig {
    /// G-force threshold for hard braking
    pub hard_brake_g: f32,
    
    /// G-force threshold for crash
    pub crash_g: f32,
    
    /// Speeding threshold (km/h over limit)
    pub speeding_threshold_kmh: u32,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            hard_brake_g: 0.4,
            crash_g: 3.0,
            speeding_threshold_kmh: 10,
        }
    }
}

impl EventFusion {
    /// Create new fusion engine
    pub fn new(config: FusionConfig) -> Self {
        Self {
            obd_window: SlidingWindow::new(300),   // 60s @ 5Hz
            dms_window: SlidingWindow::new(150),   // 10s @ 15fps
            adas_window: SlidingWindow::new(60),   // 10s @ 6fps
            imu_window: SlidingWindow::new(1000),  // 10s @ 100Hz
            config,
            driver_id: None,
        }
    }

    /// Add OBD frame
    pub fn add_obd(&mut self, frame: ObdFrame) {
        self.obd_window.push(frame);
    }

    /// Add DMS analysis
    pub fn add_dms(&mut self, analysis: DmsAnalysis) {
        self.dms_window.push(analysis);
    }

    /// Add ADAS analysis
    pub fn add_adas(&mut self, analysis: AdasAnalysis) {
        self.adas_window.push(analysis);
    }

    /// Add IMU data
    pub fn add_imu(&mut self, data: ImuData) {
        self.imu_window.push(data);
    }

    /// Set current driver
    pub fn set_driver(&mut self, driver_id: Option<String>) {
        self.driver_id = driver_id;
    }

    /// Fuse events and return any detected incidents
    pub fn fuse(&self) -> Option<FusedEvent> {
        // Check for crash (highest priority)
        if let Some(imu) = self.imu_window.back() {
            if imu.g_force > self.config.crash_g {
                return Some(FusedEvent::Crash {
                    severity: Severity::Critical,
                    g_force: imu.g_force,
                    airbag_deployed: false,
                });
            }
        }

        // Check for hard braking
        if let Some(imu) = self.imu_window.back() {
            if imu.accel_x.abs() > self.config.hard_brake_g {
                if let Some(obd) = self.obd_window.back() {
                    if obd.brake_pedal > 80 {
                        return Some(FusedEvent::HardBraking {
                            severity: Severity::Medium,
                            decel_g: imu.accel_x.abs(),
                            speed_before_kmh: obd.speed_kmh as f32,
                        });
                    }
                }
            }
        }

        // Check for drowsiness + lane departure
        if let (Some(dms), Some(adas)) = (self.dms_window.back(), self.adas_window.back()) {
            if dms.drowsiness_level as u8 >= 2 && adas.lane_state.departing {
                return Some(FusedEvent::DrowsinessLaneDeparture {
                    severity: Severity::High,
                    eyes_closed_ms: 0,
                });
            }
        }

        None
    }
}

