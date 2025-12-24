//! Object detection (vehicles, pedestrians, cyclists)

use serde::{Deserialize, Serialize};
use camera_capture::frame::VideoFrame;
use crate::{AdasConfig, AdasError};

/// Object class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectClass {
    Vehicle,
    Pedestrian,
    Cyclist,
    Motorcycle,
    Truck,
    Unknown,
}

/// Detected object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedObject {
    /// Object class
    pub class: ObjectClass,
    
    /// Bounding box [x, y, width, height]
    pub bbox: [f32; 4],
    
    /// Detection confidence
    pub confidence: f32,
    
    /// Estimated distance (meters)
    pub distance_m: f32,
    
    /// Estimated relative velocity (m/s)
    pub velocity_mps: f32,
    
    /// Time to collision (seconds)
    pub ttc_s: Option<f32>,
}

/// Object detector using YOLO or similar
pub struct ObjectDetector {
    confidence_threshold: f32,
}

impl ObjectDetector {
    pub fn new(config: &AdasConfig) -> Result<Self, AdasError> {
        Ok(Self {
            confidence_threshold: config.object_confidence,
        })
    }

    /// Detect objects in frame
    pub fn detect(&self, _frame: &VideoFrame) -> Result<Vec<DetectedObject>, AdasError> {
        // Real implementation would:
        // 1. Preprocess image for YOLO
        // 2. Run YOLOv5s inference
        // 3. NMS and filtering
        // 4. Estimate distance using monocular depth
        
        // Mock: one vehicle ahead
        Ok(vec![DetectedObject {
            class: ObjectClass::Vehicle,
            bbox: [800.0, 400.0, 300.0, 200.0],
            confidence: 0.92,
            distance_m: 25.0,
            velocity_mps: -2.0, // Approaching
            ttc_s: Some(12.5),
        }])
    }
}
