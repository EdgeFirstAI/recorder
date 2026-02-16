# EdgeFirst MCAP Recorder

[![Build Status](https://github.com/EdgeFirstAI/recorder/workflows/CI/badge.svg)](https://github.com/EdgeFirstAI/recorder/actions)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![EdgeFirst](https://img.shields.io/badge/EdgeFirst-Perception-green)](https://doc.edgefirst.ai/perception/)

**Record EdgeFirst Perception topics to MCAP files for analysis in Foxglove Studio**

EdgeFirst Recorder subscribes to Zenoh topics published by the EdgeFirst Perception stack and writes them to [MCAP](https://mcap.dev/) files with embedded ROS 2 message schemas. The resulting files can be opened directly in [Foxglove Studio](https://foxglove.dev/) for visualization, debugging, and data analysis.

## Features

- **Selective or Automatic Recording** - Specify individual topics or discover all active topics
- **Embedded Schemas** - ROS 2 message definitions are bundled at compile time for self-describing MCAP files
- **MCAP Compression** - Optional LZ4 or Zstd compression for smaller files
- **Radar Cube Rate Limiting** - Configurable FPS cap for high-bandwidth radar cube data
- **Storage Monitoring** - Automatic shutdown when disk space runs low
- **Duration Limits** - Optional time-bounded recording sessions
- **Flexible Zenoh Configuration** - Peer/client modes, custom endpoints, multicast control

## Supported Topics

The recorder includes schemas for the following message types:

| Package | Messages |
|---------|----------|
| `edgefirst_msgs` | Detect, DmaBuffer, LocalTime, Mask, Model, ModelInfo, RadarCube, RadarInfo |
| `sensor_msgs` | CameraInfo, CompressedImage, Image, Imu, NavSatFix, NavSatStatus, PointCloud2 |
| `geometry_msgs` | TransformStamped |
| `foxglove_msgs` | CompressedImage, CompressedVideo, ImageAnnotations |
| `visualization_msgs` | Marker |

## Quick Start

### Prerequisites

- Rust toolchain 1.70+
- Running EdgeFirst Perception stack (or any Zenoh publisher)

### Installation

**From Source:**

```bash
git clone https://github.com/EdgeFirstAI/recorder.git
cd recorder
cargo build --release
sudo cp target/release/edgefirst-recorder /usr/local/bin/
```

**Cross-Compile for ARM64:**

```bash
cargo install cross
cross build --target aarch64-unknown-linux-gnu --release
```

### Basic Usage

**Record specific topics:**

```bash
edgefirst-recorder /camera/image /radar/targets /detect
```

**Discover and record all active topics:**

```bash
edgefirst-recorder --all-topics
```

**Record for 30 seconds with Zstd compression:**

```bash
edgefirst-recorder --all-topics --duration 30 --compression zstd
```

**Limit radar cube recording to 5 FPS:**

```bash
edgefirst-recorder --all-topics --cube-fps 5
```

**Save to a specific directory:**

```bash
STORAGE=/data/recordings edgefirst-recorder --all-topics
```

### Viewing Recordings

Open the generated `.mcap` file in [Foxglove Studio](https://foxglove.dev/):

1. Launch Foxglove Studio
2. Open the MCAP file from the file menu
3. Add panels for the recorded topics (image, point cloud, plot, etc.)

## CLI Reference

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `<TOPICS>` | `TOPICS` | | Space-delimited topics to record |
| `-a, --all-topics` | | | Discover and record all active topics |
| `-d, --duration` | `DURATION` | | Recording duration in seconds |
| `-t, --timeout` | | `5` | Topic discovery timeout in seconds |
| `-z, --compression` | `COMPRESSION` | `none` | Compression: `none`, `lz4`, `zstd` |
| `--cube-fps` | `CUBE_FPS` | | Radar cube FPS limit (`MAX` for native rate) |
| `--mode` | `MODE` | `peer` | Zenoh mode: `peer`, `client`, `router` |
| `--connect` | `CONNECT` | | Zenoh endpoints to connect to |
| `--listen` | `LISTEN` | | Zenoh endpoints to listen on |
| `--no-multicast-scouting` | `NO_MULTICAST_SCOUTING` | | Disable Zenoh multicast discovery |

Topic names are automatically normalized to the `rt/` prefix convention used by the Zenoh-ROS 2 bridge. For example, `/camera/image` becomes `rt/camera/image`.

## Output

Recordings are saved as `<hostname>_<timestamp>.mcap` in the current directory, or in the directory specified by the `STORAGE` environment variable. The filename format is:

```
maivin_2025_06_15_14_30_00.mcap
```

## EdgeFirst Ecosystem

EdgeFirst Recorder is part of the [EdgeFirst Perception Middleware](https://doc.edgefirst.ai/perception/), capturing sensor and inference data for offline analysis.

- **[EdgeFirst Perception](https://doc.edgefirst.ai/perception/)** - Multi-sensor fusion middleware
- **[EdgeFirst Studio](https://edgefirst.studio)** - MLOps platform for edge AI
- **[EdgeFirst Modules](https://www.edgefirst.ai/edgefirstmodules)** - Maivin and Raivin hardware platforms

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Software architecture and design
- [TESTING.md](TESTING.md) - Testing and verification workflow
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [CONTRIBUTING.md](CONTRIBUTING.md) - Contribution guidelines

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.

Copyright (c) 2025 Au-Zone Technologies. All Rights Reserved.
