# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Au-Zone Technologies. All Rights Reserved.

# TensorStamped -- a Tensor with a timestamp and frame of reference.
#
# The standalone tensor IPC message. Byte-identical in layout to CameraFrame;
# the two are distinct types because in ROS the schema name is the contract on
# the topic.

std_msgs/Header header
uint64 seq
edgefirst_msgs/Tensor tensor

================================================================================
MSG: edgefirst_msgs/Tensor
# SPDX-License-Identifier: Apache-2.0
# Copyright (c) 2026 Au-Zone Technologies. All Rights Reserved.

# Tensor -- generic n-dimensional buffer reference.

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

================================================================================
MSG: std_msgs/Header
# Standard metadata for higher-level stamped data types.
# This is generally used to communicate timestamped data
# in a particular coordinate frame.

builtin_interfaces/Time stamp
string frame_id

================================================================================
MSG: builtin_interfaces/Time
# This message communicates ROS Time defined here:
# https://design.ros2.org/articles/clock_and_time.html

int32 sec
uint32 nanosec
