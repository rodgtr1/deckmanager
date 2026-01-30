import { useState, useEffect } from "react";
import {
  InputRef,
  Capability,
  CapabilityInfo,
  Binding,
  inputsMatch,
  getInputDisplayName,
} from "../types";

interface BindingEditorProps {
  selectedInput: InputRef | null;
  bindings: Binding[];
  capabilities: CapabilityInfo[];
  onSetBinding: (input: InputRef, capability: Capability) => void;
  onRemoveBinding: (input: InputRef) => void;
}

export default function BindingEditor({
  selectedInput,
  bindings,
  capabilities,
  onSetBinding,
  onRemoveBinding,
}: BindingEditorProps) {
  const [selectedCapabilityId, setSelectedCapabilityId] = useState<string>("");
  const [step, setStep] = useState<number>(0.02);
  const [command, setCommand] = useState<string>("");
  const [url, setUrl] = useState<string>("https://");

  // Get current binding for selected input
  const currentBinding = selectedInput
    ? bindings.find((b) => inputsMatch(b.input, selectedInput))
    : undefined;

  // Filter capabilities based on input type
  const availableCapabilities = selectedInput
    ? capabilities.filter((cap) => {
        switch (selectedInput.type) {
          case "Button":
            return cap.supports_button;
          case "Encoder":
            return cap.supports_encoder;
          case "EncoderPress":
            return cap.supports_encoder_press;
          case "Swipe":
            return false; // No capabilities for swipe yet
        }
      })
    : [];

  // Update local state when selection changes
  useEffect(() => {
    if (currentBinding) {
      setSelectedCapabilityId(currentBinding.capability.type);
      if (currentBinding.capability.type === "SystemVolume") {
        setStep(currentBinding.capability.step);
      }
      if (
        currentBinding.capability.type === "RunCommand" ||
        currentBinding.capability.type === "LaunchApp"
      ) {
        setCommand(currentBinding.capability.command);
      }
      if (currentBinding.capability.type === "OpenURL") {
        setUrl(currentBinding.capability.url);
      }
    } else {
      setSelectedCapabilityId("");
      setStep(0.02);
      setCommand("");
      setUrl("https://");
    }
  }, [currentBinding, selectedInput]);

  const handleSave = () => {
    if (!selectedInput || !selectedCapabilityId) return;

    let capability: Capability;
    switch (selectedCapabilityId) {
      case "SystemVolume":
        capability = { type: "SystemVolume", step };
        break;
      case "ToggleMute":
        capability = { type: "ToggleMute" };
        break;
      case "MediaPlayPause":
        capability = { type: "MediaPlayPause" };
        break;
      case "MediaNext":
        capability = { type: "MediaNext" };
        break;
      case "MediaPrevious":
        capability = { type: "MediaPrevious" };
        break;
      case "MediaStop":
        capability = { type: "MediaStop" };
        break;
      case "RunCommand":
        if (!command.trim()) return;
        capability = { type: "RunCommand", command: command.trim() };
        break;
      case "LaunchApp":
        if (!command.trim()) return;
        capability = { type: "LaunchApp", command: command.trim() };
        break;
      case "OpenURL":
        if (!url.trim() || url === "https://") return;
        capability = { type: "OpenURL", url: url.trim() };
        break;
      default:
        return;
    }

    onSetBinding(selectedInput, capability);
  };

  const handleRemove = () => {
    if (!selectedInput) return;
    onRemoveBinding(selectedInput);
    setSelectedCapabilityId("");
  };

  if (!selectedInput) {
    return (
      <div className="binding-editor">
        <div className="editor-placeholder">
          <p>Select an input to configure</p>
        </div>
      </div>
    );
  }

  const selectedCapability = capabilities.find(
    (c) => c.id === selectedCapabilityId
  );

  return (
    <div className="binding-editor">
      <h3 className="editor-title">{getInputDisplayName(selectedInput)}</h3>

      <div className="editor-field">
        <label htmlFor="capability-select">Action</label>
        <select
          id="capability-select"
          value={selectedCapabilityId}
          onChange={(e) => setSelectedCapabilityId(e.target.value)}
        >
          <option value="">-- None --</option>
          {availableCapabilities.map((cap) => (
            <option key={cap.id} value={cap.id}>
              {cap.name}
            </option>
          ))}
        </select>
        {selectedCapability && (
          <p className="field-description">{selectedCapability.description}</p>
        )}
      </div>

      {selectedCapabilityId === "SystemVolume" && (
        <div className="editor-field">
          <label htmlFor="step-input">Step Size</label>
          <input
            id="step-input"
            type="number"
            min="0.01"
            max="0.5"
            step="0.01"
            value={step}
            onChange={(e) => setStep(parseFloat(e.target.value) || 0.02)}
          />
          <p className="field-description">
            Volume change per tick (0.02 = 2%)
          </p>
        </div>
      )}

      {selectedCapabilityId === "RunCommand" && (
        <div className="editor-field">
          <label htmlFor="command-input">Command</label>
          <input
            id="command-input"
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder="e.g., notify-send 'Hello!'"
          />
          <p className="field-description">Shell command to execute</p>
        </div>
      )}

      {selectedCapabilityId === "LaunchApp" && (
        <div className="editor-field">
          <label htmlFor="app-input">Application</label>
          <input
            id="app-input"
            type="text"
            value={command}
            onChange={(e) => setCommand(e.target.value)}
            placeholder="e.g., firefox, code, kitty"
          />
          <p className="field-description">Application name or path</p>
        </div>
      )}

      {selectedCapabilityId === "OpenURL" && (
        <div className="editor-field">
          <label htmlFor="url-input">URL</label>
          <input
            id="url-input"
            type="url"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://example.com"
          />
          <p className="field-description">URL to open in your browser</p>
        </div>
      )}

      <div className="editor-actions">
        <button
          className="btn-save"
          onClick={handleSave}
          disabled={!selectedCapabilityId}
        >
          Save
        </button>
        <button
          className="btn-remove"
          onClick={handleRemove}
          disabled={!currentBinding}
        >
          Remove
        </button>
      </div>
    </div>
  );
}
