import {
  DeviceInfo,
  InputRef,
  Binding,
  inputsMatch,
  getCapabilityDisplayName,
  buttonRef,
  encoderRef,
  encoderPressRef,
} from "../types";

interface DeviceLayoutProps {
  device: DeviceInfo;
  bindings: Binding[];
  selectedInput: InputRef | null;
  activeInputs: Set<string>;
  onSelectInput: (input: InputRef) => void;
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
  onSelectInput,
}: DeviceLayoutProps) {
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

  // Render button grid
  const renderButtons = () => {
    const buttons = [];
    for (let i = 0; i < device.button_count; i++) {
      const input = buttonRef(i);
      const binding = getBinding(input);
      const selected = isSelected(input);
      const active = isActive(input);

      buttons.push(
        <button
          key={`btn-${i}`}
          className={`deck-button ${selected ? "selected" : ""} ${active ? "active" : ""}`}
          onClick={() => onSelectInput(input)}
        >
          <span className="button-index">{i + 1}</span>
          {binding && (
            <span className="button-label">
              {getCapabilityDisplayName(binding.capability)}
            </span>
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
      const rotateSelected = isSelected(rotateInput);
      const pressSelected = isSelected(pressInput);
      const rotateActive = isActive(rotateInput);
      const pressActive = isActive(pressInput);

      encoders.push(
        <div key={`enc-${i}`} className="encoder-group">
          <button
            className={`encoder-ring ${rotateSelected ? "selected" : ""} ${rotateActive ? "active" : ""}`}
            onClick={() => onSelectInput(rotateInput)}
            title="Encoder rotation"
          >
            <div
              className={`encoder-center ${pressSelected ? "selected" : ""} ${pressActive ? "active" : ""}`}
              onClick={(e) => {
                e.stopPropagation();
                onSelectInput(pressInput);
              }}
              title="Encoder press"
            >
              {pressBinding && (
                <span className="encoder-label">
                  {getCapabilityDisplayName(pressBinding.capability)}
                </span>
              )}
            </div>
          </button>
          <div className="encoder-info">
            <span className="encoder-index">E{i + 1}</span>
            {rotateBinding && (
              <span className="encoder-rotate-label">
                {getCapabilityDisplayName(rotateBinding.capability)}
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

    return (
      <div
        className={`touch-strip ${selected ? "selected" : ""} ${active ? "active" : ""}`}
        onClick={() => onSelectInput(swipeInput)}
      >
        <span className="touch-strip-label">
          {binding ? getCapabilityDisplayName(binding.capability) : "Touch Strip"}
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
