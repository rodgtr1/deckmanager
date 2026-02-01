import { Zap, Lightbulb, Plug, LucideIcon } from "lucide-react";
import { PluginInfo } from "../types";

// Map plugin IDs to Lucide icons
const PLUGIN_ICONS: Record<string, LucideIcon> = {
  core: Zap,
  elgato: Lightbulb,
};

interface PluginCardProps {
  plugin: PluginInfo;
  isSelected: boolean;
  onSelect: () => void;
  onToggle: (enabled: boolean) => void;
}

export default function PluginCard({
  plugin,
  isSelected,
  onSelect,
  onToggle,
}: PluginCardProps) {
  const handleToggleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (!plugin.is_core) {
      onToggle(!plugin.enabled);
    }
  };

  const PluginIcon = PLUGIN_ICONS[plugin.id] || Plug;

  return (
    <div
      className={`plugin-card ${isSelected ? "selected" : ""} ${!plugin.enabled ? "disabled" : ""}`}
      onClick={onSelect}
    >
      <div className="plugin-card-header">
        <span className="plugin-icon">
          <PluginIcon size={20} />
        </span>
        <div className="plugin-info">
          <div className="plugin-name">
            {plugin.name}
            {plugin.is_core && <span className="plugin-badge core">Core</span>}
            {!plugin.enabled && !plugin.is_core && <span className="plugin-badge disabled">Disabled</span>}
          </div>
          <div className="plugin-meta">
            <span>{plugin.category}</span>
            <span>{plugin.capability_count} {plugin.capability_count === 1 ? "capability" : "capabilities"}</span>
          </div>
        </div>
        <div className="plugin-toggle">
          <label className="toggle-switch" onClick={handleToggleClick}>
            <input
              type="checkbox"
              checked={plugin.enabled}
              disabled={plugin.is_core}
              onChange={() => {}}
            />
            <span className="toggle-slider" />
          </label>
        </div>
      </div>
      <div className="plugin-description">{plugin.description}</div>
      {!plugin.enabled && !plugin.is_core && (
        <div className="plugin-disabled-hint">
          {plugin.capability_count} {plugin.capability_count === 1 ? "capability" : "capabilities"} hidden from browser
        </div>
      )}
    </div>
  );
}
