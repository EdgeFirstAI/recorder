# mavros_msgs/msg/StatusText
# MAVLink STATUSTEXT message

# MAV_SEVERITY constants
uint8 EMERGENCY = 0
uint8 ALERT = 1
uint8 CRITICAL = 2
uint8 ERROR = 3
uint8 WARNING = 4
uint8 NOTICE = 5
uint8 INFO = 6
uint8 DEBUG = 7

std_msgs/Header header
uint8 severity             # MAV_SEVERITY level
string text                # Status text message

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
