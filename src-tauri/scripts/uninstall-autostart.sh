#!/bin/bash
# Uninstall autostart systemd user service for Stream Deck controller
# Reads app name from tauri.conf.json for consistent naming

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Find tauri.conf.json
TAURI_CONF="$SCRIPT_DIR/../tauri.conf.json"
if [ ! -f "$TAURI_CONF" ]; then
    TAURI_CONF="/usr/share/archdeck/tauri.conf.json"
fi

# Extract productName from tauri.conf.json
if [ -f "$TAURI_CONF" ]; then
    APP_NAME=$(grep -o '"productName"[[:space:]]*:[[:space:]]*"[^"]*"' "$TAURI_CONF" | sed 's/.*: *"\([^"]*\)".*/\1/')
fi

# Fallback to default if extraction failed
if [ -z "$APP_NAME" ]; then
    APP_NAME="ArchDeck"
fi

APP_NAME_LOWER=$(echo "$APP_NAME" | tr '[:upper:]' '[:lower:]')

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
SERVICE_FILE="$SYSTEMD_USER_DIR/${APP_NAME_LOWER}.service"
CONFIG_DIR="$HOME/.config/$APP_NAME_LOWER"

echo -e "${YELLOW}Uninstalling $APP_NAME autostart service...${NC}"

# Stop service if running
if systemctl --user is-active "$APP_NAME_LOWER" &>/dev/null; then
    echo "Stopping $APP_NAME_LOWER service..."
    systemctl --user stop "$APP_NAME_LOWER"
fi

# Disable service
if systemctl --user is-enabled "$APP_NAME_LOWER" &>/dev/null; then
    echo "Disabling $APP_NAME_LOWER service..."
    systemctl --user disable "$APP_NAME_LOWER"
fi

# Remove service file
if [ -f "$SERVICE_FILE" ]; then
    echo "Removing service file: $SERVICE_FILE"
    rm -f "$SERVICE_FILE"
fi

# Reload systemd
systemctl --user daemon-reload

echo -e "${GREEN}Service uninstalled successfully.${NC}"
echo ""

# Ask about config directory
if [ -d "$CONFIG_DIR" ]; then
    echo -e "${YELLOW}Config directory exists: $CONFIG_DIR${NC}"
    read -p "Delete config directory and all bindings? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        rm -rf "$CONFIG_DIR"
        echo -e "${GREEN}Config directory removed.${NC}"
    else
        echo "Config directory preserved."
    fi
fi

echo ""
echo "Uninstall complete."
