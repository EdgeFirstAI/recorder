# mavros_msgs/msg/TimesyncStatus
# Time synchronization offset and round-trip metrics

std_msgs/Header header
uint64 remote_timestamp_ns     # Remote system timestamp (ns)
int64 observed_offset_ns       # Observed clock offset (ns)
int64 estimated_offset_ns      # Estimated (filtered) clock offset (ns)
float32 round_trip_time_ms     # Round-trip time (ms)

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
