//! Advanced Driver Assistance System (ADAS)
//!
//! Road scene analysis using computer vision:
//! - Lane detection and departure warning
//! - Object detection (vehicles, pedestrians)
//! - Traffic sign recognition
//! - Forward collision warning
//! - Monocular depth estimation

pub mod analysis;
pub mod config;
pub mod lane;
pub mod object;
pub mod sign;

pub use analysis::{AdasAnalysis, AdasAlert};
pub use config::AdasConfig;
pub use lane::{LaneDetector, LaneState, LanePosition};
pub use object::{ObjectDetector, DetectedObject, ObjectClass};
pub use sign::{SignClassifier, TrafficSign};

use camera_capture::frame::VideoFrame;
use thiserror::Error;

/// ADAS error types
#[derive(Error, Debug)]
pub enum AdasError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    
    #[error("Inference failed: {0}")]
    Inference(String),
    
    #[error("Invalid frame format")]
    InvalidFrame,
}

/// ADAS module
pub struct AdasModule {
    config: AdasConfig,
    lane_detector: LaneDetector,
    object_detector: ObjectDetector,
    sign_classifier: SignClassifier,
}

impl AdasModule {
    /// Create new ADAS module
    pub fn new(config: AdasConfig) -> Result<Self, AdasError> {
        Ok(Self {
            lane_detector: LaneDetector::new(&config)?,
            object_detector: ObjectDetector::new(&config)?,
            sign_classifier: SignClassifier::new(&config)?,
            config,
        })
    }

    /// Analyze road scene
    pub async fn analyze(&mut self, frame: &VideoFrame) -> Result<AdasAnalysis, AdasError> {
        // Run detections in parallel
        let lane_state = self.lane_detector.detect(frame)?;
        let objects = self.object_detector.detect(frame)?;
        let signs = self.sign_classifier.classify(frame)?;

        // Generate alerts
        let mut alerts = Vec::new();

        // Lane departure warning
        if lane_state.departing {
            if !lane_state.signal_active {
                alerts.push(AdasAlert::LaneDeparture);
            }
        }

        // Forward collision warning
        for obj in &objects {
            if obj.class == ObjectClass::Vehicle && obj.distance_m < self.config.fcw_distance_m {
                alerts.push(AdasAlert::ForwardCollision {
                    distance_m: obj.distance_m,
                    object_type: obj.class,
                });
                break;
            }
        }

        // Speed limit warning
        for sign in &signs {
            if let TrafficSign::SpeedLimit(limit) = sign {
                // TODO: Compare with actual speed from OBD
                alerts.push(AdasAlert::SpeedLimitDetected { limit_kmh: *limit });
            }
        }

        Ok(AdasAnalysis {
            lane_state,
            objects,
            signs,
            alerts,
        })
    }
}
