# Vehicle AI Diagnostics Pipeline

A real-time vehicle predictive maintenance system built in Rust for Raspberry Pi 4B.

## Overview

This system reads OBD-II data from vehicles, processes it through a feature engineering pipeline, runs ML inference for fault prediction, and exposes results via REST/WebSocket APIs.

## Project Structure

```
VehicallOS/
├── Cargo.toml              # Workspace configuration
├── crates/
│   ├── obd-protocol/       # OBD-II serial communication
│   ├── obd-scheduler/      # Adaptive PID scheduling
│   ├── data-validator/     # Input validation & normalization
│   ├── ring-buffer/        # Lock-free sensor frame storage
│   ├── feature-engine/     # Statistical & FFT feature extraction
│   ├── inference-engine/   # ONNX ML inference
│   ├── fallback/           # Rule-based fallback logic
│   ├── alerting/           # Alert deduplication & throttling
│   ├── storage/            # SQLite persistence layer
│   └── api/                # REST + WebSocket server
├── models/                 # ONNX model files
└── config/                 # Configuration files
```

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- For cross-compilation: `cargo install cross`

### Build

```bash
# Build all crates
cargo build --workspace

# Build for Raspberry Pi (ARM64)
cross build --target aarch64-unknown-linux-gnu --release
```

### Run

```bash
# Run the API server
cargo run -p api

# Access the health endpoint
curl http://localhost:8080/api/v1/health
```

### Test

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p obd-protocol
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | System health metrics |
| GET | `/api/v1/sensors/live` | Recent sensor readings |
| GET | `/api/v1/predictions` | ML predictions |
| GET | `/api/v1/alerts` | Active alerts |

## Architecture

- **OBD-II Acquisition**: Async serial communication with ELM327 adapters
- **Feature Engineering**: Statistical features + FFT spectral analysis
- **ML Inference**: ONNX model using tract-onnx
- **Fallback System**: Rule-based heuristics when ML is unavailable
- **Storage**: In-memory with SQLite persistence
- **API**: axum-based REST server with Prometheus metrics

## License

MIT
