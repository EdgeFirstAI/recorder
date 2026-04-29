# edgefirst_msgs/msg/Track
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
