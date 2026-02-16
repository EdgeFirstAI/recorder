// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

mod args;
mod schemas;

use anyhow::{anyhow, bail, Context, Result};
use args::Args;
use bus::{Bus, BusReader};
use chrono::Local;
use clap::Parser;
use log::{debug, error, info, warn};
use mcap::{records::MessageHeader, WriteOptions, Writer};
use signal_hook::{consts::signal::*, iterator::Signals};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io::BufWriter,
    path::Path,
    sync::{
        mpsc::{self, Receiver, SyncSender, TryRecvError},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::time::{timeout, Duration as TokioDuration};
use zenoh::{handlers::FifoChannelHandler, pubsub::Subscriber, sample::Sample, Session};

const NANOS_PER_SEC: u64 = 1_000_000_000;

/// Maximum number of messages buffered between stream threads and the writer.
const CHANNEL_CAPACITY: usize = 64;

fn timestamp_nanos() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_nanos() as u64
}

fn should_exit(exit_signal: &mut BusReader<i32>) -> bool {
    match exit_signal.try_recv() {
        Ok(_) | Err(TryRecvError::Disconnected) => true,
        Err(TryRecvError::Empty) => false,
    }
}

fn get_available_space(path: &Path) -> Result<u64> {
    fs2::available_space(path).context("failed to query available disk space")
}

/// Receives messages from all stream threads and writes them to the MCAP file.
/// Monitors disk space and triggers shutdown if storage runs low.
fn write_to_file(
    mut out: Writer<BufWriter<fs::File>>,
    rx: Receiver<(MessageHeader, Vec<u8>)>,
    file_path: &Path,
    bus: Arc<Mutex<Bus<i32>>>,
) -> Result<()> {
    let mut last_size_check = Instant::now();
    let mut last_file_size = 0u64;
    let mut growth_rate = 0f64;
    let check_interval = Duration::from_secs(5);
    let min_buffer_seconds = 60;

    loop {
        match rx.recv() {
            Ok((header, data)) => {
                out.write_to_known_channel(&header, &data)
                    .context("failed to write message to MCAP")?;

                if last_size_check.elapsed() >= check_interval {
                    let elapsed = last_size_check.elapsed().as_secs_f64();
                    let current_size = file_path
                        .metadata()
                        .context("failed to read MCAP file metadata")?
                        .len();

                    if last_file_size > 0 {
                        growth_rate = (current_size - last_file_size) as f64 / elapsed;
                    }

                    let available_space = get_available_space(file_path)?;
                    let estimated_space_needed = (growth_rate * min_buffer_seconds as f64) as u64;

                    debug!(
                        "Storage: file={:.1}MB, rate={:.1}MB/s, available={:.1}MB, needed={:.1}MB",
                        current_size as f64 / (1024.0 * 1024.0),
                        growth_rate / (1024.0 * 1024.0),
                        available_space as f64 / (1024.0 * 1024.0),
                        estimated_space_needed as f64 / (1024.0 * 1024.0),
                    );

                    if available_space < estimated_space_needed {
                        info!(
                            "Low storage: {:.1}MB available, {:.1}MB needed in next 60s. Stopping.",
                            available_space as f64 / (1024.0 * 1024.0),
                            estimated_space_needed as f64 / (1024.0 * 1024.0),
                        );
                        out.finish().context("failed to finalize MCAP on low storage")?;
                        bus.lock().unwrap().broadcast(1);
                        return Ok(());
                    }

                    last_file_size = current_size;
                    last_size_check = Instant::now();
                }
            }
            Err(_) => {
                // All senders dropped — recording complete
                out.finish().context("failed to finalize MCAP")?;
                return Ok(());
            }
        }
    }
}

struct TopicRecorder {
    channel_id: u16,
    start_nanos: u64,
    duration_secs: Option<u64>,
    tx: SyncSender<(MessageHeader, Vec<u8>)>,
    frame_duration: Option<Duration>,
}

/// Records messages from a single Zenoh topic to the MCAP writer channel.
/// Optionally rate-limits frames when `frame_duration` is provided (used for radar cube).
fn record_topic(
    rec: TopicRecorder,
    subscriber: Subscriber<FifoChannelHandler<Sample>>,
    topic: &str,
    mut exit_signal: BusReader<i32>,
) {
    let mut sequence = 0u32;
    let mut next_frame_time = Instant::now();

    loop {
        if should_exit(&mut exit_signal) {
            debug!("Shutting down recorder for {topic}");
            break;
        }

        let sample = match subscriber.recv_timeout(Duration::from_secs(10)) {
            Ok(sample) => sample,
            Err(_) => {
                warn!("No data received on {topic} for 10s, retrying...");
                continue;
            }
        };

        let sample = match sample {
            Some(sample) => sample,
            None => continue,
        };

        // Rate limiting for high-bandwidth topics (e.g. radar cube)
        if let Some(interval) = rec.frame_duration {
            let now = Instant::now();
            if now < next_frame_time {
                continue;
            }
            next_frame_time += interval;
            if next_frame_time < now {
                next_frame_time = now + interval;
            }
        }

        let data = sample.payload().to_bytes().into_owned();
        let timestamp_ns = timestamp_nanos();

        let header = MessageHeader {
            channel_id: rec.channel_id,
            sequence,
            log_time: timestamp_ns,
            publish_time: timestamp_ns,
        };

        if rec.tx.send((header, data)).is_err() {
            debug!("Writer channel closed, stopping {topic}");
            break;
        }

        sequence += 1;

        if let Some(max_secs) = rec.duration_secs {
            if (timestamp_ns - rec.start_nanos) / NANOS_PER_SEC >= max_secs {
                debug!("Duration limit reached for {topic}");
                break;
            }
        }
    }
}

fn get_storage_dir() -> Result<String> {
    match std::env::var("STORAGE") {
        Err(_) => Ok(String::new()),
        Ok(storage) => {
            debug!("STORAGE={storage}");
            fs::create_dir_all(&storage)
                .with_context(|| format!("failed to create storage directory: {storage}"))?;
            Ok(storage)
        }
    }
}

fn get_filename() -> String {
    let timestamp = Local::now().format("%Y_%m_%d_%H_%M_%S");
    let prefix = hostname::get()
        .ok()
        .and_then(|h| h.to_str().map(String::from))
        .unwrap_or_else(|| "maivin-recorder".to_string());
    format!("{prefix}_{timestamp}.mcap")
}

/// Normalizes topic names to use the `rt/` prefix convention used by Zenoh
/// for ROS 2 topic bridging.
fn normalize_topic(topic: &str) -> String {
    if topic.starts_with('/') {
        format!("rt{topic}")
    } else if !topic.starts_with("rt/") {
        format!("rt/{topic}")
    } else {
        topic.to_string()
    }
}

/// Strips the `rt` prefix from a topic name for use as the MCAP channel topic.
fn mcap_topic(topic: &str) -> &str {
    topic.strip_prefix("rt").unwrap_or(topic)
}

async fn discover_topics(session: &Session, timeout_secs: u64) -> Result<Vec<String>> {
    let subscriber = timeout(
        TokioDuration::from_secs(timeout_secs),
        session.declare_subscriber("**"),
    )
    .await
    .context("timed out connecting to Zenoh for topic discovery")?
    .map_err(|e| anyhow!("failed to declare wildcard subscriber: {e}"))?;

    let mut topics = HashSet::new();
    let start = Instant::now();
    let deadline = Duration::from_secs(timeout_secs);

    while start.elapsed() < deadline {
        match subscriber.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(Some(sample)) => {
                let topic = sample.key_expr().to_string();
                if topics.insert(topic.clone()) {
                    let remaining = deadline.saturating_sub(start.elapsed()).as_secs();
                    info!("Discovered {topic}, recording starts in {remaining}s");
                }
            }
            Ok(None) => continue,
            Err(_) => break,
        }
    }

    drop(subscriber);
    Ok(topics.into_iter().collect())
}

/// Resolves the encoding (schema name) for a topic by reading the first sample.
async fn resolve_encoding(
    session: &Session,
    topic: &str,
    timeout_secs: u64,
) -> Result<Option<String>> {
    let subscriber = timeout(
        TokioDuration::from_secs(timeout_secs),
        session.declare_subscriber(topic),
    )
    .await
    .with_context(|| format!("timed out subscribing to {topic}"))?
    .map_err(|e| anyhow!("failed to declare subscriber for {topic}: {e}"))?;

    match subscriber.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(Some(sample)) => {
            let encoding = sample
                .encoding()
                .to_string()
                .split(';')
                .next_back()
                .unwrap_or_default()
                .to_string();
            Ok(Some(encoding))
        }
        Ok(None) => {
            warn!("Received empty sample from {topic}");
            Ok(None)
        }
        Err(_) => {
            warn!("Timed out waiting for data on {topic}");
            Ok(None)
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let schemas = schemas::get_all();

    let session = zenoh::open(args.clone())
        .await
        .map_err(|e| anyhow!("failed to open Zenoh session: {e}"))?;

    if args.topics.is_empty() && !args.all_topics {
        bail!("no topics specified — provide topics as arguments or use --all-topics");
    }

    // Discover or normalize topics
    let topics: Vec<String> = if args.all_topics {
        discover_topics(&session, args.timeout)
            .await
            .context("topic discovery failed")?
    } else {
        args.topics.iter().map(|t| normalize_topic(t)).collect()
    };

    if topics.is_empty() {
        bail!("no topics found during discovery");
    }

    // Resolve encodings for all topics concurrently
    let mut tasks = Vec::new();
    for topic in &topics {
        let session = session.clone();
        let topic = topic.clone();
        let timeout_secs = args.timeout;
        tasks.push(tokio::spawn(async move {
            let encoding = resolve_encoding(&session, &topic, timeout_secs).await;
            (topic, encoding)
        }));
    }

    let mut topic_encodings = HashMap::new();
    for task in tasks {
        let (topic, result) = task.await.context("topic resolution task panicked")?;
        match result {
            Ok(Some(encoding)) => {
                info!("Subscribed to {topic} (encoding: {encoding})");
                topic_encodings.insert(topic, encoding);
            }
            Ok(None) => {
                warn!("Skipping {topic}: no data received within timeout");
            }
            Err(e) => {
                warn!("Skipping {topic}: {e}");
            }
        }
    }

    if topic_encodings.is_empty() {
        bail!("no topics with valid encodings found — check that Zenoh publishers are running");
    }

    // Create MCAP output file
    let filename = Path::new(&get_storage_dir()?).join(get_filename());
    info!("Recording to {}", filename.display());

    let file = fs::File::create(&filename)
        .with_context(|| format!("failed to create {}", filename.display()))?;

    let mut out = WriteOptions::new()
        .compression(args.compression.clone().into())
        .create(BufWriter::new(file))
        .context("failed to initialize MCAP writer")?;

    let bus = Arc::new(Mutex::new(Bus::new(1)));
    let (tx, rx) = mpsc::sync_channel(CHANNEL_CAPACITY);
    let mut thread_handles = Vec::new();

    for (topic, encoding) in &topic_encodings {
        let schema_key = format!("schemas/{encoding}.msg");
        let schema = match schemas.get(schema_key.as_str()) {
            Some(schema) => schema,
            None => {
                warn!("No schema for {topic} (encoding: {encoding}), skipping");
                continue;
            }
        };

        let schema_id = out
            .add_schema(encoding, "ros2msg", schema.as_bytes())
            .with_context(|| format!("failed to add schema for {encoding}"))?;

        let channel_id = out
            .add_channel(
                schema_id,
                mcap_topic(topic),
                "cdr",
                &BTreeMap::default(),
            )
            .with_context(|| format!("failed to add MCAP channel for {topic}"))?;

        let frame_duration = if args.cube_fps.is_some() && topic == "rt/radar/cube" {
            args.cube_fps()
                .map(|fps| Duration::from_secs_f64(1.0 / f64::from(fps)))
        } else {
            None
        };

        let duration_secs = args.duration;
        let tx = tx.clone();
        let session = session.clone();
        let exit_signal = bus.lock().unwrap().add_rx();
        let topic = topic.clone();

        thread_handles.push(std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create per-topic Tokio runtime");

            rt.block_on(async {
                let start_nanos = timestamp_nanos();
                let subscriber = match session.declare_subscriber(&topic).await {
                    Ok(sub) => sub,
                    Err(e) => {
                        error!("Failed to subscribe to {topic}: {e}");
                        return;
                    }
                };

                record_topic(
                    TopicRecorder {
                        channel_id,
                        start_nanos,
                        duration_secs,
                        tx,
                        frame_duration,
                    },
                    subscriber,
                    &topic,
                    exit_signal,
                );
            });
        }));
    }

    // Signal handler thread
    let bus_clone = bus.clone();
    std::thread::spawn(move || {
        let mut signals =
            Signals::new([SIGINT, SIGTERM]).expect("failed to register signal handlers");
        for signal in signals.forever() {
            match signal {
                SIGINT => debug!("Received SIGINT"),
                SIGTERM => debug!("Received SIGTERM"),
                _ => continue,
            }
            bus_clone.lock().unwrap().broadcast(1);
            break;
        }
    });

    // Drop the sender so the writer knows when all streams finish
    drop(tx);

    // Writer runs on the main thread
    write_to_file(out, rx, &filename, bus)?;

    // Wait for all stream threads to finish
    for handle in thread_handles {
        let _ = handle.join();
    }

    info!("Saved MCAP to {}", filename.display());
    Ok(())
}
