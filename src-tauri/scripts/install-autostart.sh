#!/bin/bash
# Install autostart for Stream Deck controller
# Uses XDG autostart (most compatible) with optional systemd service

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Find tauri.conf.json (look in parent directory from scripts/)
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
    echo -e "${YELLOW}Warning: Could not read productName from tauri.conf.json, using default${NC}"
    APP_NAME="ArchDeck"
fi

APP_NAME_LOWER=$(echo "$APP_NAME" | tr '[:upper:]' '[:lower:]')

echo -e "${GREEN}Installing $APP_NAME autostart...${NC}"
echo "  App Name: $APP_NAME"

# --- Determine install path ---
INSTALL_PATH=""
BINARY_PATH=""
if [ -x "/usr/bin/$APP_NAME_LOWER" ]; then
    BINARY_PATH="/usr/bin/$APP_NAME_LOWER"
elif [ -x "$HOME/.local/bin/$APP_NAME_LOWER" ]; then
    BINARY_PATH="$HOME/.local/bin/$APP_NAME_LOWER"
elif [ -x "$(dirname "$SCRIPT_DIR")/target/release/$APP_NAME_LOWER" ]; then
    BINARY_PATH="$(dirname "$SCRIPT_DIR")/target/release/$APP_NAME_LOWER"
else
    echo -e "${RED}Error: Could not find $APP_NAME_LOWER binary${NC}"
    echo "Expected locations:"
    echo "  - /usr/bin/$APP_NAME_LOWER"
    echo "  - ~/.local/bin/$APP_NAME_LOWER"
    echo "  - ./target/release/$APP_NAME_LOWER"
    exit 1
fi

echo "  Binary: $BINARY_PATH"

# --- Find icon ---
ICON_PATH=""
for icon_loc in \
    "/usr/share/icons/hicolor/256x256/apps/${APP_NAME_LOWER}.png" \
    "/usr/share/icons/hicolor/128x128/apps/${APP_NAME_LOWER}.png" \
    "/usr/share/pixmaps/${APP_NAME_LOWER}.png" \
    "$HOME/.local/share/icons/hicolor/256x256/apps/${APP_NAME_LOWER}.png" \
    "$(dirname "$SCRIPT_DIR")/icons/icon.png"; do
    if [ -f "$icon_loc" ]; then
        ICON_PATH="$icon_loc"
        break
    fi
done

# Fallback to app name (system will search for it)
if [ -z "$ICON_PATH" ]; then
    ICON_PATH="$APP_NAME_LOWER"
fi

# --- Cleanup old systemd services ---
SYSTEMD_USER_DIR="$HOME/.config/systemd/user"
PREVIOUS_NAMES=("archdeck" "streamdeck-linux" "streamdecklinux")

for OLD_NAME in "${PREVIOUS_NAMES[@]}"; do
    OLD_SERVICE="$SYSTEMD_USER_DIR/${OLD_NAME}.service"
    if [ -f "$OLD_SERVICE" ]; then
        echo -e "${YELLOW}Cleaning up old systemd service: $OLD_NAME${NC}"
        systemctl --user stop "$OLD_NAME" 2>/dev/null || true
        systemctl --user disable "$OLD_NAME" 2>/dev/null || true
        rm -f "$OLD_SERVICE"
    fi
done

if [ -d "$SYSTEMD_USER_DIR" ]; then
    systemctl --user daemon-reload 2>/dev/null || true
fi

# --- Cleanup old XDG autostart entries ---
AUTOSTART_DIR="$HOME/.config/autostart"
for OLD_NAME in "${PREVIOUS_NAMES[@]}"; do
    OLD_DESKTOP="$AUTOSTART_DIR/${OLD_NAME}.desktop"
    if [ -f "$OLD_DESKTOP" ] && [ "$OLD_NAME" != "$APP_NAME_LOWER" ]; then
        echo -e "${YELLOW}Cleaning up old autostart: $OLD_NAME.desktop${NC}"
        rm -f "$OLD_DESKTOP"
    fi
done

# ============================================================
# Detect compositor/DE
# ============================================================

COMPOSITOR=""
if [ -n "$HYPRLAND_INSTANCE_SIGNATURE" ] || pgrep -x "Hyprland" > /dev/null 2>&1; then
    COMPOSITOR="hyprland"
elif [ -n "$SWAYSOCK" ] || pgrep -x "sway" > /dev/null 2>&1; then
    COMPOSITOR="sway"
fi

# ============================================================
# HYPRLAND/SWAY: Add exec-once to config
# ============================================================

HYPRLAND_CONF="$HOME/.config/hypr/hyprland.conf"
SWAY_CONF="$HOME/.config/sway/config"
EXEC_LINE="exec-once = $BINARY_PATH --hidden"
SWAY_EXEC_LINE="exec $BINARY_PATH --hidden"

if [ "$COMPOSITOR" = "hyprland" ] && [ -f "$HYPRLAND_CONF" ]; then
    echo ""
    echo -e "${BLUE}Hyprland detected - configuring autostart...${NC}"

    # Check if already configured
    if grep -qF "$BINARY_PATH" "$HYPRLAND_CONF" 2>/dev/null; then
        echo -e "${YELLOW}Already configured in hyprland.conf${NC}"
    else
        # Add exec-once line
        echo "" >> "$HYPRLAND_CONF"
        echo "# $APP_NAME - Stream Deck Controller" >> "$HYPRLAND_CONF"
        echo "$EXEC_LINE" >> "$HYPRLAND_CONF"
        echo -e "${GREEN}Added to $HYPRLAND_CONF:${NC}"
        echo "  $EXEC_LINE"
    fi

elif [ "$COMPOSITOR" = "sway" ] && [ -f "$SWAY_CONF" ]; then
    echo ""
    echo -e "${BLUE}Sway detected - configuring autostart...${NC}"

    # Check if already configured
    if grep -qF "$BINARY_PATH" "$SWAY_CONF" 2>/dev/null; then
        echo -e "${YELLOW}Already configured in sway config${NC}"
    else
        # Add exec line
        echo "" >> "$SWAY_CONF"
        echo "# $APP_NAME - Stream Deck Controller" >> "$SWAY_CONF"
        echo "$SWAY_EXEC_LINE" >> "$SWAY_CONF"
        echo -e "${GREEN}Added to $SWAY_CONF:${NC}"
        echo "  $SWAY_EXEC_LINE"
    fi

else
    # ============================================================
    # OTHER DEs: XDG Autostart (.desktop file)
    # ============================================================

    echo ""
    echo -e "${BLUE}Installing XDG autostart...${NC}"

    mkdir -p "$AUTOSTART_DIR"

    DESKTOP_FILE="$AUTOSTART_DIR/${APP_NAME_LOWER}.desktop"

    cat > "$DESKTOP_FILE" << EOF
[Desktop Entry]
Type=Application
Name=$APP_NAME
Comment=Stream Deck Controller for Linux
Exec=$BINARY_PATH --hidden
Icon=$ICON_PATH
Terminal=false
Categories=Utility;
StartupNotify=false
StartupWMClass=$APP_NAME_LOWER
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=2
X-KDE-autostart-after=panel
X-MATE-Autostart-Delay=2
EOF

    chmod +x "$DESKTOP_FILE"
    echo -e "${GREEN}Created: $DESKTOP_FILE${NC}"
fi

# ============================================================
# Summary
# ============================================================

echo ""
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}Installation complete!${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""
echo "$APP_NAME will start automatically on login."
echo ""

if [ "$COMPOSITOR" = "hyprland" ]; then
    echo -e "${BLUE}Configured for Hyprland${NC}"
    echo "  Config: $HYPRLAND_CONF"
    echo "  To disable: Remove the exec-once line from your config"
elif [ "$COMPOSITOR" = "sway" ]; then
    echo -e "${BLUE}Configured for Sway${NC}"
    echo "  Config: $SWAY_CONF"
    echo "  To disable: Remove the exec line from your config"
else
    echo -e "${BLUE}XDG Autostart enabled${NC}"
    echo "  Works with: GNOME, KDE, XFCE, Cinnamon, MATE, LXQt, and most DEs"
    echo "  To disable: rm $DESKTOP_FILE"
fi

echo ""
echo "To start it now (without logging out):"
echo "  $BINARY_PATH --hidden &"
echo ""
