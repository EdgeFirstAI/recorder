# mavros_msgs/msg/SysStatus
# MAVLink SYS_STATUS system health and power telemetry
#
# Field units follow the raw MAVLink SYS_STATUS message definition.
# No unit conversions are applied.

std_msgs/Header header
uint32 sensors_present      # Bitmask of onboard sensors present
uint32 sensors_enabled      # Bitmask of onboard sensors enabled
uint32 sensors_health       # Bitmask of onboard sensors reporting healthy
uint16 load                 # CPU/MCU usage in per-mille (0-1000)
uint16 voltage_battery      # Battery voltage in millivolts (mV)
int16 current_battery       # Battery current in centi-amperes (cA); -1 if unknown
int8 battery_remaining      # Battery remaining 0-100 (%); -1 if not estimated
uint16 drop_rate_comm       # Communication drop rate in per-mille
uint16 errors_comm          # Communication error count
uint16 errors_count1        # Autopilot-specific error count 1
uint16 errors_count2        # Autopilot-specific error count 2
uint16 errors_count3        # Autopilot-specific error count 3
uint16 errors_count4        # Autopilot-specific error count 4

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
