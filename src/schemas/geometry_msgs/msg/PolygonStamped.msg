# This represents a Polygon with reference coordinate frame and timestamp.

std_msgs/Header header
geometry_msgs/Polygon polygon

================================================================================
MSG: geometry_msgs/Polygon
# A specification of a polygon where the first and last points are assumed to be connected.

geometry_msgs/Point32[] points

================================================================================
MSG: geometry_msgs/Point32
# This contains the position of a point in free space (32-bit).

float32 x
float32 y
float32 z

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
