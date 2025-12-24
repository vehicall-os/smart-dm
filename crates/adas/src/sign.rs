//! Traffic sign recognition

use serde::{Deserialize, Serialize};
use camera_capture::frame::VideoFrame;
use crate::{AdasConfig, AdasError};

/// Traffic sign types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrafficSign {
    /// Speed limit (km/h)
    SpeedLimit(u32),
    /// Stop sign
    Stop,
    /// Yield sign
    Yield,
    /// No entry
    NoEntry,
    /// No overtaking
    NoOvertaking,
    /// End of restriction
    EndRestriction,
    /// Unknown sign
    Unknown,
}

/// Traffic sign classifier
pub struct SignClassifier {
    enabled: bool,
}

impl SignClassifier {
    pub fn new(config: &AdasConfig) -> Result<Self, AdasError> {
        Ok(Self {
            enabled: config.sign_detection_enabled,
        })
    }

    /// Classify traffic signs in frame
    pub fn classify(&self, _frame: &VideoFrame) -> Result<Vec<TrafficSign>, AdasError> {
        if !self.enabled {
            return Ok(vec![]);
        }

        // Real implementation would:
        // 1. Detect sign regions using YOLO or R-CNN
        // 2. Classify each sign using ResNet
        // 3. Return recognized signs
        
        // Mock: no signs detected
        Ok(vec![])
    }
}
