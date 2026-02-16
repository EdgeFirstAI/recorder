// Copyright 2025 Au-Zone Technologies Inc.
// SPDX-License-Identifier: Apache-2.0

fn main() {
    include_walk::from("src/schemas/")
        .to("src/schemas.rs")
        .unwrap();
}
