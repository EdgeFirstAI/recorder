# Model Message Interface - edgefirst/msg/Model
#
# The model interface carries model inference results.

std_msgs/Header header        # Metadata including timestamp and coordinate frame

builtin_interfaces/Duration input_time      # Duration to load inputs into the model
builtin_interfaces/Duration model_time      # Duration to run the model, not including input/output/decoding
builtin_interfaces/Duration output_time     # Duration to read outputs from the model
builtin_interfaces/Duration decode_time     # Duration to decode the outputs from the model, including nms and tracking.

edgefirst_msgs/Box[] boxes          # Array of detected object bounding boxes
                                    # Each box includes normalized center point, width, and height
edgefirst_msgs/Mask[] masks         # Segmentation mask from the model.

================================================================================
MSG: edgefirst_msgs/Box
# Box Message Interface - edgefirst/msg/Box
#
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
# Definition of an object track
# Represents the trajectory and lifetime of a detected object

string id           # Unique identifier for the object track
int32 lifetime     # Number of consecutive frames the object has been tracked
builtin_interfaces/Time created       # Time the track was first added

================================================================================
MSG: edgefirst_msgs/Mask
# Segmentation Mask as sent by the Model message.  Masks can be classical
# segmentation masks or occupancy grids, including voxel grids when using 3D
# masks (width, height, and length are non-zero).

uint32 height       # The height of the mask
uint32 width        # The width of the mask
uint32 length       # The length of the mask when using 3D masks.

string encoding     # The optional encoding for the mask (currently unused).

uint8[] mask        # The segmentation mask

bool boxed          # If the mask is matched to a box for instance segmentation

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

================================================================================
MSG: builtin_interfaces/Duration
# This message communicates ROS Duration defined here:
# https://design.ros2.org/articles/clock_and_time.html

# The seconds component, valid over all int32 values.
int32 sec

# The nanoseconds component, valid in the range [0, 10e9].
uint32 nanosec