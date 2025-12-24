//! ADAS analysis results and alerts

use serde::{Deserialize, Serialize};
use crate::lane::LaneState;
use crate::object::{DetectedObject, ObjectClass};
use crate::sign::TrafficSign;

/// ADAS alert types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AdasAlert {
    /// Lane departure without signal
    LaneDeparture,
    
    /// Forward collision imminent
    ForwardCollision {
        distance_m: f32,
        object_type: ObjectClass,
    },
    
    /// Speed limit detected
    SpeedLimitDetected { limit_kmh: u32 },
    
    /// Pedestrian in path
    PedestrianWarning { distance_m: f32 },
    
    /// Stop sign detected
    StopSignDetected,
    
    /// Tailgating warning (too close)
    Tailgating { distance_m: f32 },
}

/// Complete ADAS analysis result
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AdasAnalysis {
    /// Lane detection state
    pub lane_state: LaneState,
    
    /// Detected objects
    pub objects: Vec<DetectedObject>,
    
    /// Detected traffic signs
    pub signs: Vec<TrafficSign>,
    
    /// Active alerts
    pub alerts: Vec<AdasAlert>,
}

impl AdasAnalysis {
    /// Check if any critical alerts
    pub fn has_critical_alerts(&self) -> bool {
        self.alerts.iter().any(|a| matches!(a, 
            AdasAlert::ForwardCollision { .. } | 
            AdasAlert::PedestrianWarning { .. }
        ))
    }
}
