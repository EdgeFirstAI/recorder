// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

use clap::{Parser, ValueEnum};
use serde_json::json;
use zenoh::config::{Config, WhatAmI};

/// MCAP compression algorithm.
///
/// Controls how recorded data is compressed inside the MCAP file.
/// Compression reduces file size at the cost of CPU during recording.
#[derive(ValueEnum, Debug, Clone)]
pub enum Compression {
    /// No compression (fastest recording, largest files)
    None,
    /// LZ4 compression (fast, moderate compression ratio)
    Lz4,
    /// Zstandard compression (slower, best compression ratio)
    Zstd,
}

impl From<Compression> for Option<mcap::Compression> {
    fn from(compression: Compression) -> Self {
        match compression {
            Compression::None => None,
            Compression::Lz4 => Some(mcap::Compression::Lz4),
            Compression::Zstd => Some(mcap::Compression::Zstd),
        }
    }
}

/// Command-line arguments for EdgeFirst Recorder.
///
/// This structure defines all configuration options for the recorder node,
/// including recording duration, topic selection, MCAP compression, and
/// Zenoh configuration. Arguments can be specified via command line or
/// environment variables.
///
/// # Example
///
/// ```bash
/// # Via command line
/// edgefirst-recorder --duration 60 --compression lz4 --all-topics
///
/// # Via environment variables
/// export DURATION=60
/// export COMPRESSION=lz4
/// edgefirst-recorder --all-topics
/// ```
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Duration for the recording in seconds
    #[arg(short, long, env = "DURATION")]
    pub duration: Option<u64>,

    /// Topic detection timeout in seconds
    #[arg(short, long, default_value = "5")]
    pub timeout: u64,

    /// MCAP compression algorithm
    #[arg(env = "COMPRESSION", short = 'z', long, value_enum, default_value_t = Compression::None)]
    pub compression: Compression,

    /// Topics to record (space-delimited)
    #[arg(env = "TOPICS", required = false, value_delimiter = ' ')]
    pub topics: Vec<String>,

    /// Discover and record all available topics
    #[arg(short, long)]
    pub all_topics: bool,

    /// Limit the frame rate of the radar cube topic. Use 'MAX' for native rate.
    #[arg(long, env = "CUBE_FPS", value_parser = parse_fps)]
    pub cube_fps: Option<u32>,

    /// Zenoh participant mode (peer, client, or router)
    #[arg(long, env = "MODE", default_value = "peer")]
    pub mode: WhatAmI,

    /// Zenoh endpoints to connect to (can specify multiple)
    #[arg(long, env = "CONNECT")]
    pub connect: Vec<String>,

    /// Zenoh endpoints to listen on (can specify multiple)
    #[arg(long, env = "LISTEN")]
    pub listen: Vec<String>,

    /// Disable Zenoh multicast scouting
    #[arg(long, env = "NO_MULTICAST_SCOUTING")]
    pub no_multicast_scouting: bool,
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        let mut config = Config::default();

        config
            .insert_json5("mode", &json!(args.mode).to_string())
            .unwrap();

        if !args.connect.is_empty() {
            config
                .insert_json5("connect/endpoints", &json!(args.connect).to_string())
                .unwrap();
        }

        if !args.listen.is_empty() {
            config
                .insert_json5("listen/endpoints", &json!(args.listen).to_string())
                .unwrap();
        }

        if args.no_multicast_scouting {
            config
                .insert_json5("scouting/multicast/enabled", &json!(false).to_string())
                .unwrap();
        }

        config
            .insert_json5("scouting/multicast/interface", &json!("lo").to_string())
            .unwrap();

        config
    }
}

fn parse_fps(s: &str) -> Result<u32, String> {
    if s.is_empty() || s.eq_ignore_ascii_case("MAX") {
        return Ok(0);
    }

    let fps: u32 = s
        .parse()
        .map_err(|_| "expected a positive integer or 'MAX'")?;

    if fps == 0 {
        Err("FPS must be greater than 0".to_string())
    } else {
        Ok(fps)
    }
}

impl Args {
    pub fn cube_fps(&self) -> Option<u32> {
        match self.cube_fps {
            Some(0) => None,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_fps_max() {
        assert_eq!(parse_fps("MAX").unwrap(), 0);
        assert_eq!(parse_fps("max").unwrap(), 0);
        assert_eq!(parse_fps("Max").unwrap(), 0);
    }

    #[test]
    fn parse_fps_valid() {
        assert_eq!(parse_fps("10").unwrap(), 10);
        assert_eq!(parse_fps("1").unwrap(), 1);
        assert_eq!(parse_fps("30").unwrap(), 30);
    }

    #[test]
    fn parse_fps_zero_rejected() {
        assert!(parse_fps("0").is_err());
    }

    #[test]
    fn parse_fps_invalid() {
        assert!(parse_fps("abc").is_err());
        assert!(parse_fps("-1").is_err());
    }

    #[test]
    fn parse_fps_empty() {
        assert_eq!(parse_fps("").unwrap(), 0);
    }

    #[test]
    fn cube_fps_zero_is_none() {
        let args = Args::parse_from(["test", "--cube-fps", "MAX", "--all-topics"]);
        assert_eq!(args.cube_fps(), None);
    }

    #[test]
    fn cube_fps_value() {
        let args = Args::parse_from(["test", "--cube-fps", "10", "--all-topics"]);
        assert_eq!(args.cube_fps(), Some(10));
    }
}
