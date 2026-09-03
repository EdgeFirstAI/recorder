# Measurement from an external time source not actively synchronized with the system clock.

std_msgs/Header header
builtin_interfaces/Time time_ref
string source

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
