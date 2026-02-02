# Deck Manager — Open-Source Stream Deck Software for Linux

Configure your Elgato Stream Deck on Linux with a modern UI and extensible plugin system.

![License](https://img.shields.io/badge/license-MIT-blue)
![Platform](https://img.shields.io/badge/platform-Linux-lightgrey)

## Features

- Works with all Elgato Stream Deck models (Original, Mini, XL, MK.2, Plus, Pedal, Neo)
- Media controls, volume, app launchers, custom commands
- Elgato Key Light integration
- OBS Studio control
- Plugin system for custom integrations
- Runs on Wayland and X11

## Quick Install

```bash
curl -sSL https://raw.githubusercontent.com/rodgtr1/deckmanager/main/install.sh | bash
```

Supports Arch, Debian, Fedora, and derivatives.

## Usage

1. Plug in your Stream Deck
2. Run `deckmanager`
3. Click any button in the UI
4. Choose an action and configure it
5. Click Save

Configuration is stored in `~/.config/deckmanager/bindings.toml`.

## Architecture: Core + Plugins

Deck Manager separates **core functionality** from **plugins**:

### Core (always included)
- Media playback controls (play/pause, next, previous)
- System volume and mute
- Run shell commands
- Open applications and URLs
- Multi-action sequences
- Button images and labels

### Plugins (optional, feature-flagged)
| Plugin | Feature Flag | Description |
|--------|--------------|-------------|
| Elgato | `plugin-elgato` | Key Light brightness and color control |
| OBS | `plugin-obs` | Scene switching, recording, streaming |

Plugins are compiled in via Cargo feature flags. To build without OBS support:
```bash
cargo build --release --no-default-features --features plugin-elgato
```

## Contributing Plugins

Plugins extend Deck Manager with new capabilities. Each plugin:
- Provides actions that can be bound to buttons/encoders
- Handles input events
- Can maintain state (e.g., "is muted?")

### Quick Start

1. Add feature flag to `Cargo.toml`
2. Create `src/plugins/yourplugin/` directory
3. Implement the `Plugin` trait
4. Register in `src/lib.rs`
5. Add TypeScript types in `src/types.ts`

See [PLUGIN_API.md](PLUGIN_API.md) for the full guide.

### Plugin Ideas
- Philips Hue / Home Assistant
- Spotify / Tidal
- Discord mute/deafen
- Keyboard macros
- MIDI control

## Building from Source

```bash
# Dependencies (Arch)
sudo pacman -S rust npm webkit2gtk-4.1 gtk3 hidapi

# Build
git clone https://github.com/rodgtr1/deckmanager
cd deckmanager
npm ci
npm run tauri build
```

## Troubleshooting

**Device not detected:** Install udev rules and replug the device:
```bash
sudo wget https://raw.githubusercontent.com/rodgtr1/deckmanager/main/src-tauri/scripts/70-streamdeck.rules -O /etc/udev/rules.d/70-streamdeck.rules
sudo udevadm control --reload-rules && sudo udevadm trigger
```

**Blank window on Wayland:**
```bash
GDK_BACKEND=x11 deckmanager
```

## License

MIT
