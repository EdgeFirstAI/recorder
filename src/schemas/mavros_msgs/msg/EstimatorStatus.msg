# mavros_msgs/msg/EstimatorStatus
# EKF estimator health flags

std_msgs/Header header
bool attitude_status_flag
bool velocity_horiz_status_flag
bool velocity_vert_status_flag
bool pos_horiz_rel_status_flag
bool pos_horiz_abs_status_flag
bool pos_vert_abs_status_flag
bool pos_vert_agl_status_flag
bool const_pos_mode_status_flag
bool pred_pos_horiz_rel_status_flag
bool pred_pos_horiz_abs_status_flag
bool gps_glitch_status_flag
bool accel_error_status_flag

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
