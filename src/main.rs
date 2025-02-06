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
use mcap::{records::MessageHeader, WriteOptions, Writer};
use signal_hook::{consts::signal::*, iterator::Signals};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    error::Error,
    fs,
    io::{BufWriter, Error as e, ErrorKind},
    path::Path,
    sync::{
        mpsc::{self, Receiver, Sender, TryRecvError},
        Arc,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::{timeout, Duration};
use zenoh::{handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample, Session};

pub const NANO_SEC: u128 = 1_000_000_000;

async fn write_to_file(
    mut out: Writer<BufWriter<fs::File>>,
    rx: Receiver<(MessageHeader, Vec<u8>)>,
) -> Result<(), Box<dyn Error>> {
    loop {
        match rx.recv() {
            Ok((header, data)) => out.write_to_known_channel(&header, &data)?,
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
    let mut frame_duration = Duration::from_secs_f64(1.0 / 30.0);
    if let Some(fps) = args.cube_fps {
        frame_duration = Duration::from_secs_f64(1.0 / fps);
    }
    let mut last_frame_time = Instant::now();
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
                let elapsed = now.duration_since(last_frame_time);

                if elapsed >= frame_duration {
                    debug!(
                        "Processing frame: {} | Elapsed time: {:?}",
                        frame_number, elapsed
                    );

                    last_frame_time = now;

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
                } else {
                    continue;
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
                    .last()
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

    let mut bus = Bus::new(1);
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
                let rx = bus.add_rx();
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
    std::thread::spawn(move || {
        for signal in signals.forever() {
            match signal {
                SIGINT => {
                    debug!("Received Ctrl+C (SIGINT) signal");
                    bus.broadcast(1);
                    break;
                }
                SIGTERM => {
                    bus.broadcast(1);
                    debug!("Received SIGTERM signal");
                    break;
                }
                _ => {}
            }
        }
    });
    drop(tx);

    if let Err(err) = write_to_file(out, rx).await {
        return Err(format!("Error writing to file: {}", err).into());
    }

    for fut in futures {
        fut.join().unwrap();
    }

    info!("Saved MCAP to {}", filename.display());
    Ok(())
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
