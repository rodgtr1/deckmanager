#!/usr/bin/env bash
# Deck Manager Uninstall Script
# Completely removes Deck Manager and all associated files
#
# WHAT THIS SCRIPT DOES:
#   1. Removes the deckmanager package via your package manager
#   2. Removes config files (~/.config/deckmanager)
#   3. Removes udev rules
#   4. Removes autostart entries (XDG, systemd, Hyprland, Sway)
#   5. Reloads udev rules

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

info() { echo -e "${BLUE}::${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

detect_distro() {
    if command -v pacman > /dev/null 2>&1; then
        echo "arch"
    elif command -v apt-get > /dev/null 2>&1; then
        echo "debian"
    elif command -v dnf > /dev/null 2>&1; then
        echo "fedora"
    else
        echo "unknown"
    fi
}

main() {
    echo ""
    echo -e "${RED}╔═══════════════════════════════════════╗${NC}"
    echo -e "${RED}║${NC}    Deck Manager Uninstall Script      ${RED}║${NC}"
    echo -e "${RED}╚═══════════════════════════════════════╝${NC}"
    echo ""

    # Confirm
    read -p "This will completely remove Deck Manager. Continue? [y/N] " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        info "Cancelled."
        exit 0
    fi

    # Stop running instances
    info "Stopping running instances..."
    pkill -f deckmanager > /dev/null 2>&1 || true
    pkill -f deckmanager-bin > /dev/null 2>&1 || true
    success "Processes stopped"

    # Remove package
    local distro=$(detect_distro)
    info "Removing package..."
    case "$distro" in
        arch)
            if pacman -Q deckmanager > /dev/null 2>&1; then
                sudo pacman -Rns --noconfirm deckmanager
                success "Package removed"
            else
                warn "Package not installed via pacman"
                # Try removing manual install
                sudo rm -f /usr/bin/deckmanager /usr/local/bin/deckmanager
                sudo rm -rf /usr/lib/deckmanager
            fi
            ;;
        debian)
            if dpkg -l deckmanager > /dev/null 2>&1; then
                sudo apt-get remove -y deckmanager
                sudo apt-get autoremove -y
                success "Package removed"
            else
                warn "Package not installed via apt"
                sudo rm -f /usr/bin/deckmanager /usr/local/bin/deckmanager
                sudo rm -rf /usr/lib/deckmanager
            fi
            ;;
        fedora)
            if rpm -q deckmanager > /dev/null 2>&1; then
                sudo dnf remove -y deckmanager
                success "Package removed"
            else
                warn "Package not installed via dnf"
                sudo rm -f /usr/bin/deckmanager /usr/local/bin/deckmanager
                sudo rm -rf /usr/lib/deckmanager
            fi
            ;;
        *)
            warn "Unknown distro, removing binaries manually..."
            sudo rm -f /usr/bin/deckmanager /usr/local/bin/deckmanager
            sudo rm -rf /usr/lib/deckmanager
            ;;
    esac

    # Remove udev rules
    info "Removing udev rules..."
    sudo rm -f /etc/udev/rules.d/70-streamdeck.rules
    sudo rm -f /usr/lib/udev/rules.d/70-streamdeck.rules
    sudo udevadm control --reload-rules > /dev/null 2>&1 || true
    sudo udevadm trigger > /dev/null 2>&1 || true
    success "udev rules removed"

    # Remove autostart
    info "Removing autostart entries..."

    # XDG autostart
    rm -f ~/.config/autostart/deckmanager.desktop

    # Systemd service
    systemctl --user disable deckmanager.service > /dev/null 2>&1 || true
    rm -f ~/.config/systemd/user/deckmanager.service
    systemctl --user daemon-reload > /dev/null 2>&1 || true

    # Hyprland config
    if [[ -f ~/.config/hypr/hyprland.conf ]]; then
        sed -i '/deckmanager/d' ~/.config/hypr/hyprland.conf
        sed -i '/Deck Manager - Stream Deck Controller/d' ~/.config/hypr/hyprland.conf
    fi

    # Sway config
    if [[ -f ~/.config/sway/config ]]; then
        sed -i '/deckmanager/d' ~/.config/sway/config
        sed -i '/Deck Manager - Stream Deck Controller/d' ~/.config/sway/config
    fi

    success "Autostart entries removed"

    # Remove desktop entry (user-local)
    rm -f ~/.local/share/applications/deckmanager.desktop

    # Remove config
    if [[ -d ~/.config/deckmanager ]]; then
        read -p "Remove config files (~/.config/deckmanager)? [y/N] " -n 1 -r
        echo
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf ~/.config/deckmanager
            success "Config removed"
        else
            info "Config preserved at ~/.config/deckmanager"
        fi
    fi

    # Remove shared files
    sudo rm -rf /usr/share/deckmanager

    # Clean build artifacts if in repo directory (ensures fresh build on reinstall)
    if [[ -f "PKGBUILD" ]]; then
        info "Cleaning build artifacts..."
        rm -f deckmanager-*.pkg.tar.zst 2>/dev/null || true
        rm -rf src-tauri/target/release/bundle 2>/dev/null || true
        rm -f src-tauri/target/release/deckmanager 2>/dev/null || true
        success "Build artifacts cleaned"
    fi

    echo ""
    success "Deck Manager has been completely removed!"
    echo ""
}

main "$@"
