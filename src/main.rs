extern crate hostname;
use anyhow::Result;
use bus::{Bus, BusReader};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use futures::Future;
use log::{debug, error, info, warn};
use mcap::{records::MessageHeader, Channel, Schema, WriteOptions, Writer};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fs,
    io::BufWriter,
    path::Path,
    process::exit,
    str::FromStr,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::{timeout, Duration};
use zenoh::{
    buffers::SplitBuffer,
    config::{Config, WhatAmI},
    prelude::{r#async::AsyncResolve, sync::SyncResolve},
    sample::Sample,
    subscriber::Subscriber,
    Session,
};

pub const FOXGLOVE_MSGS_COMPRESSED_VIDEO: &[u8] =
    include_bytes!("schema/foxglove_msgs/msg/CompressedVideo.msg");
pub const FOXGLOVE_MSGS_COMPRESSED_IMAGE: &[u8] =
    include_bytes!("schema/foxglove_msgs/msg/CompressedImage.msg");
pub const POINTCLOUD_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/PointCloud2.msg");
pub const IMU_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/Imu.msg");
pub const GPS_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/Gps.msg");
pub const BOXES_MSGS: &[u8] = include_bytes!("schema/foxglove_msgs/msg/ImageAnnotation.msg");
pub const CAMERA_INFO_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/CameraInfo.msg");
pub const RAD_CUBE_INFO_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/RadCube.msg");
pub const CAR_INFO_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/Marker.msg");

const FOXGLOVE_MSGS_COMPRESSED_VIDEO_KEY: &str = "foxglove_msgs/msg/CompressedVideo";
const FOXGLOVE_MSGS_COMPRESSED_IMAGE_KEY: &str = "sensor_msgs/msg/CompressedImage";
const POINTCLOUD_MSGS_KEY: &str = "sensor_msgs/msg/PointCloud2";
const IMU_MSGS_KEY: &str = "sensor_msgs/msg/Imu";
const GPS_MSGS_KEY: &str = "sensor_msgs/msg/NavSatFix";
const BOXES_MSGS_KEY: &str = "foxglove_msgs/msg/ImageAnnotations";
const CAMERA_INFO_MSGS_KEY: &str = "sensor_msgs/msg/CameraInfo";
const RAD_CUBE_INFO_MSGS_KEY: &str = "sensor_msgs/msg/RadCube";
const CAR_INFO_MSGS_KEY: &str = "sensor_msgs/msg/Marker";

pub const NANO_SEC: u128 = 1_000_000_000;

#[derive(ValueEnum, Debug, Clone)]
enum Compression {
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
struct Args {
    /// zenoh connection mode
    #[arg(short, long, default_value = "client")]
    mode: String,

    /// connect to endpoint
    #[arg(short, long, default_value = "tcp/127.0.0.1:7447")]
    connect: Vec<String>,

    /// listen to endpoint
    #[arg(short, long)]
    listen: Vec<String>,

    /// duration for the recording in seconds
    #[arg(short, long, env)]
    duration: Option<u128>,

    /// topic detection timeout in seconds
    #[arg(short, long, default_value = "30")]
    timeout: u64,

    /// mcap compression
    #[arg(short = 'z', long, value_enum, default_value_t = Compression::None)]
    compression: Compression,

    /// topics
    #[arg(env, required = false, value_delimiter = ' ')]
    topics: Vec<String>,

    /// will look for all topics and start recording after 'timeout' parameter
    #[arg(short, long)]
    all_topics: bool,
}

async fn run_and_log_err(name: &str, future: impl Future<Output = Result<(), Box<dyn Error>>>) {
    match future.await {
        Ok(_) => {
            log::debug!("{name} has finished running");
        }
        Err(e) => {
            log::error!("{name} exited with error: {e}");
            log::logger().flush();
        }
    }
}

async fn write_to_file(
    mut out: Writer<'_, BufWriter<fs::File>>,
    rx: Receiver<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn Error>> {
    loop {
        match rx.recv() {
            Ok((header, data)) => match out.write_to_known_channel(&header, &data) {
                Ok(_) => (),
                Err(e) => error!("Error writing to channel: {}", e),
            },
            Err(_) => {
                out.finish()?;
                return Ok(());
            }
        };
    }
}

fn stream(
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
    subscriber_topic: Subscriber<'_, flume::Receiver<Sample>>,
    topic: String,
    mut exit_signal: BusReader<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame_number = 0;
    loop {
        match exit_signal.try_recv() {
            Ok(_) => {
                debug!("Program stopped finishing writing MCAP.....");
                break;
            }
            Err(e) => match e {
                TryRecvError::Empty => {}
                TryRecvError::Disconnected => {
                    debug!("Program stopped finishing writing MCAP.....");
                    break;
                }
            },
        }
        match subscriber_topic.recv_timeout(Duration::from_secs(10)) {
            Ok(sample) => {
                let data = sample.value.payload.contiguous().to_vec();
                let current_time = SystemTime::now();
                let duration = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let unix_time_seconds = duration.as_nanos();

                let _ = tx.send((
                    MessageHeader {
                        channel_id,
                        sequence: frame_number,
                        log_time: unix_time_seconds as u64,
                        publish_time: unix_time_seconds as u64,
                    },
                    data,
                ));
                frame_number += 1;

                if let Some(duration) = args.duration {
                    if (unix_time_seconds - start_time) / NANO_SEC >= duration {
                        break;
                    }
                }
            }
            Err(_) => {
                warn!("Lost topic: {}", topic);
                continue;
            }
        }
    }

    Ok(())
}

fn get_channel<'a>(
    message_encoding: String,
    message_topic: String,
    image_schema_data: Vec<u8>,
) -> Channel<'a> {
    let image_schema = Schema {
        name: message_encoding.to_string(),
        encoding: String::from("ros2msg"),
        data: Cow::Owned(image_schema_data),
    };
    let image_schema_arc = Arc::new(image_schema);
    let modified_topic = message_topic.replace("rt", "");
    debug!("{:?}", modified_topic);
    let image_channel: Channel<'a> = Channel {
        topic: modified_topic.to_string(),
        schema: Some(Arc::clone(&image_schema_arc)),
        message_encoding: String::from("cdr"),
        metadata: BTreeMap::default(),
    };
    image_channel
}

fn create_hash_map() -> HashMap<&'static str, &'static [u8]> {
    let mut byte_arrays: HashMap<&str, &[u8]> = HashMap::new();
    byte_arrays.insert(
        FOXGLOVE_MSGS_COMPRESSED_VIDEO_KEY,
        FOXGLOVE_MSGS_COMPRESSED_VIDEO,
    );
    byte_arrays.insert(
        FOXGLOVE_MSGS_COMPRESSED_IMAGE_KEY,
        FOXGLOVE_MSGS_COMPRESSED_IMAGE,
    );
    byte_arrays.insert(POINTCLOUD_MSGS_KEY, POINTCLOUD_MSGS);
    byte_arrays.insert(IMU_MSGS_KEY, IMU_MSGS);
    byte_arrays.insert(GPS_MSGS_KEY, GPS_MSGS);
    byte_arrays.insert(BOXES_MSGS_KEY, BOXES_MSGS);
    byte_arrays.insert(CAMERA_INFO_MSGS_KEY, CAMERA_INFO_MSGS);
    byte_arrays.insert(RAD_CUBE_INFO_MSGS_KEY, RAD_CUBE_INFO_MSGS);
    byte_arrays.insert(CAR_INFO_MSGS_KEY, CAR_INFO_MSGS);
    byte_arrays
}

fn get_storage() -> Result<String, std::io::Error> {
    match std::env::var("STORAGE") {
        // If the environment variable is not set, return only the filename.
        Err(_) => Ok("".to_owned()),
        Ok(storage) => {
            debug!("STORAGE={}", storage);
            match fs::create_dir_all(&storage) {
                Ok(_) => Ok(storage),
                Err(e) => {
                    error!("Failed to create STORAGE {}: {}", storage, e);
                    Err(e)
                }
            }
        }
    }
}

fn get_filename() -> String {
    let current_time = Utc::now();
    let formatted_time = current_time.format("%Y_%m_%d_%H_%M_%S").to_string();
    match hostname::get() {
        // If hostname fails for whatever reason then use maivin-recorder as the prefix.
        Ok(hostname) => match hostname.to_str() {
            Some(hostname) => format!("{}_{}.mcap", hostname, formatted_time),
            None => format!("maivin-recorder_{}.mcap", formatted_time),
        },
        Err(e) => {
            warn!("Failed to get hostname: {}", e);
            format!("maivin-recorder_{}.mcap", formatted_time)
        }
    }
}

async fn get_all_topics(args: &Args, session: &Session) -> Vec<String> {
    let wildcard_topic = "*/**";
    let subscriber = match timeout(
        Duration::from_secs(args.timeout),
        session.declare_subscriber(wildcard_topic).res_async(),
    )
    .await
    {
        Ok(result) => match result {
            Ok(subscriber) => subscriber,
            Err(err) => {
                panic!("Error declaring subscriber: {:?}", err);
            }
        },
        Err(_) => {
            panic!("Timeout occurred while waiting for subscriber declaration.");
        }
    };

    let mut topic_names = Vec::new();
    let mut unique_topics = HashSet::new();

    let start_time = Instant::now();
    while start_time.elapsed() < Duration::from_secs(args.timeout) {
        match subscriber.recv_timeout(Duration::from_secs(args.timeout)) {
            Ok(topic) => {
                if unique_topics.insert(topic.key_expr.to_string()) {
                    topic_names.push(topic.key_expr.to_string().clone());
                    info!(
                        "Found {:?} will start recording in {:?} seconds",
                        topic.key_expr.to_string().clone(),
                        (Duration::from_secs(args.timeout) - start_time.elapsed()).as_secs() as i64
                    );
                }
            }
            Err(err) => {
                error!("Error receiving message: {:?}", err);
                break;
            }
        }
    }
    drop(subscriber);
    topic_names
}
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();
    let byte_arrays = create_hash_map();
    let mut config = Config::default();

    let mode = WhatAmI::from_str(&args.mode).unwrap();
    config.set_mode(Some(mode)).unwrap();
    config.connect.endpoints = args.connect.iter().map(|v| v.parse().unwrap()).collect();
    config.listen.endpoints = args.listen.iter().map(|v| v.parse().unwrap()).collect();
    let _ = config.scouting.multicast.set_enabled(Some(false));

    let session = zenoh::open(config).res_async().await.unwrap();

    let mut bus = Bus::new(1);

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.topics.is_empty() && !args.all_topics {
        info!("No topics are specified and --all-topics flag is FALSE exiting");
        exit(-1)
    }

    if args.all_topics {
        let topic_list: Vec<String> = get_all_topics(&args, &session).await;
        args.topics = topic_list
    }

    for topic in &mut args.topics {
        let mut fixed_topic = topic.to_string();
        if !topic.starts_with("rt/") && !topic.starts_with('/') {
            fixed_topic = format!("rt/{}", topic);
        } else if topic.starts_with('/') {
            fixed_topic = format!("rt{}", topic);
        }
        *topic = fixed_topic;
    }

    let mut cloned_msg_type_vec = Vec::new();
    let (tx, rx) = mpsc::channel();

    for topic in &args.topics {
        let subscriber = match timeout(
            Duration::from_secs(args.timeout),
            session.declare_subscriber(topic).res_async(),
        )
        .await
        {
            Ok(result) => match result {
                Ok(subscriber) => subscriber,
                Err(err) => {
                    panic!("Error declaring subscriber: {:?}", err);
                }
            },
            Err(_) => {
                continue;
            }
        };

        let msg_type: Option<String> =
            match subscriber.recv_timeout(Duration::from_secs(args.timeout)) {
                Ok(sample) => match String::from_str(sample.encoding.suffix()) {
                    Ok(v) => {
                        debug!("Received message type: {}", v);
                        Some(v)
                    }
                    Err(_) => None,
                },
                Err(_) => None,
            };
        cloned_msg_type_vec.push(msg_type);
    }

    for (idx, _item) in cloned_msg_type_vec
        .iter()
        .enumerate()
        .take(args.topics.len())
    {
        let topic = &args.topics[idx];
        let msg_type = &cloned_msg_type_vec[idx];
        match msg_type {
            Some(s) => {
                if !s.is_empty() {
                    info!("Successful subscribed to {} and started recording", topic);
                }
            }
            None => {
                warn!(
                    "Timeout occurred while waiting for a message on topic {}",
                    topic
                );
                continue;
            }
        }
    }

    debug!("Subscribed to {:?} ", cloned_msg_type_vec);
    let should_exit = cloned_msg_type_vec
        .iter()
        .all(|x| matches!(x, Some(s) if s.is_empty()) || x.is_none());

    if should_exit {
        info!("Found no suitable schema for any of the topics exiting");
        exit(-1);
    }

    assert!(args.topics.len() == cloned_msg_type_vec.len());
    let mut futures = Vec::new();

    let filename = Path::new(&get_storage()?).join(get_filename());
    info!("Recording to {}", filename.display());
    let bufwriter = BufWriter::new(fs::File::create(&filename)?);
    let mut out = WriteOptions::new()
        .compression(args.compression.clone().into())
        .create(bufwriter)?;

    let current_time = SystemTime::now();
    let duration = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let start_time: u128 = duration.as_nanos();
    let session = session.into_arc();
    for (idx, _item) in cloned_msg_type_vec
        .iter()
        .enumerate()
        .take(args.topics.len())
    {
        let topic = args.topics[idx].clone();
        let msg_type = &cloned_msg_type_vec[idx];
        match msg_type {
            Some(mtype) => {
                if let Some(compressed_video) = byte_arrays.get(mtype.as_str()) {
                    let channel =
                        get_channel(mtype.clone(), topic.clone(), compressed_video.to_vec());
                    let channel_id = out.add_channel(&channel)?;
                    let args = args.clone();
                    let tx = tx.clone();
                    let session = session.clone();
                    let rx = bus.add_rx();
                    futures.push(std::thread::spawn(move || {
                        let subscriber = session
                            .declare_subscriber(topic.clone())
                            .res_sync()
                            .unwrap();

                        stream(channel_id, start_time, &args, tx, subscriber, topic, rx).unwrap()
                    }));
                }
            }
            None => continue,
        }
    }
    ctrlc::set_handler(move || {
        bus.broadcast(1);
    })
    .expect("Error setting Ctrl-C handler");
    drop(tx);
    let write_future = run_and_log_err("Writer", write_to_file(out, rx));

    let (_, _) = async_scoped::AsyncStdScope::scope_and_block(|s| {
        s.spawn(write_future);
    });
    for handle in futures {
        handle.join().unwrap();
    }
    info!("Saved MCAP to {}", filename.display());
    Ok(())
}
