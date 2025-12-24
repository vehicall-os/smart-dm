//! Rule-Based Fallback System
//!
//! Provides rule-based heuristics when ML inference is unavailable.

mod rules;

pub use rules::{FallbackEngine, Alert, Severity, Fault};
