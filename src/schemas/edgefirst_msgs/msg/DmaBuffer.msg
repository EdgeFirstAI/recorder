# DMA Buffer Message Interface - edgefirst/dma/buffer
#
# The dma interface carries raw camera information 


std_msgs/Header header        # Metadata including timestamp and coordinate frame

uint32 pid        # The process id of the service that created the DMA buffer
int32 fd          # The file descriptor of the DMA buffer
uint32 width      # The width of the image in pixels
uint32 height     # The height of the image in pixels
uint32 stride     # The stride of the image in bytes
uint32 fourcc     # The fourcc code of the image
uint32 length     # The length of the DMA buffer in bytes, used to mmap the buffer

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