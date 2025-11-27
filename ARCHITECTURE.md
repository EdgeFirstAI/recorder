# Architecture

## Overview

Maivin Recorder is a ROS 2 node that records sensor and perception data to MCAP format files for the EdgeFirst Maivin platform.

## System Architecture

### ROS 2 Node

The Recorder node operates as a ROS 2 component with the following responsibilities:

- Subscribe to configured ROS 2 topics
- Capture message data with timestamps
- Write data to MCAP format files
- Manage file rotation and storage

### Key Components

1. **Topic Subscription**
   - Dynamic topic discovery
   - Selective topic recording
   - Schema management

2. **Data Capture**
   - Message serialization
   - Timestamp synchronization
   - Buffer management

3. **MCAP Writer**
   - File format management
   - Compression
   - Metadata handling
   - Schema storage

## Communication

### Zenoh Integration

The Recorder node uses Zenoh for distributed communication, enabling:

- Low-latency data subscription
- Efficient network utilization
- Zero-copy transfers where applicable

### Data Flow

```
ROS 2 Topics → Recorder Node → MCAP Files
     ↓             ↓               ↓
 Messages     Capture &        Storage
              Serialize
```

## Performance

### Storage Optimization

- Efficient compression algorithms
- Incremental file writing
- Configurable buffer sizes
- File rotation policies

### MCAP Format Benefits

- Self-describing format
- Schema evolution support
- Random access capabilities
- Cross-platform compatibility

## Configuration

Configuration is managed through command-line arguments and environment variables. See `args.rs` for available options.

## Future Enhancements

- Remote recording control
- Cloud upload integration
- Real-time playback
- Advanced filtering and sampling
