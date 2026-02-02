import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import ActivityBar, { ViewType } from "./components/ActivityBar";
import CapabilityBrowser from "./components/CapabilityBrowser";
import DeviceLayout from "./components/DeviceLayout";
import BindingEditor from "./components/BindingEditor";
import PluginsPage from "./components/PluginsPage";
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
  ConnectionStatusEvent,
  PageChangeEvent,
  inputsMatch,
  createDefaultCapability,
} from "./types";
import "./App.css";

export default function App() {
  const [device, setDevice] = useState<DeviceInfo | null>(null);
  const [bindings, setBindings] = useState<Binding[]>([]);
  const [capabilities, setCapabilities] = useState<CapabilityInfo[]>([]);
  const [selectedInput, setSelectedInput] = useState<InputRef | null>(null);
  const [selectedCapabilityId, setSelectedCapabilityId] = useState<string | null>(null);
  const [activeInputs, setActiveInputs] = useState<Set<string>>(new Set());
  const [systemState, setSystemState] = useState<SystemState>({ is_muted: false, is_mic_muted: false, is_playing: false });
  const [error, setError] = useState<string | null>(null);
  const [isConnected, setIsConnected] = useState<boolean>(true);
  const [currentPage, setCurrentPage] = useState<number>(0);
  const [pageCount, setPageCount] = useState<number>(1);
  const [currentView, setCurrentView] = useState<ViewType>("device");

  // Refresh capabilities (called when plugins are toggled)
  const refreshCapabilities = useCallback(async () => {
    try {
      const capsList = await invoke<CapabilityInfo[]>("get_capabilities");
      setCapabilities(capsList);
    } catch (e) {
      console.error("Failed to refresh capabilities:", e);
    }
  }, []);

  // Load initial data
  useEffect(() => {
    const loadData = async () => {
      try {
        const [deviceInfo, bindingsList, capsList, state, page, pages] = await Promise.all([
          invoke<DeviceInfo | null>("get_device_info"),
          invoke<Binding[]>("get_bindings"),
          invoke<CapabilityInfo[]>("get_capabilities"),
          invoke<SystemState>("get_system_state"),
          invoke<number>("get_current_page"),
          invoke<number>("get_page_count"),
        ]);

        setDevice(deviceInfo);
        setBindings(bindingsList);
        setCapabilities(capsList);
        setSystemState(state);
        setCurrentPage(page);
        setPageCount(pages);
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

  // Listen for connection status changes
  useEffect(() => {
    const unlistenConnection = listen<ConnectionStatusEvent>(
      "streamdeck:connection",
      async (e) => {
        setIsConnected(e.payload.connected);

        if (e.payload.connected) {
          // Device reconnected - refresh device info
          try {
            const deviceInfo = await invoke<DeviceInfo | null>("get_device_info");
            setDevice(deviceInfo);
            setError(null);
          } catch (err) {
            setError(`Failed to get device info: ${err}`);
          }
        } else {
          // Device disconnected
          setDevice(null);
        }
      }
    );

    return () => {
      unlistenConnection.then((f) => f());
    };
  }, []);

  // Listen for page changes
  useEffect(() => {
    const unlistenPage = listen<PageChangeEvent>("streamdeck:page", (e) => {
      setCurrentPage(e.payload.page);
      setPageCount(e.payload.page_count);
    });

    return () => {
      unlistenPage.then((f) => f());
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
      showLabel?: boolean,
      page?: number,
      iconColor?: string,
      iconColorAlt?: string
    ) => {
      try {
        const params = {
          input,
          capability,
          page: page ?? currentPage,
          icon: icon ?? null,
          label: label ?? null,
          button_image: buttonImage ?? null,
          button_image_alt: buttonImageAlt ?? null,
          show_label: showLabel ?? null,
          icon_color: iconColor ?? null,
          icon_color_alt: iconColorAlt ?? null,
        };
        await invoke("set_binding", { params });
        // Refresh bindings and page count
        const [updated, pages] = await Promise.all([
          invoke<Binding[]>("get_bindings"),
          invoke<number>("get_page_count"),
        ]);
        setBindings(updated);
        setPageCount(pages);
        // Auto-save to disk
        await invoke("save_bindings");
        setError(null);
      } catch (e) {
        setError(`Failed to set binding: ${e}`);
      }
    },
    [currentPage]
  );

  // Handle removing a binding
  const handleRemoveBinding = useCallback(async (input: InputRef, page?: number) => {
    try {
      const targetPage = page ?? currentPage;
      await invoke("remove_binding", { input, page: targetPage });

      // For encoders, also remove the paired binding (rotation <-> press)
      if (input.type === "Encoder") {
        const pressInput: InputRef = { type: "EncoderPress", index: input.index };
        await invoke("remove_binding", { input: pressInput, page: targetPage });
      } else if (input.type === "EncoderPress") {
        const rotateInput: InputRef = { type: "Encoder", index: input.index };
        await invoke("remove_binding", { input: rotateInput, page: targetPage });
      }

      // Refresh bindings and page count
      const [updated, pages] = await Promise.all([
        invoke<Binding[]>("get_bindings"),
        invoke<number>("get_page_count"),
      ]);
      setBindings(updated);
      setPageCount(pages);
      // Auto-save to disk
      await invoke("save_bindings");
      setError(null);
    } catch (e) {
      setError(`Failed to remove binding: ${e}`);
    }
  }, [currentPage]);

  // Handle capability selection from browser (for click-to-assign flow)
  const handleCapabilitySelect = useCallback((capabilityId: string) => {
    setSelectedCapabilityId(capabilityId);
    // If an input is already selected, assign the capability
    if (selectedInput) {
      const capInfo = capabilities.find(c => c.id === capabilityId);
      if (capInfo) {
        const capability = createDefaultCapability(capabilityId);
        if (capability) {
          handleSetBinding(selectedInput, capability);
        }
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
    const capability = createDefaultCapability(capabilityId);
    if (!capability) return;

    handleSetBinding(input, capability);
    setSelectedInput(input);
  }, [capabilities, handleSetBinding]);

  // Handle copying a binding from one input to another
  const handleCopyBinding = useCallback((fromInput: InputRef, toInput: InputRef) => {
    // Find the source binding on current page
    const sourceBinding = bindings.find(
      (b) => inputsMatch(b.input, fromInput) && b.page === currentPage
    );
    if (!sourceBinding) return;

    // Copy the binding to the new input
    handleSetBinding(
      toInput,
      sourceBinding.capability,
      sourceBinding.icon,
      sourceBinding.label,
      sourceBinding.button_image,
      sourceBinding.button_image_alt,
      sourceBinding.show_label,
      currentPage,
      sourceBinding.icon_color,
      sourceBinding.icon_color_alt
    );

    // For encoders, also copy the paired binding (rotation <-> press)
    if (fromInput.type === "Encoder" && toInput.type === "Encoder") {
      // Also copy the EncoderPress binding if it exists
      const pressBinding = bindings.find(
        (b) => b.input.type === "EncoderPress" &&
               (b.input as { index: number }).index === fromInput.index &&
               b.page === currentPage
      );
      if (pressBinding) {
        const toPressInput: InputRef = { type: "EncoderPress", index: toInput.index };
        handleSetBinding(
          toPressInput,
          pressBinding.capability,
          pressBinding.icon,
          pressBinding.label,
          pressBinding.button_image,
          pressBinding.button_image_alt,
          pressBinding.show_label,
          currentPage,
          pressBinding.icon_color,
          pressBinding.icon_color_alt
        );
      }
    } else if (fromInput.type === "EncoderPress" && toInput.type === "EncoderPress") {
      // Also copy the Encoder (rotation) binding if it exists
      const rotateBinding = bindings.find(
        (b) => b.input.type === "Encoder" &&
               (b.input as { index: number }).index === (fromInput as { type: "EncoderPress"; index: number }).index &&
               b.page === currentPage
      );
      if (rotateBinding) {
        const toRotateInput: InputRef = { type: "Encoder", index: (toInput as { type: "EncoderPress"; index: number }).index };
        handleSetBinding(
          toRotateInput,
          rotateBinding.capability,
          rotateBinding.icon,
          rotateBinding.label,
          rotateBinding.button_image,
          rotateBinding.button_image_alt,
          rotateBinding.show_label,
          currentPage,
          rotateBinding.icon_color,
          rotateBinding.icon_color_alt
        );
      }
    }

    setSelectedInput(toInput);
  }, [bindings, currentPage, handleSetBinding]);

  if (error && !device && !isConnected) {
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
        <p>{isConnected ? "Connecting to Stream Deck..." : "Waiting for Stream Deck..."}</p>
        {!isConnected && (
          <p className="hint-text">Connect your Stream Deck to continue</p>
        )}
      </div>
    );
  }

  return (
    <div className="app">
      <header className="app-header">
        <h1>ArchDeck</h1>
      </header>

      {error && <div className="error-banner">{error}</div>}

      <div className="app-container">
        <ActivityBar currentView={currentView} onViewChange={setCurrentView} />

        {currentView === "device" ? (
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
              currentPage={currentPage}
              pageCount={pageCount}
              onSelectInput={setSelectedInput}
              onDrop={handleDrop}
              onCopyBinding={handleCopyBinding}
            />

            <BindingEditor
              selectedInput={selectedInput}
              bindings={bindings}
              capabilities={capabilities}
              currentPage={currentPage}
              onSetBinding={handleSetBinding}
              onRemoveBinding={handleRemoveBinding}
            />
          </main>
        ) : (
          <PluginsPage onPluginToggle={refreshCapabilities} />
        )}
      </div>
    </div>
  );
}
