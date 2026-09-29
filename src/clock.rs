// Copyright 2026 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

//! Wall-clock step detection and clock-state records for the MCAP file.

use mcap::records::Metadata;
use std::{
    collections::BTreeMap,
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Upper bound on waiting for `chronyc` when opening a recording.
const CHRONYC_TIMEOUT: Duration = Duration::from_secs(2);

/// Offset changes larger than this are reported as clock steps.
pub const STEP_THRESHOLD_NS: i64 = 1_000_000_000;

/// Largest REALTIME read gap accepted without retrying a measurement.
const MAX_BRACKET_NS: i64 = 10_000;

/// Attempts made to get a bracket within [`MAX_BRACKET_NS`].
const MAX_ATTEMPTS: usize = 3;

/// One measurement of `CLOCK_REALTIME - CLOCK_MONOTONIC`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetSample {
    pub offset_ns: i64,
    pub monotonic_ns: u64,
}

/// Source of offset measurements, abstracted so tests can inject steps.
pub trait ClockSource {
    fn sample(&mut self) -> OffsetSample;
}

/// Computes the offset from `read`, which returns `[REALTIME, MONOTONIC,
/// REALTIME]` in nanoseconds. The two clocks cannot be read atomically, so the
/// monotonic read is bracketed by two realtime reads and compared against
/// their midpoint. Preemption inside the bracket widens it, so wide brackets
/// are retried and the tightest one is kept.
fn measure_offset(mut read: impl FnMut() -> [i64; 3]) -> OffsetSample {
    let mut best: Option<(i64, OffsetSample)> = None;
    for _ in 0..MAX_ATTEMPTS {
        let [before, monotonic, after] = read();
        let bracket = after.saturating_sub(before);
        let midpoint = before + bracket / 2;
        // A backward step between the realtime reads makes the bracket
        // negative and its midpoint meaningless.
        let bracket = if bracket < 0 { i64::MAX } else { bracket };
        let sample = OffsetSample {
            offset_ns: midpoint.saturating_sub(monotonic),
            monotonic_ns: u64::try_from(monotonic).unwrap_or(0),
        };
        if best.is_none_or(|(width, _)| bracket < width) {
            best = Some((bracket, sample));
        }
        if bracket <= MAX_BRACKET_NS {
            break;
        }
    }
    best.expect("MAX_ATTEMPTS is non-zero").1
}

// `time_t` and `c_long` are 32-bit on some Linux targets.
#[allow(clippy::useless_conversion)]
fn clock_ns(clock: libc::clockid_t) -> i64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid, writable timespec; the clock ids used here are
    // always supported on Linux, so the call cannot fail.
    unsafe { libc::clock_gettime(clock, &mut ts) };
    i64::from(ts.tv_sec) * 1_000_000_000 + i64::from(ts.tv_nsec)
}

/// The host clocks, read through `clock_gettime` (vDSO).
pub struct SystemClock;

impl ClockSource for SystemClock {
    fn sample(&mut self) -> OffsetSample {
        measure_offset(|| {
            [
                clock_ns(libc::CLOCK_REALTIME),
                clock_ns(libc::CLOCK_MONOTONIC),
                clock_ns(libc::CLOCK_REALTIME),
            ]
        })
    }
}

/// Waits for discontinuous changes of `CLOCK_REALTIME` using a timerfd armed
/// with `TFD_TIMER_CANCEL_ON_SET` and an expiry that never arrives.
///
/// The kernel refreshes the timer's reference offset each time it reports a
/// change, so the fd is kept and never re-armed. Several changes between two
/// reads are reported once; the size comes from [`ClockTracker`].
pub struct StepWatcher {
    fd: OwnedFd,
}

impl StepWatcher {
    pub fn new() -> io::Result<Self> {
        Self::with_flags(libc::TFD_CLOEXEC)
    }

    fn with_flags(flags: libc::c_int) -> io::Result<Self> {
        // SAFETY: plain syscall; the returned descriptor is checked before use.
        let raw = unsafe { libc::timerfd_create(libc::CLOCK_REALTIME, flags) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `raw` is a freshly created descriptor owned by nobody else.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };

        let spec = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: libc::time_t::MAX,
                tv_nsec: 0,
            },
        };
        // SAFETY: `fd` is a valid timerfd and `spec` outlives the call.
        let rc = unsafe {
            libc::timerfd_settime(
                fd.as_raw_fd(),
                libc::TFD_TIMER_ABSTIME | libc::TFD_TIMER_CANCEL_ON_SET,
                &spec,
                std::ptr::null_mut(),
            )
        };
        if rc < 0 {
            let err = io::Error::last_os_error();
            // The timer is armed even when a change since creation is reported.
            if err.raw_os_error() != Some(libc::ECANCELED) {
                return Err(err);
            }
        }
        Ok(Self { fd })
    }

    /// Blocks until the realtime clock is set discontinuously.
    pub fn wait(&self) -> io::Result<()> {
        let mut expirations = [0u8; 8];
        loop {
            // SAFETY: `fd` is a valid timerfd and the buffer holds the 8 bytes
            // a timerfd read requires.
            let n = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    expirations.as_mut_ptr().cast(),
                    expirations.len(),
                )
            };
            if n >= 0 {
                // An expiry cannot arrive before the end of time_t; keep waiting.
                continue;
            }
            let err = io::Error::last_os_error();
            match err.raw_os_error() {
                Some(libc::ECANCELED) => return Ok(()),
                Some(libc::EINTR) => continue,
                _ => return Err(err),
            }
        }
    }
}

/// How a clock step was noticed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Detection {
    Timerfd,
    Sample,
}

/// A discontinuous change of `CLOCK_REALTIME`, expressed in the recorder's
/// `log_time` domain (nanoseconds since the UNIX epoch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockStep {
    pub log_time_before: u64,
    pub log_time_after: u64,
    pub step_ns: i64,
    pub monotonic_ns: u64,
    pub detection: Detection,
}

impl Detection {
    fn as_str(self) -> &'static str {
        match self {
            Detection::Timerfd => "timerfd",
            Detection::Sample => "sample",
        }
    }
}

/// Name of the MCAP Metadata record written for each clock step.
pub const CLOCK_STEP_METADATA: &str = "clock_step";

impl ClockStep {
    /// MCAP Metadata record. Values are decimal strings as MCAP metadata
    /// only carries strings.
    pub fn to_metadata(&self) -> Metadata {
        let metadata = [
            ("log_time_before", self.log_time_before.to_string()),
            ("log_time_after", self.log_time_after.to_string()),
            ("step_ns", self.step_ns.to_string()),
            ("monotonic_ns", self.monotonic_ns.to_string()),
            ("detection", self.detection.as_str().to_string()),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        Metadata {
            name: CLOCK_STEP_METADATA.to_string(),
            metadata,
        }
    }

    /// Payload for the `/clock_step` JSON channel.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "log_time_before": self.log_time_before,
            "log_time_after": self.log_time_after,
            "step_ns": self.step_ns,
            "monotonic_ns": self.monotonic_ns,
            "detection": self.detection.as_str(),
        })
    }
}

/// Name of the MCAP Metadata record written when the file is opened.
pub const CLOCK_SYNC_METADATA: &str = "clock_sync";

/// Subset of `chronyc -c tracking` recorded in `clock_sync`. Values are kept
/// as chronyc prints them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronyTracking {
    pub reference_id: String,
    pub reference_name: String,
    pub stratum: String,
    pub system_time_offset_s: String,
    pub root_dispersion_s: String,
    pub leap_status: String,
}

/// Kernel NTP state from `adjtimex` with `modes = 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KernelTime {
    pub unsync: bool,
    pub maxerror_us: i64,
    pub esterror_us: i64,
}

/// Parses one line of `chronyc -c tracking` output.
fn parse_chrony_tracking(csv: &str) -> Option<ChronyTracking> {
    let fields: Vec<&str> = csv.lines().next()?.split(',').collect();
    if fields.len() != 14 || fields[2].parse::<u8>().is_err() {
        return None;
    }
    Some(ChronyTracking {
        reference_id: fields[0].to_string(),
        reference_name: fields[1].to_string(),
        stratum: fields[2].to_string(),
        system_time_offset_s: fields[4].to_string(),
        root_dispersion_s: fields[11].to_string(),
        leap_status: fields[13].to_string(),
    })
}

/// Builds the `clock_sync` record. `synchronized` comes from chrony when it
/// answered, because the kernel `STA_UNSYNC` flag is only cleared by chrony
/// when `rtcsync` is enabled and is set again by every step.
fn clock_sync_metadata(
    open: OffsetSample,
    chrony: Option<&ChronyTracking>,
    kernel: Option<KernelTime>,
) -> Metadata {
    let open_realtime_ns = i128::from(open.offset_ns) + i128::from(open.monotonic_ns);
    let (synchronized, source) = match (chrony, kernel) {
        (Some(c), _) => ((c.leap_status != "Not synchronised").to_string(), "chronyc"),
        (None, Some(k)) => ((!k.unsync).to_string(), "adjtimex"),
        (None, None) => ("unknown".to_string(), "none"),
    };

    let mut metadata = BTreeMap::new();
    let mut put = |k: &str, v: String| {
        metadata.insert(k.to_string(), v);
    };
    put("open_realtime_ns", open_realtime_ns.to_string());
    put("boot_epoch_ns", open.offset_ns.to_string());
    put("synchronized", synchronized);
    put("source", source.to_string());
    if let Some(c) = chrony {
        put("reference_id", c.reference_id.clone());
        put("reference_name", c.reference_name.clone());
        put("stratum", c.stratum.clone());
        put("system_time_offset_s", c.system_time_offset_s.clone());
        put("root_dispersion_s", c.root_dispersion_s.clone());
        put("leap_status", c.leap_status.clone());
    }
    if let Some(k) = kernel {
        put("kernel_unsync", k.unsync.to_string());
        put("maxerror_us", k.maxerror_us.to_string());
        put("esterror_us", k.esterror_us.to_string());
    }
    Metadata {
        name: CLOCK_SYNC_METADATA.to_string(),
        metadata,
    }
}

/// Reads the kernel NTP state. `adjtimex` with `modes = 0` is read-only and
/// needs no privileges.
// `c_long` is 32-bit on some Linux targets.
#[allow(clippy::useless_conversion)]
fn read_kernel_time() -> Option<KernelTime> {
    // SAFETY: `timex` is plain old data; all-zero is a valid value and sets
    // `modes = 0` (read-only query).
    let mut tx: libc::timex = unsafe { std::mem::zeroed() };
    // SAFETY: `tx` is a valid, writable timex for the duration of the call.
    if unsafe { libc::adjtimex(&mut tx) } < 0 {
        return None;
    }
    Some(KernelTime {
        unsync: tx.status & libc::STA_UNSYNC != 0,
        maxerror_us: i64::from(tx.maxerror),
        esterror_us: i64::from(tx.esterror),
    })
}

/// Runs `cmd` and returns its stdout if it exits successfully within
/// `timeout`; the child is killed otherwise.
fn run_with_timeout(mut cmd: Command, timeout: Duration) -> Option<String> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(10));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut stdout = String::new();
    io::Read::read_to_string(&mut child.stdout.take()?, &mut stdout).ok()?;
    Some(stdout)
}

/// Captures the clock state for the `clock_sync` record at file open.
pub fn clock_sync(open: OffsetSample) -> Metadata {
    let mut cmd = Command::new("chronyc");
    cmd.args(["-c", "-n", "tracking"]);
    let chrony = run_with_timeout(cmd, CHRONYC_TIMEOUT)
        .as_deref()
        .and_then(parse_chrony_tracking);
    clock_sync_metadata(open, chrony.as_ref(), read_kernel_time())
}

/// Tracks the realtime-to-monotonic offset and reports changes above
/// [`STEP_THRESHOLD_NS`].
pub struct ClockTracker<S: ClockSource> {
    source: S,
    last_offset_ns: i64,
}

impl<S: ClockSource> ClockTracker<S> {
    /// Starts from a measurement taken elsewhere, so a record describing the
    /// same instant (such as `clock_sync`) shares its baseline.
    pub fn with_baseline(source: S, baseline: OffsetSample) -> Self {
        Self {
            source,
            last_offset_ns: baseline.offset_ns,
        }
    }

    /// Measures the offset, stores it, and returns a step when it moved by
    /// more than the threshold since the previous measurement.
    pub fn check(&mut self, detection: Detection) -> Option<ClockStep> {
        let now = self.source.sample();
        let before = std::mem::replace(&mut self.last_offset_ns, now.offset_ns);
        let step_ns = now.offset_ns.saturating_sub(before);
        if step_ns.unsigned_abs() <= STEP_THRESHOLD_NS as u64 {
            return None;
        }
        let to_log_time = |offset_ns: i64| {
            u64::try_from(i128::from(offset_ns) + i128::from(now.monotonic_ns)).unwrap_or(0)
        };
        Some(ClockStep {
            log_time_before: to_log_time(before),
            log_time_after: to_log_time(now.offset_ns),
            step_ns,
            monotonic_ns: now.monotonic_ns,
            detection,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    const EPOCH_2026: i64 = 1_790_000_000 * 1_000_000_000;
    const DAYS_475: i64 = 41_054_973 * 1_000_000_000;

    struct Scripted(VecDeque<OffsetSample>);

    impl Scripted {
        fn new(samples: &[(i64, u64)]) -> Self {
            Self(
                samples
                    .iter()
                    .map(|&(offset_ns, monotonic_ns)| OffsetSample {
                        offset_ns,
                        monotonic_ns,
                    })
                    .collect(),
            )
        }
    }

    impl ClockSource for Scripted {
        fn sample(&mut self) -> OffsetSample {
            self.0.pop_front().expect("scripted source exhausted")
        }
    }

    fn reads(
        script: &[[i64; 3]],
    ) -> (
        impl FnMut() -> [i64; 3] + '_,
        std::rc::Rc<std::cell::Cell<usize>>,
    ) {
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let counter = calls.clone();
        let read = move || {
            let i = counter.get();
            counter.set(i + 1);
            script[i]
        };
        (read, calls)
    }

    #[test]
    fn tight_bracket_uses_midpoint_without_retry() {
        let (read, calls) = reads(&[[EPOCH_2026 + 1_000, 500, EPOCH_2026 + 3_000]]);
        let s = measure_offset(read);
        assert_eq!(s.offset_ns, EPOCH_2026 + 2_000 - 500);
        assert_eq!(s.monotonic_ns, 500);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn wide_bracket_is_retried() {
        let (read, calls) = reads(&[
            [EPOCH_2026, 100, EPOCH_2026 + 2 * MAX_BRACKET_NS],
            [EPOCH_2026 + 100_000, 100_200, EPOCH_2026 + 100_400],
        ]);
        let s = measure_offset(read);
        assert_eq!(s.offset_ns, EPOCH_2026 + 100_200 - 100_200);
        assert_eq!(s.monotonic_ns, 100_200);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn tightest_of_all_wide_brackets_is_kept() {
        let (read, calls) = reads(&[
            [EPOCH_2026, 0, EPOCH_2026 + 90_000],
            [EPOCH_2026, 7, EPOCH_2026 + 30_000],
            [EPOCH_2026, 0, EPOCH_2026 + 60_000],
        ]);
        let s = measure_offset(read);
        assert_eq!(s.offset_ns, EPOCH_2026 + 15_000 - 7);
        assert_eq!(calls.get(), MAX_ATTEMPTS);
    }

    #[test]
    fn backward_step_inside_bracket_is_retried() {
        let (read, calls) = reads(&[
            [EPOCH_2026, 100, EPOCH_2026 - DAYS_475],
            [EPOCH_2026 - DAYS_475, 200, EPOCH_2026 - DAYS_475 + 400],
        ]);
        let s = measure_offset(read);
        assert_eq!(s.offset_ns, EPOCH_2026 - DAYS_475 + 200 - 200);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn system_clock_offset_is_positive_and_stable() {
        let mut clock = SystemClock;
        let a = clock.sample();
        let b = clock.sample();
        assert!(a.offset_ns > 0);
        assert!((b.offset_ns - a.offset_ns).abs() < STEP_THRESHOLD_NS);
        assert!(b.monotonic_ns >= a.monotonic_ns);
    }

    #[test]
    fn step_watcher_is_armed_and_quiet_without_a_step() {
        let watcher = StepWatcher::with_flags(libc::TFD_CLOEXEC | libc::TFD_NONBLOCK)
            .expect("timerfd with TFD_TIMER_CANCEL_ON_SET");
        let err = watcher.wait().expect_err("no clock change occurred");
        assert_eq!(err.kind(), io::ErrorKind::WouldBlock);
    }

    fn sample_step() -> ClockStep {
        ClockStep {
            log_time_before: 1_748_585_553_000_000_000,
            log_time_after: 1_789_640_526_261_956_000,
            step_ns: -41_054_973_261_956_000,
            monotonic_ns: 41_400_000_000,
            detection: Detection::Timerfd,
        }
    }

    #[test]
    fn clock_step_metadata_uses_decimal_strings() {
        let md = sample_step().to_metadata();
        assert_eq!(md.name, "clock_step");
        let expected: BTreeMap<String, String> = [
            ("log_time_before", "1748585553000000000"),
            ("log_time_after", "1789640526261956000"),
            ("step_ns", "-41054973261956000"),
            ("monotonic_ns", "41400000000"),
            ("detection", "timerfd"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        assert_eq!(md.metadata, expected);
    }

    #[test]
    fn clock_step_json_carries_integer_fields() {
        let json = sample_step().to_json();
        assert_eq!(json["log_time_before"], 1_748_585_553_000_000_000u64);
        assert_eq!(json["log_time_after"], 1_789_640_526_261_956_000u64);
        assert_eq!(json["step_ns"], -41_054_973_261_956_000i64);
        assert_eq!(json["monotonic_ns"], 41_400_000_000u64);
        assert_eq!(json["detection"], "timerfd");
    }

    const CHRONY_SYNCED: &str = "5BBD5B71,91.189.91.113,3,1790650913.928945790,0.000273857,-0.000153630,0.000160776,2.475,-0.001,0.056,0.114172287,0.001270426,1027.6,Normal\n";
    const CHRONY_UNSYNCED: &str = "00000000,,0,0.000000000,0.000000000,0.000000000,0.000000000,0.000,0.000,0.000,1.000000000,1.000000000,0.0,Not synchronised\n";

    fn md(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn chrony_tracking_parses_fields_by_position() {
        let t = parse_chrony_tracking(CHRONY_SYNCED).expect("parsed");
        assert_eq!(
            t,
            ChronyTracking {
                reference_id: "5BBD5B71".into(),
                reference_name: "91.189.91.113".into(),
                stratum: "3".into(),
                system_time_offset_s: "0.000273857".into(),
                root_dispersion_s: "0.001270426".into(),
                leap_status: "Normal".into(),
            }
        );
    }

    #[test]
    fn chrony_tracking_rejects_short_or_garbled_output() {
        assert_eq!(parse_chrony_tracking(""), None);
        assert_eq!(parse_chrony_tracking("506 Cannot talk to daemon\n"), None);
        assert_eq!(
            parse_chrony_tracking("5BBD5B71,x,notanumber,1,2,3,4,5,6,7,8,9,10,Normal"),
            None
        );
    }

    const OPEN: OffsetSample = OffsetSample {
        offset_ns: 1_790_000_000_000_000_000,
        monotonic_ns: 45_000_000_000,
    };

    #[test]
    fn clock_sync_prefers_chrony_leap_status() {
        let chrony = parse_chrony_tracking(CHRONY_SYNCED).unwrap();
        let kernel = KernelTime {
            unsync: true,
            maxerror_us: 16_000_000,
            esterror_us: 16_000_000,
        };
        let m = clock_sync_metadata(OPEN, Some(&chrony), Some(kernel));
        assert_eq!(m.name, "clock_sync");
        assert_eq!(
            m.metadata,
            md(&[
                ("open_realtime_ns", "1790000045000000000"),
                ("boot_epoch_ns", "1790000000000000000"),
                ("synchronized", "true"),
                ("source", "chronyc"),
                ("reference_id", "5BBD5B71"),
                ("reference_name", "91.189.91.113"),
                ("stratum", "3"),
                ("system_time_offset_s", "0.000273857"),
                ("root_dispersion_s", "0.001270426"),
                ("leap_status", "Normal"),
                ("kernel_unsync", "true"),
                ("maxerror_us", "16000000"),
                ("esterror_us", "16000000"),
            ])
        );
    }

    #[test]
    fn clock_sync_reports_unsynchronised_chrony() {
        let chrony = parse_chrony_tracking(CHRONY_UNSYNCED).unwrap();
        let m = clock_sync_metadata(OPEN, Some(&chrony), None);
        assert_eq!(m.metadata["synchronized"], "false");
        assert_eq!(m.metadata["source"], "chronyc");
        assert!(!m.metadata.contains_key("kernel_unsync"));
    }

    #[test]
    fn clock_sync_falls_back_to_adjtimex() {
        let kernel = KernelTime {
            unsync: false,
            maxerror_us: 1_200,
            esterror_us: 40,
        };
        let m = clock_sync_metadata(OPEN, None, Some(kernel));
        assert_eq!(m.metadata["synchronized"], "true");
        assert_eq!(m.metadata["source"], "adjtimex");
        assert!(!m.metadata.contains_key("reference_id"));
    }

    #[test]
    fn clock_sync_without_any_source_is_unknown() {
        let m = clock_sync_metadata(OPEN, None, None);
        assert_eq!(m.metadata["synchronized"], "unknown");
        assert_eq!(m.metadata["source"], "none");
        assert_eq!(m.metadata["boot_epoch_ns"], "1790000000000000000");
    }

    #[test]
    fn kernel_time_is_readable_unprivileged() {
        let k = read_kernel_time().expect("adjtimex modes=0");
        assert!(k.maxerror_us >= 0);
        assert!(k.esterror_us >= 0);
    }

    #[test]
    fn run_with_timeout_returns_stdout() {
        let mut cmd = Command::new("echo");
        cmd.arg("tracking");
        assert_eq!(
            run_with_timeout(cmd, Duration::from_secs(5)).as_deref(),
            Some("tracking\n")
        );
    }

    #[test]
    fn run_with_timeout_kills_slow_child() {
        let mut cmd = Command::new("sleep");
        cmd.arg("5");
        let start = Instant::now();
        assert_eq!(run_with_timeout(cmd, Duration::from_millis(100)), None);
        assert!(start.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn run_with_timeout_handles_missing_program_and_failure() {
        assert_eq!(
            run_with_timeout(
                Command::new("edgefirst-no-such-binary"),
                Duration::from_secs(1)
            ),
            None
        );
        assert_eq!(
            run_with_timeout(Command::new("false"), Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn with_baseline_compares_against_the_given_sample() {
        let baseline = OffsetSample {
            offset_ns: EPOCH_2026 - DAYS_475,
            monotonic_ns: 1,
        };
        let mut t = ClockTracker::with_baseline(Scripted::new(&[(EPOCH_2026, 2)]), baseline);
        let step = t
            .check(Detection::Timerfd)
            .expect("step from the given baseline");
        assert_eq!(step.step_ns, DAYS_475);
    }

    /// A tracker whose baseline is the first scripted sample.
    fn tracker(samples: &[(i64, u64)]) -> ClockTracker<Scripted> {
        let mut source = Scripted::new(samples);
        let baseline = source.sample();
        ClockTracker::with_baseline(source, baseline)
    }

    #[test]
    fn stable_offset_is_not_a_step() {
        let mut t = tracker(&[(EPOCH_2026, 10), (EPOCH_2026 + 50, 20)]);
        assert_eq!(t.check(Detection::Sample), None);
    }

    #[test]
    fn forward_step_reports_before_and_after_in_log_time() {
        let before = EPOCH_2026 - DAYS_475;
        let mono = 41_400_000_000u64;
        let mut t = tracker(&[(before, 13_000_000_000), (EPOCH_2026, mono)]);
        let step = t.check(Detection::Timerfd).expect("step");
        assert_eq!(step.step_ns, DAYS_475);
        assert_eq!(step.monotonic_ns, mono);
        assert_eq!(step.log_time_before, (before + mono as i64) as u64);
        assert_eq!(step.log_time_after, (EPOCH_2026 + mono as i64) as u64);
        assert_eq!(step.detection, Detection::Timerfd);
    }

    #[test]
    fn backward_step_has_negative_step_ns() {
        let mut t = tracker(&[(EPOCH_2026, 1), (EPOCH_2026 - DAYS_475, 2)]);
        let step = t.check(Detection::Sample).expect("step");
        assert_eq!(step.step_ns, -DAYS_475);
        assert!(step.log_time_after < step.log_time_before);
    }

    #[test]
    fn change_at_or_below_threshold_is_ignored() {
        let mut t = tracker(&[
            (EPOCH_2026, 1),
            (EPOCH_2026 + 500_000_000, 2),
            (EPOCH_2026 + 500_000_000 + STEP_THRESHOLD_NS, 3),
        ]);
        assert_eq!(t.check(Detection::Sample), None);
        assert_eq!(t.check(Detection::Sample), None);
    }

    #[test]
    fn baseline_follows_each_measurement() {
        let mut t = tracker(&[
            (EPOCH_2026, 1),
            (EPOCH_2026 + DAYS_475, 2),
            (EPOCH_2026 + DAYS_475, 3),
        ]);
        assert!(t.check(Detection::Timerfd).is_some());
        assert_eq!(t.check(Detection::Timerfd), None);
    }

    #[test]
    fn pre_epoch_log_time_saturates_at_zero() {
        let mut t = tracker(&[(-5 * STEP_THRESHOLD_NS, 1), (EPOCH_2026, 2)]);
        let step = t.check(Detection::Sample).expect("step");
        assert_eq!(step.log_time_before, 0);
    }
}
