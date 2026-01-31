import { describe, it, expect } from "vitest";
import {
  buttonRef,
  encoderRef,
  encoderPressRef,
  inputsMatch,
  getInputDisplayName,
  getCapabilityDisplayName,
  type InputRef,
  type Capability,
} from "./types";

describe("Input Reference Helpers", () => {
  describe("buttonRef", () => {
    it("creates a Button InputRef", () => {
      const ref = buttonRef(0);
      expect(ref.type).toBe("Button");
      expect(ref.index).toBe(0);
    });

    it("creates Button InputRef with various indices", () => {
      expect(buttonRef(3).index).toBe(3);
      expect(buttonRef(7).index).toBe(7);
    });
  });

  describe("encoderRef", () => {
    it("creates an Encoder InputRef", () => {
      const ref = encoderRef(1);
      expect(ref.type).toBe("Encoder");
      expect(ref.index).toBe(1);
    });
  });

  describe("encoderPressRef", () => {
    it("creates an EncoderPress InputRef", () => {
      const ref = encoderPressRef(2);
      expect(ref.type).toBe("EncoderPress");
      expect(ref.index).toBe(2);
    });
  });
});

describe("inputsMatch", () => {
  it("matches identical Button refs", () => {
    expect(inputsMatch(buttonRef(0), buttonRef(0))).toBe(true);
    expect(inputsMatch(buttonRef(5), buttonRef(5))).toBe(true);
  });

  it("does not match different Button indices", () => {
    expect(inputsMatch(buttonRef(0), buttonRef(1))).toBe(false);
  });

  it("does not match different input types", () => {
    expect(inputsMatch(buttonRef(0), encoderRef(0))).toBe(false);
    expect(inputsMatch(encoderRef(0), encoderPressRef(0))).toBe(false);
  });

  it("matches Swipe inputs", () => {
    const swipe1: InputRef = { type: "Swipe" };
    const swipe2: InputRef = { type: "Swipe" };
    expect(inputsMatch(swipe1, swipe2)).toBe(true);
  });

  it("matches identical Encoder refs", () => {
    expect(inputsMatch(encoderRef(2), encoderRef(2))).toBe(true);
  });

  it("matches identical EncoderPress refs", () => {
    expect(inputsMatch(encoderPressRef(3), encoderPressRef(3))).toBe(true);
  });
});

describe("getInputDisplayName", () => {
  it("returns correct name for Button", () => {
    expect(getInputDisplayName(buttonRef(0))).toBe("Button 1");
    expect(getInputDisplayName(buttonRef(7))).toBe("Button 8");
  });

  it("returns correct name for Encoder", () => {
    expect(getInputDisplayName(encoderRef(0))).toBe("Encoder 1");
    expect(getInputDisplayName(encoderRef(3))).toBe("Encoder 4");
  });

  it("returns correct name for EncoderPress", () => {
    expect(getInputDisplayName(encoderPressRef(0))).toBe("Encoder 1 Press");
    expect(getInputDisplayName(encoderPressRef(2))).toBe("Encoder 3 Press");
  });

  it("returns correct name for Swipe", () => {
    expect(getInputDisplayName({ type: "Swipe" })).toBe("Swipe");
  });
});

describe("getCapabilityDisplayName", () => {
  it("returns correct name for SystemAudio", () => {
    const cap: Capability = { type: "SystemAudio", step: 0.02 };
    expect(getCapabilityDisplayName(cap)).toBe("Audio");
  });

  it("returns correct name for Mute", () => {
    const cap: Capability = { type: "Mute" };
    expect(getCapabilityDisplayName(cap)).toBe("Mute");
  });

  it("returns correct name for MediaPlayPause", () => {
    const cap: Capability = { type: "MediaPlayPause" };
    expect(getCapabilityDisplayName(cap)).toBe("Play/Pause");
  });

  it("returns correct name for MediaNext", () => {
    const cap: Capability = { type: "MediaNext" };
    expect(getCapabilityDisplayName(cap)).toBe("Next");
  });

  it("returns correct name for MediaPrevious", () => {
    const cap: Capability = { type: "MediaPrevious" };
    expect(getCapabilityDisplayName(cap)).toBe("Previous");
  });

  it("returns correct name for MediaStop", () => {
    const cap: Capability = { type: "MediaStop" };
    expect(getCapabilityDisplayName(cap)).toBe("Stop");
  });

  it("returns correct name for RunCommand", () => {
    const cap: Capability = { type: "RunCommand", command: "echo test" };
    expect(getCapabilityDisplayName(cap)).toBe("Command");
  });

  it("returns correct name for LaunchApp", () => {
    const cap: Capability = { type: "LaunchApp", command: "firefox" };
    expect(getCapabilityDisplayName(cap)).toBe("App");
  });

  it("returns correct name for OpenURL", () => {
    const cap: Capability = { type: "OpenURL", url: "https://example.com" };
    expect(getCapabilityDisplayName(cap)).toBe("URL");
  });

  it("returns correct name for ElgatoKeyLight", () => {
    const cap: Capability = { type: "ElgatoKeyLight", ip: "192.168.1.100", port: 9123, action: "Toggle" };
    expect(getCapabilityDisplayName(cap)).toBe("Key Light");
  });
});
