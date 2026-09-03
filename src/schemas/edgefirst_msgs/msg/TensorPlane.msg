# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Au-Zone Technologies. All Rights Reserved.

# TensorPlane -- descriptor for a single plane of a Tensor.

int64 handle
uint64 offset
uint64 stride
uint64 size
uint64 used
uint64 modifier
uint8[] handle_bytes
uint8[] data
