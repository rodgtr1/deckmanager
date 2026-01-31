import { useState, useEffect, useMemo } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import dashboardIconsData from "../icons/dashboard-icons.json";
import lucideIconsData from "../icons/lucide-icons.json";

type IconCategory = "apps" | "actions";

interface IconBrowserProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (iconUrl: string) => void;
}

export default function IconBrowser({ isOpen, onClose, onSelect }: IconBrowserProps) {
  const [category, setCategory] = useState<IconCategory>("apps");
  const [search, setSearch] = useState("");
  const [loadedIcons, setLoadedIcons] = useState<Set<string>>(new Set());

  // Reset search when modal opens
  useEffect(() => {
    if (isOpen) {
      setSearch("");
      setLoadedIcons(new Set());
    }
  }, [isOpen]);

  // Get filtered icons based on search and category
  const filteredIcons = useMemo(() => {
    const data = category === "apps" ? dashboardIconsData : lucideIconsData;
    const searchLower = search.toLowerCase().trim();

    if (!searchLower) {
      return data.icons.slice(0, 50); // Show first 50 by default
    }

    return data.icons.filter((icon) =>
      icon.toLowerCase().includes(searchLower)
    ).slice(0, 100); // Limit results
  }, [category, search]);

  // Get full URL for an icon
  const getIconUrl = (iconName: string): string => {
    const data = category === "apps" ? dashboardIconsData : lucideIconsData;
    return `${data.baseUrl}${iconName}${data.extension}`;
  };

  // Handle icon selection
  const handleIconSelect = (iconName: string) => {
    const url = getIconUrl(iconName);
    onSelect(url);
    onClose();
  };

  // Handle file browser fallback
  const handleBrowseFile = async () => {
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "gif", "webp", "svg"] }],
      });
      if (file) {
        onSelect(file);
        onClose();
      }
    } catch (e) {
      console.error("Failed to open file picker:", e);
    }
  };

  // Track loaded icons for fade-in effect
  const handleIconLoad = (iconName: string) => {
    setLoadedIcons((prev) => new Set(prev).add(iconName));
  };

  if (!isOpen) return null;

  return (
    <div className="icon-browser-overlay" onClick={onClose}>
      <div className="icon-browser-modal" onClick={(e) => e.stopPropagation()}>
        <div className="icon-browser-header">
          <h3>Choose Icon</h3>
          <button className="icon-browser-close" onClick={onClose}>×</button>
        </div>

        <div className="icon-browser-search">
          <input
            type="text"
            placeholder="Search icons..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            autoFocus
          />
        </div>

        <div className="icon-browser-tabs">
          <button
            className={`icon-browser-tab ${category === "apps" ? "active" : ""}`}
            onClick={() => setCategory("apps")}
          >
            Apps ({dashboardIconsData.icons.length})
          </button>
          <button
            className={`icon-browser-tab ${category === "actions" ? "active" : ""}`}
            onClick={() => setCategory("actions")}
          >
            Actions ({lucideIconsData.icons.length})
          </button>
        </div>

        <div className="icon-browser-grid">
          {filteredIcons.length === 0 ? (
            <div className="icon-browser-empty">
              No icons found for "{search}"
            </div>
          ) : (
            filteredIcons.map((iconName) => (
              <button
                key={iconName}
                className={`icon-browser-item ${loadedIcons.has(iconName) ? "loaded" : ""}`}
                onClick={() => handleIconSelect(iconName)}
                title={iconName}
              >
                <img
                  src={getIconUrl(iconName)}
                  alt={iconName}
                  onLoad={() => handleIconLoad(iconName)}
                  onError={(e) => {
                    (e.target as HTMLImageElement).style.display = "none";
                  }}
                />
                <span className="icon-name">{iconName}</span>
              </button>
            ))
          )}
        </div>

        <div className="icon-browser-footer">
          <button className="btn-browse-file" onClick={handleBrowseFile}>
            Browse Local File...
          </button>
        </div>
      </div>
    </div>
  );
}
