//! DMS analysis results and alerts

use serde::{Deserialize, Serialize};
use crate::detector::{FaceBbox, EyeState, HeadPose};
use crate::state::{DrowsinessLevel, DistractionType};

/// DMS alert types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmsAlert {
    /// Driver showing signs of drowsiness
    Drowsiness,
    
    /// Driver distracted (not looking at road)
    Distraction,
    
    /// Driver head tilted down
    HeadDown,
    
    /// Face not visible (camera blocked?)
    FaceNotVisible,
    
    /// Driver yawning frequently
    FrequentYawning,
    
    /// Eye closure ratio too high (PERCLOS)
    HighPerclos,
}

/// Complete DMS analysis result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DmsAnalysis {
    /// Whether a face was detected
    pub face_detected: bool,
    
    /// Face bounding box (if detected)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub face_bbox: Option<FaceBbox>,
    
    /// Eye state (open/closed, gaze direction)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eye_state: Option<EyeState>,
    
    /// Head pose (yaw, pitch, roll)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_pose: Option<HeadPose>,
    
    /// Current drowsiness level
    pub drowsiness_level: DrowsinessLevel,
    
    /// Current distraction type (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distraction_type: Option<DistractionType>,
    
    /// Active alerts
    pub alerts: Vec<DmsAlert>,
}

impl DmsAnalysis {
    /// Check if any alerts are active
    pub fn has_alerts(&self) -> bool {
        !self.alerts.is_empty()
    }
    
    /// Get highest severity alert
    pub fn highest_severity_alert(&self) -> Option<DmsAlert> {
        // Priority: Drowsiness > Distraction > HeadDown > Others
        if self.alerts.contains(&DmsAlert::Drowsiness) {
            Some(DmsAlert::Drowsiness)
        } else if self.alerts.contains(&DmsAlert::Distraction) {
            Some(DmsAlert::Distraction)
        } else if self.alerts.contains(&DmsAlert::HeadDown) {
            Some(DmsAlert::HeadDown)
        } else {
            self.alerts.first().copied()
        }
    }
}
