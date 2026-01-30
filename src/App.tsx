import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
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
} from "./types";
import "./App.css";

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [bindings, setBindings] = useState<Binding[]>([]);
  const [capabilities, setCapabilities] = useState<CapabilityInfo[]>([]);
  const [selectedInput, setSelectedInput] = useState<InputRef | null>(null);
  const [activeInputs, setActiveInputs] = useState<Set<string>>(new Set());
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  // Load initial data
  useEffect(() => {
    const loadData = async () => {
      try {
        const [deviceInfo, bindingsList, capsList] = await Promise.all([
          invoke<DeviceInfo | null>("get_device_info"),
          invoke<Binding[]>("get_bindings"),
          invoke<CapabilityInfo[]>("get_capabilities"),
        ]);

        setDevice(deviceInfo);
        setBindings(bindingsList);
        setCapabilities(capsList);
      } catch (e) {
        setError(`Failed to load: ${e}`);
      }
    };

    loadData();
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
    async (input: InputRef, capability: Capability) => {
      try {
        await invoke("set_binding", { input, capability });
        // Refresh bindings
        const updated = await invoke<Binding[]>("get_bindings");
        setBindings(updated);
        setHasUnsavedChanges(true);
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
      setHasUnsavedChanges(true);
      setError(null);
    } catch (e) {
      setError(`Failed to remove binding: ${e}`);
    }
  }, []);

  // Handle saving bindings to disk
  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      await invoke("save_bindings");
      setHasUnsavedChanges(false);
      setError(null);
    } catch (e) {
      setError(`Failed to save: ${e}`);
    } finally {
      setSaving(false);
    }
  }, []);

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
        <div className="header-actions">
          {hasUnsavedChanges && (
            <span className="unsaved-indicator">Unsaved changes</span>
          )}
          <button
            className="btn-save-config"
            onClick={handleSave}
            disabled={!hasUnsavedChanges || saving}
          >
            {saving ? "Saving..." : "Save Configuration"}
          </button>
        </div>
      </header>

      {error && <div className="error-banner">{error}</div>}

      <main className="app-main">
        <DeviceLayout
          device={device}
          bindings={bindings}
          selectedInput={selectedInput}
          activeInputs={activeInputs}
          onSelectInput={setSelectedInput}
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
