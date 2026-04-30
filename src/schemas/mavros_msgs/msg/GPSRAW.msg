# mavros_msgs/msg/GPSRAW
# MAVLink GPS_RAW_INT raw GNSS fix data
#
# Field units follow the raw MAVLink GPS_RAW_INT message definition.
# No unit conversions are applied — values are lossless from the wire.

# GPS fix type constants
uint8 GPS_FIX_TYPE_NO_GPS = 0
uint8 GPS_FIX_TYPE_NO_FIX = 1
uint8 GPS_FIX_TYPE_2D_FIX = 2
uint8 GPS_FIX_TYPE_3D_FIX = 3
uint8 GPS_FIX_TYPE_DGPS = 4
uint8 GPS_FIX_TYPE_RTK_FLOAT = 5
uint8 GPS_FIX_TYPE_RTK_FIXED = 6
uint8 GPS_FIX_TYPE_STATIC = 7
uint8 GPS_FIX_TYPE_PPP = 8

std_msgs/Header header
uint8 fix_type             # GPS fix type (see constants above)
int32 lat                  # Latitude (degE7)
int32 lon                  # Longitude (degE7)
int32 alt                  # Altitude above MSL (mm)
uint16 eph                 # GPS HDOP (cm); UINT16_MAX if unknown
uint16 epv                 # GPS VDOP (cm); UINT16_MAX if unknown
uint16 vel                 # GPS ground speed (cm/s); UINT16_MAX if unknown
uint16 cog                 # Course over ground (cdeg, 0-35999); UINT16_MAX if unknown
uint8 satellites_visible   # Number of satellites visible; UINT8_MAX if unknown
int32 alt_ellipsoid        # Altitude above WGS84 ellipsoid (mm)
uint32 h_acc               # Horizontal position uncertainty (mm)
uint32 v_acc               # Vertical position uncertainty (mm)
uint32 vel_acc             # Speed uncertainty (mm/s)
int32 hdg_acc              # Heading uncertainty (cdeg × 10^-5)
uint16 yaw                 # Yaw (cdeg); 0 if unknown
uint8 dgps_numch           # Number of DGPS satellites
uint32 dgps_age            # Age of DGPS correction (ms)

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
