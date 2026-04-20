// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

fn main() {
    println!("cargo:rerun-if-changed=src/schemas");
    include_walk::from("src/schemas/")
        .to("src/schemas.rs")
        .unwrap();
}
