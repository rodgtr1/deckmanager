import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import CapabilityBrowser, { getCapabilityIcon } from "./CapabilityBrowser";
import { CapabilityInfo } from "../types";

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  clear: vi.fn(),
};
Object.defineProperty(window, "localStorage", { value: localStorageMock });

const mockCapabilities: CapabilityInfo[] = [
  {
    id: "SystemVolume",
    name: "System Volume",
    description: "Adjust system volume",
    supports_button: false,
    supports_encoder: true,
    supports_encoder_press: false,
    parameters: [{ name: "step", param_type: "number", default_value: "0.02", description: "Volume step" }],
  },
  {
    id: "ToggleMute",
    name: "Toggle Mute",
    description: "Toggle audio mute",
    supports_button: true,
    supports_encoder: false,
    supports_encoder_press: true,
    parameters: [],
  },
  {
    id: "MediaPlayPause",
    name: "Play/Pause",
    description: "Toggle media playback",
    supports_button: true,
    supports_encoder: false,
    supports_encoder_press: true,
    parameters: [],
  },
  {
    id: "ElgatoKeyLight",
    name: "Elgato Key Light",
    description: "Control Elgato Key Light",
    supports_button: true,
    supports_encoder: true,
    supports_encoder_press: true,
    parameters: [{ name: "ip", param_type: "string", default_value: "192.168.1.100", description: "IP address" }],
  },
];

describe("CapabilityBrowser", () => {
  beforeEach(() => {
    localStorageMock.getItem.mockReturnValue(null);
    localStorageMock.setItem.mockClear();
  });

  it("renders the title", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );
    expect(screen.getByText("Capabilities")).toBeInTheDocument();
  });

  it("renders module headers", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );
    expect(screen.getByText("Audio")).toBeInTheDocument();
    expect(screen.getByText("Lighting")).toBeInTheDocument();
    expect(screen.getByText("Commands")).toBeInTheDocument();
  });

  it("shows capabilities when module is expanded", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );
    // Audio module should be expanded by default
    expect(screen.getByText("System Volume")).toBeInTheDocument();
    expect(screen.getByText("Toggle Mute")).toBeInTheDocument();
    expect(screen.getByText("Play/Pause")).toBeInTheDocument();
  });

  it("calls onSelect when capability is clicked", () => {
    const onSelect = vi.fn();
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={onSelect}
        selectedCapabilityId={null}
      />
    );

    fireEvent.click(screen.getByText("Toggle Mute"));
    expect(onSelect).toHaveBeenCalledWith("ToggleMute");
  });

  it("highlights selected capability", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId="ToggleMute"
      />
    );

    const item = screen.getByText("Toggle Mute").closest(".capability-item");
    expect(item).toHaveClass("selected");
  });

  it("collapses module when header is clicked", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );

    // Click Audio header to collapse
    fireEvent.click(screen.getByText("Audio"));

    // Capabilities should no longer be visible
    expect(screen.queryByText("System Volume")).not.toBeInTheDocument();
  });

  it("sets draggable attribute on capability items", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );

    const item = screen.getByText("Toggle Mute").closest(".capability-item");
    expect(item).toHaveAttribute("draggable", "true");
  });
});

describe("getCapabilityIcon", () => {
  it("returns correct icon for SystemVolume", () => {
    expect(getCapabilityIcon("SystemVolume")).toBe("\u{1F50A}");
  });

  it("returns correct icon for ToggleMute", () => {
    expect(getCapabilityIcon("ToggleMute")).toBe("\u{1F507}");
  });

  it("returns correct icon for ElgatoKeyLight", () => {
    expect(getCapabilityIcon("ElgatoKeyLight")).toBe("\u{1F4A1}");
  });

  it("returns question mark for unknown capability", () => {
    expect(getCapabilityIcon("UnknownCapability")).toBe("\u2753");
  });
});
