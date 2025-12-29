//! Face, eye, and pose detection models

use camera_capture::frame::VideoFrame;
use serde::{Deserialize, Serialize};
use crate::{DmsConfig, DmsError};
use ort::{Session, GraphOptimizationLevel};
use ndarray::{Array4, Axis};
use tracing::{info, warn, error};

/// Face bounding box
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceBbox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub confidence: f32,
    /// 6 landmarks: Left Eye, Right Eye, Left Ear, Right Ear, Nose, Mouth
    pub keypoints: Option<Vec<(f32, f32)>>,
}

/// Eye state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EyeState {
    /// Left eye closed
    pub left_closed: bool,
    /// Right eye closed
    pub right_closed: bool,
    /// Left eye openness ratio (0-1)
    pub left_openness: f32,
    /// Right eye openness ratio (0-1)
    pub right_openness: f32,
    /// Gaze direction (yaw, pitch) in degrees
    pub gaze_yaw: f32,
    pub gaze_pitch: f32,
}

impl Default for EyeState {
    fn default() -> Self {
        Self {
            left_closed: false,
            right_closed: false,
            left_openness: 1.0,
            right_openness: 1.0,
            gaze_yaw: 0.0,
            gaze_pitch: 0.0,
        }
    }
}

/// Head pose (Euler angles)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HeadPose {
    /// Yaw (left-right rotation) in degrees
    pub yaw: f32,
    /// Pitch (up-down tilt) in degrees
    pub pitch: f32,
    /// Roll (side tilt) in degrees
    pub roll: f32,
}

/// Face detector using BlazeFace or similar
pub struct FaceDetector {
    confidence_threshold: f32,
    session: Option<Session>,
}

impl FaceDetector {
    pub fn new(config: &DmsConfig) -> Result<Self, DmsError> {
        let session = if let Some(path) = &config.face_model_path {
            info!("Loading face detection model from {}", path);
             match Session::builder() {
                Ok(builder) => {
                    match builder.with_optimization_level(GraphOptimizationLevel::Level3) {
                        Ok(builder) => match builder.commit_from_file(path) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                error!("Failed to load face model: {}", e);
                                return Err(DmsError::ModelLoad(e.to_string()));
                            }
                        },
                        Err(e) => {
                             error!("Failed to configure model optimization: {}", e);
                             return Err(DmsError::ModelLoad(e.to_string()));
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to create session builder: {}", e);
                    return Err(DmsError::ModelLoad(e.to_string()));
                }
            }
        } else {
            warn!("No face model path configured. Using mock implementation.");
            None
        };

        Ok(Self {
            confidence_threshold: config.face_confidence,
            session,
        })
    }

    /// Detect faces in frame
    pub fn detect(&self, frame: &VideoFrame) -> Result<Vec<FaceBbox>, DmsError> {
         if let Some(session) = &self.session {
             // 1. Preprocess: Resize to 128x128
            let img = match image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(
                frame.width, 
                frame.height, 
                &frame.data
            ) {
                Some(i) => i,
                None => return Err(DmsError::ImageProcessing("Failed to create image buffer".into())),
            };

            let resized = image::imageops::resize(&img, 128, 128, image::imageops::FilterType::Triangle);

            // 2. Normalize and create tensor (1x3x128x128)
            // BlazeFace usually expects -1..1 or 0..1 normalization
            let mut input_array = Array4::<f32>::zeros((1, 3, 128, 128));
            for (x, y, pixel) in resized.enumerate_pixels() {
                input_array[[0, 0, y as usize, x as usize]] = (pixel[0] as f32 / 127.5) - 1.0;
                input_array[[0, 1, y as usize, x as usize]] = (pixel[1] as f32 / 127.5) - 1.0;
                input_array[[0, 2, y as usize, x as usize]] = (pixel[2] as f32 / 127.5) - 1.0;
            }

            // 3. Inference
            let outputs = session.run(ort::inputs![input_array].map_err(|e| DmsError::Inference(e.to_string()))?)
                .map_err(|e| DmsError::Inference(e.to_string()))?;

            // 4. Post-process
            // Parsing BlazeFace anchors (896x16)
            // Mocking the result for now until anchor decoding logic is fully ported
             Ok(vec![FaceBbox {
                x: frame.width as f32 * 0.3,
                y: frame.height as f32 * 0.2,
                width: frame.width as f32 * 0.4,
                height: frame.height as f32 * 0.5,
                confidence: 0.95,
                keypoints: Some(vec![
                    (frame.width as f32 * 0.35, frame.height as f32 * 0.3), // L Eye
                    (frame.width as f32 * 0.65, frame.height as f32 * 0.3), // R Eye
                    // ... other keypoints
                ]),
            }])
         } else {
             // Mock
             let mock_face = FaceBbox {
                x: frame.width as f32 * 0.3,
                y: frame.height as f32 * 0.2,
                width: frame.width as f32 * 0.4,
                height: frame.height as f32 * 0.5,
                confidence: 0.95,
                keypoints: Some(vec![
                    (frame.width as f32 * 0.35, frame.height as f32 * 0.3), // L Eye
                    (frame.width as f32 * 0.65, frame.height as f32 * 0.3), // R Eye
                ]),
            };
            Ok(vec![mock_face])
         }
    }
}

/// Eye openness detector
pub struct EyeDetector {
    confidence_threshold: f32,
    session: Option<Session>,
}

impl EyeDetector {
    pub fn new(config: &DmsConfig) -> Result<Self, DmsError> {
         let session = if let Some(path) = &config.eye_model_path {
            info!("Loading eye model from {}", path);
             match Session::builder() {
                Ok(builder) => {
                    match builder.with_optimization_level(GraphOptimizationLevel::Level3) {
                        Ok(builder) => match builder.commit_from_file(path) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                error!("Failed to load eye model: {}", e);
                                return Err(DmsError::ModelLoad(e.to_string()));
                            }
                        },
                         Err(e) => {
                             error!("Failed to configure model optimization: {}", e);
                             return Err(DmsError::ModelLoad(e.to_string()));
                        }
                    }
                },
                Err(e) => {
                    error!("Failed to create session builder: {}", e);
                    return Err(DmsError::ModelLoad(e.to_string()));
                }
            }
        } else {
            None
        };

        Ok(Self {
            confidence_threshold: config.eye_confidence,
            session,
        })
    }

    /// Detect eye state within face region
    pub fn detect(&self, frame: &VideoFrame, face: &FaceBbox) -> Result<EyeState, DmsError> {
        if let Some(session) = &self.session {
            // Real implementation: 
            // 1. Crop eyes from face based on keypoints or bbox heuristic
            // 2. Feed to classification model (Open/Closed)
            
            // TODO: Implement crop logic
            Ok(EyeState::default())
        } else {
             // Fallback to heuristic
             Ok(EyeState {
                left_closed: false,
                right_closed: false,
                left_openness: 0.8,
                right_openness: 0.8,
                gaze_yaw: 0.0,
                gaze_pitch: 0.0,
            })
        }
    }
}

/// Head pose estimator using facial landmarks
pub struct PoseEstimator {
    enabled: bool,
    session: Option<Session>,
}

impl PoseEstimator {
    pub fn new(config: &DmsConfig) -> Result<Self, DmsError> {
         let session = if let Some(path) = &config.pose_model_path {
             match Session::builder() {
                Ok(builder) => match builder.commit_from_file(path) {
                    Ok(s) => Some(s),
                    Err(_) => None, // Optional model
                },
                Err(_) => None,
             }
        } else {
            None
        };

        Ok(Self {
            enabled: config.enable_pose,
            session,
        })
    }

    /// Estimate head pose from face
    pub fn estimate(&self, _frame: &VideoFrame, _face: &FaceBbox) -> Result<HeadPose, DmsError> {
        if !self.enabled {
            return Ok(HeadPose::default());
        }
        
        if let Some(session) = &self.session {
             // Real PnP or model inference
             Ok(HeadPose::default())
        } else {
            // Mock
            Ok(HeadPose {
                yaw: 0.0,
                pitch: 0.0,
                roll: 0.0,
            })
        }
    }
}
