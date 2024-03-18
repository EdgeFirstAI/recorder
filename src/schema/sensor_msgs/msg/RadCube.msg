# Radar Cube Message Interface - edgefirst/mst/RadarCube
#
# The RadarCube interface carries various radar cube reprensentations of the
# Radar FFT before generally being processed by CFAR into a point cloud.  The
# cube coud be R, RD, RAD, RA, and so on where R=Range, D=Dopper, and A=Azimuth.

# Dimensional labels are used to describe the radar cube layout.  Not all cubes
# include every label.
uint8 RANGE = 0
uint8 DOPPLER = 1
uint8 AZIMUTH = 2
uint8 ELEVATION = 3
uint8 RXCHANNEL = 4
uint8 SEQUENCE = 5

std_msgs/Header header
        builtin_interfaces/Time stamp
                int32 sec
                uint32 nanosec
        string frame_id

uint8[] layout      # Radar cube layout provides labels for each dimensions
uint16[] shape      # Radar cube shape provides the shape of each dimensions
float32[] scales    # The scaling factors for the dimensions representing bins.
                    # For dimensions taken "as-is" the scale will be 1.0.
int16[] cube        # The radar cube data as 16bit integers.  If the is_complex
                    # is true then each element will be pairs of integers with
                    # the first being real and the second being imaginary.
bool is_complex     # True if the radar cube is complex

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