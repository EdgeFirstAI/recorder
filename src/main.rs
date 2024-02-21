extern crate hostname;

mod schema;
use schema::*;

use anyhow::Result;
use cdr::{CdrLe, Infinite};
use chrono::Utc;
use clap::Parser;
use futures::Future;
use mcap::{records::MessageHeader, Channel, Schema, Writer};
use std::borrow::Cow;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{collections::BTreeMap, error::Error, fs, io::BufWriter, sync::Arc};
use tokio::time::{timeout, Duration};
use zenoh::{buffers::reader::HasReader, prelude::r#async::AsyncResolve, Session};
use zenoh_ros_type::foxglove_msgs::FoxgloveCompressedVideo;
use zenoh_ros_type::sensor_msgs::{CompressedImage, NavSatFix, PointCloud2, IMU};

pub const NANO_SEC: u128 = 1000000000;

#[derive(Parser, Debug)]
struct Args {
    /// zenoh connection mode
    #[arg(short, long, default_value = "peer")]
    mode: String,

    /// connect to endpoint
    #[arg(short, long)]
    endpoint: Vec<String>,

    /// camera topic
    #[arg(short, long, default_value = "rt/camera/compressed")]
    camera_topic: String,

    /// radar topic
    #[arg(short, long, default_value = "rt/radar/targets0")]
    radar_topic: String,

    /// imu topic
    #[arg(short, long, default_value = "rt/imu")]
    imu_topic: String,

    /// gps topic
    #[arg(short, long, default_value = "rt/gps")]
    gps_topic: String,

    /// duration for the recording in seconds
    #[arg(short, long, default_value = "30")]
    duration: u128,

    /// topic detection timeout in seconds
    #[arg(short, long, default_value = "10")]
    timeout: u64,
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

async fn stream_camera(
    session: &Session,
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let camera_subscriber = session
        .declare_subscriber(&args.camera_topic)
        .res()
        .await
        .unwrap();
    let mut frame_number = 0;

    loop {
        let timeout_duration = Duration::from_secs(args.timeout);

        match timeout(timeout_duration, camera_subscriber.recv_async()).await {
            Ok(Ok(sample)) => {
                let compressed_image = cdr::deserialize_from::<_, FoxgloveCompressedVideo, _>(
                    sample.value.payload.reader(),
                    Infinite,
                )?;

                let current_time = SystemTime::now();
                let duration = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let unix_time_seconds = duration.as_nanos();
                let serialized_data_result =
                    cdr::serialize::<_, _, CdrLe>(&compressed_image, Infinite);

                match serialized_data_result {
                    Ok(serialized_data) => {
                        let _ = tx.send((
                            MessageHeader {
                                channel_id,
                                sequence: frame_number,
                                log_time: unix_time_seconds as u64,
                                publish_time: unix_time_seconds as u64,
                            },
                            serialized_data,
                        ));
                        frame_number += 1;

                        if (unix_time_seconds - start_time) / NANO_SEC >= args.duration {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Serialization error: {:?}", err);
                    }
                }
            }
            Ok(Err(_)) => {
                println!("Timeout occurred while waiting for sample from camera_subscriber");
                break;
            }
            Err(_) => {
                println!("Camera topic not found stopped looking, TIMING OUT");
                break;
            }
        }
    }
    return Ok(());
}

async fn stream_imu(
    session: &Session,
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let imu_subscriber = session
        .declare_subscriber(&args.imu_topic)
        .res()
        .await
        .unwrap();
    let mut frame_number = 0;

    loop {
        let timeout_duration = Duration::from_secs(args.timeout);

        match timeout(timeout_duration, imu_subscriber.recv_async()).await {
            Ok(Ok(sample)) => {
                let compressed_image =
                    cdr::deserialize_from::<_, IMU, _>(sample.value.payload.reader(), Infinite)?;

                let current_time = SystemTime::now();

                let duration = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                let unix_time_seconds = duration.as_nanos();
                let serialized_data_result =
                    cdr::serialize::<_, _, CdrLe>(&compressed_image, Infinite);

                match serialized_data_result {
                    Ok(serialized_data) => {
                        let _ = tx.send((
                            MessageHeader {
                                channel_id,
                                sequence: frame_number,
                                log_time: unix_time_seconds as u64,
                                publish_time: unix_time_seconds as u64,
                            },
                            serialized_data,
                        ));
                        frame_number += 1;

                        if (unix_time_seconds - start_time) / NANO_SEC >= args.duration {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Serialization error: {:?}", err);
                    }
                }
            }
            Ok(Err(_)) => {
                println!("Timeout occurred while waiting for sample from camera_subscriber");
                break;
            }
            Err(_) => {
                println!("Imu topic not found stopped looking, TIMING OUT");
                break;
            }
        }
    }
    return Ok(());
}

async fn stream_gps(
    session: &Session,
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let gps_subscriber = session
        .declare_subscriber(&args.gps_topic)
        .res()
        .await
        .unwrap();
    let mut frame_number = 0;

    loop {
        let timeout_duration = Duration::from_secs(args.timeout);

        match timeout(timeout_duration, gps_subscriber.recv_async()).await {
            Ok(Ok(sample)) => {
                let compressed_image = cdr::deserialize_from::<_, NavSatFix, _>(
                    sample.value.payload.reader(),
                    Infinite,
                )?;

                let current_time = SystemTime::now();

                let duration = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                let unix_time_seconds = duration.as_nanos();
                let serialized_data_result =
                    cdr::serialize::<_, _, CdrLe>(&compressed_image, Infinite);

                match serialized_data_result {
                    Ok(serialized_data) => {
                        let _ = tx.send((
                            MessageHeader {
                                channel_id,
                                sequence: frame_number,
                                log_time: unix_time_seconds as u64,
                                publish_time: unix_time_seconds as u64,
                            },
                            serialized_data,
                        ));
                        frame_number += 1;
                        if (unix_time_seconds - start_time) / NANO_SEC >= args.duration {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Serialization error: {:?}", err);
                    }
                }
            }
            Ok(Err(_)) => {
                println!("Timeout occurred while waiting for sample from camera_subscriber");
                break;
            }
            Err(_) => {
                println!("Gps topic not found stopped looking, TIMING OUT");
                break;
            }
        }
    }
    return Ok(());
}

async fn stream_radar(
    session: &Session,
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn Error>> {
    let radar_subscriber = session
        .declare_subscriber(&args.radar_topic)
        .res()
        .await
        .unwrap();

    let mut frame_number = 0;

    loop {
        let timeout_duration = Duration::from_secs(args.timeout);

        match timeout(timeout_duration, radar_subscriber.recv_async()).await {
            Ok(Ok(sample)) => {
                let points = cdr::deserialize_from::<_, PointCloud2, _>(
                    sample.value.payload.reader(),
                    Infinite,
                )?;

                assert!(points.width as usize == points.data.len() / 16);

                let current_time = SystemTime::now();

                let duration = current_time
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");

                let unix_time_seconds = duration.as_nanos();

                let serialized_data_result = cdr::serialize::<_, _, CdrLe>(&points, Infinite);

                match serialized_data_result {
                    Ok(serialized_data) => {
                        let _ = tx.send((
                            MessageHeader {
                                channel_id,
                                sequence: frame_number,
                                log_time: unix_time_seconds as u64,
                                publish_time: unix_time_seconds as u64,
                            },
                            serialized_data,
                        ));
                        frame_number += 1;

                        if (unix_time_seconds - start_time) / NANO_SEC >= args.duration {
                            break;
                        }
                    }
                    Err(err) => {
                        eprintln!("Serialization error: {:?}", err);
                    }
                }
            }
            Ok(Err(_)) => {
                println!("Timeout occurred while waiting for sample from camera_subscriber");
                break;
            }
            Err(_) => {
                println!("Radar topic not found stopped looking, TIMING OUT");
                break;
            }
        }
    }
    return Ok(());
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

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut zenoh_config = zenoh::config::Config::default();

    let mode = zenoh::scouting::WhatAmI::from_str(&args.mode)?;
    zenoh_config.set_mode(Some(mode)).unwrap();
    zenoh_config.connect.endpoints = args.endpoint.iter().map(|v| v.parse().unwrap()).collect();

    let session = zenoh::open(zenoh_config).res().await.unwrap();

    let current_time = Utc::now();
    let formatted_time = current_time.format("%Y_%m_%d_%H_%M_%S").to_string();
    let result: String;
    match hostname::get() {
        Ok(hostname) => {
            if let Some(name) = hostname.to_str() {
                result = name.to_owned() + "_" + &formatted_time + ".mcap";
            } else {
                result = "maivin_mcap_".to_string() + &formatted_time + ".mcap";
                println!("Hostname is not valid UTF-8, using {:?}", result);
            }
        }
        Err(_e) => {
            result = "maivin_mcap_".to_string() + &formatted_time + ".mcap";
            println!("Failed to get hostname, using {:?}", result);
        }
    }

    let mut out: Writer<'_, BufWriter<fs::File>> =
        Writer::new(BufWriter::new(fs::File::create(result)?))?;

    let image_schema_data: Vec<u8> = VIDEO_SCHEMA_DATA.to_owned();
    let image_channel = get_channel(
        "foxglove_msgs/msg/CompressedVideo",
        &args.camera_topic,
        image_schema_data,
    );
    let image_channel_id = out.add_channel(&image_channel)?;

    let radar_schema_data: Vec<u8> = RADAR_SCHEMA_DATA.to_owned();
    let radar_channel = get_channel(
        "sensor_msgs/msg/PointCloud2",
        &args.radar_topic,
        radar_schema_data,
    );
    let radar_channel_id = out.add_channel(&radar_channel)?;

    let imu_schema_data: Vec<u8> = IMU_SCHEMA_DATA.to_owned();
    let imu_channel = get_channel("sensor_msgs/msg/Imu", &args.imu_topic, imu_schema_data);
    let imu_channel_id = out.add_channel(&imu_channel)?;

    let gps_schema_data: Vec<u8> = GPS_SCHEMA_DATA.to_owned();
    let gps_channel = get_channel(
        "sensor_msgs/msg/NavSatFix",
        &args.gps_topic,
        gps_schema_data,
    );
    let gps_channel_id = out.add_channel(&gps_channel)?;

    let (tx, rx) = mpsc::channel();

    let current_time = SystemTime::now();
    let duration = current_time
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    let start_time: u128 = duration.as_nanos();

    let cam_future = run_and_log_err(
        "Camera",
        stream_camera(&session, image_channel_id, start_time, &args, tx.clone()),
    );
    let rad_future = run_and_log_err(
        "Radar",
        stream_radar(&session, radar_channel_id, start_time, &args, tx.clone()),
    );
    let imu_future = run_and_log_err(
        "Imu",
        stream_imu(&session, imu_channel_id, start_time, &args, tx.clone()),
    );

    let gps_future = run_and_log_err(
        "Gps",
        stream_gps(&session, gps_channel_id, start_time, &args, tx.clone()),
    );
    drop(tx);
    let write_future = run_and_log_err("Writer", write_to_file(out, rx));
    let (_, _) = async_scoped::AsyncStdScope::scope_and_block(|s| {
        s.spawn(cam_future);
        s.spawn(rad_future);
        s.spawn(imu_future);
        s.spawn(gps_future);
        s.spawn(write_future);
    });

    Ok(())
}
