# OBS Studio Plugin

Control OBS Studio via WebSocket (obs-websocket 5.x protocol, built into OBS 28+).

## Setup

### Enabling WebSocket Server in OBS

1. Open OBS Studio (version 28 or newer)
2. Go to **Tools > WebSocket Server Settings**
3. Check **Enable WebSocket server**
4. Note the **Server Port** (default: 4455)
5. Optionally set a password for security

### Finding Connection Details

- **Host**: Usually `127.0.0.1` for local OBS
- **Port**: Default is `4455`
- **Password**: Set in OBS WebSocket Server Settings (optional)

### Configuring in Deck Manager

1. Select a button or encoder
2. Choose an OBS capability
3. Enter your OBS connection details
4. Configure capability-specific options

## Capabilities

### Scene

**OBSScene** - Switch to a specific scene.

- `host`: OBS WebSocket host (default: 127.0.0.1)
- `port`: WebSocket port (default: 4455)
- `password`: WebSocket password (if enabled)
- `scene`: Name of the scene to switch to

**Actions:**
- **Button/Encoder press**: Switch to the configured scene

### Stream

**OBSStream** - Control streaming.

- `action`: Toggle, Start, or Stop

**Actions:**
- **Button/Encoder press**: Execute the configured action

### Record

**OBSRecord** - Control recording.

- `action`: Toggle, Start, Stop, or TogglePause

**Actions:**
- **Button/Encoder press**: Execute the configured action

### Source Visibility

**OBSSourceVisibility** - Toggle source visibility in a scene.

- `scene`: Name of the scene containing the source
- `source`: Name of the source to toggle

**Actions:**
- **Button/Encoder press**: Toggle visibility

### Audio

**OBSAudio** - Control audio inputs (volume and mute).

- `input_name`: Name of the audio input in OBS (e.g., "Mic/Aux", "Desktop Audio")
- `step`: Volume change per encoder tick (default: 0.02)

**Actions:**
- **Encoder rotation**: Adjust volume
- **Button/Encoder press**: Toggle mute

### Studio Mode

**OBSStudioMode** - Toggle Studio Mode.

**Actions:**
- **Button/Encoder press**: Toggle Studio Mode on/off

### Replay Buffer

**OBSReplayBuffer** - Control the replay buffer.

- `action`: Toggle, Start, Stop, or Save

**Actions:**
- **Button/Encoder press**: Execute the configured action

### Virtual Camera

**OBSVirtualCam** - Toggle the virtual camera.

**Actions:**
- **Button/Encoder press**: Toggle virtual camera on/off

### Transition

**OBSTransition** - Trigger Studio Mode transition.

**Actions:**
- **Button/Encoder press**: Transition from preview to program (requires Studio Mode)

## Usage Examples

### Stream Control Setup

Bind OBS Stream to a button:
- Set action to "Toggle" for one-button stream control
- Use alternate images to show streaming/not streaming state

### Audio Mixer Setup

Bind OBS Audio to an encoder:
- Rotate to adjust volume (0-100%)
- Press to toggle mute
- Create multiple bindings for different audio sources

### Scene Switching

Create buttons for quick scene changes:
- Bind each button to OBSScene with different scene names
- Button shows active state when that scene is current

### Instant Replay

Bind OBS Replay Buffer with action "Save":
- Press to save the last X seconds (configured in OBS)
- Useful for capturing highlights during streams

## Troubleshooting

### Cannot Connect to OBS

1. **Verify OBS is running**: The WebSocket server only works when OBS is open
2. **Check WebSocket is enabled**: Tools > WebSocket Server Settings
3. **Verify port number**: Default is 4455, not 4444 (that was v4.x)
4. **Check firewall**: Ensure port 4455 is not blocked
5. **Test connection**:
   ```bash
   # Using websocat
   websocat ws://127.0.0.1:4455
   ```

### Authentication Failed

1. **Check password**: Ensure the password matches exactly (case-sensitive)
2. **No password needed**: If WebSocket server has no password, leave the field empty

### Actions Not Working

1. **Scene names are case-sensitive**: Ensure exact match with OBS
2. **Source must exist in scene**: Verify the source is in the specified scene
3. **Input names must match**: Check exact audio input names in OBS

### Remote OBS Control

To control OBS on another computer:
1. Use the remote machine's IP address instead of 127.0.0.1
2. Ensure the WebSocket port is accessible through any firewalls
3. Consider using a password for security

## API Reference

The plugin uses the obs-websocket 5.x protocol:
- Default port: 4455
- Authentication: SHA256-based challenge-response
- Full documentation: https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md
