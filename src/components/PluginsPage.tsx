import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Zap, Lightbulb, Plug, LucideIcon } from "lucide-react";
import { PluginInfo } from "../types";
import PluginCard from "./PluginCard";

// Map plugin IDs to Lucide icons
const PLUGIN_ICONS: Record<string, LucideIcon> = {
  core: Zap,
  elgato: Lightbulb,
};

interface PluginsPageProps {
  onPluginToggle: () => void;
}

export default function PluginsPage({ onPluginToggle }: PluginsPageProps) {
  const [plugins, setPlugins] = useState<PluginInfo[]>([]);
  const [selectedPluginId, setSelectedPluginId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Load plugins on mount
  useEffect(() => {
    const loadPlugins = async () => {
      try {
        const pluginsList = await invoke<PluginInfo[]>("get_plugins");
        setPlugins(pluginsList);
        // Select first plugin by default
        if (pluginsList.length > 0 && !selectedPluginId) {
          setSelectedPluginId(pluginsList[0].id);
        }
      } catch (e) {
        setError(`Failed to load plugins: ${e}`);
      }
    };
    loadPlugins();
  }, []);

  const handleToggle = async (pluginId: string, enabled: boolean) => {
    try {
      await invoke("set_plugin_enabled", { pluginId, enabled });
      // Update local state
      setPlugins((prev) =>
        prev.map((p) => (p.id === pluginId ? { ...p, enabled } : p))
      );
      // Notify parent to refresh capabilities
      onPluginToggle();
      setError(null);
    } catch (e) {
      setError(`Failed to toggle plugin: ${e}`);
    }
  };

  const selectedPlugin = plugins.find((p) => p.id === selectedPluginId);

  return (
    <div className="plugins-page">
      <div className="plugins-list">
        <div className="plugins-list-header">Plugins</div>
        {error && <div className="error-banner">{error}</div>}
        {plugins.map((plugin) => (
          <PluginCard
            key={plugin.id}
            plugin={plugin}
            isSelected={selectedPluginId === plugin.id}
            onSelect={() => setSelectedPluginId(plugin.id)}
            onToggle={(enabled) => handleToggle(plugin.id, enabled)}
          />
        ))}
      </div>

      <div className="plugin-docs">
        {selectedPlugin ? (
          <>
            <div className="plugin-docs-header">
              <span className="plugin-docs-icon">
                {(() => {
                  const Icon = PLUGIN_ICONS[selectedPlugin.id] || Plug;
                  return <Icon size={28} />;
                })()}
              </span>
              <div className="plugin-docs-title">
                <h2>{selectedPlugin.name}</h2>
                <span className="version">v{selectedPlugin.version}</span>
              </div>
            </div>
            <div className="plugin-docs-content">
              <MarkdownRenderer content={selectedPlugin.documentation} />
            </div>
          </>
        ) : (
          <div className="plugin-docs-placeholder">
            Select a plugin to view documentation
          </div>
        )}
      </div>
    </div>
  );
}

// Simple markdown renderer for plugin documentation
function MarkdownRenderer({ content }: { content: string }) {
  // Parse markdown to HTML (simple implementation)
  const html = parseMarkdown(content);
  return <div dangerouslySetInnerHTML={{ __html: html }} />;
}

function parseMarkdown(markdown: string): string {
  // Split into lines for processing
  const lines = markdown.split('\n');
  const result: string[] = [];
  let inCodeBlock = false;
  let codeBlockContent: string[] = [];
  let listItems: string[] = [];

  const flushList = () => {
    if (listItems.length > 0) {
      result.push(`<ul>${listItems.join('')}</ul>`);
      listItems = [];
    }
  };

  for (let i = 0; i < lines.length; i++) {
    let line = lines[i];

    // Code blocks
    if (line.startsWith('```')) {
      if (inCodeBlock) {
        result.push(`<pre><code>${escapeHtml(codeBlockContent.join('\n'))}</code></pre>`);
        codeBlockContent = [];
        inCodeBlock = false;
      } else {
        flushList();
        inCodeBlock = true;
      }
      continue;
    }

    if (inCodeBlock) {
      codeBlockContent.push(line);
      continue;
    }

    // Empty line - flush list and skip
    if (line.trim() === '') {
      flushList();
      continue;
    }

    // Headers
    if (line.startsWith('### ')) {
      flushList();
      result.push(`<h3>${processInline(line.slice(4))}</h3>`);
      continue;
    }
    if (line.startsWith('## ')) {
      flushList();
      result.push(`<h2>${processInline(line.slice(3))}</h2>`);
      continue;
    }
    if (line.startsWith('# ')) {
      flushList();
      result.push(`<h1>${processInline(line.slice(2))}</h1>`);
      continue;
    }

    // List items
    if (line.startsWith('- ')) {
      listItems.push(`<li>${processInline(line.slice(2))}</li>`);
      continue;
    }
    if (/^\d+\. /.test(line)) {
      listItems.push(`<li>${processInline(line.replace(/^\d+\. /, ''))}</li>`);
      continue;
    }

    // Regular paragraph line
    flushList();
    result.push(`<p>${processInline(line)}</p>`);
  }

  flushList();
  return result.join('\n');
}

function processInline(text: string): string {
  // Escape HTML first to prevent XSS
  text = escapeHtml(text);
  // Bold
  text = text.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');
  // Inline code
  text = text.replace(/`([^`]+)`/g, '<code>$1</code>');
  return text;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}
