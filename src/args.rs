use clap::{Parser, ValueEnum};
use serde_json::json;
use zenoh::config::{Config, WhatAmI};

#[derive(ValueEnum, Debug, Clone)]
pub enum Compression {
    None,
    Lz4,
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

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// duration for the recording in seconds
    #[arg(short, long, env)]
    pub duration: Option<u128>,

    /// topic detection timeout in seconds
    #[arg(short, long, default_value = "30")]
    pub timeout: u64,

    /// mcap compression
    #[arg(env, short = 'z', long, value_enum, default_value_t = Compression::None)]
    pub compression: Compression,

    /// topics
    #[arg(env, required = false, value_delimiter = ' ')]
    pub topics: Vec<String>,

    /// will look for all topics and start recording after 'timeout' parameter
    #[arg(short, long)]
    pub all_topics: bool,

    /// Limit the frame rate of the cube topic, otherwise record at the native
    /// rate
    #[arg(long, env)]
    pub cube_fps: Option<f64>,

    /// zenoh connection mode
    #[arg(long, env, default_value = "peer")]
    pub mode: WhatAmI,

    /// connect to zenoh endpoints
    #[arg(long, env)]
    pub connect: Vec<String>,

    /// listen to zenoh endpoints
    #[arg(long, env)]
    pub listen: Vec<String>,

    /// disable zenoh multicast scouting
    #[arg(long, env)]
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
    }
}
