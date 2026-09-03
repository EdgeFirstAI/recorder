# This hold basic information about the characteristics of the OccupancyGrid.

builtin_interfaces/Time map_load_time
float32 resolution
uint32 width
uint32 height
geometry_msgs/Pose origin

================================================================================
MSG: geometry_msgs/Pose
# A representation of pose in free space, composed of position and orientation.

geometry_msgs/Point position
geometry_msgs/Quaternion orientation

================================================================================
MSG: geometry_msgs/Point
# This contains the position of a point in free space.

float64 x
float64 y
float64 z

================================================================================
MSG: geometry_msgs/Quaternion
# This represents an orientation in free space in quaternion form.

float64 x 0
float64 y 0
float64 z 0
float64 w 1

================================================================================
MSG: builtin_interfaces/Time
# This message communicates ROS Time defined here:
# https://design.ros2.org/articles/clock_and_time.html

int32 sec
uint32 nanosec
