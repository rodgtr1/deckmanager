import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import CapabilityBrowser from "./components/CapabilityBrowser";
import DeviceLayout from "./components/DeviceLayout";
import BindingEditor from "./components/BindingEditor";
import {
  DeviceInfo,
  InputRef,
  Capability,
  Binding,
  CapabilityInfo,
  ButtonEvent,
  EncoderEvent,
  TouchSwipeEvent,
  SystemState,
} from "./types";
import "./App.css";

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [bindings, setBindings] = useState<Binding[]>([]);
  const [capabilities, setCapabilities] = useState<CapabilityInfo[]>([]);
  const [selectedInput, setSelectedInput] = useState<InputRef | null>(null);
  const [selectedCapabilityId, setSelectedCapabilityId] = useState<string | null>(null);
  const [activeInputs, setActiveInputs] = useState<Set<string>>(new Set());
  const [systemState, setSystemState] = useState<SystemState>({ is_muted: false, is_playing: false });
  const [error, setError] = useState<string | null>(null);

  // Load initial data
  useEffect(() => {
    const loadData = async () => {
      try {
        const [deviceInfo, bindingsList, capsList, state] = await Promise.all([
          invoke<DeviceInfo | null>("get_device_info"),
          invoke<Binding[]>("get_bindings"),
          invoke<CapabilityInfo[]>("get_capabilities"),
          invoke<SystemState>("get_system_state"),
        ]);

        setDevice(deviceInfo);
        setBindings(bindingsList);
        setCapabilities(capsList);
        setSystemState(state);
      } catch (e) {
        setError(`Failed to load: ${e}`);
      }
    };

    loadData();
  }, []);

  // Listen for system state changes
  useEffect(() => {
    const unlistenState = listen<SystemState>("state:change", (e) => {
      setSystemState(e.payload);
    });

    return () => {
      unlistenState.then((f) => f());
    };
  }, []);

  // Listen for Stream Deck events
  useEffect(() => {
    const unlistenButton = listen<ButtonEvent>("streamdeck:button", (e) => {
      const key = `Button:${e.payload.index}`;
      setActiveInputs((prev) => {
        const next = new Set(prev);
        if (e.payload.pressed) {
          next.add(key);
        } else {
          next.delete(key);
        }
        return next;
      });
    });

    const unlistenEncoder = listen<EncoderEvent>("streamdeck:encoder", (e) => {
      // Flash the encoder briefly on rotation
      const key = `Encoder:${e.payload.index}`;
      setActiveInputs((prev) => new Set(prev).add(key));
      setTimeout(() => {
        setActiveInputs((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }, 150);
    });

    const unlistenEncoderPress = listen<ButtonEvent>(
      "streamdeck:encoder-press",
      (e) => {
        const key = `EncoderPress:${e.payload.index}`;
        setActiveInputs((prev) => {
          const next = new Set(prev);
          if (e.payload.pressed) {
            next.add(key);
          } else {
            next.delete(key);
          }
          return next;
        });
      }
    );

    const unlistenSwipe = listen<TouchSwipeEvent>("streamdeck:swipe", () => {
      // Flash the touch strip briefly on swipe
      const key = "swipe";
      setActiveInputs((prev) => new Set(prev).add(key));
      setTimeout(() => {
        setActiveInputs((prev) => {
          const next = new Set(prev);
          next.delete(key);
          return next;
        });
      }, 200);
    });

    return () => {
      unlistenButton.then((f) => f());
      unlistenEncoder.then((f) => f());
      unlistenEncoderPress.then((f) => f());
      unlistenSwipe.then((f) => f());
    };
  }, []);

  // Handle setting a binding
  const handleSetBinding = useCallback(
    async (
      input: InputRef,
      capability: Capability,
      icon?: string,
      label?: string,
      buttonImage?: string,
      buttonImageAlt?: string,
      showLabel?: boolean
    ) => {
      try {
        const params = {
          input,
          capability,
          icon: icon ?? null,
          label: label ?? null,
          button_image: buttonImage ?? null,
          button_image_alt: buttonImageAlt ?? null,
          show_label: showLabel ?? null,
        };
        console.log("handleSetBinding params:", params);
        await invoke("set_binding", { params });
        // Refresh bindings
        const updated = await invoke<Binding[]>("get_bindings");
        console.log("Updated bindings:", updated);
        setBindings(updated);
        // Auto-save to disk
        await invoke("save_bindings");
        setError(null);
      } catch (e) {
        setError(`Failed to set binding: ${e}`);
      }
    },
    []
  );

  // Handle removing a binding
  const handleRemoveBinding = useCallback(async (input: InputRef) => {
    try {
      await invoke("remove_binding", { input });
      // Refresh bindings
      const updated = await invoke<Binding[]>("get_bindings");
      setBindings(updated);
      // Auto-save to disk
      await invoke("save_bindings");
      setError(null);
    } catch (e) {
      setError(`Failed to remove binding: ${e}`);
    }
  }, []);

  // Handle capability selection from browser (for click-to-assign flow)
  const handleCapabilitySelect = useCallback((capabilityId: string) => {
    setSelectedCapabilityId(capabilityId);
    // If an input is already selected, assign the capability
    if (selectedInput) {
      const capInfo = capabilities.find(c => c.id === capabilityId);
      if (capInfo) {
        // Create default capability object based on type
        let capability: Capability;
        switch (capabilityId) {
          case "SystemVolume":
            capability = { type: "SystemVolume", step: 0.02 };
            break;
          case "ToggleMute":
            capability = { type: "ToggleMute" };
            break;
          case "MediaPlayPause":
            capability = { type: "MediaPlayPause" };
            break;
          case "MediaNext":
            capability = { type: "MediaNext" };
            break;
          case "MediaPrevious":
            capability = { type: "MediaPrevious" };
            break;
          case "MediaStop":
            capability = { type: "MediaStop" };
            break;
          case "RunCommand":
            capability = { type: "RunCommand", command: "" };
            break;
          case "LaunchApp":
            capability = { type: "LaunchApp", command: "" };
            break;
          case "OpenURL":
            capability = { type: "OpenURL", url: "https://" };
            break;
          case "ElgatoKeyLightToggle":
            capability = { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "Toggle" };
            break;
          case "ElgatoKeyLightBrightness":
            capability = { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "SetBrightness" };
            break;
          default:
            return;
        }
        handleSetBinding(selectedInput, capability);
      }
    }
  }, [selectedInput, capabilities, handleSetBinding]);

  // Handle drop from capability browser onto device layout
  const handleDrop = useCallback((input: InputRef, capabilityId: string) => {
    const capInfo = capabilities.find(c => c.id === capabilityId);
    if (!capInfo) return;

    // Check if this capability is supported for this input type
    const isSupported = (
      (input.type === "Button" && capInfo.supports_button) ||
      (input.type === "Encoder" && capInfo.supports_encoder) ||
      (input.type === "EncoderPress" && capInfo.supports_encoder_press)
    );
    if (!isSupported) return;

    // Create default capability object
    let capability: Capability;
    switch (capabilityId) {
      case "SystemVolume":
        capability = { type: "SystemVolume", step: 0.02 };
        break;
      case "ToggleMute":
        capability = { type: "ToggleMute" };
        break;
      case "MediaPlayPause":
        capability = { type: "MediaPlayPause" };
        break;
      case "MediaNext":
        capability = { type: "MediaNext" };
        break;
      case "MediaPrevious":
        capability = { type: "MediaPrevious" };
        break;
      case "MediaStop":
        capability = { type: "MediaStop" };
        break;
      case "RunCommand":
        capability = { type: "RunCommand", command: "" };
        break;
      case "LaunchApp":
        capability = { type: "LaunchApp", command: "" };
        break;
      case "OpenURL":
        capability = { type: "OpenURL", url: "https://" };
        break;
      case "ElgatoKeyLightToggle":
        capability = { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "Toggle" };
        break;
      case "ElgatoKeyLightBrightness":
        capability = { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "SetBrightness" };
        break;
      default:
        return;
    }

    handleSetBinding(input, capability);
    setSelectedInput(input);
  }, [capabilities, handleSetBinding]);

  if (error && !device) {
    return (
      <div className="app error-state">
        <h1>ArchDeck</h1>
        <p className="error-message">{error}</p>
        <p>Make sure your Stream Deck is connected.</p>
      </div>
    );
  }

  if (!device) {
    return (
      <div className="app loading-state">
        <h1>ArchDeck</h1>
        <p>Connecting to Stream Deck...</p>
      </div>
    );
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>ArchDeck</h1>
      </header>

      {error && <div className="error-banner">{error}</div>}

      <main className="app-main three-column">
        <CapabilityBrowser
          capabilities={capabilities}
          onSelect={handleCapabilitySelect}
          selectedCapabilityId={selectedCapabilityId}
        />

        <DeviceLayout
          device={device}
          bindings={bindings}
          selectedInput={selectedInput}
          activeInputs={activeInputs}
          systemState={systemState}
          onSelectInput={setSelectedInput}
          onDrop={handleDrop}
        />

        <BindingEditor
          selectedInput={selectedInput}
          bindings={bindings}
          capabilities={capabilities}
          onSetBinding={handleSetBinding}
          onRemoveBinding={handleRemoveBinding}
        />
      </main>
    </div>
  );
}
