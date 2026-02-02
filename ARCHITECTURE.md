# Deck Manager Architecture

Stream Deck controller for Linux built with Tauri (Rust + React).

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         React Frontend                          │
│                    (receives events via Tauri)                  │
└─────────────────────────────────────────────────────────────────┘
                                ▲
                                │ emit_event()
┌─────────────────────────────────────────────────────────────────┐
│                      Plugin Registry                            │
│         Routes events to appropriate plugin handlers            │
└─────────────────────────────────────────────────────────────────┘
                                ▲
                                │
┌─────────────────────────────────────────────────────────────────┐
│                      InputProcessor                             │
│         Converts raw input → LogicalEvent (normalized)          │
└─────────────────────────────────────────────────────────────────┘
                                ▲
                                │
┌─────────────────────────────────────────────────────────────────┐
│                   elgato-streamdeck crate                       │
│              StreamDeckInput (raw HID events)                   │
└─────────────────────────────────────────────────────────────────┘
                                ▲
                                │
┌─────────────────────────────────────────────────────────────────┐
│                      Stream Deck Hardware                       │
└─────────────────────────────────────────────────────────────────┘
```

## Event Flow

1. **Raw Input**: `elgato-streamdeck` crate reads HID reports
2. **Normalization**: `InputProcessor` converts to `LogicalEvent`
3. **Dual Dispatch**:
   - **Frontend**: Events emitted via Tauri for UI updates
   - **Backend**: Plugin registry routes to appropriate handler

```
StreamDeckInput (raw)
        │
        ▼
InputProcessor.process_*()
        │
        ▼
LogicalEvent (normalized)
        │
        ├──────────────────┐
        ▼                  ▼
emit_event()        PluginRegistry.handle_event()
(→ React UI)               │
                           ▼
                    Plugin.handle_event()
                           │
                           ▼
                    CapabilityEffect
                           │
                           ▼
                    System command / API call
```

## Core Data Structures

### LogicalEvent
Normalized input events from the Stream Deck:

```rust
pub enum LogicalEvent {
    Button(ButtonEvent),       // Physical button press/release
    Encoder(EncoderEvent),     // Rotary encoder twist (delta)
    EncoderPress(ButtonEvent), // Encoder push button
    Swipe(TouchSwipeEvent),    // Touch screen swipe gesture
}
```

### InputRef
References a specific input for binding configuration:

```rust
pub enum InputRef {
    Button { index: usize },
    Encoder { index: usize },
    EncoderPress { index: usize },
    Swipe,
}
```

### Capability
Actions that can be bound to inputs. Core capabilities:

```rust
pub enum Capability {
    // Audio
    SystemAudio { step: f32 },
    Mute,
    VolumeUp { step: f32 },
    VolumeDown { step: f32 },
    Microphone { step: f32 },
    MicMute,

    // Media
    MediaPlayPause,
    MediaNext,
    MediaPrevious,
    MediaStop,

    // System
    RunCommand { command: String, toggle: bool },
    LaunchApp { command: String },
    OpenURL { url: String },

    // Plugins (feature-flagged)
    ElgatoKeyLight { ip, port, action },
    OBSScene { host, port, password, scene },
    OBSStream { ... },
    OBSRecord { ... },
    // ... more OBS capabilities
}
```

### Binding
Maps an input to a capability:

```rust
pub struct Binding {
    pub input: InputRef,
    pub capability: Capability,
}
```

## Module Responsibilities

| Module | Responsibility |
|--------|----------------|
| `lib.rs` | App initialization, spawns backend thread |
| `streamdeck.rs` | Main event loop, device communication |
| `input_processor.rs` | Raw → LogicalEvent normalization |
| `binding.rs` | Input→Capability mapping, serialization |
| `capability.rs` | Capability definitions, effect generation |
| `config.rs` | Config file loading/saving |
| `plugin/` | Plugin trait, registry, metadata |
| `plugins/` | Plugin implementations (elgato, obs) |
| `state_manager.rs` | Shared state (mute status, etc.) |

## Plugin System

Plugins are feature-flagged and implement the `Plugin` trait:

```rust
pub trait Plugin: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn capabilities(&self) -> Vec<CapabilityMetadata>;
    fn handle_event(&self, event: &LogicalEvent, binding: &Binding, state: &Arc<Mutex<SystemState>>) -> bool;
    fn owns_capability(&self, capability_type: &str) -> bool;
    // ...
}
```

Current plugins:
- `plugin-elgato` - Elgato Key Light control
- `plugin-obs` - OBS Studio integration

See [PLUGIN_API.md](PLUGIN_API.md) for creating new plugins.

## Configuration

Bindings stored in `~/.config/deckmanager/bindings.toml`:

```toml
[[bindings]]
input = { type = "Button", index = 0 }
capability = { type = "MediaPlayPause" }

[[bindings]]
input = { type = "Encoder", index = 0 }
capability = { type = "SystemAudio", step = 0.02 }
```
