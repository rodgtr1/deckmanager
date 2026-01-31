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
  | { type: "ElgatoKeyLight"; ip: string; port: number; action: KeyLightAction };

export interface Binding {
  input: InputRef;
  capability: Capability;
  page: number;              // Which page this binding belongs to (0-indexed)
  icon?: string;             // Custom emoji or icon name (UI only)
  label?: string;            // Custom display text (UI only)
  button_image?: string;     // File path or URL for hardware button (default state)
  button_image_alt?: string; // Alternate image (shown when state is "active", e.g., muted)
  show_label?: boolean;      // Render label on hardware button
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
  }
}
