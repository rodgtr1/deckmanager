import { useState, DragEvent } from "react";
import { convertFileSrc } from "@tauri-apps/api/core";
import {
  DeviceInfo,
  InputRef,
  Binding,
  SystemState,
  inputsMatch,
  getCapabilityDisplayName,
  buttonRef,
  encoderRef,
  encoderPressRef,
} from "../types";
import { getCapabilityIcon } from "./CapabilityBrowser";

// Get preview URL for button image (handles both URLs and local paths)
function getImageUrl(imagePath: string): string {
  if (imagePath.startsWith("http://") || imagePath.startsWith("https://")) {
    return imagePath;
  }
  const assetUrl = convertFileSrc(imagePath);
  console.log("getImageUrl:", { imagePath, assetUrl });
  return assetUrl;
}

interface DeviceLayoutProps {
  device: DeviceInfo;
  bindings: Binding[];
  selectedInput: InputRef | null;
  activeInputs: Set<string>;
  systemState: SystemState;
  onSelectInput: (input: InputRef) => void;
  onDrop?: (input: InputRef, capabilityId: string) => void;
}

// Get effective image based on system state
function getEffectiveImage(binding: Binding, state: SystemState): string | undefined {
  const capType = binding.capability.type;

  // Check if this capability has an "active" state
  const isActive =
    (capType === "ToggleMute" && state.is_muted) ||
    (capType === "MediaPlayPause" && state.is_playing);

  // If active and we have an alt image, use it
  if (isActive && binding.button_image_alt) {
    return binding.button_image_alt;
  }

  // Otherwise use the default image
  return binding.button_image;
}

// Serialize InputRef for Set membership
function inputKey(input: InputRef): string {
  if (input.type === "Swipe") return "swipe";
  return `${input.type}:${input.index}`;
}

export default function DeviceLayout({
  device,
  bindings,
  selectedInput,
  activeInputs,
  systemState,
  onSelectInput,
  onDrop,
}: DeviceLayoutProps) {
  // Track which input is being dragged over
  const [dragOverInput, setDragOverInput] = useState<string | null>(null);

  // Find binding for a given input
  const getBinding = (input: InputRef): Binding | undefined => {
    return bindings.find((b) => inputsMatch(b.input, input));
  };

  // Check if an input is selected
  const isSelected = (input: InputRef): boolean => {
    return selectedInput !== null && inputsMatch(selectedInput, input);
  };

  // Check if an input is currently active (pressed)
  const isActive = (input: InputRef): boolean => {
    return activeInputs.has(inputKey(input));
  };

  // Check if an input is being dragged over
  const isDragOver = (input: InputRef): boolean => {
    return dragOverInput === inputKey(input);
  };

  // Handle drag over event
  const handleDragOver = (e: DragEvent, input: InputRef) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setDragOverInput(inputKey(input));
  };

  // Handle drag leave event
  const handleDragLeave = () => {
    setDragOverInput(null);
  };

  // Handle drop event
  const handleDrop = (e: DragEvent, input: InputRef) => {
    e.preventDefault();
    setDragOverInput(null);
    const capabilityId = e.dataTransfer.getData("text/plain");
    if (capabilityId && onDrop) {
      onDrop(input, capabilityId);
    }
  };

  // Get display content for a binding (icon + label or default)
  const getBindingDisplay = (binding: Binding | undefined): { icon: string; label: string } | null => {
    if (!binding) return null;
    const icon = binding.icon || getCapabilityIcon(binding.capability.type);
    const label = binding.label || getCapabilityDisplayName(binding.capability);
    return { icon, label };
  };

  // Render button grid
  const renderButtons = () => {
    const buttons = [];
    for (let i = 0; i < device.button_count; i++) {
      const input = buttonRef(i);
      const binding = getBinding(input);
      const display = getBindingDisplay(binding);
      const selected = isSelected(input);
      const active = isActive(input);
      const dragOver = isDragOver(input);
      // Get effective image based on state (uses alt image when active)
      const effectiveImage = binding ? getEffectiveImage(binding, systemState) : undefined;
      const hasButtonImage = !!effectiveImage;

      buttons.push(
        <button
          key={`btn-${i}`}
          className={`deck-button ${selected ? "selected" : ""} ${active ? "active" : ""} ${dragOver ? "drag-over" : ""} ${hasButtonImage ? "has-image" : ""}`}
          onClick={() => onSelectInput(input)}
          onDragOver={(e) => handleDragOver(e, input)}
          onDragLeave={handleDragLeave}
          onDrop={(e) => handleDrop(e, input)}
        >
          <span className="button-index">{i + 1}</span>
          {hasButtonImage ? (
            <>
              <img
                src={getImageUrl(effectiveImage)}
                alt=""
                className="button-image"
                onError={(e) => {
                  console.error("Failed to load button image:", effectiveImage);
                  (e.target as HTMLImageElement).style.display = 'none';
                }}
              />
              {display && <span className="button-label">{display.label}</span>}
            </>
          ) : (
            display && (
              <>
                <span className="button-icon">{display.icon}</span>
                <span className="button-label">{display.label}</span>
              </>
            )
          )}
        </button>
      );
    }
    return buttons;
  };

  // Render encoder row
  const renderEncoders = () => {
    const encoders = [];
    for (let i = 0; i < device.encoder_count; i++) {
      const rotateInput = encoderRef(i);
      const pressInput = encoderPressRef(i);
      const rotateBinding = getBinding(rotateInput);
      const pressBinding = getBinding(pressInput);
      const rotateDisplay = getBindingDisplay(rotateBinding);
      const pressDisplay = getBindingDisplay(pressBinding);
      const rotateSelected = isSelected(rotateInput);
      const pressSelected = isSelected(pressInput);
      const rotateActive = isActive(rotateInput);
      const pressActive = isActive(pressInput);
      const rotateDragOver = isDragOver(rotateInput);
      const pressDragOver = isDragOver(pressInput);

      // Determine which image to show (priority: pressBinding > rotateBinding, considering state)
      const pressImage = pressBinding ? getEffectiveImage(pressBinding, systemState) : undefined;
      const rotateImage = rotateBinding ? getEffectiveImage(rotateBinding, systemState) : undefined;
      const encoderImage = pressImage || rotateImage;
      const hasEncoderImage = !!encoderImage;

      encoders.push(
        <div key={`enc-${i}`} className="encoder-group">
          <button
            className={`encoder-ring ${rotateSelected ? "selected" : ""} ${rotateActive ? "active" : ""} ${rotateDragOver ? "drag-over" : ""}`}
            onClick={() => onSelectInput(rotateInput)}
            onDragOver={(e) => handleDragOver(e, rotateInput)}
            onDragLeave={handleDragLeave}
            onDrop={(e) => handleDrop(e, rotateInput)}
            title="Encoder rotation"
          >
            <div
              className={`encoder-center ${pressSelected ? "selected" : ""} ${pressActive ? "active" : ""} ${pressDragOver ? "drag-over" : ""} ${hasEncoderImage ? "has-image" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onSelectInput(pressInput);
              }}
              onDragOver={(e) => {
                e.stopPropagation();
                handleDragOver(e, pressInput);
              }}
              onDragLeave={(e) => {
                e.stopPropagation();
                handleDragLeave();
              }}
              onDrop={(e) => {
                e.stopPropagation();
                handleDrop(e, pressInput);
              }}
              title="Encoder press"
            >
              {hasEncoderImage && encoderImage ? (
                <img
                  src={getImageUrl(encoderImage)}
                  alt=""
                  className="encoder-image"
                  onError={(e) => {
                    console.error("Failed to load encoder image:", encoderImage);
                    // Hide broken image
                    (e.target as HTMLImageElement).style.display = 'none';
                  }}
                />
              ) : (
                pressDisplay && (
                  <span className="encoder-label">{pressDisplay.icon}</span>
                )
              )}
            </div>
          </button>
          <div className="encoder-info">
            <span className="encoder-index">E{i + 1}</span>
            {rotateDisplay && (
              <span className="encoder-rotate-label">
                {rotateDisplay.icon} {rotateDisplay.label}
              </span>
            )}
          </div>
        </div>
      );
    }
    return encoders;
  };

  // Render touch strip
  const renderTouchStrip = () => {
    const swipeInput: InputRef = { type: "Swipe" };
    const selected = isSelected(swipeInput);
    const active = isActive(swipeInput);
    const binding = getBinding(swipeInput);
    const display = getBindingDisplay(binding);
    const dragOver = isDragOver(swipeInput);

    return (
      <div
        className={`touch-strip ${selected ? "selected" : ""} ${active ? "active" : ""} ${dragOver ? "drag-over" : ""}`}
        onClick={() => onSelectInput(swipeInput)}
        onDragOver={(e) => handleDragOver(e, swipeInput)}
        onDragLeave={handleDragLeave}
        onDrop={(e) => handleDrop(e, swipeInput)}
      >
        <span className="touch-strip-label">
          {display ? `${display.icon} ${display.label}` : "Touch Strip"}
        </span>
      </div>
    );
  };

  return (
    <div className="device-layout">
      <h2 className="device-title">{device.model}</h2>

      <div
        className="button-grid"
        style={{
          gridTemplateColumns: `repeat(${device.columns}, 1fr)`,
          gridTemplateRows: `repeat(${device.rows}, 1fr)`,
        }}
      >
        {renderButtons()}
      </div>

      {device.has_touch_strip && renderTouchStrip()}

      {device.encoder_count > 0 && (
        <div className="encoder-row">{renderEncoders()}</div>
      )}
    </div>
  );
}

// Export helper for creating input keys (for activeInputs Set)
export { inputKey };
