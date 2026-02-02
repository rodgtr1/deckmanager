import { useState, useEffect, useCallback } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  InputRef,
  Capability,
  CapabilityInfo,
  Binding,
  inputsMatch,
  getInputDisplayName,
} from "../types";
import IconBrowser from "./IconBrowser";
import { isSvgUrl, colorizeSvgForPreview } from "../utils/svg";

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
    page?: number,
    iconColor?: string,
    iconColorAlt?: string
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
  const [customLabel, setCustomLabel] = useState<string>("");
  const [buttonImage, setButtonImage] = useState<string>("");
  const [buttonImageAlt, setButtonImageAlt] = useState<string>("");
  const [showLabel, setShowLabel] = useState<boolean>(false);
  const [showIconBrowser, setShowIconBrowser] = useState<boolean>(false);
  const [iconBrowserTarget, setIconBrowserTarget] = useState<"default" | "alt">("default");
  const [keyLightIp, setKeyLightIp] = useState<string>("192.168.1.100");
  const [commandToggle, setCommandToggle] = useState<boolean>(false);
  const [iconColor, setIconColor] = useState<string>("#ffffff");
  const [iconColorAlt, setIconColorAlt] = useState<string>("#ffffff");
  // Preview URLs (colorized SVG data URLs for UI display)
  const [previewUrl, setPreviewUrl] = useState<string>("");
  const [previewUrlAlt, setPreviewUrlAlt] = useState<string>("");

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
    const loadBinding = async () => {
      if (currentBinding) {
        setSelectedCapabilityId(currentBinding.capability.type);
        setCustomLabel(currentBinding.label || "");
        setButtonImage(currentBinding.button_image || "");
        setButtonImageAlt(currentBinding.button_image_alt || "");
        setShowLabel(currentBinding.show_label || false);
        setIconColor(currentBinding.icon_color || "#ffffff");
        setIconColorAlt(currentBinding.icon_color_alt || "#ffffff");

        // Generate preview URLs for SVG icons
        const imgUrl = currentBinding.button_image || "";
        const imgColor = currentBinding.icon_color || "#ffffff";
        if (imgUrl && isSvgUrl(imgUrl)) {
          const preview = await colorizeSvgForPreview(imgUrl, imgColor);
          setPreviewUrl(preview);
        } else {
          setPreviewUrl("");
        }

        const altUrl = currentBinding.button_image_alt || "";
        const altColor = currentBinding.icon_color_alt || "#ffffff";
        if (altUrl && isSvgUrl(altUrl)) {
          const preview = await colorizeSvgForPreview(altUrl, altColor);
          setPreviewUrlAlt(preview);
        } else {
          setPreviewUrlAlt("");
        }

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
        setCustomLabel("");
        setButtonImage("");
        setButtonImageAlt("");
        setShowLabel(false);
        setKeyLightIp("192.168.1.100");
        setCommandToggle(false);
        setIconColor("#ffffff");
        setIconColorAlt("#ffffff");
        setPreviewUrl("");
        setPreviewUrlAlt("");
      }
    };
    loadBinding();
  }, [currentBinding, selectedInput]);

  const handleOpenIconBrowser = (target: "default" | "alt") => {
    setIconBrowserTarget(target);
    setShowIconBrowser(true);
  };

  const handleIconSelect = async (iconUrl: string) => {
    if (iconBrowserTarget === "default") {
      // Store original URL (Rust will handle colorization)
      setButtonImage(iconUrl);
      if (isSvgUrl(iconUrl)) {
        // Generate colorized preview for UI
        const colorized = await colorizeSvgForPreview(iconUrl, iconColor);
        setPreviewUrl(colorized);
      } else {
        setPreviewUrl("");
      }
    } else {
      setButtonImageAlt(iconUrl);
      if (isSvgUrl(iconUrl)) {
        const colorized = await colorizeSvgForPreview(iconUrl, iconColorAlt);
        setPreviewUrlAlt(colorized);
      } else {
        setPreviewUrlAlt("");
      }
    }
  };

  // Handle color change for default icon
  const handleColorChange = useCallback(async (newColor: string) => {
    setIconColor(newColor);
    if (buttonImage && isSvgUrl(buttonImage)) {
      const colorized = await colorizeSvgForPreview(buttonImage, newColor);
      setPreviewUrl(colorized);
    }
  }, [buttonImage]);

  // Handle color change for alt icon
  const handleColorChangeAlt = useCallback(async (newColor: string) => {
    setIconColorAlt(newColor);
    if (buttonImageAlt && isSvgUrl(buttonImageAlt)) {
      const colorized = await colorizeSvgForPreview(buttonImageAlt, newColor);
      setPreviewUrlAlt(colorized);
    }
  }, [buttonImageAlt]);

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

    // Pass label only if customized (no icon picker anymore)
    const icon = undefined;
    const label = customLabel || undefined;
    // For button image and show label, pass the actual values (not using || which breaks false)
    const image = buttonImage.trim() || undefined;
    const imageAlt = buttonImageAlt.trim() || undefined;
    const showLabelOnButton = showLabel;
    // Pass icon colors only if image is an SVG
    const color = image && isSvgUrl(image) ? iconColor : undefined;
    const colorAlt = imageAlt && isSvgUrl(imageAlt) ? iconColorAlt : undefined;

    onSetBinding(selectedInput, capability, icon, label, image, imageAlt, showLabelOnButton, currentPage, color, colorAlt);

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
        onSetBinding(pressInput, capability, icon, label, image, imageAlt, showLabelOnButton, currentPage, color, colorAlt);
      } else if (selectedInput.type === "EncoderPress") {
        // Also create Encoder binding
        const rotateInput: InputRef = { type: "Encoder", index: selectedInput.index };
        onSetBinding(rotateInput, capability, icon, label, image, imageAlt, showLabelOnButton, currentPage, color, colorAlt);
      }
    }
  };

  const handleRemove = () => {
    if (!selectedInput) return;
    onRemoveBinding(selectedInput, currentPage);
    setSelectedCapabilityId("");
    setCustomLabel("");
    setButtonImage("");
    setButtonImageAlt("");
    setShowLabel(false);
    setIconColor("#ffffff");
    setIconColorAlt("#ffffff");
    setPreviewUrl("");
    setPreviewUrlAlt("");
  };

  // Get preview URL for button image
  // Use colorized preview for SVGs, otherwise use original URL
  const getPreviewUrl = (): string | null => {
    if (!buttonImage) return null;
    // If we have a colorized SVG preview, use it
    if (previewUrl) return previewUrl;
    // For URLs, return as-is
    if (buttonImage.startsWith("http://") || buttonImage.startsWith("https://")) {
      return buttonImage;
    }
    // Convert local file path to Tauri asset URL
    return convertFileSrc(buttonImage);
  };

  // Get preview URL for alternate button image
  const getAltPreviewUrl = (): string | null => {
    if (!buttonImageAlt) return null;
    // If we have a colorized SVG preview, use it
    if (previewUrlAlt) return previewUrlAlt;
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
                {getPreviewUrl() && (
                  <div className="image-preview-container">
                    <img src={getPreviewUrl()!} alt="Button preview" className="image-preview" />
                    {buttonImage && isSvgUrl(buttonImage) && (
                      <div className="color-picker-inline">
                        <label htmlFor="icon-color">Color:</label>
                        <input
                          id="icon-color"
                          type="color"
                          value={iconColor}
                          onChange={(e) => handleColorChange(e.target.value)}
                        />
                      </div>
                    )}
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
                      {buttonImageAlt && isSvgUrl(buttonImageAlt) && (
                        <div className="color-picker-inline">
                          <label htmlFor="icon-color-alt">Color:</label>
                          <input
                            id="icon-color-alt"
                            type="color"
                            value={iconColorAlt}
                            onChange={(e) => handleColorChangeAlt(e.target.value)}
                          />
                        </div>
                      )}
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
