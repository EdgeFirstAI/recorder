# edgefirst_msgs/msg/CameraPlane
# Descriptor for a single image plane.
#
# Two exclusive delivery modes:
#   fd >= 0  : plane bytes live in DMA-BUF; consumer uses offset/size/used
#              to mmap. `data` MUST be empty.
#   fd == -1 : plane bytes are inlined in `data`; offset is ignored;
#              size and used describe `data` length.

int32 fd                     # DMA-BUF fd, or -1 for inlined data[]
uint32 offset                # Byte offset of plane in fd (ignored when fd == -1)
uint32 stride                # Bytes per line of this plane
uint32 size                  # Plane capacity (buffer span)
uint32 used                  # Valid payload bytes (used <= size)
uint8[] data                 # Inlined plane bytes; non-empty only when fd == -1
