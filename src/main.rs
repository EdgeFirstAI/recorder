extern crate fs2;
extern crate hostname;
extern crate signal_hook;

mod args;
mod schemas;

use anyhow::Result;
use args::Args;
use bus::{Bus, BusReader};
use chrono::Local;
use clap::Parser;
use log::{debug, error, info, warn};
use mcap::{WriteOptions, Writer, records::MessageHeader};
use signal_hook::{consts::signal::*, iterator::Signals};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fs,
    io::{BufWriter, Error as e, ErrorKind},
    path::Path,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender, TryRecvError},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::{Duration, timeout};
use zenoh::{Session, handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample};

pub const NANO_SEC: u128 = 1_000_000_000;

fn get_available_space(path: &Path) -> Result<u64, std::io::Error> {
    fs2::available_space(path)
}

async fn write_to_file(
    mut out: Writer<BufWriter<fs::File>>,
    rx: Receiver<(MessageHeader, Vec<u8>)>,
    file_path: &Path,
    bus: Arc<Mutex<Bus<i32>>>,
) -> Result<(), Box<dyn Error>> {
    let mut last_size_check = Instant::now();
    let mut last_file_size = 0u64;
    let mut growth_rate = 0f64;
    let check_interval = Duration::from_secs(5);
    let min_buffer_seconds = 60;

    loop {
        match rx.recv() {
            Ok((header, data)) => {
                out.write_to_known_channel(&header, &data)?;
                if last_size_check.elapsed() >= check_interval {
                    debug!(
                        "Storage check - Time since last check: {:.2}s",
                        last_size_check.elapsed().as_secs_f64()
                    );
                    let current_size = file_path.metadata()?.len();
                    let time_diff = last_size_check.elapsed().as_secs_f64();

                    if last_file_size > 0 {
                        growth_rate = (current_size - last_file_size) as f64 / time_diff;
                    }

                    let available_space = get_available_space(file_path)?;
                    let estimated_space_needed = (growth_rate * min_buffer_seconds as f64) as u64;

                    debug!(
                        "Storage stats:\n\
                        - Current file size: {:.2} MB\n\
                        - Last file size: {:.2} MB\n\
                        - Growth rate: {:.2} MB/s\n\
                        - Available space: {:.2} MB\n\
                        - Estimated space needed (1min): {:.2} MB\n\
                        - Time difference: {:.2}s",
                        current_size as f64 / (1024.0 * 1024.0),
                        last_file_size as f64 / (1024.0 * 1024.0),
                        growth_rate / (1024.0 * 1024.0),
                        available_space as f64 / (1024.0 * 1024.0),
                        estimated_space_needed as f64 / (1024.0 * 1024.0),
                        time_diff
                    );

                    if available_space < estimated_space_needed {
                        info!(
                            "Low storage space detected. Growth rate: {:.2} MB/s, Available: {:.2} MB, Needed: {:.2} MB. Stopping recording...",
                            growth_rate / (1024.0 * 1024.0),
                            available_space as f64 / (1024.0 * 1024.0),
                            estimated_space_needed as f64 / (1024.0 * 1024.0)
                        );
                        out.finish()?;
                        bus.lock().unwrap().broadcast(1);
                        return Ok(());
                    }

                    last_file_size = current_size;
                    last_size_check = Instant::now();
                }
            }
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
    subscriber_topic: Subscriber<FifoChannelHandler<Sample>>,
    topic: String,
    mut exit_signal: BusReader<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame_number = 0;
    loop {
        match exit_signal.try_recv() {
            Ok(_) => {
                debug!(
                    "Program stopped finishing writing MCAP for {:?}.....",
                    topic
                );
                break;
            }
            Err(e) => match e {
                TryRecvError::Empty => {}
                TryRecvError::Disconnected => {
                    debug!(
                        "Program stopped finishing writing MCAP for {:?}.....",
                        topic
                    );
                    break;
                }
            },
        }
        match subscriber_topic.recv_timeout(Duration::from_secs(10)) {
            Ok(sample) => {
                let data = sample.unwrap().payload().to_bytes().to_vec();
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

fn cube_stream(
    channel_id: u16,
    start_time: u128,
    args: &Args,
    tx: Sender<(MessageHeader, Vec<u8>)>,
    subscriber_topic: Subscriber<FifoChannelHandler<Sample>>,
    topic: String,
    mut exit_signal: BusReader<i32>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut frame_number = 0;
    let frame_duration =
        Duration::from_secs_f64(1.0 / f64::from(args.get_cube_fps().unwrap_or(30)));
    let mut next_frame_time = Instant::now();

    loop {
        match exit_signal.try_recv() {
            Ok(_) => {
                debug!(
                    "Program stopped finishing writing MCAP for {:?}.....",
                    topic
                );
                break;
            }
            Err(e) => match e {
                TryRecvError::Empty => {}
                TryRecvError::Disconnected => {
                    debug!(
                        "Program stopped finishing writing MCAP for {:?}.....",
                        topic
                    );
                    break;
                }
            },
        }

        match subscriber_topic.recv_timeout(Duration::from_secs(10)) {
            Ok(sample) => {
                let now = Instant::now();

                if now >= next_frame_time {
                    debug!(
                        "Processing frame: {} | Time since last frame: {:?}",
                        frame_number,
                        now.duration_since(
                            next_frame_time.checked_sub(frame_duration).unwrap_or(now)
                        )
                    );

                    let data = sample.unwrap().payload().to_bytes().to_vec();
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

                    next_frame_time += frame_duration;
                    if next_frame_time < now {
                        next_frame_time = now + frame_duration;
                    }

                    if let Some(duration) = args.duration {
                        if (unix_time_seconds - start_time) / NANO_SEC >= duration {
                            break;
                        }
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
    let current_time = Local::now();
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

async fn discover_topics(args: &Args, session: &Session) -> Result<Vec<String>, Box<dyn Error>> {
    let subscriber = match timeout(
        Duration::from_secs(args.timeout),
        session.declare_subscriber("**"),
    )
    .await?
    {
        Ok(subscriber) => subscriber,
        Err(err) => return Err(err),
    };

    let mut topics = HashSet::new();
    let start_time = Instant::now();

    while start_time.elapsed() < Duration::from_secs(args.timeout) {
        let sample = subscriber
            .recv_timeout(Duration::from_secs(args.timeout))
            .unwrap()
            .unwrap();
        let topic = sample.key_expr().to_string();
        if topics.insert(topic.clone()) {
            info!(
                "Found {:?} will start recording in {:?} seconds",
                topic,
                (Duration::from_secs(args.timeout) - start_time.elapsed()).as_secs() as i64
            );
        }
    }

    drop(subscriber);
    Ok(Vec::from_iter(topics))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = Args::parse();
    let schemas = schemas::get_all();
    let session = zenoh::open(args.clone()).await.unwrap();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    if args.topics.is_empty() && !args.all_topics {
        return Err("No topics are specified and --all-topics flag is FALSE exiting".into());
    }

    args.topics = match args.all_topics {
        true => match discover_topics(&args, &session).await {
            Ok(topics) => topics,
            Err(e) => return Err(format!("unable to discover topics: {}", e).into()),
        },
        false => args.topics,
    };

    args.topics = args
        .topics
        .iter()
        .map(|topic| {
            if topic.starts_with('/') {
                format!("rt{}", topic)
            } else if !topic.starts_with("rt/") {
                format!("rt/{}", topic)
            } else {
                topic.to_owned()
            }
        })
        .collect();

    let mut tasks = Vec::new();

    for topic in &args.topics {
        let topic = topic.clone(); // Cloning topic if needed later
        let session_arc = Arc::new(session.clone());
        let task = tokio::spawn(async move {
            let subscriber = match timeout(
                Duration::from_secs(args.timeout),
                session_arc.declare_subscriber(&topic),
            )
            .await
            {
                Ok(Ok(subscriber)) => subscriber,
                Ok(Err(err)) => {
                    return Err(e::new(
                        ErrorKind::Other,
                        format!("failed to declare subscriber: {}", err),
                    ));
                }
                Err(_) => {
                    return Err(e::new(
                        ErrorKind::Other,
                        "timeout while declaring subscriber",
                    ));
                }
            };

            let enc = match subscriber.recv_timeout(Duration::from_secs(args.timeout)) {
                Ok(sample) => sample
                    .unwrap()
                    .encoding()
                    .to_string()
                    .split(';')
                    .next_back()
                    .unwrap()
                    .to_string(),
                Err(_) => {
                    warn!("Timed out waiting on topic {}", topic);
                    "Topic Unavailable".to_string()
                }
            };
            Ok((topic, enc))
        });

        tasks.push(task);
    }

    let mut topics = HashMap::new();

    for task in tasks {
        match task.await {
            Ok(result) => match result {
                Ok((topic, encoding)) => {
                    debug!("Encoding for {:?} is {:?}", topic, encoding);
                    if encoding != "Topic Unavailable" {
                        info!("Subscribed to {} and started recording", topic);
                        topics.insert(topic, encoding);
                    }
                }
                Err(err) => {
                    warn!("{}", err);
                }
            },
            Err(err) => {
                warn!("Error occurred: {}", err);
            }
        }
    }

    if topics.is_empty() {
        return Err("No valid topics discovered".into());
    }

    let filename = Path::new(&get_storage()?).join(get_filename());
    info!("Recording to {}", filename.display());
    let file = match fs::File::create(&filename) {
        Ok(file) => file,
        Err(e) => return Err(format!("Failed to create {}: {}", filename.display(), e).into()),
    };
    let bufwriter = BufWriter::new(file);
    let mut out = WriteOptions::new()
        .compression(args.compression.clone().into())
        .create(bufwriter)?;

    let bus = Arc::new(Mutex::new(Bus::new(1)));
    let mut futures = Vec::new();
    let (tx, rx) = mpsc::channel();

    for (topic, encoding) in topics {
        match schemas.get(format!("schemas/{}.msg", encoding).as_str()) {
            Some(schema) => {
                let schema_id = out.add_schema(&encoding, "ros2msg", schema.as_bytes())?;
                let channel_id = out.add_channel(
                    schema_id,
                    &topic.replace("rt", ""),
                    "cdr",
                    &BTreeMap::default(),
                )?;

                let args = args.clone();
                let tx = tx.clone();
                let session = session.clone();
                let rx = bus.lock().unwrap().add_rx();
                let topic = topic.clone();

                futures.push(std::thread::spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on(launch_stream(session, args, topic, channel_id, tx, rx));
                }));
            }
            None => {
                warn!("No schema found for topic: {topic} encoding: {encoding}");
                continue;
            }
        }
    }

    let mut signals = Signals::new([SIGINT, SIGTERM]).expect("Error creating signal iterator");
    let bus_clone = bus.clone();
    std::thread::spawn(move || {
        for signal in signals.forever() {
            match signal {
                SIGINT => {
                    debug!("Received Ctrl+C (SIGINT) signal");
                    bus_clone.lock().unwrap().broadcast(1);
                    break;
                }
                SIGTERM => {
                    bus_clone.lock().unwrap().broadcast(1);
                    debug!("Received SIGTERM signal");
                    break;
                }
                _ => {}
            }
        }
    });
    drop(tx);

    if let Err(err) = write_to_file(out, rx, &filename, bus).await {
        return Err(format!("Error writing to file: {}", err).into());
    }

    for fut in futures {
        fut.join().unwrap();
    }

    info!("Saved MCAP to {}", filename.display());
    std::process::exit(0);
}

async fn launch_stream(
    session: Session,
    args: Args,
    topic: String,
    channel_id: u16,
    tx: Sender<(MessageHeader, Vec<u8>)>,
    rx: BusReader<i32>,
) {
    let start_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    let subscriber = session.declare_subscriber(topic.clone()).await.unwrap();
    if args.cube_fps.is_some() && topic == "rt/radar/cube" {
        cube_stream(channel_id, start_time, &args, tx, subscriber, topic, rx).unwrap();
    } else {
        stream(channel_id, start_time, &args, tx, subscriber, topic, rx).unwrap();
    }
}
