# edgefirst_msgs/msg/CameraFrame
# Multi-plane video frame reference.
#
# Carries references (DMA-BUF file descriptors) and/or inlined bytes to one or
# more image planes, plus frame-level metadata.

std_msgs/Header header       # Timestamp and coordinate frame_id
uint64 seq                   # Monotonic frame index for drop detection
uint32 pid                   # Producer pid for pidfd_open; 0 when all planes use inlined data[]
uint32 width                 # Image width in pixels
uint32 height                # Image height in pixels
string format                # Format descriptor (e.g., "NV12", "rgb8", "h264")
string color_space           # Color primaries: "bt709", "bt2020", "srgb", "smpte170m", "" = unknown
string color_transfer        # Transfer function: "bt709", "srgb", "pq", "hlg", "linear", ""
string color_encoding        # YCbCr matrix: "bt601", "bt709", "bt2020", "" (N/A for RGB)
string color_range           # "full", "limited", ""
int32 fence_fd               # DMA-fence sync_file fd; -1 = already signalled / no fence
edgefirst_msgs/CameraPlane[] planes  # Typical len>=1; len=0 permitted for metadata-only frames

================================================================================
MSG: std_msgs/Header
# Standard metadata for higher-level stamped data types.
# This is generally used to communicate timestamped data
# in a particular coordinate frame.

# Two-integer timestamp that is expressed as seconds and nanoseconds.
builtin_interfaces/Time stamp

# Transform frame with which this data is associated.
string frame_id

================================================================================
MSG: builtin_interfaces/Time
# This message communicates ROS Time defined here:
# https://design.ros2.org/articles/clock_and_time.html

# The seconds component, valid over all int32 values.
int32 sec

# The nanoseconds component, valid in the range [0, 10e9].
uint32 nanosec

================================================================================
MSG: edgefirst_msgs/CameraPlane
# Descriptor for a single image plane.

int32 fd                     # DMA-BUF fd, or -1 for inlined data[]
uint32 offset                # Byte offset of plane in fd (ignored when fd == -1)
uint32 stride                # Bytes per line of this plane
uint32 size                  # Plane capacity (buffer span)
uint32 used                  # Valid payload bytes (used <= size)
uint8[] data                 # Inlined plane bytes; non-empty only when fd == -1
