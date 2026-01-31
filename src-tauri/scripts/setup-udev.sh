#!/bin/bash
# ArchDeck - Stream Deck udev rules installer
# Run with: sudo ./setup-udev.sh

set -e

RULES_FILE="70-streamdeck.rules"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST="/etc/udev/rules.d/$RULES_FILE"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "This script must be run as root (use sudo)"
    exit 1
fi

# Check if rules file exists in script directory
if [ -f "$SCRIPT_DIR/$RULES_FILE" ]; then
    SOURCE="$SCRIPT_DIR/$RULES_FILE"
elif [ -f "/usr/share/archdeck/$RULES_FILE" ]; then
    SOURCE="/usr/share/archdeck/$RULES_FILE"
else
    # Inline the rules if file not found
    echo "Creating udev rules inline..."
    cat > "$DEST" << 'EOF'
# Elgato Stream Deck udev rules
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0fd9", MODE="0660", TAG+="uaccess"
EOF
    echo "Installed udev rules to $DEST"
    udevadm control --reload-rules
    udevadm trigger
    echo "Done! You may need to reconnect your Stream Deck."
    exit 0
fi

# Copy rules file
echo "Installing udev rules from $SOURCE..."
cp "$SOURCE" "$DEST"
chmod 644 "$DEST"

# Reload udev rules
echo "Reloading udev rules..."
udevadm control --reload-rules
udevadm trigger

echo ""
echo "Done! Stream Deck udev rules installed."
echo "You may need to reconnect your Stream Deck for changes to take effect."
