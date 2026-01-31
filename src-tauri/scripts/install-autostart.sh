#!/bin/bash
# Install autostart systemd user service for Stream Deck controller
# Reads app name from tauri.conf.json for consistent naming across all components

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find tauri.conf.json (look in parent directory from scripts/)
TAURI_CONF="$SCRIPT_DIR/../tauri.conf.json"
if [ ! -f "$TAURI_CONF" ]; then
    # Try looking relative to installed location
    TAURI_CONF="/usr/share/archdeck/tauri.conf.json"
fi

# Extract productName from tauri.conf.json
if [ -f "$TAURI_CONF" ]; then
    # Simple extraction without jq dependency
    APP_NAME=$(grep -o '"productName"[[:space:]]*:[[:space:]]*"[^"]*"' "$TAURI_CONF" | sed 's/.*: *"\([^"]*\)".*/\1/')
fi

# Fallback to default if extraction failed
if [ -z "$APP_NAME" ]; then
    echo -e "${YELLOW}Warning: Could not read productName from tauri.conf.json, using default${NC}"
    APP_NAME="ArchDeck"
fi

APP_NAME_LOWER=$(echo "$APP_NAME" | tr '[:upper:]' '[:lower:]')

# Known previous app names to clean up (add old names here when renaming)
PREVIOUS_NAMES=("archdeck" "streamdeck-linux" "streamdecklinux")

echo -e "${GREEN}Installing $APP_NAME autostart service...${NC}"
echo "  App Name: $APP_NAME"
echo "  Service Name: ${APP_NAME_LOWER}.service"

# --- Cleanup old installations with different names ---
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
CONFIG_BASE_DIR="$HOME/.config"

for OLD_NAME in "${PREVIOUS_NAMES[@]}"; do
    # Skip if it's the current name
    if [ "$OLD_NAME" = "$APP_NAME_LOWER" ]; then
        continue
    fi

    OLD_SERVICE="$SYSTEMD_USER_DIR/${OLD_NAME}.service"
    OLD_CONFIG_DIR="$CONFIG_BASE_DIR/$OLD_NAME"
    NEW_CONFIG_DIR="$CONFIG_BASE_DIR/$APP_NAME_LOWER"

    # Clean up old systemd service
    if [ -f "$OLD_SERVICE" ]; then
        echo -e "${YELLOW}Found old service: $OLD_SERVICE${NC}"

        # Stop the service if running
        if systemctl --user is-active "$OLD_NAME" &>/dev/null; then
            echo "  Stopping old service..."
            systemctl --user stop "$OLD_NAME" 2>/dev/null || true
        fi

        # Disable the service
        if systemctl --user is-enabled "$OLD_NAME" &>/dev/null; then
            echo "  Disabling old service..."
            systemctl --user disable "$OLD_NAME" 2>/dev/null || true
        fi

        # Remove the old service file
        echo "  Removing old service file..."
        rm -f "$OLD_SERVICE"
        echo -e "${GREEN}  Cleaned up old service: $OLD_NAME${NC}"
    fi

    # Migrate config directory if old exists and new doesn't
    if [ -d "$OLD_CONFIG_DIR" ] && [ ! -d "$NEW_CONFIG_DIR" ]; then
        echo -e "${YELLOW}Migrating config from $OLD_CONFIG_DIR to $NEW_CONFIG_DIR${NC}"
        mv "$OLD_CONFIG_DIR" "$NEW_CONFIG_DIR"
        echo -e "${GREEN}  Config migrated successfully${NC}"
    elif [ -d "$OLD_CONFIG_DIR" ] && [ -d "$NEW_CONFIG_DIR" ]; then
        echo -e "${YELLOW}Warning: Both old ($OLD_CONFIG_DIR) and new ($NEW_CONFIG_DIR) config dirs exist${NC}"
        echo "  Old config preserved - merge manually if needed"
    fi
done

# Reload after cleanup
if [ -d "$SYSTEMD_USER_DIR" ]; then
    systemctl --user daemon-reload 2>/dev/null || true
fi

echo ""

# Determine install path
INSTALL_PATH=""
if [ -x "/usr/bin/$APP_NAME_LOWER" ]; then
    INSTALL_PATH="/usr/bin"
elif [ -x "$HOME/.local/bin/$APP_NAME_LOWER" ]; then
    INSTALL_PATH="$HOME/.local/bin"
elif [ -x "$(dirname "$SCRIPT_DIR")/target/release/$APP_NAME_LOWER" ]; then
    INSTALL_PATH="$(dirname "$SCRIPT_DIR")/target/release"
else
    echo -e "${RED}Error: Could not find $APP_NAME_LOWER binary${NC}"
    echo "Expected locations:"
    echo "  - /usr/bin/$APP_NAME_LOWER"
    echo "  - ~/.local/bin/$APP_NAME_LOWER"
    echo "  - ./target/release/$APP_NAME_LOWER"
    exit 1
fi

echo "  Binary Path: $INSTALL_PATH/$APP_NAME_LOWER"

# Create systemd user directory
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
mkdir -p "$SYSTEMD_USER_DIR"

# Generate service file from template
SERVICE_TEMPLATE="$SCRIPT_DIR/archdeck.service.template"
SERVICE_FILE="$SYSTEMD_USER_DIR/${APP_NAME_LOWER}.service"

if [ -f "$SERVICE_TEMPLATE" ]; then
    # Replace placeholders in template
    sed -e "s|{{APP_NAME}}|$APP_NAME|g" \
        -e "s|{{APP_NAME_LOWER}}|$APP_NAME_LOWER|g" \
        -e "s|{{INSTALL_PATH}}|$INSTALL_PATH|g" \
        "$SERVICE_TEMPLATE" > "$SERVICE_FILE"
else
    # Generate service file inline if template not found
    cat > "$SERVICE_FILE" << EOF
[Unit]
Description=$APP_NAME - Stream Deck Controller
After=graphical-session.target
Wants=graphical-session.target

[Service]
Type=simple
ExecStart=$INSTALL_PATH/$APP_NAME_LOWER --hidden
Restart=on-failure
RestartSec=5
Environment=DISPLAY=:0
Environment=WAYLAND_DISPLAY=wayland-0

[Install]
WantedBy=default.target
EOF
fi

echo -e "${GREEN}Created service file: $SERVICE_FILE${NC}"

# Reload systemd user daemon
echo "Reloading systemd user daemon..."
systemctl --user daemon-reload

# Enable the service
echo "Enabling $APP_NAME_LOWER service..."
systemctl --user enable "$APP_NAME_LOWER.service"

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Commands to manage the service:"
echo "  Start now:    systemctl --user start $APP_NAME_LOWER"
echo "  Stop:         systemctl --user stop $APP_NAME_LOWER"
echo "  Status:       systemctl --user status $APP_NAME_LOWER"
echo "  Disable:      systemctl --user disable $APP_NAME_LOWER"
echo "  View logs:    journalctl --user -u $APP_NAME_LOWER -f"
echo ""
echo "The service will automatically start on login."
echo "To start it now without rebooting, run:"
echo "  systemctl --user start $APP_NAME_LOWER"
