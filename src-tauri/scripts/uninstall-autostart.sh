#!/bin/bash
# Uninstall autostart for Stream Deck controller
# Removes XDG autostart, systemd service, and Hyprland/Sway config entries

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
    TAURI_CONF="/usr/share/deckmanager/tauri.conf.json"
fi

# Extract productName from tauri.conf.json
if [ -f "$TAURI_CONF" ]; then
    APP_NAME=$(grep -o '"productName"[[:space:]]*:[[:space:]]*"[^"]*"' "$TAURI_CONF" | sed 's/.*: *"\([^"]*\)".*/\1/')
fi

# Fallback to default if extraction failed
if [ -z "$APP_NAME" ]; then
    APP_NAME="Deck Manager"
fi

APP_NAME_LOWER=$(echo "$APP_NAME" | tr '[:upper:]' '[:lower:]' | tr -d ' ')

SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
SERVICE_FILE="$SYSTEMD_USER_DIR/${APP_NAME_LOWER}.service"
AUTOSTART_DIR="$HOME/.config/autostart"
DESKTOP_FILE="$AUTOSTART_DIR/${APP_NAME_LOWER}.desktop"
CONFIG_DIR="$HOME/.config/$APP_NAME_LOWER"
HYPRLAND_CONF="$HOME/.config/hypr/hyprland.conf"
SWAY_CONF="$HOME/.config/sway/config"

echo -e "${YELLOW}Uninstalling $APP_NAME autostart...${NC}"

# --- Remove from Hyprland config ---
if [ -f "$HYPRLAND_CONF" ]; then
    if grep -qF "$APP_NAME_LOWER" "$HYPRLAND_CONF" 2>/dev/null; then
        echo "Removing from Hyprland config..."
        # Remove the exec-once line and the comment above it
        sed -i "/$APP_NAME_LOWER/d" "$HYPRLAND_CONF"
        sed -i "/# $APP_NAME - Stream Deck Controller/d" "$HYPRLAND_CONF"
        echo -e "${GREEN}Removed from $HYPRLAND_CONF${NC}"
    fi
fi

# --- Remove from Sway config ---
if [ -f "$SWAY_CONF" ]; then
    if grep -qF "$APP_NAME_LOWER" "$SWAY_CONF" 2>/dev/null; then
        echo "Removing from Sway config..."
        sed -i "/$APP_NAME_LOWER/d" "$SWAY_CONF"
        sed -i "/# $APP_NAME - Stream Deck Controller/d" "$SWAY_CONF"
        echo -e "${GREEN}Removed from $SWAY_CONF${NC}"
    fi
fi

# --- Remove XDG autostart ---
if [ -f "$DESKTOP_FILE" ]; then
    echo "Removing XDG autostart: $DESKTOP_FILE"
    rm -f "$DESKTOP_FILE"
    echo -e "${GREEN}XDG autostart removed.${NC}"
fi

# --- Remove systemd service ---
if systemctl --user is-active "$APP_NAME_LOWER" &>/dev/null; then
    echo "Stopping $APP_NAME_LOWER service..."
    systemctl --user stop "$APP_NAME_LOWER"
fi

if systemctl --user is-enabled "$APP_NAME_LOWER" &>/dev/null; then
    echo "Disabling $APP_NAME_LOWER service..."
    systemctl --user disable "$APP_NAME_LOWER"
fi

if [ -f "$SERVICE_FILE" ]; then
    echo "Removing systemd service: $SERVICE_FILE"
    rm -f "$SERVICE_FILE"
    echo -e "${GREEN}Systemd service removed.${NC}"
fi

systemctl --user daemon-reload 2>/dev/null || true

echo ""
echo -e "${GREEN}Autostart uninstalled successfully.${NC}"
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
