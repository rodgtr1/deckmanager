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
    id: "SystemAudio",
    name: "System Audio",
    description: "Full audio control for encoders",
    supports_button: false,
    supports_encoder: true,
    supports_encoder_press: true,
    parameters: [{ name: "step", param_type: "number", default_value: "0.02", description: "Volume step" }],
  },
  {
    id: "Mute",
    name: "Mute",
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
    name: "Key Light",
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
    expect(screen.getByText("System Audio")).toBeInTheDocument();
    expect(screen.getByText("Mute")).toBeInTheDocument();
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

    fireEvent.click(screen.getByText("Mute"));
    expect(onSelect).toHaveBeenCalledWith("Mute");
  });

  it("highlights selected capability", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId="Mute"
      />
    );

    const item = screen.getByText("Mute").closest(".capability-item");
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
    expect(screen.queryByText("System Audio")).not.toBeInTheDocument();
  });

  it("sets draggable attribute on capability items", () => {
    render(
      <CapabilityBrowser
        capabilities={mockCapabilities}
        onSelect={vi.fn()}
        selectedCapabilityId={null}
      />
    );

    const item = screen.getByText("Mute").closest(".capability-item");
    expect(item).toHaveAttribute("draggable", "true");
  });
});

describe("getCapabilityIcon", () => {
  it("returns correct icon for SystemAudio", () => {
    expect(getCapabilityIcon("SystemAudio")).toBe("\u{1F50A}");
  });

  it("returns correct icon for Mute", () => {
    expect(getCapabilityIcon("Mute")).toBe("\u{1F507}");
  });

  it("returns correct icon for ElgatoKeyLight", () => {
    expect(getCapabilityIcon("ElgatoKeyLight")).toBe("\u{1F4A1}");
  });

  it("returns question mark for unknown capability", () => {
    expect(getCapabilityIcon("UnknownCapability")).toBe("\u2753");
  });
});
