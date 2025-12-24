//! Alert Routes

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;

/// Query parameters for alerts endpoint
#[derive(Debug, Deserialize)]
pub struct AlertQuery {
    /// Filter by severity
    pub severity: Option<String>,
    /// Filter by acknowledged status
    pub acknowledged: Option<bool>,
    /// Maximum number of records
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Alert record
#[derive(Debug, Serialize)]
pub struct AlertRecord {
    pub id: i64,
    pub timestamp_ms: i64,
    pub fault_type: String,
    pub severity: String,
    pub message: String,
    pub acknowledged: bool,
}

/// Response for alerts endpoint
#[derive(Debug, Serialize)]
pub struct AlertResponse {
    pub data: Vec<AlertRecord>,
    pub count: usize,
    pub unacknowledged_count: usize,
}

/// Get alerts
pub async fn get_alerts(
    State(_state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<AlertQuery>,
) -> Json<AlertResponse> {
    // Mock response - in real implementation, this would query the database
    let alerts = vec![
        AlertRecord {
            id: 1,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
            fault_type: "none".to_string(),
            severity: "low".to_string(),
            message: "System operating normally".to_string(),
            acknowledged: true,
        }
    ];

    let unack = alerts.iter().filter(|a| !a.acknowledged).count();

    Json(AlertResponse {
        count: alerts.len(),
        unacknowledged_count: unack,
        data: alerts,
    })
}
