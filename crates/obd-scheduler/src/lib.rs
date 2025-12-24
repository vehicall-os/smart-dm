//! OBD-II Scheduler for Adaptive PID Sampling
//!
//! Provides priority-based scheduling for OBD-II PID queries with
//! adaptive rate boosting based on sensor thresholds.

mod scheduler;

pub use scheduler::{PidScheduler, SchedulerConfig, ScheduledPid};
