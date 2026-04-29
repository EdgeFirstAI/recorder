# mavros_msgs/msg/ExtendedState
# VTOL and landed state

# VTOL state constants
uint8 VTOL_STATE_UNDEFINED = 0
uint8 VTOL_STATE_TRANSITION_TO_FW = 1
uint8 VTOL_STATE_TRANSITION_TO_MC = 2
uint8 VTOL_STATE_MC = 3
uint8 VTOL_STATE_FW = 4

# Landed state constants
uint8 LANDED_STATE_UNDEFINED = 0
uint8 LANDED_STATE_ON_GROUND = 1
uint8 LANDED_STATE_IN_AIR = 2
uint8 LANDED_STATE_TAKEOFF = 3
uint8 LANDED_STATE_LANDING = 4

std_msgs/Header header
uint8 vtol_state
uint8 landed_state

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
