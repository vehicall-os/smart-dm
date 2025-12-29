//! Driver Monitoring System (DMS)
//!
//! Real-time driver state analysis using computer vision:
//! - Face detection and tracking
//! - Eye openness detection (drowsiness)
//! - Head pose estimation
//! - Gaze direction tracking
//! - Distraction detection

pub mod analysis;
pub mod config;
pub mod detector;
pub mod state;

pub use analysis::{DmsAnalysis, DmsAlert};
pub use config::DmsConfig;
pub use detector::{FaceDetector, EyeDetector, PoseEstimator};
pub use state::{DriverState, DrowsinessLevel, DistractionType};

use camera_capture::frame::VideoFrame;
use thiserror::Error;

/// DMS error types
#[derive(Error, Debug)]
pub enum DmsError {
    #[error("Model loading failed: {0}")]
    ModelLoad(String),
    
    #[error("Inference failed: {0}")]
    Inference(String),
    
    #[error("No face detected")]
    NoFace,
    
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Image processing failed: {0}")]
    ImageProcessing(String),

    #[error("Keypoints missing for feature calculation")]
    KeypointsMissing,
}

/// Driver monitoring module
pub struct DmsModule {
    config: DmsConfig,
    face_detector: FaceDetector,
    eye_detector: EyeDetector,
    pose_estimator: PoseEstimator,
    state: DriverState,
}

impl DmsModule {
    /// Create a new DMS module with configuration
    pub fn new(config: DmsConfig) -> Result<Self, DmsError> {
        Ok(Self {
            face_detector: FaceDetector::new(&config)?,
            eye_detector: EyeDetector::new(&config)?,
            pose_estimator: PoseEstimator::new(&config)?,
            state: DriverState::default(),
            config,
        })
    }

    /// Analyze a single frame for driver state
    pub async fn analyze(&mut self, frame: &VideoFrame) -> Result<DmsAnalysis, DmsError> {
        // Detect face
        let faces = self.face_detector.detect(frame)?;
        
        if faces.is_empty() {
            self.state.face_absent_frames += 1;
            return Ok(DmsAnalysis {
                face_detected: false,
                alerts: if self.state.face_absent_frames > 30 {
                    vec![DmsAlert::FaceNotVisible]
                } else {
                    vec![]
                },
                ..Default::default()
            });
        }

        self.state.face_absent_frames = 0;
        let face = &faces[0];

        // Detect eye state
        let eyes = self.eye_detector.detect(frame, face)?;
        
        // Estimate head pose
        let pose = self.pose_estimator.estimate(frame, face)?;

        // Update state and detect alerts
        let alerts = self.update_state(&eyes, &pose);

        Ok(DmsAnalysis {
            face_detected: true,
            face_bbox: Some(face.clone()),
            eye_state: Some(eyes),
            head_pose: Some(pose),
            drowsiness_level: self.state.drowsiness_level,
            distraction_type: self.state.distraction,
            alerts,
        })
    }

    fn update_state(
        &mut self,
        eyes: &detector::EyeState,
        pose: &detector::HeadPose,
    ) -> Vec<DmsAlert> {
        let mut alerts = Vec::new();

        // Drowsiness detection (eyes closed >1.5s)
        if eyes.left_closed && eyes.right_closed {
            self.state.eyes_closed_ms += 33; // Assume ~30fps
            if self.state.eyes_closed_ms > self.config.drowsiness_threshold_ms {
                self.state.drowsiness_level = DrowsinessLevel::High;
                alerts.push(DmsAlert::Drowsiness);
            }
        } else {
            self.state.eyes_closed_ms = 0;
            self.state.drowsiness_level = DrowsinessLevel::Normal;
        }

        // Distraction detection (gaze away >3s)
        let looking_forward = pose.yaw.abs() < self.config.gaze_threshold_degrees
            && pose.pitch.abs() < self.config.gaze_threshold_degrees;

        if !looking_forward {
            self.state.distraction_ms += 33;
            if self.state.distraction_ms > self.config.distraction_threshold_ms {
                self.state.distraction = Some(DistractionType::LookingAway);
                alerts.push(DmsAlert::Distraction);
            }
        } else {
            self.state.distraction_ms = 0;
            self.state.distraction = None;
        }

        // Head pose alerts
        if pose.pitch.abs() > 45.0 {
            alerts.push(DmsAlert::HeadDown);
        }

        alerts
    }

    /// Reset driver state (on driver change)
    pub fn reset_state(&mut self) {
        self.state = DriverState::default();
    }
}
