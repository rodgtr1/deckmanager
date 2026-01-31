import { useState, useEffect } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  InputRef,
  Capability,
  CapabilityInfo,
  Binding,
  inputsMatch,
  getInputDisplayName,
} from "../types";
import { getCapabilityIcon } from "./CapabilityBrowser";
import IconBrowser from "./IconBrowser";

// Icon picker options
const ICONS = [
  "\u{1F50A}", "\u{1F507}", "\u25B6\uFE0F", "\u23F8", "\u23ED", "\u23EE", "\u23F9",
  "\u{1F310}", "\u{1F4C1}", "\u2699\uFE0F", "\u{1F3AE}", "\u{1F4A1}", "\u{1F5A5}\uFE0F",
  "\u{1F3A4}", "\u{1F4F7}", "\u{1F4F9}", "\u{1F4DD}", "\u{1F512}", "\u{1F513}",
  "\u2B50", "\u2764\uFE0F", "\u{1F525}", "\u26A1", "\u2601\uFE0F", "\u{1F319}",
];

interface BindingEditorProps {
  selectedInput: InputRef | null;
  bindings: Binding[];
  capabilities: CapabilityInfo[];
  currentPage: number;
  onSetBinding: (
    input: InputRef,
    capability: Capability,
    icon?: string,
    label?: string,
    buttonImage?: string,
    buttonImageAlt?: string,
    showLabel?: boolean,
    page?: number
  ) => void;
  onRemoveBinding: (input: InputRef, page?: number) => void;
}

export default function BindingEditor({
  selectedInput,
  bindings,
  capabilities,
  currentPage,
  onSetBinding,
  onRemoveBinding,
}: BindingEditorProps) {
  const [selectedCapabilityId, setSelectedCapabilityId] = useState<string>("");
  const [step, setStep] = useState<number>(0.02);
  const [command, setCommand] = useState<string>("");
  const [url, setUrl] = useState<string>("https://");
  const [customIcon, setCustomIcon] = useState<string>("");
  const [customLabel, setCustomLabel] = useState<string>("");
  const [buttonImage, setButtonImage] = useState<string>("");
  const [buttonImageAlt, setButtonImageAlt] = useState<string>("");
  const [showLabel, setShowLabel] = useState<boolean>(false);
  const [showIconBrowser, setShowIconBrowser] = useState<boolean>(false);
  const [iconBrowserTarget, setIconBrowserTarget] = useState<"default" | "alt">("default");
  const [keyLightIp, setKeyLightIp] = useState<string>("192.168.1.100");
  const [commandToggle, setCommandToggle] = useState<boolean>(false);

  // Get current binding for selected input on current page
  const currentBinding = selectedInput
    ? bindings.find((b) => inputsMatch(b.input, selectedInput) && b.page === currentPage)
    : undefined;

  // Check if this input type supports hardware images
  // Buttons: direct button display
  // EncoderPress: LCD strip display
  // Encoder: LCD strip fallback display
  const supportsHardwareImage =
    selectedInput?.type === "Button" ||
    selectedInput?.type === "EncoderPress" ||
    selectedInput?.type === "Encoder";

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
      setCustomIcon(currentBinding.icon || "");
      setCustomLabel(currentBinding.label || "");
      setButtonImage(currentBinding.button_image || "");
      setButtonImageAlt(currentBinding.button_image_alt || "");
      setShowLabel(currentBinding.show_label || false);
      if (
        currentBinding.capability.type === "SystemAudio" ||
        currentBinding.capability.type === "VolumeUp" ||
        currentBinding.capability.type === "VolumeDown" ||
        currentBinding.capability.type === "Microphone" ||
        currentBinding.capability.type === "MicVolumeUp" ||
        currentBinding.capability.type === "MicVolumeDown"
      ) {
        setStep(currentBinding.capability.step);
      }
      if (currentBinding.capability.type === "RunCommand") {
        setCommand(currentBinding.capability.command);
        setCommandToggle(currentBinding.capability.toggle || false);
      }
      if (currentBinding.capability.type === "LaunchApp") {
        setCommand(currentBinding.capability.command);
      }
      if (currentBinding.capability.type === "OpenURL") {
        setUrl(currentBinding.capability.url);
      }
      if (currentBinding.capability.type === "ElgatoKeyLight") {
        setKeyLightIp(currentBinding.capability.ip);
        setSelectedCapabilityId("ElgatoKeyLight");
      }
    } else {
      setSelectedCapabilityId("");
      setStep(0.02);
      setCommand("");
      setUrl("https://");
      setCustomIcon("");
      setCustomLabel("");
      setButtonImage("");
      setButtonImageAlt("");
      setShowLabel(false);
      setKeyLightIp("192.168.1.100");
      setCommandToggle(false);
    }
  }, [currentBinding, selectedInput]);

  const handleOpenIconBrowser = (target: "default" | "alt") => {
    setIconBrowserTarget(target);
    setShowIconBrowser(true);
  };

  const handleIconSelect = (iconUrl: string) => {
    if (iconBrowserTarget === "default") {
      setButtonImage(iconUrl);
    } else {
      setButtonImageAlt(iconUrl);
    }
  };

  const handleSave = () => {
    if (!selectedInput || !selectedCapabilityId) return;

    let capability: Capability;
    switch (selectedCapabilityId) {
      case "SystemAudio":
        capability = { type: "SystemAudio", step };
        break;
      case "Mute":
        capability = { type: "Mute" };
        break;
      case "VolumeUp":
        capability = { type: "VolumeUp", step };
        break;
      case "VolumeDown":
        capability = { type: "VolumeDown", step };
        break;
      case "Microphone":
        capability = { type: "Microphone", step };
        break;
      case "MicMute":
        capability = { type: "MicMute" };
        break;
      case "MicVolumeUp":
        capability = { type: "MicVolumeUp", step };
        break;
      case "MicVolumeDown":
        capability = { type: "MicVolumeDown", step };
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
        capability = { type: "RunCommand", command: command.trim(), toggle: commandToggle };
        break;
      case "LaunchApp":
        if (!command.trim()) return;
        capability = { type: "LaunchApp", command: command.trim() };
        break;
      case "OpenURL":
        if (!url.trim() || url === "https://") return;
        capability = { type: "OpenURL", url: url.trim() };
        break;
      case "ElgatoKeyLight":
        if (!keyLightIp.trim()) return;
        // Combined capability - action determined by input type at runtime
        capability = { type: "ElgatoKeyLight", ip: keyLightIp.trim(), port: 9123, action: "Toggle" };
        break;
      default:
        return;
    }

    // Pass icon/label only if customized
    const icon = customIcon || undefined;
    const label = customLabel || undefined;
    // For button image and show label, pass the actual values (not using || which breaks false)
    const image = buttonImage.trim() || undefined;
    const imageAlt = buttonImageAlt.trim() || undefined;
    const showLabelOnButton = showLabel;

    onSetBinding(selectedInput, capability, icon, label, image, imageAlt, showLabelOnButton);

    // For unified capabilities on encoders, automatically create both rotation and press bindings
    // This applies to SystemAudio, Microphone, and ElgatoKeyLight
    const needsBothBindings =
      selectedCapabilityId === "SystemAudio" ||
      selectedCapabilityId === "Microphone" ||
      selectedCapabilityId === "ElgatoKeyLight";

    if (needsBothBindings && selectedInput) {
      if (selectedInput.type === "Encoder") {
        // Also create EncoderPress binding
        const pressInput: InputRef = { type: "EncoderPress", index: selectedInput.index };
        onSetBinding(pressInput, capability, icon, label, image, imageAlt, showLabelOnButton);
      } else if (selectedInput.type === "EncoderPress") {
        // Also create Encoder binding
        const rotateInput: InputRef = { type: "Encoder", index: selectedInput.index };
        onSetBinding(rotateInput, capability, icon, label, image, imageAlt, showLabelOnButton);
      }
    }
  };

  const handleRemove = () => {
    if (!selectedInput) return;
    onRemoveBinding(selectedInput, currentPage);
    setSelectedCapabilityId("");
    setCustomIcon("");
    setCustomLabel("");
    setButtonImage("");
    setButtonImageAlt("");
    setShowLabel(false);
  };

  // Get default icon for current capability
  const getDefaultIcon = (): string => {
    return selectedCapabilityId ? getCapabilityIcon(selectedCapabilityId) : "";
  };

  // Get preview URL for button image
  const getPreviewUrl = (): string | null => {
    if (!buttonImage) return null;
    if (buttonImage.startsWith("http://") || buttonImage.startsWith("https://")) {
      return buttonImage;
    }
    // Convert local file path to Tauri asset URL
    return convertFileSrc(buttonImage);
  };

  // Get preview URL for alternate button image
  const getAltPreviewUrl = (): string | null => {
    if (!buttonImageAlt) return null;
    if (buttonImageAlt.startsWith("http://") || buttonImageAlt.startsWith("https://")) {
      return buttonImageAlt;
    }
    return convertFileSrc(buttonImageAlt);
  };

  // Check if the selected capability supports state-based images
  const supportsStateImages =
    selectedCapabilityId === "SystemAudio" ||
    selectedCapabilityId === "Mute" ||
    selectedCapabilityId === "Microphone" ||
    selectedCapabilityId === "MicMute" ||
    selectedCapabilityId === "MediaPlayPause" ||
    selectedCapabilityId === "ElgatoKeyLight" ||
    (selectedCapabilityId === "RunCommand" && commandToggle);

  // Check if this is a Key Light capability
  const isKeyLightCapability = selectedCapabilityId === "ElgatoKeyLight";

  // Get description for alternate image based on capability
  const getAltImageDescription = (): string => {
    if (selectedCapabilityId === "SystemAudio" || selectedCapabilityId === "Mute") {
      return "Image shown when audio is muted";
    }
    if (selectedCapabilityId === "Microphone" || selectedCapabilityId === "MicMute") {
      return "Image shown when microphone is muted";
    }
    if (selectedCapabilityId === "MediaPlayPause") {
      return "Image shown when media is playing";
    }
    if (selectedCapabilityId === "ElgatoKeyLight") {
      return "Image shown when light is on";
    }
    if (selectedCapabilityId === "RunCommand" && commandToggle) {
      return "Image shown when toggled active";
    }
    return "Alternate state image";
  };

  // Get label for alternate image based on capability
  const getAltImageLabel = (): string => {
    if (selectedCapabilityId === "SystemAudio" || selectedCapabilityId === "Mute") {
      return "Muted Image";
    }
    if (selectedCapabilityId === "Microphone" || selectedCapabilityId === "MicMute") {
      return "Mic Muted Image";
    }
    if (selectedCapabilityId === "MediaPlayPause") {
      return "Playing Image";
    }
    if (selectedCapabilityId === "ElgatoKeyLight") {
      return "Light On Image";
    }
    if (selectedCapabilityId === "RunCommand" && commandToggle) {
      return "Active Image";
    }
    return "Alternate Image";
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

  const previewUrl = getPreviewUrl();

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

      {(selectedCapabilityId === "SystemAudio" ||
        selectedCapabilityId === "VolumeUp" ||
        selectedCapabilityId === "VolumeDown" ||
        selectedCapabilityId === "Microphone" ||
        selectedCapabilityId === "MicVolumeUp" ||
        selectedCapabilityId === "MicVolumeDown") && (
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
        <>
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
          <div className="editor-field checkbox-field">
            <label className="checkbox-label" htmlFor="toggle-checkbox">
              <input
                id="toggle-checkbox"
                type="checkbox"
                checked={commandToggle}
                onChange={(e) => setCommandToggle(e.target.checked)}
              />
              Toggle Mode
            </label>
            <p className="field-description">
              Alternate between default and active image on each press (e.g., for start/stop dictation)
            </p>
          </div>
        </>
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

      {isKeyLightCapability && (
        <div className="editor-field">
          <label htmlFor="keylight-ip-input">Key Light IP Address</label>
          <input
            id="keylight-ip-input"
            type="text"
            value={keyLightIp}
            onChange={(e) => setKeyLightIp(e.target.value)}
            placeholder="192.168.1.100"
          />
          <p className="field-description">
            IP address of your Elgato Key Light (default port 9123)
          </p>
        </div>
      )}

      {selectedCapabilityId && (
        <>
          <div className="editor-field">
            <label>Icon</label>
            <div className="icon-picker">
              <button
                type="button"
                className={`icon-option ${customIcon === "" ? "selected" : ""}`}
                onClick={() => setCustomIcon("")}
                title="Use default icon"
              >
                {getDefaultIcon()}
              </button>
              {ICONS.map((icon) => (
                <button
                  key={icon}
                  type="button"
                  className={`icon-option ${customIcon === icon ? "selected" : ""}`}
                  onClick={() => setCustomIcon(icon)}
                >
                  {icon}
                </button>
              ))}
            </div>
          </div>

          <div className="editor-field">
            <label htmlFor="label-input">Custom Label</label>
            <input
              id="label-input"
              type="text"
              value={customLabel}
              onChange={(e) => setCustomLabel(e.target.value)}
              placeholder="Optional custom text"
            />
            <p className="field-description">
              Leave empty to use default name
            </p>
          </div>

          {/* Hardware Image - for buttons and encoders */}
          {supportsHardwareImage && (
            <>
              <div className="editor-field">
                <label htmlFor="button-image-input">Button Image (Hardware)</label>
                <div className="image-source">
                  <input
                    id="button-image-input"
                    type="text"
                    value={buttonImage}
                    onChange={(e) => setButtonImage(e.target.value)}
                    placeholder="File path or URL"
                  />
                  <button
                    type="button"
                    className="btn-browse"
                    onClick={() => handleOpenIconBrowser("default")}
                  >
                    Browse
                  </button>
                </div>
                {previewUrl && (
                  <div className="image-preview-container">
                    <img src={previewUrl} alt="Button preview" className="image-preview" />
                  </div>
                )}
                <p className="field-description">
                  {selectedInput?.type === "Button"
                    ? "PNG/JPEG file or URL for the hardware button display"
                    : "PNG/JPEG file or URL for the LCD strip display"}
                </p>
              </div>

              <div className="editor-field checkbox-field">
                <label className="checkbox-label" htmlFor="show-label-checkbox">
                  <input
                    id="show-label-checkbox"
                    type="checkbox"
                    checked={showLabel}
                    onChange={(e) => setShowLabel(e.target.checked)}
                  />
                  Show label on hardware button
                </label>
                <p className="field-description">
                  Renders the label text on the hardware display
                </p>
              </div>

              {/* Alternate image for stateful capabilities */}
              {supportsStateImages && (
                <div className="editor-field">
                  <label htmlFor="button-image-alt-input">
                    {getAltImageLabel()}
                  </label>
                  <div className="image-source">
                    <input
                      id="button-image-alt-input"
                      type="text"
                      value={buttonImageAlt}
                      onChange={(e) => setButtonImageAlt(e.target.value)}
                      placeholder="File path or URL"
                    />
                    <button
                      type="button"
                      className="btn-browse"
                      onClick={() => handleOpenIconBrowser("alt")}
                    >
                      Browse
                    </button>
                  </div>
                  {getAltPreviewUrl() && (
                    <div className="image-preview-container">
                      <img src={getAltPreviewUrl()!} alt="Alternate preview" className="image-preview" />
                    </div>
                  )}
                  <p className="field-description">
                    {getAltImageDescription()}
                  </p>
                </div>
              )}
            </>
          )}
        </>
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

      <IconBrowser
        isOpen={showIconBrowser}
        onClose={() => setShowIconBrowser(false)}
        onSelect={handleIconSelect}
      />
    </div>
  );
}
