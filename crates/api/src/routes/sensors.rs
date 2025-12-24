//! Sensor Routes

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;
use storage::SensorRecord;

/// Query parameters for sensors endpoint
#[derive(Debug, Deserialize)]
pub struct SensorQuery {
    /// Maximum number of records to return
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Return records since this timestamp (ms)
    pub since: Option<i64>,
}

fn default_limit() -> usize {
    100
}

/// Response for sensors endpoint
#[derive(Debug, Serialize)]
pub struct SensorResponse {
    pub data: Vec<SensorRecord>,
    pub meta: SensorMeta,
}

#[derive(Debug, Serialize)]
pub struct SensorMeta {
    pub count: usize,
    pub limit: usize,
}

/// Get live sensor data
pub async fn get_live(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<SensorQuery>,
) -> Json<SensorResponse> {
    let state = state.read().await;
    let limit = params.limit.min(1000);

    let data = if let Some(since) = params.since {
        state.repository.get_sensors_since(since).unwrap_or_default()
    } else {
        state.repository.get_sensors(limit).unwrap_or_default()
    };

    Json(SensorResponse {
        meta: SensorMeta {
            count: data.len(),
            limit,
        },
        data,
    })
}
