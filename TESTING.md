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
| `parse_duration_empty` | Empty string means unlimited duration |
| `parse_duration_valid` | Numeric duration values parse correctly |
| `parse_duration_invalid` | Non-numeric duration strings are rejected |
| `duration_zero_is_none` | Empty/unset duration resolves to unlimited |
| `topics_empty_from_default` | No topics triggers discover-all mode |
| `topics_empty_string_filtered` | `TOPICS=""` env override is treated as unset |
| `topics_positional` | Positional topic arguments parse correctly |
| `parse_fps_max` | `MAX` (case-insensitive) parses to native rate |
| `parse_fps_valid` | Numeric FPS values parse correctly |
| `parse_fps_zero_rejected` | Zero is rejected as invalid FPS |
| `parse_fps_invalid` | Non-numeric strings are rejected |
| `parse_fps_empty` | Empty string defaults to native rate |
| `cube_fps_zero_is_none` | `MAX` resolves to no rate limiting |
| `cube_fps_value` | Numeric FPS resolves to the specified value |

The clock step and MCAP output tests (`clock::tests`, `sink::tests`, `time_tests`) cover:

| Area | Validates |
|------|-----------|
| Offset measurement | Midpoint of the REALTIME/MONOTONIC/REALTIME bracket, retry of wide brackets, tightest bracket kept |
| Step tracking | Forward and backward steps, the 1 s threshold, baseline update, pre-epoch saturation |
| `timerfd` | The fd is created with `TFD_TIMER_CANCEL_ON_SET` and stays quiet without a clock change |
| `clock_sync` | chronyc CSV parsing, chrony vs adjtimex source selection, unprivileged `adjtimex`, chronyc timeout |
| MCAP round trip | `clock_step` records indexed in the summary, `/clock_step` JSON messages at `log_time_after`, no channel without steps, records found by a linear scan when the summary is missing |
| Time helpers | No panic on a pre-epoch clock, the 10 s publish-time skew threshold, `--duration` on elapsed monotonic time |

A real clock step needs `CAP_SYS_TIME` (time namespaces cannot offset `CLOCK_REALTIME`), so it is covered by the on-target scenario below.

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
edgefirst-recorder --duration 30
```

**Or record specific topics:**

```bash
edgefirst-recorder camera/h264 radar/targets model/output --duration 30
```

The recorder logs each discovered topic and its encoding:

```
[INFO] Subscribed to camera/h264 (encoding: foxglove_msgs/msg/CompressedVideo)
[INFO] Subscribed to radar/targets (encoding: sensor_msgs/msg/PointCloud2)
[INFO] Subscribed to model/output (encoding: edgefirst_msgs/msg/Model)
[INFO] Recording to maivin_2025_06_15_14_30_00.mcap
```

### Step 2: Open in Foxglove

1. Open Foxglove Studio
2. **File > Open local file** and select the `.mcap` file
3. Verify topics appear in the sidebar with correct message types

### Step 3: Verify Each Topic Type

| Topic | Foxglove Panel | What to Check |
|-------|---------------|---------------|
| `camera/h264` | Image | Frames decode, timestamps advance |
| `camera/frame` | Raw Messages | CameraFrame samples present |
| `camera/info` | Raw Messages | Intrinsics and distortion parameters |
| `radar/targets` | 3D Panel | Point cloud with x, y, z positions |
| `model/output` | Raw Messages | Detection boxes / masks present |
| `lidar/points` | 3D Panel | LiDAR point cloud |
| `fusion/lidar` | 3D Panel | Fused lidar |
| `tf_static` | 3D Panel | Transform frame visible |
| `imu` | Plot | Accelerometer/gyroscope traces |
| `gps` | Raw Messages | NavSatFix updates |

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
edgefirst-recorder --duration 10 --compression none
edgefirst-recorder --duration 10 --compression lz4
edgefirst-recorder --duration 10 --compression zstd
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
edgefirst-recorder --duration 5
```

Verify the recorder stops after approximately 5 seconds and the MCAP file is properly finalized.

### Clock Steps (on target)

Run on a Maivin with root access. Stepping the clock affects every service on the device.

```bash
# 1. Stop time synchronization and move the clock 475 days back
sudo systemctl stop chronyd
sudo date -s "-475 days"

# 2. Record while the clock is stepped forward, then backward
edgefirst-recorder --duration 60 &
sleep 10; sudo date -s "+475 days"
sleep 10; sudo date -s "-1 hour"
wait

# 3. Restore time synchronization
sudo systemctl start chronyd && sudo chronyc makestep
```

Verify:

- The recorder log shows one `Clock step of ...` warning per step and `Saved MCAP ... (2 clock steps)`.
- `mcap info` reports a single file with `metadata: 3` (one `clock_sync`, two `clock_step`), and `mcap list metadata` lists them.
- In Foxglove, the `/clock_step` topic shows two messages at the step instants, and the recording still covers the full 60 s of monotonic time.
- Without steps, no `/clock_step` topic is present.

### Storage Directory

```bash
STORAGE=/tmp/test-recordings edgefirst-recorder --duration 5
ls /tmp/test-recordings/*.mcap
```

Verify the file is created in the specified directory.

### Zenoh Connectivity

```bash
# Client mode connecting to a specific router
edgefirst-recorder --mode client --connect tcp/192.168.1.100:7447

# Disable multicast scouting
edgefirst-recorder --no-multicast-scouting --connect tcp/localhost:7447
```

---

## CI/CD

CI calls the shared tiered workflows in [EdgeFirstAI/.github](https://github.com/EdgeFirstAI/.github). `ci-gate` is the only required check.

| Tier | Runs when | What |
|------|-----------|------|
| Quick | every push to a ready (non-draft) pull request, and pushes to `main` | `cargo fmt --check`, clippy (host and aarch64 check), `cargo nextest`, dependency license policy, NOTICE validation, workflow lint |
| Full | the `ci:full` label, a merge queue batch, or a manual dispatch | Linux x86_64 and aarch64 tests with coverage, SonarCloud, full source SBOM |
| Nightly | daily, only when `main` moved | Full, plus the ungated `cargo audit` advisory scan |
| Release | push to `release/X.Y.Z` | version and CHANGELOG checks, SBOM, zigbuild binaries for x86_64 and aarch64; the merge tags `vX.Y.Z` and `publish.yml` attaches the artifacts to the GitHub Release |

Draft pull requests run nothing; mark the PR ready for review to run Quick.

Manual Foxglove verification is performed before each release using the workflow described above.

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| No topics discovered | Zenoh publishers not running | Start the EdgeFirst Perception stack |
| Topic skipped: no data | Publisher started after discovery timeout | Increase `--timeout` or specify topics explicitly |
| Topic skipped: no schema | Unknown message encoding | Add the `.msg` schema — see [ARCHITECTURE.md § Adding a Topic / Schema](ARCHITECTURE.md#adding-a-topic--schema) |
| MCAP won't open in Foxglove | File not finalized (crash/kill -9) | Re-record; use SIGINT for clean shutdown |
| Large file size | Uncompressed radar cube data | Use `--compression zstd` and `--cube-fps 5` |
| Low storage shutdown | Disk filling during recording | Free disk space or use `STORAGE` to point to larger volume |
