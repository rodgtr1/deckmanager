# ArchDeck

Stream Deck controller for Linux. Configure buttons, encoders, and touch strips with media controls, system commands, app launchers, and Elgato Key Light integration.

## Requirements

- Linux with PipeWire or PulseAudio
- `playerctl` (for media controls)

```bash
# Arch Linux
sudo pacman -S playerctl

# Ubuntu/Debian
sudo apt install playerctl

# Fedora
sudo dnf install playerctl
```

## Installation

### Option A: AppImage (Recommended)

1. Download `ArchDeck.AppImage` from the [Releases](https://github.com/yourusername/archdeck/releases) page

2. Make it executable and run:
   ```bash
   chmod +x ArchDeck.AppImage
   ./ArchDeck.AppImage
   ```

3. Set up udev rules for device access:
   ```bash
   # Download and run the udev setup script
   curl -sSL https://raw.githubusercontent.com/yourusername/archdeck/main/src-tauri/scripts/setup-udev.sh | sudo bash
   ```
   Reconnect your Stream Deck after running this.

4. (Optional) Move to a permanent location:
   ```bash
   mkdir -p ~/.local/bin
   mv ArchDeck.AppImage ~/.local/bin/archdeck
   ```

### Option B: Deb Package (Ubuntu/Debian)

```bash
# Download the .deb from Releases, then:
sudo apt install ./archdeck_*.deb

# Set up udev rules
sudo /usr/share/archdeck/setup-udev.sh
```

### Option C: RPM Package (Fedora)

```bash
# Download the .rpm from Releases, then:
sudo dnf install ./archdeck-*.rpm

# Set up udev rules
sudo /usr/share/archdeck/setup-udev.sh
```

### Option D: Build from Source

<details>
<summary>Click to expand build instructions</summary>

#### Additional build dependencies

```bash
# Arch Linux
sudo pacman -S rust nodejs npm webkit2gtk-4.1

# Ubuntu/Debian
sudo apt install rustc cargo nodejs npm libwebkit2gtk-4.1-dev libgtk-3-dev

# Fedora
sudo dnf install rust cargo nodejs npm webkit2gtk4.1-devel gtk3-devel
```

#### Build

```bash
git clone https://github.com/yourusername/archdeck.git
cd archdeck
npm install
cd src-tauri && cargo build --release && cd ..
```

#### Set up udev rules

```bash
sudo ./src-tauri/scripts/setup-udev.sh
```

Reconnect your Stream Deck after running this.

</details>

## Autostart (Optional)

To run ArchDeck automatically on login in the background:

```bash
# If installed via package:
/usr/share/archdeck/install-autostart.sh

# If built from source:
./src-tauri/scripts/install-autostart.sh
```

Start immediately without rebooting:
```bash
systemctl --user start archdeck
```

## Running

**With GUI:**
```bash
archdeck
# Or for AppImage: ./ArchDeck.AppImage
# Or from source: ./src-tauri/target/release/archdeck
```

**Headless (background):**
```bash
archdeck --hidden
```

## Usage

1. Connect your Stream Deck
2. Launch ArchDeck
3. Click a button or encoder in the device layout
4. Select a capability from the sidebar
5. Configure any parameters
6. Click Save

Bindings are saved to `~/.config/archdeck/bindings.toml`.

## Service Management

```bash
systemctl --user status archdeck    # Check status
systemctl --user stop archdeck      # Stop
systemctl --user restart archdeck   # Restart
journalctl --user -u archdeck -f    # View logs
```

To disable autostart:
```bash
# If installed via package:
/usr/share/archdeck/uninstall-autostart.sh

# If built from source:
./src-tauri/scripts/uninstall-autostart.sh
```

## Troubleshooting

**Device not detected:**
- Ensure udev rules are installed and reconnect the device
- Check device is visible: `lsusb | grep Elgato`
- Verify permissions: `ls -la /dev/hidraw*`

**Service won't start:**
- Check logs: `journalctl --user -u archdeck -f`
- Ensure Wayland/X11 session is active

**Media controls not working:**
- Install `playerctl`
- Verify a media player is running: `playerctl status`
