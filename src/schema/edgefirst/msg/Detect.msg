# Detect Message Interface - edgefirst/detect/boxes2d
#
# The detect interface carries detection information 


Header header        # Metadata including timestamp and coordinate frame

Time input_timestamp # Timestamp of the input data (e.g., from camera)
Time model_time      # Timestamp when the object was processed by the model
Time output_time     # Timestamp when the processed output was available

Box[] boxes          # Array of detected object bounding boxes
                     # Each box includes normalized center point, width, and height

================================================================================
# Definition of a bounding box
# Represents a detected object's location in an image
# Normalized coordinates range from 0.0 to 1.0
# (0.0 being the top/left and 1.0 being the bottom/right)
msg Box:
  float32 center_x   # Normalized x-coordinate of the center
  float32 center_y   # Normalized y-coordinate of the center
  float32 width      # Normalized width of the box
  float32 height     # Normalized height of the box
  string label      # object label
  float32 score     # confidence score for detection
  float32 distance  # Distance of object (if known)
  float32 speed     # Speed of object (if known)
  bool is_tracked    # Is the object being tracked
  Track track       # object tracking, each track includes ID and lifetime information

================================================================================
# Definition of an object track
# Represents the trajectory and lifetime of a detected object
msg Track:
  string id           # Unique identifier for the object track
  int32 lifetime     # Number of consecutive frames the object has been tracked
  Time created       # Time the track was first added

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