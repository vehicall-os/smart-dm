//! Prediction Routes

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::AppState;
use storage::PredictionRecord;

/// Query parameters for predictions endpoint
#[derive(Debug, Deserialize)]
pub struct PredictionQuery {
    /// Filter by severity
    pub severity: Option<String>,
    /// Maximum number of records
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// Response for predictions endpoint
#[derive(Debug, Serialize)]
pub struct PredictionResponse {
    pub data: Vec<PredictionRecord>,
    pub count: usize,
}

/// Get predictions
pub async fn get_predictions(
    State(state): State<Arc<RwLock<AppState>>>,
    Query(params): Query<PredictionQuery>,
) -> Json<PredictionResponse> {
    let state = state.read().await;
    let limit = params.limit.min(500);

    let data = state.repository
        .get_predictions(params.severity.as_deref(), limit)
        .unwrap_or_default();

    Json(PredictionResponse {
        count: data.len(),
        data,
    })
}
