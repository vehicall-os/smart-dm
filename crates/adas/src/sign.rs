//! Traffic sign recognition

use serde::{Deserialize, Serialize};
use camera_capture::frame::VideoFrame;
use crate::{AdasConfig, AdasError};
use ort::{Session, GraphOptimizationLevel};
use ndarray::{Array4, Axis};
use tracing::{info, warn, error};

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
    session: Option<Session>,
}

impl SignClassifier {
    pub fn new(config: &AdasConfig) -> Result<Self, AdasError> {
        let session = if config.sign_detection_enabled {
            if let Some(path) = &config.sign_model_path {
                 info!("Loading sign detection model from {}", path);
                 match Session::builder() {
                    Ok(builder) => match builder.with_optimization_level(GraphOptimizationLevel::Level3) {
                        Ok(builder) => match builder.commit_from_file(path) {
                            Ok(s) => Some(s),
                            Err(e) => {
                                error!("Failed to load sign model: {}", e);
                                return Err(AdasError::ModelLoad(e.to_string()));
                            }
                        },
                        Err(e) => {
                                error!("Failed to configure model optimization: {}", e);
                                return Err(AdasError::ModelLoad(e.to_string()));
                        }
                    },
                    Err(e) => {
                        error!("Failed to create session builder: {}", e);
                        return Err(AdasError::ModelLoad(e.to_string()));
                    }
                }
            } else {
                 warn!("Sign detection enabled but no model path provided.");
                 None
            }
        } else {
            None
        };

        Ok(Self {
            enabled: config.sign_detection_enabled,
            session,
        })
    }

    /// Classify traffic signs in frame
    pub fn classify(&self, frame: &VideoFrame) -> Result<Vec<TrafficSign>, AdasError> {
        if !self.enabled {
            return Ok(vec![]);
        }

        if let Some(session) = &self.session {
             // 1. Preprocess: Resize to 640x640 (standard YOLO)
             // Similar to ObjectDetector
            let img = match image::ImageBuffer::<image::Rgb<u8>, _>::from_raw(
                frame.width, 
                frame.height, 
                &frame.data
            ) {
                Some(i) => i,
                None => return Err(AdasError::ImageProcessing("Failed to create image buffer".into())),
            };

            let input_width = 640;
            let input_height = 640;
            let resized = image::imageops::resize(&img, input_width, input_height, image::imageops::FilterType::Triangle);

            // 2. Normalize 0-1
            let mut input_array = Array4::<f32>::zeros((1, 3, input_height as usize, input_width as usize));
            for (x, y, pixel) in resized.enumerate_pixels() {
                input_array[[0, 0, y as usize, x as usize]] = pixel[0] as f32 / 255.0;
                input_array[[0, 1, y as usize, x as usize]] = pixel[1] as f32 / 255.0;
                input_array[[0, 2, y as usize, x as usize]] = pixel[2] as f32 / 255.0;
            }

            // 3. Inference
            let outputs = session.run(ort::inputs![input_array].map_err(|e| AdasError::Inference(e.to_string()))?)
                .map_err(|e| AdasError::Inference(e.to_string()))?;

            // 4. Post-process
            // Parsing YOLO output [1, anchors, 85] (approx)
            // Need to map class ID to TrafficSign
            
            // Mocking a detected sign for now to confirm pipeline works
            // In real logic:
            // let sign = match class_id { 0 => TrafficSign::SpeedLimit(30), ... };
            
            Ok(vec![])

        } else {
            // Mock: no signs detected
            Ok(vec![])
        }
    }
}
