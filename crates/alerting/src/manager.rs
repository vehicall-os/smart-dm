//! Alert Manager Implementation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Confidence threshold for alerts (default: 0.75)
    pub confidence_threshold: f64,
    /// Confidence threshold for critical alerts (default: 0.90)
    pub critical_threshold: f64,
    /// Cooldown period between duplicate alerts (seconds)
    pub cooldown_seconds: u64,
    /// Maximum alerts per hour before throttling
    pub max_alerts_per_hour: usize,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.75,
            critical_threshold: 0.90,
            cooldown_seconds: 1800, // 30 minutes
            max_alerts_per_hour: 10,
        }
    }
}

/// State of an alert
#[derive(Debug, Clone)]
pub struct AlertState {
    /// Last time this alert was fired
    pub last_fired: Instant,
    /// Number of times fired
    pub fire_count: usize,
    /// Whether alert is acknowledged
    pub acknowledged: bool,
}

/// Alert manager for deduplication and throttling
pub struct AlertManager {
    /// Configuration
    config: AlertConfig,
    /// Alert states by fault type
    states: HashMap<String, AlertState>,
    /// Alerts fired in current hour
    hourly_count: usize,
    /// Hour start time
    hour_start: Instant,
}

impl AlertManager {
    /// Create a new alert manager
    pub fn new(config: AlertConfig) -> Self {
        info!("Creating alert manager with config: {:?}", config);
        Self {
            config,
            states: HashMap::new(),
            hourly_count: 0,
            hour_start: Instant::now(),
        }
    }

    /// Check if an alert should be fired based on confidence and deduplication
    pub fn should_fire(&mut self, fault_type: &str, confidence: f64) -> bool {
        // Check confidence threshold
        if confidence < self.config.confidence_threshold {
            debug!("Alert suppressed: confidence {} < threshold {}", 
                confidence, self.config.confidence_threshold);
            return false;
        }

        // Reset hourly counter if needed
        if self.hour_start.elapsed() > Duration::from_secs(3600) {
            self.hourly_count = 0;
            self.hour_start = Instant::now();
        }

        // Check hourly throttle
        if self.hourly_count >= self.config.max_alerts_per_hour {
            warn!("Alert throttled: max alerts per hour reached");
            return false;
        }

        // Check cooldown
        if let Some(state) = self.states.get(fault_type) {
            let cooldown = Duration::from_secs(self.config.cooldown_seconds);
            if state.last_fired.elapsed() < cooldown {
                debug!("Alert suppressed: in cooldown period");
                return false;
            }
        }

        true
    }

    /// Record that an alert was fired
    pub fn record_fire(&mut self, fault_type: &str) {
        self.hourly_count += 1;
        
        let state = self.states.entry(fault_type.to_string()).or_insert(AlertState {
            last_fired: Instant::now(),
            fire_count: 0,
            acknowledged: false,
        });
        
        state.last_fired = Instant::now();
        state.fire_count += 1;
        state.acknowledged = false;

        info!("Alert recorded: {} (count: {})", fault_type, state.fire_count);
    }

    /// Acknowledge an alert
    pub fn acknowledge(&mut self, fault_type: &str) -> bool {
        if let Some(state) = self.states.get_mut(fault_type) {
            state.acknowledged = true;
            info!("Alert acknowledged: {}", fault_type);
            true
        } else {
            false
        }
    }

    /// Get severity level based on confidence
    pub fn get_severity(&self, confidence: f64) -> &'static str {
        if confidence >= self.config.critical_threshold {
            "critical"
        } else if confidence >= 0.85 {
            "high"
        } else if confidence >= self.config.confidence_threshold {
            "medium"
        } else {
            "low"
        }
    }

    /// Get pending (unacknowledged) alerts
    pub fn get_pending(&self) -> Vec<(&str, &AlertState)> {
        self.states
            .iter()
            .filter(|(_, state)| !state.acknowledged)
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }

    /// Get hourly alert count
    pub fn hourly_count(&self) -> usize {
        self.hourly_count
    }

    /// Clear all alert states
    pub fn clear(&mut self) {
        self.states.clear();
        self.hourly_count = 0;
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new(AlertConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_threshold() {
        let mut manager = AlertManager::default();
        
        // Low confidence should not fire
        assert!(!manager.should_fire("overheating", 0.5));
        
        // High confidence should fire
        assert!(manager.should_fire("overheating", 0.85));
    }

    #[test]
    fn test_deduplication() {
        let config = AlertConfig {
            cooldown_seconds: 60, // Short cooldown for test
            ..Default::default()
        };
        let mut manager = AlertManager::new(config);
        
        // First alert should fire
        assert!(manager.should_fire("overheating", 0.85));
        manager.record_fire("overheating");
        
        // Immediate duplicate should not fire
        assert!(!manager.should_fire("overheating", 0.85));
    }

    #[test]
    fn test_severity_levels() {
        let manager = AlertManager::default();
        
        assert_eq!(manager.get_severity(0.95), "critical");
        assert_eq!(manager.get_severity(0.87), "high");
        assert_eq!(manager.get_severity(0.78), "medium");
        assert_eq!(manager.get_severity(0.5), "low");
    }

    #[test]
    fn test_acknowledgement() {
        let mut manager = AlertManager::default();
        manager.record_fire("overheating");
        
        assert!(!manager.states.get("overheating").unwrap().acknowledged);
        manager.acknowledge("overheating");
        assert!(manager.states.get("overheating").unwrap().acknowledged);
    }
}
