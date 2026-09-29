// Copyright 2026 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! MCAP output: recorded messages plus clock-state records.

use crate::clock::ClockStep;
use anyhow::{Context, Result};
use mcap::{
    records::{MessageHeader, Metadata},
    Writer,
};
use std::{
    collections::BTreeMap,
    io::{Seek, Write},
};

/// Root topic carrying clock steps as JSON messages for timeline display.
/// The `clock_step` Metadata records remain the authoritative copy.
pub const CLOCK_STEP_TOPIC: &str = "/clock_step";

const CLOCK_STEP_SCHEMA_NAME: &str = "edgefirst/ClockStep";

const CLOCK_STEP_SCHEMA: &str = r#"{
  "title": "edgefirst/ClockStep",
  "description": "Discontinuous change of the recorder host CLOCK_REALTIME",
  "type": "object",
  "properties": {
    "log_time_before": { "type": "integer", "description": "Wall time just before the step, ns since the UNIX epoch" },
    "log_time_after": { "type": "integer", "description": "Wall time just after the step, ns since the UNIX epoch" },
    "step_ns": { "type": "integer", "description": "Signed size of the step in ns" },
    "monotonic_ns": { "type": "integer", "description": "CLOCK_MONOTONIC when the step was detected, ns" },
    "detection": { "type": "string", "enum": ["timerfd", "sample"] }
  },
  "required": ["log_time_before", "log_time_after", "step_ns", "monotonic_ns", "detection"]
}"#;

/// Wraps the MCAP writer so clock records are written consistently.
pub struct McapSink<W: Write + Seek> {
    out: Writer<W>,
    clock_step_channel: Option<u16>,
    clock_step_count: u32,
}

impl<W: Write + Seek> McapSink<W> {
    pub fn new(out: Writer<W>) -> Self {
        Self {
            out,
            clock_step_channel: None,
            clock_step_count: 0,
        }
    }

    pub fn writer(&mut self) -> &mut Writer<W> {
        &mut self.out
    }

    pub fn write_metadata(&mut self, metadata: &Metadata) -> Result<()> {
        self.out
            .write_metadata(metadata)
            .with_context(|| format!("failed to write {} metadata to MCAP", metadata.name))
    }

    pub fn write_message(&mut self, header: &MessageHeader, data: &[u8]) -> Result<()> {
        self.out
            .write_to_known_channel(header, data)
            .context("failed to write message to MCAP")
    }

    /// Writes the `clock_step` Metadata record and the `/clock_step` message.
    /// The channel is created on the first step.
    pub fn write_clock_step(&mut self, step: &ClockStep) -> Result<()> {
        self.write_metadata(&step.to_metadata())?;

        let channel_id = match self.clock_step_channel {
            Some(id) => id,
            None => {
                let schema_id = self
                    .out
                    .add_schema(
                        CLOCK_STEP_SCHEMA_NAME,
                        "jsonschema",
                        CLOCK_STEP_SCHEMA.as_bytes(),
                    )
                    .context("failed to add clock step schema")?;
                let id = self
                    .out
                    .add_channel(schema_id, CLOCK_STEP_TOPIC, "json", &BTreeMap::new())
                    .context("failed to add clock step channel")?;
                self.clock_step_channel = Some(id);
                id
            }
        };

        let header = MessageHeader {
            channel_id,
            sequence: self.clock_step_count,
            log_time: step.log_time_after,
            publish_time: step.log_time_after,
        };
        let data = serde_json::to_vec(&step.to_json()).context("failed to encode clock step")?;
        self.write_message(&header, &data)?;
        self.clock_step_count += 1;
        Ok(())
    }

    pub fn clock_step_count(&self) -> u32 {
        self.clock_step_count
    }

    /// Writes the summary and returns the underlying writer.
    pub fn finish(mut self) -> Result<W> {
        self.out.finish().context("failed to finalize MCAP")?;
        Ok(self.out.into_inner())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Detection;
    use mcap::{
        read::{LinearReader, Options},
        records::Record,
        MessageStream, Summary, WriteOptions,
    };
    use std::io::Cursor;

    fn step(before: u64, after: u64) -> ClockStep {
        ClockStep {
            log_time_before: before,
            log_time_after: after,
            step_ns: after as i64 - before as i64,
            monotonic_ns: 41_400_000_000,
            detection: Detection::Timerfd,
        }
    }

    fn sync_record() -> Metadata {
        Metadata {
            name: "clock_sync".into(),
            metadata: BTreeMap::from([("source".to_string(), "none".to_string())]),
        }
    }

    /// Records `steps` between CDR messages and returns the finished file.
    fn record(steps: &[ClockStep]) -> Vec<u8> {
        let out = WriteOptions::new().create(Cursor::new(Vec::new())).unwrap();
        let mut sink = McapSink::new(out);
        sink.write_metadata(&sync_record()).unwrap();
        let schema = sink
            .writer()
            .add_schema("std_msgs/msg/Header", "ros2msg", b"")
            .unwrap();
        let channel = sink
            .writer()
            .add_channel(schema, "/imu", "cdr", &BTreeMap::new())
            .unwrap();
        let mut sequence = 0u32;
        let mut message = |sink: &mut McapSink<Cursor<Vec<u8>>>| {
            let header = MessageHeader {
                channel_id: channel,
                sequence,
                log_time: 1_000 + u64::from(sequence),
                publish_time: 1_000 + u64::from(sequence),
            };
            sequence += 1;
            sink.write_message(&header, &[0, 1, 0, 0]).unwrap();
        };
        message(&mut sink);
        for s in steps {
            sink.write_clock_step(s).unwrap();
            message(&mut sink);
        }
        assert_eq!(sink.clock_step_count(), steps.len() as u32);
        sink.finish().unwrap().into_inner()
    }

    fn metadata_names(buf: &[u8]) -> Vec<String> {
        let summary = Summary::read(buf).unwrap().expect("summary");
        summary
            .metadata_indexes
            .iter()
            .map(|i| mcap::read::metadata(buf, i).unwrap().name)
            .collect()
    }

    fn topic_counts(buf: &[u8]) -> BTreeMap<String, usize> {
        let mut counts = BTreeMap::new();
        for m in MessageStream::new(buf).unwrap() {
            *counts.entry(m.unwrap().channel.topic.clone()).or_default() += 1;
        }
        counts
    }

    #[test]
    fn steps_are_indexed_as_metadata() {
        let buf = record(&[step(1_000, 9_000_000_000), step(9_500_000_000, 2_000)]);
        assert_eq!(
            metadata_names(&buf),
            ["clock_sync", "clock_step", "clock_step"]
        );
        let summary = Summary::read(&buf).unwrap().unwrap();
        let second = mcap::read::metadata(&buf, &summary.metadata_indexes[2]).unwrap();
        assert_eq!(second.metadata["step_ns"], "-9499998000");
    }

    #[test]
    fn steps_are_json_messages_at_log_time_after() {
        let buf = record(&[step(1_000, 9_000_000_000), step(9_500_000_000, 2_000)]);
        assert_eq!(
            topic_counts(&buf),
            BTreeMap::from([("/clock_step".to_string(), 2), ("/imu".to_string(), 3)])
        );
        let steps: Vec<_> = MessageStream::new(&buf)
            .unwrap()
            .map(Result::unwrap)
            .filter(|m| m.channel.topic == CLOCK_STEP_TOPIC)
            .collect();
        assert_eq!(steps[0].channel.message_encoding, "json");
        let schema = steps[0].channel.schema.as_ref().expect("schema");
        assert_eq!(schema.name, "edgefirst/ClockStep");
        assert_eq!(schema.encoding, "jsonschema");
        let log_times: Vec<u64> = steps.iter().map(|m| m.log_time).collect();
        assert_eq!(log_times, [9_000_000_000, 2_000]);
        assert_eq!(steps[1].publish_time, 2_000);
        assert_eq!(steps[1].sequence, 1);
        let json: serde_json::Value = serde_json::from_slice(&steps[1].data).unwrap();
        assert_eq!(json["step_ns"], -9_499_998_000i64);
    }

    #[test]
    fn no_step_means_no_clock_step_channel() {
        let buf = record(&[]);
        assert_eq!(metadata_names(&buf), ["clock_sync"]);
        assert_eq!(
            topic_counts(&buf),
            BTreeMap::from([("/imu".to_string(), 1)])
        );
    }

    #[test]
    fn steps_survive_a_missing_summary() {
        let buf = record(&[step(1_000, 9_000_000_000)]);
        let summary_start = mcap::read::footer(&buf).unwrap().summary_start as usize;
        let truncated = &buf[..summary_start];
        let names: Vec<String> =
            LinearReader::new_with_options(truncated, Options::IgnoreEndMagic.into())
                .unwrap()
                .filter_map(|r| match r.unwrap() {
                    Record::Metadata(m) => Some(m.name),
                    _ => None,
                })
                .collect();
        assert_eq!(names, ["clock_sync", "clock_step"]);
    }
}
