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

// Capability types matching Rust's Capability enum
export type Capability =
  | { type: "SystemVolume"; step: number }
  | { type: "ToggleMute" }
  | { type: "MediaPlayPause" }
  | { type: "MediaNext" }
  | { type: "MediaPrevious" }
  | { type: "MediaStop" }
  | { type: "RunCommand"; command: string }
  | { type: "LaunchApp"; command: string }
  | { type: "OpenURL"; url: string };

export interface Binding {
  input: InputRef;
  capability: Capability;
  icon?: string;         // Custom emoji or icon name (UI only)
  label?: string;        // Custom display text (UI only)
  button_image?: string; // File path or URL for hardware button
  show_label?: boolean;  // Render label on hardware button
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
    case "SystemVolume":
      return "Volume";
    case "ToggleMute":
      return "Mute";
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
  }
}
