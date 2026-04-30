# mavros_msgs/msg/VfrHud
# VFR head-up display telemetry

std_msgs/Header header
float32 airspeed       # Airspeed (m/s)
float32 groundspeed    # Ground speed (m/s)
int16 heading          # Heading (degrees, 0..359)
float32 throttle       # Throttle (0.0..1.0)
float32 altitude       # Altitude (m)
float32 climb          # Climb rate (m/s)

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
