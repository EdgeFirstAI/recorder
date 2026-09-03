# Single reading from a relative humidity sensor.
# Defines the ratio of partial pressure of water vapor to the saturation
# pressure at a temperature.

std_msgs/Header header
float64 relative_humidity
float64 variance

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
