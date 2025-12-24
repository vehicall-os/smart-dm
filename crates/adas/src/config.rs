//! ADAS configuration

use serde::{Deserialize, Serialize};

/// ADAS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdasConfig {
    /// Forward collision warning distance (meters)
    pub fcw_distance_m: f32,
    
    /// Lane departure warning enabled
    pub lane_departure_enabled: bool,
    
    /// Object detection confidence threshold
    pub object_confidence: f32,
    
    /// Lane detection confidence threshold
    pub lane_confidence: f32,
    
    /// Traffic sign detection enabled
    pub sign_detection_enabled: bool,
    
    /// Model paths
    pub lane_model_path: Option<String>,
    pub object_model_path: Option<String>,
    pub sign_model_path: Option<String>,
}

impl Default for AdasConfig {
    fn default() -> Self {
        Self {
            fcw_distance_m: 10.0,
            lane_departure_enabled: true,
            object_confidence: 0.5,
            lane_confidence: 0.7,
            sign_detection_enabled: true,
            lane_model_path: None,
            object_model_path: None,
            sign_model_path: None,
        }
    }
}
