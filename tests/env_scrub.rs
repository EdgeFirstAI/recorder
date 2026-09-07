//! End-to-end check that `KEY=""` in the environment behaves as unset.
//!
//! Runs with `harness = false` so this `main` is the only thread in the
//! process when the environment is mutated, which `scrub_empty_env` requires.
// args.rs's own `#[cfg(test)] mod tests` is compiled here but its `#[test]` fns
// are stripped without the libtest harness, leaving its imports and helpers unused.
#![allow(dead_code, unused_imports)]

#[path = "../src/args.rs"]
mod args;
use args::{scrub_empty_env, Args, Compression, KEEP};
use clap::Parser;
use zenoh::config::WhatAmI;

const VARS: [&str; 5] = [
    "CUBE_FPS",
    "COMPRESSION",
    "MODE",
    "TOPICS",
    "NO_MULTICAST_SCOUTING",
];
const ARGV: [&str; 1] = ["edgefirst-recorder"];

fn main() {
    for name in VARS {
        // SAFETY: single-threaded — this is `main` before any thread is spawned.
        std::env::set_var(name, "");
    }
    let before = Args::try_parse_from(ARGV);
    assert!(
        before.is_err(),
        "empty vars must fail to parse before scrubbing: {before:?}"
    );

    // SAFETY: still single-threaded.
    unsafe { scrub_empty_env::<Args>(KEEP) };
    for name in VARS {
        assert!(
            std::env::var_os(name).is_none(),
            "{name} should have been removed"
        );
    }

    let args = Args::try_parse_from(ARGV).expect("defaults must apply after scrubbing");
    assert_eq!(
        args.cube_fps, None,
        "CUBE_FPS=\"\" must not reach the parser"
    );
    assert!(
        matches!(args.compression, Compression::None),
        "COMPRESSION default must apply: {:?}",
        args.compression
    );
    assert_eq!(args.mode, WhatAmI::Peer, "MODE default must apply");
    assert!(args.topics.is_empty(), "TOPICS=\"\" must not yield [\"\"]");
    assert!(
        !args.no_multicast_scouting,
        "NO_MULTICAST_SCOUTING default must apply"
    );
    assert_eq!(args.cube_fps(), None);
    assert!(args.topics().is_empty());
    println!("env_scrub: ok");
}
