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

builtin_interfaces/Time stamp
string frame_id

================================================================================
MSG: builtin_interfaces/Time
# Time indicates a specific point in time, relative to a clock's 0 point.

int32 sec
uint32 nanosec
