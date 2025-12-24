//! Vehicle Diagnostics API Server
//!
//! REST API and WebSocket server for the vehicle diagnostics dashboard.

use axum::{
    routing::{get, post},
    Router,
    Json,
    extract::{Query, State},
    response::IntoResponse,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod routes;

use storage::Repository;

/// Application state shared across handlers
pub struct AppState {
    /// Storage repository
    pub repository: Repository,
    /// Version string
    pub version: String,
    /// Start time
    pub start_time: std::time::Instant,
}

impl AppState {
    /// Create new application state
    pub fn new() -> Self {
        Self {
            repository: Repository::new(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: std::time::Instant::now(),
        }
    }
}

/// Health response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub version: String,
    pub uptime_seconds: u64,
    pub components: ComponentStatus,
    pub metrics: SystemMetrics,
}

/// Component status
#[derive(Debug, Serialize)]
pub struct ComponentStatus {
    pub obd: ComponentHealth,
    pub inference: ComponentHealth,
    pub database: ComponentHealth,
}

/// Individual component health
#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: String,
    pub last_activity_ms: Option<u64>,
}

/// System metrics
#[derive(Debug, Serialize)]
pub struct SystemMetrics {
    pub sensor_count: usize,
    pub prediction_count: usize,
}

/// Create the application router
pub fn create_router(state: Arc<RwLock<AppState>>) -> Router {
    Router::new()
        .route("/api/v1/health", get(health_handler))
        .route("/api/v1/sensors/live", get(routes::sensors::get_live))
        .route("/api/v1/predictions", get(routes::predictions::get_predictions))
        .route("/api/v1/alerts", get(routes::alerts::get_alerts))
        .with_state(state)
}

/// Health check handler
async fn health_handler(
    State(state): State<Arc<RwLock<AppState>>>,
) -> impl IntoResponse {
    let state = state.read().await;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let response = HealthResponse {
        status: "healthy".to_string(),
        timestamp,
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
        components: ComponentStatus {
            obd: ComponentHealth {
                status: "ok".to_string(),
                last_activity_ms: Some(100),
            },
            inference: ComponentHealth {
                status: "ok".to_string(),
                last_activity_ms: Some(150),
            },
            database: ComponentHealth {
                status: "ok".to_string(),
                last_activity_ms: None,
            },
        },
        metrics: SystemMetrics {
            sensor_count: state.repository.sensor_count(),
            prediction_count: state.repository.prediction_count(),
        },
    };

    Json(response)
}

/// Initialize logging
pub fn init_logging() {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
}

/// Run the server
pub async fn run_server(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    
    let state = Arc::new(RwLock::new(AppState::new()));
    let app = create_router(state);

    info!("Starting API server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
