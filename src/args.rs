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
/// Empty-string environment variables (e.g. `DURATION=""`, `TOPICS=""`) are
/// treated as unset so that systemd EnvironmentFile defaults work without
/// commenting out optional parameters. When no topics are specified, all
/// active Zenoh topics are discovered and recorded.
///
/// # Example
///
/// ```bash
/// # Record all active topics for 60 seconds
/// edgefirst-recorder --duration 60 --compression lz4
///
/// # Via environment variables
/// export DURATION=60
/// export COMPRESSION=lz4
/// edgefirst-recorder
/// ```
#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Duration for the recording in seconds (empty = unlimited)
    #[arg(short, long, env = "DURATION", value_parser = parse_duration)]
    pub duration: Option<u64>,

    /// Topic detection timeout in seconds
    #[arg(short, long, default_value = "5")]
    pub timeout: u64,

    /// MCAP compression algorithm
    #[arg(env = "COMPRESSION", short = 'z', long, value_enum, default_value_t = Compression::None)]
    pub compression: Compression,

    /// Topics to record (space-delimited). When empty, all active topics are recorded.
    #[arg(env = "TOPICS", required = false, value_delimiter = ' ')]
    pub topics: Vec<String>,

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

    /// Strip the hostname prefix from MCAP channel topics
    #[arg(long, env = "STRIP_HOSTNAME")]
    pub strip_hostname: bool,

    /// Disable Zenoh multicast scouting
    #[arg(
        long,
        env = "NO_MULTICAST_SCOUTING",
        default_value = "false",
        default_missing_value = "true",
        num_args(0..=1),
        value_parser = parse_bool
    )]
    pub no_multicast_scouting: bool,
}

/// System hostname used as the Zenoh session namespace.
///
/// Empty or `/`-containing hostnames would create unintended sub-keys, so we
/// fall back to `"localhost"` and warn. Two devices both falling back would
/// silently share a namespace; that is a deployment defect.
fn zenoh_namespace() -> String {
    let raw = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_default();
    zenoh_namespace_from(&raw)
}

/// Resolve a Zenoh session namespace from a raw hostname string.
fn zenoh_namespace_from(raw: &str) -> String {
    if raw.is_empty() || raw.contains('/') {
        log::warn!(
            "system hostname `{raw}` is empty or contains '/' — falling back to \"localhost\""
        );
        "localhost".into()
    } else {
        raw.to_owned()
    }
}

impl From<Args> for Config {
    fn from(args: Args) -> Self {
        let mut config = Config::default();

        // Session namespace = hostname: application keys are bare
        // (`camera/h264`) and the wire form is `{hostname}/camera/h264`.
        config
            .insert_json5("namespace", &json!(zenoh_namespace()).to_string())
            .unwrap();

        config
            .insert_json5("mode", &json!(args.mode).to_string())
            .unwrap();

        let connect: Vec<_> = args.connect.into_iter().filter(|s| !s.is_empty()).collect();
        if !connect.is_empty() {
            config
                .insert_json5("connect/endpoints", &json!(connect).to_string())
                .unwrap();
        }

        let listen: Vec<_> = args.listen.into_iter().filter(|s| !s.is_empty()).collect();
        if !listen.is_empty() {
            config
                .insert_json5("listen/endpoints", &json!(listen).to_string())
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

fn parse_duration(s: &str) -> Result<u64, String> {
    if s.is_empty() {
        return Ok(0);
    }

    let secs: u64 = s
        .parse()
        .map_err(|_| "expected a positive integer".to_string())?;

    if secs == 0 {
        Err("duration must be greater than 0".to_string())
    } else {
        Ok(secs)
    }
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_ascii_lowercase().as_str() {
        "" | "false" | "0" | "no" => Ok(false),
        "true" | "1" | "yes" => Ok(true),
        other => Err(format!("invalid boolean value '{other}'")),
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
    pub fn duration(&self) -> Option<u64> {
        match self.duration {
            Some(0) => None,
            other => other,
        }
    }

    pub fn cube_fps(&self) -> Option<u32> {
        match self.cube_fps {
            Some(0) => None,
            other => other,
        }
    }

    /// Topics with empty strings removed (from `TOPICS=""` env overrides).
    pub fn topics(&self) -> Vec<String> {
        self.topics
            .iter()
            .filter(|t| !t.is_empty())
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_empty() {
        assert_eq!(parse_duration("").unwrap(), 0);
    }

    #[test]
    fn parse_duration_valid() {
        assert_eq!(parse_duration("60").unwrap(), 60);
        assert_eq!(parse_duration("1").unwrap(), 1);
    }

    #[test]
    fn parse_duration_zero_rejected() {
        assert!(parse_duration("0").is_err());
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
    }

    #[test]
    fn duration_zero_is_none() {
        let args = Args::parse_from(["test", "--duration", ""]);
        assert_eq!(args.duration(), None);
    }

    #[test]
    fn duration_value() {
        let args = Args::parse_from(["test", "--duration", "60"]);
        assert_eq!(args.duration(), Some(60));
    }

    #[test]
    fn topics_empty_from_default() {
        let args = Args::parse_from(["test"]);
        assert!(args.topics().is_empty());
    }

    #[test]
    fn topics_positional() {
        let args = Args::parse_from(["test", "camera/h264", "radar/cube"]);
        assert_eq!(
            args.topics(),
            vec!["camera/h264".to_string(), "radar/cube".to_string()]
        );
    }

    #[test]
    fn zenoh_config_sets_namespace() {
        let args = Args::parse_from(["test"]);
        let cfg = Config::from(args);
        let ns: String = serde_json::from_str(&cfg.to_string())
            .ok()
            .and_then(|v: serde_json::Value| {
                v.pointer("/namespace")
                    .and_then(|n| n.as_str().map(String::from))
            })
            .expect("namespace should be set in config");
        assert!(!ns.is_empty(), "namespace should be non-empty");
        assert!(!ns.contains('/'), "namespace must not contain '/'");
    }

    #[test]
    fn zenoh_namespace_from_valid() {
        assert_eq!(zenoh_namespace_from("adis-uav1"), "adis-uav1");
    }

    #[test]
    fn zenoh_namespace_from_empty_falls_back() {
        assert_eq!(zenoh_namespace_from(""), "localhost");
    }

    #[test]
    fn zenoh_namespace_from_slash_falls_back() {
        assert_eq!(zenoh_namespace_from("bad/name"), "localhost");
    }

    #[test]
    fn compression_into_mcap() {
        assert!(Option::<mcap::Compression>::from(Compression::None).is_none());
        assert!(matches!(
            Option::<mcap::Compression>::from(Compression::Lz4),
            Some(mcap::Compression::Lz4)
        ));
        assert!(matches!(
            Option::<mcap::Compression>::from(Compression::Zstd),
            Some(mcap::Compression::Zstd)
        ));
    }

    #[test]
    fn zenoh_config_connect_listen_and_scouting() {
        let args = Args {
            duration: None,
            timeout: 5,
            compression: Compression::None,
            topics: vec![],
            cube_fps: None,
            mode: WhatAmI::Peer,
            connect: vec!["".to_string(), "tcp/127.0.0.1:7447".to_string()],
            listen: vec!["".to_string(), "tcp/0.0.0.0:7447".to_string()],
            strip_hostname: false,
            no_multicast_scouting: true,
        };
        let cfg = Config::from(args);
        let v: serde_json::Value =
            serde_json::from_str(&cfg.to_string()).expect("config serializes to JSON");
        let connect = v
            .pointer("/connect/endpoints")
            .and_then(|n| n.as_array())
            .expect("connect endpoints set");
        assert_eq!(connect.len(), 1);
        let listen = v
            .pointer("/listen/endpoints")
            .and_then(|n| n.as_array())
            .expect("listen endpoints set");
        assert_eq!(listen.len(), 1);
        assert_eq!(
            v.pointer("/scouting/multicast/enabled")
                .and_then(|n| n.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn topics_empty_string_filtered() {
        let args = Args {
            duration: None,
            timeout: 5,
            compression: Compression::None,
            topics: vec!["".to_string()],
            cube_fps: None,
            mode: WhatAmI::Peer,
            connect: vec![],
            listen: vec![],
            strip_hostname: false,
            no_multicast_scouting: false,
        };
        assert!(args.topics().is_empty());
    }

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
        let args = Args::parse_from(["test", "--cube-fps", "MAX"]);
        assert_eq!(args.cube_fps(), None);
    }

    #[test]
    fn cube_fps_value() {
        let args = Args::parse_from(["test", "--cube-fps", "10"]);
        assert_eq!(args.cube_fps(), Some(10));
    }

    #[test]
    fn parse_bool_empty() {
        assert!(!parse_bool("").unwrap());
    }

    #[test]
    fn parse_bool_truthy() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(parse_bool("YES").unwrap());
    }

    #[test]
    fn parse_bool_falsy() {
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
    }

    #[test]
    fn parse_bool_invalid() {
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn no_multicast_scouting_as_flag() {
        let args = Args::parse_from(["test", "--no-multicast-scouting"]);
        assert!(args.no_multicast_scouting);
    }

    #[test]
    fn no_multicast_scouting_explicit_false() {
        let args = Args::parse_from(["test", "--no-multicast-scouting", "false"]);
        assert!(!args.no_multicast_scouting);
    }

    #[test]
    fn connect_empty_filtered() {
        let connect: Vec<_> = vec!["".to_string(), "tcp/localhost:7447".to_string()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();
        assert_eq!(connect, vec!["tcp/localhost:7447".to_string()]);
    }
}
