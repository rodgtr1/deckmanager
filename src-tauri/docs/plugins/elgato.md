# Elgato Key Light Plugin

Control Elgato Key Light devices over your local network.

## Setup

### Finding Your Key Light IP

1. Open the Elgato Control Center app
2. Click on your Key Light device
3. Look for the IP address in device settings
4. Note: The default port is **9123**

Alternatively, use network scanning:
```bash
# Using nmap
nmap -p 9123 192.168.1.0/24

# Using avahi (mDNS)
avahi-browse -rt _elg._tcp
```

### Configuring in ArchDeck

1. Select a button or encoder
2. Choose "Key Light" capability
3. Enter the IP address of your Key Light
4. The port defaults to 9123 (change if needed)

## Capabilities

### Key Light Control

**Elgato Key Light** - Full control over your Key Light.
- `ip`: IP address of the Key Light (e.g., "192.168.1.100")
- `port`: HTTP API port (default: 9123)

**Actions:**
- **Button press**: Toggle light on/off
- **Encoder press**: Toggle light on/off
- **Encoder rotation**: Adjust brightness

## Usage Examples

### Encoder Setup (Recommended)

Bind Key Light to an encoder for the best experience:
- Rotate to smoothly adjust brightness
- Press to toggle on/off
- Visual feedback updates automatically

### Button Setup

Use a button for simple on/off toggle:
- Press to toggle light state
- Use alternate images to show on/off state

## Troubleshooting

### Light Not Responding

1. **Check network connectivity**: Ensure your computer and Key Light are on the same network
2. **Verify IP address**: IP may change if using DHCP; consider setting a static IP
3. **Check firewall**: Ensure port 9123 is not blocked
4. **Test with curl**:
   ```bash
   curl http://192.168.1.100:9123/elgato/lights
   ```

### Brightness Changes Not Smooth

The plugin uses debouncing to batch rapid adjustments. This is normal behavior to prevent overwhelming the device with requests.

### Multiple Key Lights

Create separate bindings for each Key Light with different IP addresses. Each binding controls one specific device.

## API Reference

The Key Light uses a REST API on port 9123:

- `GET /elgato/lights` - Get current state
- `PUT /elgato/lights` - Set state (on, brightness, temperature)

The plugin handles all API communication automatically.
