//! DMS configuration

use serde::{Deserialize, Serialize};

/// DMS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DmsConfig {
    /// Eyes closed threshold for drowsiness alert (milliseconds)
    pub drowsiness_threshold_ms: u64,
    
    /// Gaze away threshold for distraction alert (milliseconds)
    pub distraction_threshold_ms: u64,
    
    /// Gaze deviation threshold (degrees from center)
    pub gaze_threshold_degrees: f32,
    
    /// Face detection confidence threshold
    pub face_confidence: f32,
    
    /// Eye detection confidence threshold
    pub eye_confidence: f32,
    
    /// Enable head pose estimation
    pub enable_pose: bool,
    
    /// Model paths
    pub face_model_path: Option<String>,
    pub eye_model_path: Option<String>,
    pub pose_model_path: Option<String>,
}

impl Default for DmsConfig {
    fn default() -> Self {
        Self {
            drowsiness_threshold_ms: 1500,
            distraction_threshold_ms: 3000,
            gaze_threshold_degrees: 30.0,
            face_confidence: 0.7,
            eye_confidence: 0.6,
            enable_pose: true,
            face_model_path: None,
            eye_model_path: None,
            pose_model_path: None,
        }
    }
}

impl DmsConfig {
    /// Create strict config (lower thresholds)
    pub fn strict() -> Self {
        Self {
            drowsiness_threshold_ms: 1000,
            distraction_threshold_ms: 2000,
            gaze_threshold_degrees: 20.0,
            ..Default::default()
        }
    }

    /// Create lenient config (higher thresholds)
    pub fn lenient() -> Self {
        Self {
            drowsiness_threshold_ms: 2500,
            distraction_threshold_ms: 5000,
            gaze_threshold_degrees: 45.0,
            ..Default::default()
        }
    }
}
