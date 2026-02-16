# Testing

**Version:** 1.0
**Last Updated:** 2026-02-15

---

## Overview

EdgeFirst Recorder testing combines automated unit tests with manual verification in Foxglove Studio. The primary validation workflow is: record topics into an MCAP file, then open it in Foxglove to visually confirm all data was captured correctly.

---

## Unit Tests

Run the unit test suite:

```bash
cargo test
```

The tests cover CLI argument parsing and validation:

| Test | Validates |
|------|-----------|
| `parse_fps_max` | `MAX` (case-insensitive) parses to native rate |
| `parse_fps_valid` | Numeric FPS values parse correctly |
| `parse_fps_zero_rejected` | Zero is rejected as invalid FPS |
| `parse_fps_invalid` | Non-numeric strings are rejected |
| `parse_fps_empty` | Empty string defaults to native rate |
| `cube_fps_zero_is_none` | `MAX` resolves to no rate limiting |
| `cube_fps_value` | Numeric FPS resolves to the specified value |

### Running with Coverage

```bash
make test
```

This uses `cargo-llvm-cov` with `cargo-nextest` and generates an LCOV report at `target/rust-coverage.lcov`.

---

## Manual Verification with Foxglove

This is the primary testing workflow. It validates that the recorder correctly captures Zenoh topics and produces valid, viewable MCAP files.

### Prerequisites

- EdgeFirst Perception stack running (camera, radar, detect nodes)
- [Foxglove Studio](https://foxglove.dev/) installed
- Zenoh connectivity between recorder and publishers

### Step 1: Record Topics

**Record all active topics for 30 seconds:**

```bash
edgefirst-recorder --all-topics --duration 30
```

**Or record specific topics:**

```bash
edgefirst-recorder /camera/image /radar/targets /detect --duration 30
```

The recorder logs each discovered topic and its encoding:

```
[INFO] Subscribed to rt/camera/image (encoding: sensor_msgs/msg/Image)
[INFO] Subscribed to rt/radar/targets (encoding: sensor_msgs/msg/PointCloud2)
[INFO] Subscribed to rt/detect (encoding: edgefirst_msgs/msg/Detect)
[INFO] Recording to maivin_2025_06_15_14_30_00.mcap
```

### Step 2: Open in Foxglove

1. Open Foxglove Studio
2. **File > Open local file** and select the `.mcap` file
3. Verify topics appear in the sidebar with correct message types

### Step 3: Verify Each Topic Type

| Topic | Foxglove Panel | What to Check |
|-------|---------------|---------------|
| `/camera/image` | Image | Frames render, timestamps advance |
| `/camera/compressed` | Image | JPEG frames decode correctly |
| `/radar/targets` | 3D Panel | Point cloud with x, y, z positions |
| `/radar/clusters` | 3D Panel | Clustered points with track IDs |
| `/detect` | Raw Messages | Detection bounding boxes present |
| `/tf_static` | 3D Panel | Transform frame visible |
| `/imu/data` | Plot | Accelerometer/gyroscope traces |
| `/camera/info` | Raw Messages | Intrinsics and distortion parameters |

### Step 4: Validate Recording Quality

- **Timeline**: Scrub through the recording, verify continuous data without gaps
- **Message count**: Check the topic statistics panel for expected message rates
- **Timestamps**: Verify log timestamps are monotonically increasing
- **Schema**: Right-click a topic and inspect the schema definition

---

## Test Scenarios

### Compression Modes

Record with each compression option and verify the MCAP opens correctly:

```bash
edgefirst-recorder --all-topics --duration 10 --compression none
edgefirst-recorder --all-topics --duration 10 --compression lz4
edgefirst-recorder --all-topics --duration 10 --compression zstd
```

Compare file sizes to validate compression is applied.

### Radar Cube Rate Limiting

```bash
# Native rate (no limiting)
edgefirst-recorder radar/cube --cube-fps MAX --duration 10

# Limited to 5 FPS
edgefirst-recorder radar/cube --cube-fps 5 --duration 10
```

In Foxglove, verify the cube topic message rate matches the specified FPS.

### Duration Limiting

```bash
edgefirst-recorder --all-topics --duration 5
```

Verify the recorder stops after approximately 5 seconds and the MCAP file is properly finalized.

### Storage Directory

```bash
STORAGE=/tmp/test-recordings edgefirst-recorder --all-topics --duration 5
ls /tmp/test-recordings/*.mcap
```

Verify the file is created in the specified directory.

### Zenoh Connectivity

```bash
# Client mode connecting to a specific router
edgefirst-recorder --all-topics --mode client --connect tcp/192.168.1.100:7447

# Disable multicast scouting
edgefirst-recorder --all-topics --no-multicast-scouting --connect tcp/localhost:7447
```

---

## CI/CD

The CI pipeline runs on every push and pull request:

1. **Format check** - `cargo fmt --check`
2. **Clippy** - `cargo clippy -- -D warnings`
3. **Unit tests** - `cargo test`
4. **Build** - Release build for `x86_64` and `aarch64`

Manual Foxglove verification is performed before each release using the workflow described above.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| No topics discovered | Zenoh publishers not running | Start the EdgeFirst Perception stack |
| Topic skipped: no data | Publisher started after discovery timeout | Increase `--timeout` or specify topics explicitly |
| Topic skipped: no schema | Unknown message encoding | Add the `.msg` schema to `src/schemas/` |
| MCAP won't open in Foxglove | File not finalized (crash/kill -9) | Re-record; use SIGINT for clean shutdown |
| Large file size | Uncompressed radar cube data | Use `--compression zstd` and `--cube-fps 5` |
| Low storage shutdown | Disk filling during recording | Free disk space or use `STORAGE` to point to larger volume |
