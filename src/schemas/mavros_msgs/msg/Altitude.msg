# mavros_msgs/msg/Altitude
# Barometric/AMSL/terrain altitude measurements

std_msgs/Header header
float32 monotonic          # Monotonic altitude (local frame)
float32 amsl               # Altitude above mean sea level
float32 local              # Altitude in local frame (NED z-down)
float32 relative           # Relative altitude from home
float32 terrain            # Terrain altitude estimate
float32 bottom_clearance   # Clearance below vehicle

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
