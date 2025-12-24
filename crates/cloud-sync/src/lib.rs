//! Cloud Synchronization Module
//!
//! MQTT-based event publishing with:
//! - Bandwidth-aware upload scheduling
//! - Event prioritization
//! - Video upload management
//! - Driver roster sync

use chrono::{DateTime, Utc};
use event_fusion::{FusedEvent, Severity};
use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use thiserror::Error;
use tracing::{debug, error, info};
use uuid::Uuid;

/// Cloud sync error types
#[derive(Error, Debug)]
pub enum CloudError {
    #[error("Connection failed: {0}")]
    Connection(String),
    
    #[error("Publish failed: {0}")]
    Publish(String),
    
    #[error("Bandwidth limit exceeded")]
    BandwidthLimit,
    
    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// Upload schedule
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadSchedule {
    /// Upload immediately (critical events)
    Immediate,
    /// Upload when Wi-Fi available
    Opportunistic,
    /// Upload during night hours
    Nightly,
    /// Upload only at depot
    Manual,
}

/// Cloud sync configuration
#[derive(Debug, Clone)]
pub struct CloudConfig {
    /// MQTT broker URL
    pub broker_url: String,
    /// MQTT port
    pub broker_port: u16,
    /// Vehicle ID
    pub vehicle_id: String,
    /// Daily upload quota (MB)
    pub daily_quota_mb: u32,
    /// Upload schedule
    pub schedule: UploadSchedule,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            broker_url: "localhost".to_string(),
            broker_port: 1883,
            vehicle_id: "unknown".to_string(),
            daily_quota_mb: 500,
            schedule: UploadSchedule::Opportunistic,
        }
    }
}

/// Event message for cloud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMessage {
    pub message_type: String,
    pub vehicle_id: String,
    pub timestamp: DateTime<Utc>,
    pub driver_id: Option<String>,
    pub event: FusedEvent,
    pub video_references: Option<VideoReferences>,
}

/// Video file references
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoReferences {
    pub cabin: Option<String>,
    pub road: Option<String>,
}

/// Cloud sync manager
pub struct CloudSync {
    config: CloudConfig,
    client: Option<AsyncClient>,
    used_today_mb: AtomicU32,
}

impl CloudSync {
    /// Create new cloud sync manager
    pub fn new(config: CloudConfig) -> Self {
        Self {
            config,
            client: None,
            used_today_mb: AtomicU32::new(0),
        }
    }

    /// Connect to MQTT broker
    pub async fn connect(&mut self) -> Result<(), CloudError> {
        let mut options = MqttOptions::new(
            format!("vehicle-{}", self.config.vehicle_id),
            &self.config.broker_url,
            self.config.broker_port,
        );
        options.set_keep_alive(std::time::Duration::from_secs(30));

        let (client, mut eventloop) = AsyncClient::new(options, 10);

        // Spawn event loop handler
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(incoming)) => {
                        debug!("MQTT incoming: {:?}", incoming);
                    }
                    Err(e) => {
                        error!("MQTT error: {}", e);
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                    _ => {}
                }
            }
        });

        self.client = Some(client);
        info!("Connected to MQTT broker: {}", self.config.broker_url);
        Ok(())
    }

    /// Publish event to cloud
    pub async fn publish_event(
        &self,
        event: FusedEvent,
        driver_id: Option<String>,
    ) -> Result<(), CloudError> {
        // Check if we should upload
        if !self.should_upload(&event) {
            return Err(CloudError::BandwidthLimit);
        }

        let client = self.client.as_ref()
            .ok_or_else(|| CloudError::Connection("Not connected".to_string()))?;

        let message = EventMessage {
            message_type: "event".to_string(),
            vehicle_id: self.config.vehicle_id.clone(),
            timestamp: Utc::now(),
            driver_id,
            event,
            video_references: None,
        };

        let payload = serde_json::to_vec(&message)
            .map_err(|e| CloudError::Serialization(e.to_string()))?;

        let topic = format!("vehicles/{}/events", self.config.vehicle_id);
        
        client.publish(&topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(|e| CloudError::Publish(e.to_string()))?;

        // Track bandwidth usage
        self.used_today_mb.fetch_add(1, Ordering::Relaxed); // Approximate

        Ok(())
    }

    /// Check if event should be uploaded
    fn should_upload(&self, event: &FusedEvent) -> bool {
        // Critical events bypass quota
        if matches!(event, FusedEvent::Crash { .. }) {
            return true;
        }

        // Check quota
        let used = self.used_today_mb.load(Ordering::Relaxed);
        if used >= self.config.daily_quota_mb {
            return false;
        }

        // Check schedule
        match self.config.schedule {
            UploadSchedule::Immediate => true,
            UploadSchedule::Opportunistic => true, // TODO: Check Wi-Fi
            UploadSchedule::Nightly => self.is_nightly_window(),
            UploadSchedule::Manual => false,
        }
    }

    fn is_nightly_window(&self) -> bool {
        let hour = Utc::now().format("%H").to_string().parse::<u32>().unwrap_or(12);
        hour >= 2 && hour <= 6
    }

    /// Reset daily quota (call at midnight)
    pub fn reset_daily_quota(&self) {
        self.used_today_mb.store(0, Ordering::Relaxed);
    }
}
