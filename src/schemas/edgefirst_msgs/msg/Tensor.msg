# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Au-Zone Technologies. All Rights Reserved.

# Tensor -- generic n-dimensional buffer reference.
#
# A 1:1 projection of the edgefirst-tensor HAL blob. Carries storage,
# geometry, optional image interpretation and optional quantization. This
# library carries these fields; it never interprets them.

uint32 storage_kind
uint32 pid
int32 fence_fd
uint32 dtype
int32 quant_axis
uint64[] shape
int64[] strides
float32[] quant_scales
int32[] quant_zero_points
string format
string color_space
string color_transfer
string color_encoding
string color_range
edgefirst_msgs/TensorPlane[] planes

================================================================================
MSG: edgefirst_msgs/TensorPlane
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
