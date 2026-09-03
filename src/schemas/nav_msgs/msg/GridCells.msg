# An array of cells in a 2D grid.

std_msgs/Header header
float32 cell_width
float32 cell_height
geometry_msgs/Point[] cells

================================================================================
MSG: geometry_msgs/Point
# This contains the position of a point in free space.

float64 x
float64 y
float64 z

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
