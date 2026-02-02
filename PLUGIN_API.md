# Deck Manager Plugin API

Guide for creating plugins that extend Deck Manager's capabilities.

## Current Plugins

| Plugin | Feature Flag | Capabilities |
|--------|--------------|--------------|
| Elgato | `plugin-elgato` | Key Light on/off/toggle, brightness control |
| OBS | `plugin-obs` | Scene switching, stream/record control, audio, virtual cam, replay buffer |

## Plugin Trait

All plugins implement the `Plugin` trait:

```rust
pub trait Plugin: Send + Sync {
    // Required
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn category(&self) -> &'static str;
    fn capabilities(&self) -> Vec<CapabilityMetadata>;
    fn handle_event(&self, event: &LogicalEvent, binding: &Binding, state: &Arc<Mutex<SystemState>>) -> bool;
    fn owns_capability(&self, capability_type: &str) -> bool;
    fn is_active(&self, binding: &Binding, state: &SystemState) -> bool;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    // Optional
    fn initialize(&mut self, config: &PluginConfig) -> anyhow::Result<()> { Ok(()) }
    fn shutdown(&mut self) {}
    fn version(&self) -> &'static str { "1.0.0" }
    fn description(&self) -> &'static str { "" }
    fn icon(&self) -> &'static str { "plug" }
    fn is_core(&self) -> bool { false }
}
```

## Creating a Plugin

### 1. Add Feature Flag

In `Cargo.toml`:

```toml
[features]
default = ["plugin-elgato", "plugin-obs", "plugin-myplugin"]
plugin-myplugin = ["some-optional-dep"]
```

### 2. Create Module

```
src/plugins/
├── mod.rs
└── myplugin/
    ├── mod.rs
    └── plugin.rs
```

In `src/plugins/mod.rs`:

```rust
#[cfg(feature = "plugin-myplugin")]
pub mod myplugin;
```

### 3. Add Capability Variant

In `src/capability.rs`:

```rust
pub enum Capability {
    // ... existing
    MyFeature { param1: String },
}
```

And in `CapabilityEffect`:

```rust
pub enum CapabilityEffect {
    // ... existing
    MyFeatureActivate { param1: String },
}
```

### 4. Implement the Plugin

```rust
// src/plugins/myplugin/plugin.rs
use crate::binding::Binding;
use crate::capability::Capability;
use crate::impl_owns_capability;
use crate::input_processor::LogicalEvent;
use crate::plugin::{CapabilityMetadata, ParameterDef, ParameterType, Plugin, PluginConfig};
use crate::state_manager::SystemState;
use std::any::Any;
use std::sync::{Arc, Mutex};

pub struct MyPlugin;

impl MyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for MyPlugin {
    fn id(&self) -> &'static str { "myplugin" }
    fn name(&self) -> &'static str { "My Plugin" }
    fn category(&self) -> &'static str { "Custom" }

    fn capabilities(&self) -> Vec<CapabilityMetadata> {
        vec![CapabilityMetadata {
            id: "MyFeature",
            name: "My Feature",
            description: "Does something cool",
            plugin_id: "myplugin",
            supports_button: true,
            supports_encoder: false,
            supports_encoder_press: false,
            parameters: vec![
                ParameterDef {
                    name: "param1",
                    param_type: ParameterType::String,
                    default_value: "default",
                    description: "The parameter",
                },
            ],
        }]
    }

    fn handle_event(
        &self,
        event: &LogicalEvent,
        binding: &Binding,
        _state: &Arc<Mutex<SystemState>>,
    ) -> bool {
        match (&binding.capability, event) {
            (Capability::MyFeature { param1 }, LogicalEvent::Button(e)) if e.pressed => {
                println!("MyFeature activated: {}", param1);
                // Do the thing
                true
            }
            _ => false,
        }
    }

    impl_owns_capability!("MyFeature");

    fn is_active(&self, _binding: &Binding, _state: &SystemState) -> bool {
        false
    }

    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }

    fn icon(&self) -> &'static str { "sparkles" }
    fn description(&self) -> &'static str { "My awesome plugin" }
}
```

### 5. Register the Plugin

In `src/lib.rs`:

```rust
fn create_plugin_registry() -> Arc<PluginRegistry> {
    let registry = PluginRegistry::new();

    #[cfg(feature = "plugin-myplugin")]
    registry.register(
        Box::new(plugins::myplugin::MyPlugin::new()),
        Some(&make_config("myplugin", true)),
    );

    Arc::new(registry)
}
```

### 6. Add Frontend Types

In `src/types.ts`:

```typescript
export type Capability =
  | { type: "MyFeature"; param1: string }
  // ... existing types
```

And in `createDefaultCapability`:

```typescript
case "MyFeature":
  return { type: "MyFeature", param1: "default" };
```

## Parameter Types

| Type | Description |
|------|-------------|
| `Float` | Floating point number |
| `Integer` | Integer number |
| `String` | Text string |
| `Bool` | Boolean flag |
| `IpAddress` | IP address string |

## Event Types

```rust
pub enum LogicalEvent {
    Button(ButtonEvent),       // press/release
    Encoder(EncoderEvent),     // rotation delta
    EncoderPress(ButtonEvent), // encoder button
    Swipe(SwipeEvent),         // touch strip
}
```

## State Management

For stateful capabilities (toggles, etc.):

1. Add state to `SystemState` struct
2. Update state in `handle_event`
3. Return `true` from `is_active` when binding should show active state
4. UI shows alternate button image when active

## Best Practices

1. **Non-blocking**: Use background threads for network/IO
2. **Error handling**: Log errors, return `false` from `handle_event` on failure
3. **Debouncing**: Batch rapid encoder events to avoid overwhelming external devices
4. **Graceful degradation**: Handle disconnected devices, network failures

## Plugin Ideas

- Philips Hue / Home Assistant
- Spotify / Tidal
- Discord mute/deafen
- Keyboard macros (xdotool)
- MQTT publisher
- Zoom/Teams mute
