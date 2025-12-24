//! Vehicle Diagnostics Pipeline - Main Entry Point

use api::{init_logging, run_server};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    
    info!("=== Vehicle AI Pipeline v{} ===", env!("CARGO_PKG_VERSION"));
    info!("Starting vehicle diagnostics system...");

    // Start the API server
    let addr = "0.0.0.0:8080";
    run_server(addr).await?;

    Ok(())
}
