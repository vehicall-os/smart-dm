//! PID Scheduler Implementation

use obd_protocol::{ObdClient, ObdError, Pid, SensorFrame};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Configuration for the PID scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Base sampling rate in Hz (default: 5.0)
    pub base_rate_hz: f64,
    /// Maximum retry attempts before marking adapter unhealthy
    pub max_retries: u8,
    /// Retry backoff base in milliseconds
    pub retry_backoff_ms: u64,
    /// Coolant temperature threshold for rate boost (°C)
    pub coolant_boost_threshold: f64,
    /// Boosted rate multiplier
    pub boost_multiplier: f64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            base_rate_hz: 5.0,
            max_retries: 3,
            retry_backoff_ms: 100,
            coolant_boost_threshold: 95.0,
            boost_multiplier: 2.0,
        }
    }
}

/// A scheduled PID with priority and timing info
#[derive(Debug, Clone)]
pub struct ScheduledPid {
    /// The PID to query
    pub pid: Pid,
    /// Current sampling rate in Hz
    pub rate_hz: f64,
    /// Next scheduled query time
    pub next_query: Instant,
    /// Priority (higher = more important)
    pub priority: u8,
    /// Consecutive failure count
    pub failures: u8,
}

impl ScheduledPid {
    /// Create a new scheduled PID
    pub fn new(pid: Pid, rate_hz: f64) -> Self {
        Self {
            pid,
            rate_hz,
            next_query: Instant::now(),
            priority: pid.sampling_priority(),
            failures: 0,
        }
    }

    /// Calculate interval between queries
    pub fn interval(&self) -> Duration {
        Duration::from_secs_f64(1.0 / self.rate_hz)
    }

    /// Schedule next query
    pub fn schedule_next(&mut self) {
        self.next_query = Instant::now() + self.interval();
    }
}

impl Eq for ScheduledPid {}

impl PartialEq for ScheduledPid {
    fn eq(&self, other: &Self) -> bool {
        self.next_query == other.next_query && self.priority == other.priority
    }
}

impl Ord for ScheduledPid {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap behavior (earliest time first)
        // Then by priority (higher priority first)
        other.next_query.cmp(&self.next_query)
            .then_with(|| self.priority.cmp(&other.priority))
    }
}

impl PartialOrd for ScheduledPid {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// PID Scheduler for managing OBD-II queries
pub struct PidScheduler {
    /// Scheduled PIDs in priority queue
    queue: BinaryHeap<ScheduledPid>,
    /// Configuration
    config: SchedulerConfig,
    /// Whether scheduler is running
    running: bool,
    /// Last known coolant temperature
    last_coolant_temp: f64,
}

impl PidScheduler {
    /// Create a new PID scheduler with default PIDs
    pub fn new(config: SchedulerConfig) -> Self {
        let mut queue = BinaryHeap::new();
        
        // Add critical PIDs at high rate (5Hz)
        queue.push(ScheduledPid::new(Pid::Rpm, config.base_rate_hz));
        queue.push(ScheduledPid::new(Pid::Speed, config.base_rate_hz));
        queue.push(ScheduledPid::new(Pid::CoolantTemp, config.base_rate_hz));
        queue.push(ScheduledPid::new(Pid::EngineLoad, config.base_rate_hz));
        
        // Add diagnostic PIDs at lower rate (1Hz)
        queue.push(ScheduledPid::new(Pid::Maf, 1.0));
        
        // Add slow PIDs (0.5Hz)
        queue.push(ScheduledPid::new(Pid::ShortFuelTrim, 0.5));
        queue.push(ScheduledPid::new(Pid::LongFuelTrim, 0.5));
        queue.push(ScheduledPid::new(Pid::O2Voltage, 0.5));

        info!("PID scheduler created with {} PIDs", queue.len());

        Self {
            queue,
            config,
            running: false,
            last_coolant_temp: 0.0,
        }
    }

    /// Boost priority for a specific PID
    pub fn boost_priority(&mut self, pid: Pid, new_rate_hz: f64) {
        let items: Vec<_> = self.queue.drain().collect();
        for mut item in items {
            if item.pid == pid {
                debug!("Boosting {} rate to {} Hz", pid as u8, new_rate_hz);
                item.rate_hz = new_rate_hz;
            }
            self.queue.push(item);
        }
    }

    /// Run the scheduler loop
    pub async fn run(
        &mut self,
        client: &mut ObdClient,
        frame_tx: mpsc::Sender<SensorFrame>,
    ) -> Result<(), ObdError> {
        info!("Starting PID scheduler");
        self.running = true;

        let mut current_frame = SensorFrame::new(0);

        while self.running {
            // Get next PID to query
            if let Some(mut scheduled) = self.queue.pop() {
                // Wait until it's time
                let now = Instant::now();
                if scheduled.next_query > now {
                    tokio::time::sleep(scheduled.next_query - now).await;
                }

                // Query the PID
                match client.query_pid(scheduled.pid.as_hex()).await {
                    Ok(response) => {
                        scheduled.failures = 0;
                        current_frame.update_from_response(&response);
                        current_frame.timestamp_ms = response.timestamp_ms;

                        // Check for adaptive rate boosting
                        if scheduled.pid == Pid::CoolantTemp {
                            self.last_coolant_temp = response.value;
                            if response.value > self.config.coolant_boost_threshold {
                                warn!("Coolant temp {} > threshold, boosting rate", response.value);
                                scheduled.rate_hz = self.config.base_rate_hz * self.config.boost_multiplier;
                            }
                        }

                        // Send frame (non-blocking)
                        let _ = frame_tx.try_send(current_frame.clone());
                    }
                    Err(e) => {
                        scheduled.failures += 1;
                        warn!("PID {:02X} query failed (attempt {}): {}", 
                            scheduled.pid.as_hex(), scheduled.failures, e);

                        if scheduled.failures >= self.config.max_retries {
                            warn!("Max retries reached for PID {:02X}", scheduled.pid.as_hex());
                            // Still reschedule but with longer delay
                        }
                    }
                }

                // Reschedule
                scheduled.schedule_next();
                self.queue.push(scheduled);
            }
        }

        info!("PID scheduler stopped");
        Ok(())
    }

    /// Stop the scheduler
    pub fn stop(&mut self) {
        info!("Stopping PID scheduler");
        self.running = false;
    }

    /// Check if scheduler is running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get the number of scheduled PIDs
    pub fn pid_count(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = PidScheduler::new(SchedulerConfig::default());
        assert_eq!(scheduler.pid_count(), 8);
    }

    #[test]
    fn test_scheduled_pid_ordering() {
        let mut pid1 = ScheduledPid::new(Pid::Rpm, 5.0);
        let mut pid2 = ScheduledPid::new(Pid::Maf, 1.0);
        
        // pid1 should come first (higher priority, assuming equal time)
        pid1.next_query = Instant::now();
        pid2.next_query = Instant::now();
        
        assert!(pid1 > pid2); // Higher priority
    }
}
