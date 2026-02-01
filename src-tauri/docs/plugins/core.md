# Core Plugin

The Core plugin provides essential system control capabilities that are always available.

## Capabilities

### Audio Control

**System Audio** - Control system volume with an encoder (rotate to adjust, press to mute).
- `step`: Volume change per encoder tick (default: 0.02 = 2%)

**Mute** - Toggle system audio mute on button press.

**Volume Up/Down** - Dedicated buttons to increase or decrease volume.
- `step`: Volume change per press (default: 0.05 = 5%)

### Microphone Control

**Microphone** - Control microphone volume with an encoder (rotate to adjust, press to mute).
- `step`: Volume change per encoder tick (default: 0.02 = 2%)

**Mic Mute** - Toggle microphone mute on button press.

**Mic Volume Up/Down** - Dedicated buttons to increase or decrease mic volume.
- `step`: Volume change per press (default: 0.05 = 5%)

### Media Control

**Play/Pause** - Toggle media playback.

**Next Track** - Skip to next track.

**Previous Track** - Go to previous track.

**Stop** - Stop media playback.

### Commands

**Run Command** - Execute a shell command on button press.
- `command`: The shell command to run
- `toggle`: If true, track command state for alternate button images

**Launch App** - Launch an application.
- `command`: Application name or path

**Open URL** - Open a URL in the default browser.
- `url`: The URL to open

## Usage Examples

### Volume Control on Encoder

Bind **System Audio** to an encoder for intuitive volume control:
- Rotate clockwise to increase volume
- Rotate counter-clockwise to decrease volume
- Press to toggle mute

### Media Keys

Bind media controls to buttons for quick access:
- Button 1: Play/Pause
- Button 2: Previous
- Button 3: Next

### Quick Launch

Use **Run Command** to launch frequently used apps or scripts:
- `command`: `firefox`
- `command`: `~/.local/bin/my-script.sh`
