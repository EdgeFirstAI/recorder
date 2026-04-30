# edgefirst_msgs/msg/Box
# The Box message has the box attributes

float32 center_x   # Normalized x-coordinate of the center
float32 center_y   # Normalized y-coordinate of the center
float32 width      # Normalized width of the box
float32 height     # Normalized height of the box
string label       # object label
float32 score      # confidence score for detection
float32 distance   # Distance of object (if known)
float32 speed      # Speed of object (if known)
edgefirst_msgs/Track track       # object tracking, each track includes ID and lifetime information

================================================================================
MSG: edgefirst_msgs/Track
# The Track message has the track attributes

string id           # Unique identifier for the object track (if the object is not tracked then "")
int32 lifetime      # Number of consecutive frames the object has been tracked
builtin_interfaces/Time created       # Time the track was first added

================================================================================
MSG: builtin_interfaces/Time
# This message communicates ROS Time defined here:
# https://design.ros2.org/articles/clock_and_time.html

# The seconds component, valid over all int32 values.
int32 sec

# The nanoseconds component, valid in the range [0, 10e9].
uint32 nanosec
