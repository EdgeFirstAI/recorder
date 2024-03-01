extern crate hostname;
use anyhow::Result;
use chrono::Utc;
use clap::Parser;
use futures::Future;
use log::{error, info, warn};
use mcap::{records::MessageHeader, Channel, Schema, Writer};
use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    error::Error,
    fs,
    io::BufWriter,
    process::exit,
    str::FromStr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    signal::unix::{signal, SignalKind},
    time::{timeout, Duration},
};
use zenoh::{
    buffers::SplitBuffer, prelude::r#async::AsyncResolve, sample::Sample, subscriber::Subscriber,
};

pub const FOXGLOVE_MSGS_COMPRESSED_VIDEO: &[u8] =
    include_bytes!("schema/foxglove_msgs/msg/CompressedVideo.msg");

pub const FOXGLOVE_MSGS_COMPRESSED_IMAGE: &[u8] =
    include_bytes!("schema/foxglove_msgs/msg/CompressedImage.msg");

pub const POINTCLOUD_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/PointCloud2.msg");

pub const IMU_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/Imu.msg");

pub const GPS_MSGS: &[u8] = include_bytes!("schema/sensor_msgs/msg/Gps.msg");

pub const BOXES_MSGS: &[u8] = include_bytes!("schema/foxglove_msgs/msg/ImageAnnotation.msg");

const FOXGLOVE_MSGS_COMPRESSED_VIDEO_KEY: &str = "foxglove_msgs/msg/CompressedVideo";
const FOXGLOVE_MSGS_COMPRESSED_IMAGE_KEY: &str = "sensor_msgs/msg/CompressedImage";
const POINTCLOUD_MSGS_KEY: &str = "sensor_msgs/msg/PointCloud2";
const IMU_MSGS_KEY: &str = "sensor_msgs/msg/Imu";
const GPS_MSGS_KEY: &str = "sensor_msgs/msg/NavSatFix";
const BOXES_MSGS_KEY: &str = "foxglove_msgs/msg/ImageAnnotations";

pub const NANO_SEC: u128 = 1000000000;

#[derive(Parser, Debug)]
struct Args {
    /// zenoh connection mode
    #[arg(short, long, default_value = "peer")]
    mode: String,

    /// connect to endpoint
    #[arg(short, long)]
    endpoint: Vec<String>,

    /// duration for the recording in seconds
    #[arg(short, long)]
    duration: Option<u128>,

    /// topic detection timeout in seconds
    #[arg(short, long, default_value = "10")]
    timeout: u64,

    /// topics
    topics: Vec<String>,
}

async fn run_and_log_err(name: &str, future: impl Future<Output = Result<(), Box<dyn Error>>>) {
    match future.await {
        Ok(_) => {
            log::info!("{name} has finished running");
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
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Error writing to channel: {:?}", e);
                }
            },
            Err(_) => {
                out.finish()?;
                return Ok(());
            }
        };
    }
}

async fn handle_ctrl_c(running: Arc<AtomicBool>) {
    let mut stream = signal(SignalKind::interrupt()).unwrap();
    stream.recv().await;
    running.store(false, Ordering::SeqCst);
}

async fn stream(
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
    subscriber_topic: Subscriber<'_, flume::Receiver<Sample>>,
    topic: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));
    let running_clone = running.clone();
    tokio::spawn(handle_ctrl_c(running_clone));

    let mut frame_number = 0;

    loop {
        if !running.load(Ordering::SeqCst) {
            info!("Program stopped finishing writing MCAP.....");
            break;
        }

        let timeout_duration = Duration::from_secs(args.timeout);
        match timeout(timeout_duration, subscriber_topic.recv_async()).await {
            Ok(Ok(sample)) => {
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

                match args.duration {
                    Some(duration) => {
                        if (unix_time_seconds - start_time) / NANO_SEC >= duration {
                            break;
                        }
                    }
                    None => {}
                }
            }
            Ok(Err(_)) => {
                info!("Timeout occurred while waiting for sample from camera_subscriber");
                break;
            }
            Err(_) => {
                warn!("Topic {:?} not found stopped looking, TIMING OUT", topic);
                break;
            }
        }
    }

    Ok(())
}

fn get_channel<'a>(
    message_encoding: &'a str,
    message_topic: &'a str,
    image_schema_data: Vec<u8>,
) -> Channel<'a> {
    let image_schema = Schema {
        name: message_encoding.to_string(),
        encoding: String::from("ros2msg"),
        data: Cow::Owned(image_schema_data),
    };
    let image_schema_arc = Arc::new(image_schema);
    let image_channel: Channel<'a> = Channel {
        topic: message_topic.to_string(),
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
        include_bytes!("schema/foxglove_msgs/msg/CompressedVideo.msg"),
    );
    byte_arrays.insert(
        FOXGLOVE_MSGS_COMPRESSED_IMAGE_KEY,
        include_bytes!("schema/foxglove_msgs/msg/CompressedImage.msg"),
    );
    byte_arrays.insert(
        POINTCLOUD_MSGS_KEY,
        include_bytes!("schema/sensor_msgs/msg/PointCloud2.msg"),
    );
    byte_arrays.insert(
        IMU_MSGS_KEY,
        include_bytes!("schema/sensor_msgs/msg/Imu.msg"),
    );
    byte_arrays.insert(
        GPS_MSGS_KEY,
        include_bytes!("schema/sensor_msgs/msg/Gps.msg"),
    );
    byte_arrays.insert(
        BOXES_MSGS_KEY,
        include_bytes!("schema/foxglove_msgs/msg/ImageAnnotation.msg"),
    );
    return byte_arrays;
}

fn get_file_name() -> Result<String, Box<dyn std::error::Error>> {
    let current_time = Utc::now();
    let formatted_time = current_time.format("%Y_%m_%d_%H_%M_%S").to_string();
    let mut result: String;
    match hostname::get() {
        Ok(hostname) => {
            if let Some(name) = hostname.to_str() {
                result = name.to_owned() + "_" + &formatted_time + ".mcap";
            } else {
                result = "maivin_mcap_".to_string() + &formatted_time + ".mcap";
                warn!("Hostname is not valid UTF-8, using {:?}", result);
            }
        }
        Err(_e) => {
            result = "maivin_mcap_".to_string() + &formatted_time + ".mcap";
            info!("Failed to get hostname, using {:?}", result);
        }
    }

    match std::env::var("STORAGE") {
        Ok(value) => {
            match fs::create_dir_all(&value) {
                Ok(()) => {
                    info!("Directory {} created successfully.", value);
                }
                Err(_) => {
                    error!("Failed to create directory {}", value);
                    exit(-1);
                }
            }

            result = value.to_string() + "/" + &result;
            info!(
                "STORAGE environment variable is set storing MCAP in: {}",
                value
            );
            Ok(result)
        }
        Err(_) => {
            let path = std::env::current_dir()?;
            info!(
                "STORAGE environment variable is not set, storing the MCAP in: {:?}",
                path
            );
            Ok(result)
        }
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut zenoh_config = zenoh::config::Config::default();

    let mode = zenoh::scouting::WhatAmI::from_str(&args.mode)?;
    zenoh_config.set_mode(Some(mode)).unwrap();
    zenoh_config.connect.endpoints = args.endpoint.iter().map(|v| v.parse().unwrap()).collect();

    let session = zenoh::open(zenoh_config).res().await.unwrap();
    let byte_arrays = create_hash_map();

    let mut cloned_msg_type_vec = Vec::new();
    let mut out: Writer<'_, BufWriter<fs::File>>;
    let file_name;
    match get_file_name() {
        Ok(r) => {
            file_name = r.clone();
            out = Writer::new(BufWriter::new(fs::File::create(r)?))?;
        }
        Err(_) => {
            error!("File name invalid");
            exit(-1);
        }
    };

    let (tx, rx) = mpsc::channel();

    let current_time = SystemTime::now();
    let duration = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let start_time: u128 = duration.as_nanos();

    for topic in &args.topics {
        let subscriber = session.declare_subscriber(topic).res().await.unwrap();
        let msg_type: Option<String> = match subscriber.recv_timeout(Duration::from_secs(2)) {
            Ok(sample) => match String::try_from(sample.encoding.suffix()) {
                Ok(v) => Some(v),
                Err(_) => None,
            },
            Err(_) => {
                warn!("Did not find schema");
                None
            }
        };
        cloned_msg_type_vec.push(msg_type);
    }
    assert!(args.topics.len() == cloned_msg_type_vec.len());
    let mut futures = Vec::new();
    for idx in 0..args.topics.len() {
        let topic = &args.topics[idx];
        let msg_type = &cloned_msg_type_vec[idx];
        match msg_type {
            Some(mtype) => {
                if let Some(compressed_video) = byte_arrays.get(mtype.as_str()) {
                    let channel = get_channel(mtype, topic, compressed_video.to_vec());
                    let channel_id = out.add_channel(&channel)?;

                    let subscriber = session.declare_subscriber(topic).res().await.unwrap();

                    futures.push(run_and_log_err(
                        topic,
                        stream(
                            channel_id,
                            start_time,
                            &args,
                            tx.clone(),
                            subscriber,
                            &topic,
                        ),
                    ));
                }
            }
            None => continue,
        }
    }

    drop(tx);
    let write_future = run_and_log_err("Writer", write_to_file(out, rx));

    let (_, _) = async_scoped::AsyncStdScope::scope_and_block(|s| {
        for future in futures {
            s.spawn(future);
        }
        s.spawn(write_future);
    });
    info!("Saved MCAP at {:?}", file_name);
    Ok(())
}
