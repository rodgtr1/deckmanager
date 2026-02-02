// Types mirroring Rust structs for Tauri IPC

export interface DeviceInfo {
  model: string;
  button_count: number;
  encoder_count: number;
  rows: number;
  columns: number;
  has_touch_strip: boolean;
}

// Input reference types matching Rust's InputRef enum
export type InputRef =
  | { type: "Button"; index: number }
  | { type: "Encoder"; index: number }
  | { type: "EncoderPress"; index: number }
  | { type: "Swipe" };

// Key Light action types
export type KeyLightAction = "Toggle" | "On" | "Off" | "SetBrightness";

// OBS action types
export type OBSStreamAction = "Toggle" | "Start" | "Stop";
export type OBSRecordAction = "Toggle" | "Start" | "Stop" | "TogglePause";
export type OBSReplayAction = "Toggle" | "Start" | "Stop" | "Save";

// Capability types matching Rust's Capability enum
export type Capability =
  | { type: "SystemAudio"; step: number }
  | { type: "Mute" }
  | { type: "VolumeUp"; step: number }
  | { type: "VolumeDown"; step: number }
  | { type: "Microphone"; step: number }
  | { type: "MicMute" }
  | { type: "MicVolumeUp"; step: number }
  | { type: "MicVolumeDown"; step: number }
  | { type: "MediaPlayPause" }
  | { type: "MediaNext" }
  | { type: "MediaPrevious" }
  | { type: "MediaStop" }
  | { type: "RunCommand"; command: string; toggle?: boolean }
  | { type: "LaunchApp"; command: string }
  | { type: "OpenURL"; url: string }
  | { type: "ElgatoKeyLight"; ip: string; port: number; action: KeyLightAction }
  // OBS Studio capabilities
  | { type: "OBSScene"; host: string; port: number; password?: string; scene: string }
  | { type: "OBSStream"; host: string; port: number; password?: string; action: OBSStreamAction }
  | { type: "OBSRecord"; host: string; port: number; password?: string; action: OBSRecordAction }
  | { type: "OBSSourceVisibility"; host: string; port: number; password?: string; scene: string; source: string }
  | { type: "OBSAudio"; host: string; port: number; password?: string; input_name: string; step: number }
  | { type: "OBSStudioMode"; host: string; port: number; password?: string }
  | { type: "OBSReplayBuffer"; host: string; port: number; password?: string; action: OBSReplayAction }
  | { type: "OBSVirtualCam"; host: string; port: number; password?: string }
  | { type: "OBSTransition"; host: string; port: number; password?: string };

export interface Binding {
  input: InputRef;
  capability: Capability;
  page: number;              // Which page this binding belongs to (0-indexed)
  icon?: string;             // Custom emoji or icon name (UI only)
  label?: string;            // Custom display text (UI only)
  button_image?: string;     // File path or URL for hardware button (default state)
  button_image_alt?: string; // Alternate image (shown when state is "active", e.g., muted)
  show_label?: boolean;      // Render label on hardware button
  icon_color?: string;       // Color for SVG icons (hex, e.g., "#ffffff")
  icon_color_alt?: string;   // Color for alternate SVG icons
}

// System state for stateful capabilities
export interface SystemState {
  is_muted: boolean;
  is_mic_muted: boolean;
  is_playing: boolean;
}

// Capability metadata for UI
export interface CapabilityParameter {
  name: string;
  param_type: string;
  default_value: string;
  description: string;
}

export interface CapabilityInfo {
  id: string;
  name: string;
  description: string;
  supports_button: boolean;
  supports_encoder: boolean;
  supports_encoder_press: boolean;
  parameters: CapabilityParameter[];
}

// Plugin metadata for plugins page
export interface PluginInfo {
  id: string;
  name: string;
  category: string;
  enabled: boolean;
  capability_count: number;
  version: string;
  description: string;
  documentation: string;
  icon: string;
  is_core: boolean;
}

// Event types from Stream Deck
export interface ButtonEvent {
  index: number;
  pressed: boolean;
}

export interface EncoderEvent {
  index: number;
  delta: number;
}

export interface TouchSwipeEvent {
  start: [number, number];
  end: [number, number];
}

// Device connection status event
export interface ConnectionStatusEvent {
  connected: boolean;
  model: string | null;
}

// Page change event
export interface PageChangeEvent {
  page: number;
  page_count: number;
}

// Helper to create InputRef
export function buttonRef(index: number): InputRef {
  return { type: "Button", index };
}

export function encoderRef(index: number): InputRef {
  return { type: "Encoder", index };
}

export function encoderPressRef(index: number): InputRef {
  return { type: "EncoderPress", index };
}

// Helper to compare InputRefs
export function inputsMatch(a: InputRef, b: InputRef): boolean {
  if (a.type !== b.type) return false;
  if (a.type === "Swipe") return true;
  return (a as { index: number }).index === (b as { index: number }).index;
}

// Get display name for an input
export function getInputDisplayName(input: InputRef): string {
  switch (input.type) {
    case "Button":
      return `Button ${input.index + 1}`;
    case "Encoder":
      return `Encoder ${input.index + 1}`;
    case "EncoderPress":
      return `Encoder ${input.index + 1} Press`;
    case "Swipe":
      return "Swipe";
  }
}

/**
 * Create a default Capability object from a capability ID.
 * Returns null for unknown capability IDs.
 */
export function createDefaultCapability(capabilityId: string): Capability | null {
  switch (capabilityId) {
    case "SystemAudio":
      return { type: "SystemAudio", step: 0.02 };
    case "Mute":
      return { type: "Mute" };
    case "VolumeUp":
      return { type: "VolumeUp", step: 0.05 };
    case "VolumeDown":
      return { type: "VolumeDown", step: 0.05 };
    case "Microphone":
      return { type: "Microphone", step: 0.02 };
    case "MicMute":
      return { type: "MicMute" };
    case "MicVolumeUp":
      return { type: "MicVolumeUp", step: 0.05 };
    case "MicVolumeDown":
      return { type: "MicVolumeDown", step: 0.05 };
    case "MediaPlayPause":
      return { type: "MediaPlayPause" };
    case "MediaNext":
      return { type: "MediaNext" };
    case "MediaPrevious":
      return { type: "MediaPrevious" };
    case "MediaStop":
      return { type: "MediaStop" };
    case "RunCommand":
      return { type: "RunCommand", command: "", toggle: false };
    case "LaunchApp":
      return { type: "LaunchApp", command: "" };
    case "OpenURL":
      return { type: "OpenURL", url: "https://" };
    case "ElgatoKeyLight":
      return { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "Toggle" };
    // OBS capabilities
    case "OBSScene":
      return { type: "OBSScene", host: "127.0.0.1", port: 4455, scene: "Scene" };
    case "OBSStream":
      return { type: "OBSStream", host: "127.0.0.1", port: 4455, action: "Toggle" };
    case "OBSRecord":
      return { type: "OBSRecord", host: "127.0.0.1", port: 4455, action: "Toggle" };
    case "OBSSourceVisibility":
      return { type: "OBSSourceVisibility", host: "127.0.0.1", port: 4455, scene: "Scene", source: "Source" };
    case "OBSAudio":
      return { type: "OBSAudio", host: "127.0.0.1", port: 4455, input_name: "Mic/Aux", step: 0.02 };
    case "OBSStudioMode":
      return { type: "OBSStudioMode", host: "127.0.0.1", port: 4455 };
    case "OBSReplayBuffer":
      return { type: "OBSReplayBuffer", host: "127.0.0.1", port: 4455, action: "Save" };
    case "OBSVirtualCam":
      return { type: "OBSVirtualCam", host: "127.0.0.1", port: 4455 };
    case "OBSTransition":
      return { type: "OBSTransition", host: "127.0.0.1", port: 4455 };
    default:
      return null;
  }
}

// Get display name for a capability
export function getCapabilityDisplayName(cap: Capability): string {
  switch (cap.type) {
    case "SystemAudio":
      return "Audio";
    case "Mute":
      return "Mute";
    case "VolumeUp":
      return "Vol+";
    case "VolumeDown":
      return "Vol-";
    case "Microphone":
      return "Mic";
    case "MicMute":
      return "Mic Mute";
    case "MicVolumeUp":
      return "Mic+";
    case "MicVolumeDown":
      return "Mic-";
    case "MediaPlayPause":
      return "Play/Pause";
    case "MediaNext":
      return "Next";
    case "MediaPrevious":
      return "Previous";
    case "MediaStop":
      return "Stop";
    case "RunCommand":
      return "Command";
    case "LaunchApp":
      return "App";
    case "OpenURL":
      return "URL";
    case "ElgatoKeyLight":
      return "Key Light";
    // OBS capabilities
    case "OBSScene":
      return "OBS Scene";
    case "OBSStream":
      return "OBS Stream";
    case "OBSRecord":
      return "OBS Record";
    case "OBSSourceVisibility":
      return "OBS Source";
    case "OBSAudio":
      return "OBS Audio";
    case "OBSStudioMode":
      return "Studio Mode";
    case "OBSReplayBuffer":
      return "Replay";
    case "OBSVirtualCam":
      return "Virtual Cam";
    case "OBSTransition":
      return "Transition";
  }
}
