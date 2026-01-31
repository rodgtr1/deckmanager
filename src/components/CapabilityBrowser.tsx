import { useState, useEffect, DragEvent } from "react";
import { CapabilityInfo } from "../types";

interface CapabilityBrowserProps {
  capabilities: CapabilityInfo[];
  onSelect: (capabilityId: string) => void;
  selectedCapabilityId: string | null;
}

// Module definitions with icons and capability groupings
const MODULES: Record<string, { icon: string; capabilities: string[] }> = {
  Audio: {
    icon: "\u{1F3B5}",
    capabilities: [
      "SystemAudio",
      "Mute",
      "VolumeUp",
      "VolumeDown",
      "Microphone",
      "MicMute",
      "MicVolumeUp",
      "MicVolumeDown",
      "MediaPlayPause",
      "MediaNext",
      "MediaPrevious",
      "MediaStop",
    ],
  },
  Lighting: {
    icon: "\u{1F4A1}",
    capabilities: [
      "ElgatoKeyLight",
    ],
  },
  Commands: {
    icon: "\u26A1",
    capabilities: ["RunCommand", "LaunchApp", "OpenURL"],
  },
};

// Default icons for capabilities
const CAPABILITY_ICONS: Record<string, string> = {
  SystemAudio: "\u{1F50A}",
  Mute: "\u{1F507}",
  VolumeUp: "\u{1F50A}",
  VolumeDown: "\u{1F509}",
  Microphone: "\u{1F3A4}",
  MicMute: "\u{1F507}",
  MicVolumeUp: "\u{1F3A4}",
  MicVolumeDown: "\u{1F3A4}",
  MediaPlayPause: "\u25B6\uFE0F",
  MediaNext: "\u23ED",
  MediaPrevious: "\u23EE",
  MediaStop: "\u23F9",
  RunCommand: "\u2699\uFE0F",
  LaunchApp: "\u{1F4C1}",
  OpenURL: "\u{1F310}",
  ElgatoKeyLight: "\u{1F4A1}",
};

export function getCapabilityIcon(capabilityId: string): string {
  return CAPABILITY_ICONS[capabilityId] || "\u2753";
}

export default function CapabilityBrowser({
  capabilities,
  onSelect,
  selectedCapabilityId,
}: CapabilityBrowserProps) {
  // Track expanded/collapsed state for each module
  const [expandedModules, setExpandedModules] = useState<Set<string>>(() => {
    const stored = localStorage.getItem("archdeck-expanded-modules");
    if (stored) {
      try {
        return new Set(JSON.parse(stored));
      } catch {
        return new Set(Object.keys(MODULES));
      }
    }
    return new Set(Object.keys(MODULES));
  });

  // Persist expanded state
  useEffect(() => {
    localStorage.setItem(
      "archdeck-expanded-modules",
      JSON.stringify([...expandedModules])
    );
  }, [expandedModules]);

  const toggleModule = (moduleName: string) => {
    setExpandedModules((prev) => {
      const next = new Set(prev);
      if (next.has(moduleName)) {
        next.delete(moduleName);
      } else {
        next.add(moduleName);
      }
      return next;
    });
  };

  const handleDragStart = (e: DragEvent<HTMLDivElement>, capabilityId: string) => {
    e.dataTransfer.setData("text/plain", capabilityId);
    e.dataTransfer.effectAllowed = "copy";
  };

  const getCapabilityInfo = (id: string): CapabilityInfo | undefined => {
    return capabilities.find((c) => c.id === id);
  };

  return (
    <div className="capability-browser">
      <h2 className="browser-title">Capabilities</h2>

      <div className="module-list">
        {Object.entries(MODULES).map(([moduleName, { icon, capabilities: capIds }]) => {
          const isExpanded = expandedModules.has(moduleName);
          const moduleCapabilities = capIds
            .map(getCapabilityInfo)
            .filter((c): c is CapabilityInfo => c !== undefined);

          return (
            <div key={moduleName} className="module-section">
              <button
                className="module-header"
                onClick={() => toggleModule(moduleName)}
                aria-expanded={isExpanded}
              >
                <span className="module-expand-icon">
                  {isExpanded ? "\u25BC" : "\u25B6"}
                </span>
                <span className="module-icon">{icon}</span>
                <span className="module-name">{moduleName}</span>
              </button>

              {isExpanded && (
                <div className="capability-list">
                  {moduleCapabilities.map((cap) => (
                    <div
                      key={cap.id}
                      className={`capability-item ${selectedCapabilityId === cap.id ? "selected" : ""}`}
                      draggable
                      onDragStart={(e) => handleDragStart(e, cap.id)}
                      onClick={() => onSelect(cap.id)}
                      title={cap.description}
                    >
                      <span className="capability-icon">
                        {CAPABILITY_ICONS[cap.id] || "\u2753"}
                      </span>
                      <span className="capability-name">{cap.name}</span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
