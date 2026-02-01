import { useState, useEffect, DragEvent, ReactNode } from "react";
import {
  Volume2,
  VolumeX,
  Volume1,
  Mic,
  MicOff,
  Play,
  SkipForward,
  SkipBack,
  Square,
  Terminal,
  AppWindow,
  Globe,
  Lightbulb,
  Music,
  Zap,
  LucideIcon,
} from "lucide-react";
import { CapabilityInfo } from "../types";

interface CapabilityBrowserProps {
  capabilities: CapabilityInfo[];
  onSelect: (capabilityId: string) => void;
  selectedCapabilityId: string | null;
}

// Module definitions with icons and capability groupings
const MODULES: Record<string, { icon: LucideIcon; capabilities: string[] }> = {
  Audio: {
    icon: Music,
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
    icon: Lightbulb,
    capabilities: [
      "ElgatoKeyLight",
    ],
  },
  Commands: {
    icon: Zap,
    capabilities: ["RunCommand", "LaunchApp", "OpenURL"],
  },
};

// Lucide icons for capabilities
const CAPABILITY_ICONS: Record<string, LucideIcon> = {
  SystemAudio: Volume2,
  Mute: VolumeX,
  VolumeUp: Volume2,
  VolumeDown: Volume1,
  Microphone: Mic,
  MicMute: MicOff,
  MicVolumeUp: Mic,
  MicVolumeDown: Mic,
  MediaPlayPause: Play,
  MediaNext: SkipForward,
  MediaPrevious: SkipBack,
  MediaStop: Square,
  RunCommand: Terminal,
  LaunchApp: AppWindow,
  OpenURL: Globe,
  ElgatoKeyLight: Lightbulb,
};

export function getCapabilityIcon(capabilityId: string): ReactNode {
  const Icon = CAPABILITY_ICONS[capabilityId];
  return Icon ? <Icon size={14} /> : null;
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
        {Object.entries(MODULES).map(([moduleName, { icon: ModuleIcon, capabilities: capIds }]) => {
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
                <span className="module-icon">
                  <ModuleIcon size={16} />
                </span>
                <span className="module-name">{moduleName}</span>
              </button>

              {isExpanded && (
                <div className="capability-list">
                  {moduleCapabilities.map((cap) => {
                    const CapIcon = CAPABILITY_ICONS[cap.id];
                    return (
                      <div
                        key={cap.id}
                        className={`capability-item ${selectedCapabilityId === cap.id ? "selected" : ""}`}
                        draggable
                        onDragStart={(e) => handleDragStart(e, cap.id)}
                        onClick={() => onSelect(cap.id)}
                        title={cap.description}
                      >
                        <span className="capability-icon">
                          {CapIcon && <CapIcon size={14} />}
                        </span>
                        <span className="capability-name">{cap.name}</span>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
