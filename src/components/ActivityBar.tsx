import { Grid3X3, Puzzle } from "lucide-react";

export type ViewType = "device" | "plugins";

interface ActivityBarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
}

export default function ActivityBar({ currentView, onViewChange }: ActivityBarProps) {
  return (
    <div className="activity-bar">
      <button
        className={`activity-item ${currentView === "device" ? "active" : ""}`}
        onClick={() => onViewChange("device")}
        title="Device"
      >
        <Grid3X3 size={24} />
      </button>
      <button
        className={`activity-item ${currentView === "plugins" ? "active" : ""}`}
        onClick={() => onViewChange("plugins")}
        title="Plugins"
      >
        <Puzzle size={24} />
      </button>
    </div>
  );
}
