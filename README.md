<div align="center">
  <img src="assets/social_preview.png" width="100%" alt="VehicallOS Social Preview" />
</div>

<div align="center">
  <img src="assets/logo.png" width="150" alt="VehicallOS Logo" />
  <h1>VehicallOS</h1>
</div>


A real-time vehicle diagnostics and fleet safety platform built with a **C++/Rust hybrid architecture** for Raspberry Pi 4B.

## Features

- **OBD-II/CAN Bus Integration** - Real-time engine diagnostics via SocketCAN and ELM327
- **Driver Monitoring System (DMS)** - Drowsiness and distraction detection using IR camera
- **Advanced Driver Assistance (ADAS)** - Lane departure and forward collision warnings
- **Driver Authentication** - Face recognition with ignition lockout
- **Cloud Fleet Management** - MQTT-based event publishing with bandwidth management
- **ML Inference** - ONNX model inference for predictive maintenance

## Architecture

```mermaid
graph TB
    subgraph Hardware["Hardware Layer"]
        OBD["OBD-II Port"]
        CAM_IR["IR Camera (Cabin)"]
        CAM_ROAD["Dashcam (Road)"]
        IMU["MPU-6050 IMU"]
    end

    subgraph CPP["C++ Driver Layer"]
        CAN_DRV["can_driver.cpp"]
        CABIN_CPP["cabin_capture.cpp"]
        ROAD_CPP["road_capture.cpp"]
        IMU_CPP["imu_driver.cpp"]
    end

    subgraph Rust["Rust Application Layer"]
        OBD_CRATE["obd-protocol"]
        CAM_CRATE["camera-capture"]
        DMS_CRATE["dms"]
        ADAS_CRATE["adas"]
        FUSION["event-fusion"]
        AUTH["driver-auth"]
        CLOUD["cloud-sync"]
        API["api server"]
    end

    subgraph External["External Services"]
        MQTT["MQTT Broker"]
        FLEET["Fleet Dashboard"]
    end

    OBD --> CAN_DRV --> OBD_CRATE
    CAM_IR --> CABIN_CPP --> CAM_CRATE --> DMS_CRATE
    CAM_ROAD --> ROAD_CPP --> CAM_CRATE --> ADAS_CRATE
    IMU --> IMU_CPP --> CAM_CRATE

    DMS_CRATE --> FUSION
    ADAS_CRATE --> FUSION
    OBD_CRATE --> FUSION
    AUTH --> FUSION

    FUSION --> CLOUD --> MQTT --> FLEET
    FUSION --> API
```

## Data Flow

```mermaid
flowchart LR
    subgraph Acquisition["Data Acquisition"]
        A1["CAN Frames<br/>5Hz"]
        A2["IR Frames<br/>15fps"]
        A3["Road Frames<br/>30fps"]
        A4["IMU Data<br/>100Hz"]
    end

    subgraph Processing["Processing"]
        P1["Feature Engine"]
        P2["Face Detection"]
        P3["Lane Detection"]
        P4["G-Force Calc"]
    end

    subgraph Analysis["Analysis"]
        AN1["ML Inference"]
        AN2["Drowsiness"]
        AN3["Collision Warning"]
        AN4["Crash Detection"]
    end

    subgraph Output["Output"]
        O1["REST API"]
        O2["MQTT Events"]
        O3["Alerts"]
    end

    A1 --> P1 --> AN1 --> O1
    A2 --> P2 --> AN2 --> O2
    A3 --> P3 --> AN3 --> O3
    A4 --> P4 --> AN4 --> O2
```

## Project Structure

```
VehicallOS/
├── Cargo.toml                 # Workspace configuration
├── CMakeLists.txt             # C++ build configuration
├── include/
│   ├── can_obd_driver.h       # CAN/OBD FFI header
│   └── camera_capture.h       # Camera FFI header
├── src_cpp/
│   ├── can_driver.cpp         # SocketCAN driver
│   ├── cabin_capture.cpp      # IR camera (V4L2)
│   ├── road_capture.cpp       # Dashcam (H264)
│   └── imu_driver.cpp         # MPU-6050 driver
└── crates/
    ├── obd-protocol/          # OBD-II + CAN FFI bindings
    ├── camera-capture/        # V4L2 camera FFI
    ├── dms/                   # Driver Monitoring System
    ├── adas/                  # Advanced Driver Assistance
    ├── event-fusion/          # Multi-modal correlation
    ├── driver-auth/           # Face recognition
    ├── cloud-sync/            # MQTT + S3 sync
    ├── ring-buffer/           # Lock-free buffer
    ├── feature-engine/        # Statistical features
    ├── inference-engine/      # ONNX inference
    ├── fallback/              # Rule-based fallback
    ├── alerting/              # Alert management
    ├── storage/               # SQLite persistence
    └── api/                   # REST + WebSocket server
```

## Quick Start

### Prerequisites

- Rust 1.75+ (stable)
- C++17 compiler (GCC 10+ or Clang 12+)
- CMake 3.20+
- For Pi: `cross` for ARM64 cross-compilation

### Build

```bash
# Build all Rust crates
cargo build --workspace

# Build for Raspberry Pi (ARM64)
cross build --target aarch64-unknown-linux-gnu --release
```

### Run

```bash
# Run the API server
cargo run -p api

# Health check
curl http://localhost:8080/api/v1/health
```

### Test

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p dms
```

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | System health metrics |
| GET | `/api/v1/sensors/live` | Recent sensor readings |
| GET | `/api/v1/predictions` | ML predictions |
| GET | `/api/v1/alerts` | Active alerts |

## Crate Overview

| Crate | Purpose |
|-------|---------|
| `obd-protocol` | OBD-II serial + SocketCAN FFI |
| `camera-capture` | V4L2 camera + IMU bindings |
| `dms` | Drowsiness, distraction, gaze |
| `adas` | Lane, objects, traffic signs |
| `event-fusion` | OBD + CV + IMU correlation |
| `driver-auth` | Face recognition enrollment |
| `cloud-sync` | MQTT with bandwidth limits |
| `api` | REST server with rate limiting |

## Hardware Requirements

| Component | Specification | Purpose |
|-----------|---------------|---------|
| Raspberry Pi 4B | 4GB RAM | Compute |
| Pi Camera v3 (IR) | 640x480 @ 15fps | DMS |
| USB Dashcam | 1080p @ 30fps | ADAS |
| OBD-II Adapter | ELM327 or SocketCAN | Engine data |
| MPU-6050 | I2C IMU | Crash detection |

## Performance (Pi 4B)

| Module | CPU | RAM | GPU |
|--------|-----|-----|-----|
| Camera Capture | 13% | 150MB | 35% |
| DMS Pipeline | 40% | 120MB | 20% |
| ADAS Pipeline | 65% | 200MB | 35% |
| **Total** | ~80% | ~500MB | ~70% |

## License

MIT
