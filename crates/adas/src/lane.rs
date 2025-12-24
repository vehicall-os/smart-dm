//! Lane detection

use serde::{Deserialize, Serialize};
use camera_capture::frame::VideoFrame;
use crate::{AdasConfig, AdasError};

/// Lane position relative to vehicle
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanePosition {
    #[default]
    Center,
    Left,
    Right,
    Unknown,
}

/// Lane detection state
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaneState {
    /// Lane lines detected
    pub lanes_detected: bool,
    
    /// Vehicle position in lane
    pub position: LanePosition,
    
    /// Departing from lane
    pub departing: bool,
    
    /// Turn signal active
    pub signal_active: bool,
    
    /// Left lane line points [(x, y), ...]
    pub left_lane: Vec<(f32, f32)>,
    
    /// Right lane line points
    pub right_lane: Vec<(f32, f32)>,
    
    /// Lane curvature (1/radius)
    pub curvature: f32,
    
    /// Offset from lane center (meters)
    pub center_offset_m: f32,
}

/// Lane detector
pub struct LaneDetector {
    confidence_threshold: f32,
}

impl LaneDetector {
    pub fn new(config: &AdasConfig) -> Result<Self, AdasError> {
        Ok(Self {
            confidence_threshold: config.lane_confidence,
        })
    }

    /// Detect lane lines
    pub fn detect(&self, _frame: &VideoFrame) -> Result<LaneState, AdasError> {
        // Real implementation would:
        // 1. Preprocess image (resize, normalize)
        // 2. Run Ultra-Fast-Lane-Detection model
        // 3. Post-process outputs to lane polynomials
        // 4. Calculate vehicle position and departure
        
        // Mock: lanes detected, centered
        Ok(LaneState {
            lanes_detected: true,
            position: LanePosition::Center,
            departing: false,
            signal_active: false,
            left_lane: vec![(100.0, 1080.0), (400.0, 540.0)],
            right_lane: vec![(1820.0, 1080.0), (1520.0, 540.0)],
            curvature: 0.0,
            center_offset_m: 0.0,
        })
    }
}
