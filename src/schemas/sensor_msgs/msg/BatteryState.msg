# Constants are chosen to match the enums in the linux kernel
# defined in include/linux/power_supply.h as of version 3.7
# The one difference is for style reasons the constants are
# temporary.

# Power supply status constants
uint8 POWER_SUPPLY_STATUS_UNKNOWN = 0
uint8 POWER_SUPPLY_STATUS_CHARGING = 1
uint8 POWER_SUPPLY_STATUS_DISCHARGING = 2
uint8 POWER_SUPPLY_STATUS_NOT_CHARGING = 3
uint8 POWER_SUPPLY_STATUS_FULL = 4

# Power supply health constants
uint8 POWER_SUPPLY_HEALTH_UNKNOWN = 0
uint8 POWER_SUPPLY_HEALTH_GOOD = 1
uint8 POWER_SUPPLY_HEALTH_OVERHEAT = 2
uint8 POWER_SUPPLY_HEALTH_DEAD = 3
uint8 POWER_SUPPLY_HEALTH_OVERVOLTAGE = 4
uint8 POWER_SUPPLY_HEALTH_UNSPEC_FAILURE = 5
uint8 POWER_SUPPLY_HEALTH_COLD = 6
uint8 POWER_SUPPLY_HEALTH_WATCHDOG_TIMER_EXPIRE = 7
uint8 POWER_SUPPLY_HEALTH_SAFETY_TIMER_EXPIRE = 8

# Power supply technology (chemistry) constants
uint8 POWER_SUPPLY_TECHNOLOGY_UNKNOWN = 0
uint8 POWER_SUPPLY_TECHNOLOGY_NIMH = 1
uint8 POWER_SUPPLY_TECHNOLOGY_LION = 2
uint8 POWER_SUPPLY_TECHNOLOGY_LIPO = 3
uint8 POWER_SUPPLY_TECHNOLOGY_LIFE = 4
uint8 POWER_SUPPLY_TECHNOLOGY_NICD = 5
uint8 POWER_SUPPLY_TECHNOLOGY_LIMN = 6

std_msgs/Header header

# Voltage in Volts (Mandatory)
float32 voltage

# Temperature in Degrees Celsius (If unmeasured NaN)
float32 temperature

# Negative when discharging (A) (If unmeasured NaN)
float32 current

# Current charge in Ah (If unmeasured NaN)
float32 charge

# Capacity in Ah (last full capacity) (If unmeasured NaN)
float32 capacity

# Capacity in Ah (design capacity) (If unmeasured NaN)
float32 design_capacity

# Charge percentage on 0 to 1 range (If unmeasured NaN)
float32 percentage

# The charging status as reported.
uint8 power_supply_status

# The battery health metric.
uint8 power_supply_health

# The battery chemistry.
uint8 power_supply_technology

# True if the battery is present
bool present

# An array of individual cell voltages for each cell in the pack.
# If individual voltages unknown but number of cells known set each to NaN.
float32[] cell_voltage

# An array of individual cell temperatures for each cell in the pack.
# If individual temperatures unknown but number of cells known set each to NaN.
float32[] cell_temperature

# The location of the battery (slot number or plug).
string location

# The best approximation of the battery serial number.
string serial_number

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
