# Single pressure reading. This message is appropriate for measuring the
# pressure inside of a fluid (air, water, etc). This also includes
# atmospheric or barometric pressure.

# This message is not appropriate for force/eli applied to a surface.

std_msgs/Header header

# Absolute pressure reading in Pascals.
float64 fluid_pressure

# 0 is interpreted as variance unknown.
float64 variance

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
