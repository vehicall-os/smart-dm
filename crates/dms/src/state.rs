//! Driver state tracking

use serde::{Deserialize, Serialize};

/// Drowsiness level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DrowsinessLevel {
    #[default]
    Normal,
    Mild,
    Moderate,
    High,
}

/// Distraction type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DistractionType {
    LookingAway,
    PhoneUse,
    Eating,
    Smoking,
    Unknown,
}

/// Driver state (tracked over time)
#[derive(Debug, Clone, Default)]
pub struct DriverState {
    /// Frames where face was not detected
    pub face_absent_frames: u32,
    
    /// Continuous time eyes are closed (ms)
    pub eyes_closed_ms: u64,
    
    /// Continuous time driver is distracted (ms)
    pub distraction_ms: u64,
    
    /// Current drowsiness level
    pub drowsiness_level: DrowsinessLevel,
    
    /// Current distraction type
    pub distraction: Option<DistractionType>,
    
    /// Yawning count in last 10 minutes
    pub yawn_count: u32,
    
    /// Eye openness ratio history (for PERCLOS)
    pub eye_openness_history: Vec<f32>,
}

impl DriverState {
    /// Calculate PERCLOS (Percentage of Eye Closure)
    /// Higher PERCLOS indicates drowsiness
    pub fn perclos(&self) -> f32 {
        if self.eye_openness_history.is_empty() {
            return 0.0;
        }
        
        let closed_count = self.eye_openness_history
            .iter()
            .filter(|&&v| v < 0.2)  // Less than 20% open = closed
            .count();
        
        closed_count as f32 / self.eye_openness_history.len() as f32
    }
    
    /// Add eye openness sample for PERCLOS calculation
    pub fn add_eye_sample(&mut self, openness: f32) {
        self.eye_openness_history.push(openness);
        
        // Keep last 900 samples (~1 minute at 15fps)
        if self.eye_openness_history.len() > 900 {
            self.eye_openness_history.remove(0);
        }
    }
    
    /// Reset state (on driver change)
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
