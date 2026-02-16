# Changelog

All notable changes to EdgeFirst Recorder will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.6.1] - 2026-02-16

### Fixed

- Replaced incorrect DeepView VisionPack Trial EULA with Apache License 2.0 text in LICENSE file
- Added missing SPDX license headers to build.rs and src/schemas.rs

## [1.6.0] - 2026-02-16

### Changed

- Refactored error handling to use anyhow consistently with descriptive context messages
- Merged duplicate stream recording functions into single `record_topic` with optional rate limiting
- Replaced unbounded message channel with bounded channel (capacity 64) for backpressure
- Removed nightly-only rustfmt options for stable toolchain compatibility
- Rewrote README.md with features, supported topics, CLI reference, and quick start guide
- Rewrote ARCHITECTURE.md with mermaid diagrams for threading, data flow, and shutdown
- Release workflow now reuses CI build artifacts instead of rebuilding

### Added

- Created TESTING.md with Foxglove Studio verification workflow
- Added unit tests for CLI argument parsing (parse_fps, cube_fps)

### Fixed

- Fixed topic prefix stripping bug where `topic.replace("rt", "")` could corrupt topic names containing "rt" (now uses `strip_prefix`)

### Removed

- Removed dead test file (src/test.rs) that was never compiled
- Removed legacy Bitbucket Pipelines configuration

## [1.5.0] - 2025-12-26

### Changed

- Migrated from Bitbucket to GitHub
- Updated license to Apache-2.0
- Updated CI/CD to use GitHub Actions with SPS v2.1.1 compliance
- Updated to use cargo-zigbuild for manylinux2014 compatibility

### Security

- Added security policy and vulnerability reporting process

## [1.4.1] - 2025-06-09

### Changed

- Updated dependencies to latest versions
- Improved storage space monitoring with 60-second buffer check

### Fixed

- Fixed storage space check to prevent device from running out of space (EDGEAI-646)
- Fixed process exit code to return 0 on successful completion

## [1.4.0] - 2025-05-14

### Added

- Added Model message schema for edgefirst_msgs
- Added cargo config.toml for build configuration

### Changed

- Updated Mask message schema with boxed field support

## [1.3.3] - 2025-05-10

### Changed

- Lowered default topic discovery timeout from 10 to 5 seconds
- Updated dependencies

## [1.3.2] - 2025-04-30

### Changed

- Dependency updates

## [1.3.1] - 2025-03-25

### Fixed

- Fixed radar cube FPS regulation to allow 0 as valid value (EDGEAI-511)
- Fixed cube FPS parameter validation with proper max value

### Changed

- Updated dependencies

## [1.3.0] - 2025-02-24

### Changed

- Upgraded to Zenoh 1.2 and MCAP 0.15
- Zenoh now only gossips on loopback interface for improved network behavior
- Major dependency cleanup and upgrades

## [1.2.5] - 2025-01-29

### Added

- Added sensor_msgs/msg/Image.msg schema

### Changed

- Updated Bitbucket Pipelines for compatibility

## [1.2.4] - 2024-10-01

### Added

- Added RadarInfo IDL schema

## [1.2.3] - 2024-08-31

### Added

- Added TransformStamped message schema (RVN-291)
- Added radar cube FPS regulation option (EDGEAI-250)

### Changed

- Environment variable option for compression configuration

## [1.2.2] - 2024-08-30

### Added

- Added segmentation Mask message schema

## [1.2.1] - 2024-08-22

### Changed

- Updated cube_fps parameter documentation

## [1.2.0] - 2024-08-12

### Added

- Added segmentation mask schema support

## [1.1.5] - 2024-06-24

### Added

- Added LocalTime schema
- Added CompressedImage (JPEG) schema for sensor_msgs

### Fixed

- Fixed systemd MCAP recovery issue
- Fixed sonarcloud warnings (MVN2-93)

### Changed

- Updated timestamp from UTC to local time
- Made topic selection asynchronous for improved performance (MVN2-293)
- Improved signal handling for unwanted signals

## [1.1.4] - 2024-05-10

### Changed

- Removed unneeded dependencies
- Refactored library to move shared code to maivin-publisher

## [1.1.3] - 2024-04-29

### Added

- Added get_detect_data function for Detect message parsing

## [1.1.2] - 2024-04-26

### Changed

- Updated edgefirst-schemas to 1.1

## [1.1.1] - 2024-04-25

### Added

- Added DmaBuffer message schema

## [1.1.0] - 2024-04-23

### Added

- Added compression parameter (defaults to none)
- Added marker schema
- Added CameraInfo schema
- Added RadarCube schema
- Added all-topics flag for automatic topic discovery

### Changed

- Refactored schemas to automatically embed all locally known schemas
- Updated async threading for better multi-topic handling
- Various memory optimizations using Vec::with_capacity()

### Fixed

- Fixed topic prefix handling in MCAP output

## [1.0.4] - 2024-03-06

### Changed

- Improved storage directory and filename handling
- Cleaned up logging to avoid quotes around topics and filenames
- Updated timeout conditions

## [1.0.3] - 2024-03-05

### Fixed

- Fixed topic parsing with proper delimiter handling
- Fixed environment variable issues with clap

## [1.0.2] - 2024-03-05

### Added

- Added better logging for users and debugging
- Added topic prefix checks

### Changed

- Allow CLAP parameters to be set from environment variables
- Use connect instead of endpoints for Zenoh connection list

## [1.0.1] - 2024-03-05

### Changed

- Updated cargo dependencies
- Added dynamic topic support

## [1.0.0] - 2024-02-29

### Added

- Initial stable release
- MCAP format recording support with Zenoh integration
- STORAGE environment variable for output directory configuration
- Configurable topic recording via command line and environment

### Fixed

- Fixed AWS S3 path typo
- Added proper directory creation with error checking
