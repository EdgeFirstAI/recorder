# Per-axis vibration magnitude with explicit measurement semantics
# (RMS/Peak/Peak-to-Peak), physical unit (accel / velocity / displacement),
# and integration band.

std_msgs/Header header

# Per-axis vibration magnitude in the chosen unit/statistic.
# Unknown axis components: NaN (not 0.0).
geometry_msgs/Vector3 vibration

# Integration band (inclusive lower, inclusive upper). NaN = unknown.
float32 band_lower_hz
float32 band_upper_hz

# Broadband statistic reported in `vibration`.
uint8 MEASUREMENT_UNKNOWN      = 0
uint8 MEASUREMENT_RMS          = 1
uint8 MEASUREMENT_PEAK         = 2
uint8 MEASUREMENT_PEAK_TO_PEAK = 3
uint8 measurement_type

# Physical unit of the values in `vibration`.
uint8 UNIT_UNKNOWN           = 0
uint8 UNIT_ACCEL_M_PER_S2    = 1
uint8 UNIT_ACCEL_G           = 2
uint8 UNIT_VELOCITY_MM_PER_S = 3
uint8 UNIT_DISPLACEMENT_UM   = 4
uint8 UNIT_VELOCITY_IN_PER_S = 5
uint8 UNIT_DISPLACEMENT_MIL  = 6
uint8 unit

# Per-axis accelerometer saturation counters (monotonic).
uint32[] clipping

================================================================================
MSG: geometry_msgs/Vector3
# This represents a vector in free space.

float64 x
float64 y
float64 z

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
