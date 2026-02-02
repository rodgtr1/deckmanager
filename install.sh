#!/usr/bin/env bash
# Deck Manager Installation Script
# Usage: curl -sSL https://raw.githubusercontent.com/rodgtr1/deckmanager/main/install.sh | bash

set -e

REPO_URL="https://github.com/rodgtr1/deckmanager"
REPO_NAME="deckmanager"
UDEV_RULES_URL="https://raw.githubusercontent.com/rodgtr1/deckmanager/main/src-tauri/scripts/70-streamdeck.rules"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

info() { echo -e "${BLUE}::${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}!${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

detect_distro() {
    if command -v pacman &> /dev/null; then
        echo "arch"
    elif command -v apt-get &> /dev/null; then
        echo "debian"
    elif command -v dnf &> /dev/null; then
        echo "fedora"
    elif command -v zypper &> /dev/null; then
        echo "opensuse"
    else
        echo "unknown"
    fi
}

detect_aur_helper() {
    for helper in yay paru pikaur aura trizen; do
        if command -v "$helper" &> /dev/null; then
            echo "$helper"
            return
        fi
    done
    echo ""
}

install_arch() {
    info "Detected Arch Linux"

    local aur_helper=$(detect_aur_helper)

    if [[ -n "$aur_helper" ]]; then
        info "Using AUR helper: $aur_helper"
        # TODO: Once published to AUR, uncomment:
        # $aur_helper -S --noconfirm deckmanager
        # For now, build from source:
        install_from_source
    else
        warn "No AUR helper found, building from source..."
        install_from_source
    fi
}

install_from_source() {
    info "Building from source..."

    # Check dependencies
    local missing_deps=()
    for dep in git rust cargo npm; do
        if ! command -v "$dep" &> /dev/null; then
            missing_deps+=("$dep")
        fi
    done

    if [[ ${#missing_deps[@]} -gt 0 ]]; then
        warn "Missing build dependencies: ${missing_deps[*]}"
        info "Installing dependencies..."

        case $(detect_distro) in
            arch)
                sudo pacman -S --needed --noconfirm base-devel rust npm git webkit2gtk-4.1 gtk3 hidapi
                ;;
            debian)
                sudo apt-get update
                sudo apt-get install -y build-essential curl git libwebkit2gtk-4.1-dev libgtk-3-dev libhidapi-dev
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source "$HOME/.cargo/env"
                ;;
            fedora)
                sudo dnf install -y gcc gcc-c++ git webkit2gtk4.1-devel gtk3-devel hidapi-devel
                curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
                source "$HOME/.cargo/env"
                ;;
            *)
                error "Please install: git, rust, cargo, npm, and your distro's webkit2gtk/gtk3/hidapi dev packages"
                ;;
        esac
    fi

    # Check if we're already in the repo directory
    local build_dir
    if [[ -f "PKGBUILD" && -f "src-tauri/tauri.conf.json" ]]; then
        info "Already in repo directory, building in place..."
        build_dir="$(pwd)"
    else
        # Clone repo
        build_dir="/tmp/deckmanager-build"
        rm -rf "$build_dir"
        info "Cloning repository..."
        git clone --depth 1 "$REPO_URL" "$build_dir"
        cd "$build_dir"
    fi

    # Build and install based on distro
    case $(detect_distro) in
        arch)
            info "Building Arch package..."
            makepkg -si --noconfirm
            ;;
        debian)
            info "Building with Tauri..."
            npm ci
            npm run tauri build -- --bundles deb
            local debfile=$(find src-tauri/target/release/bundle/deb -name '*.deb' | head -1)
            sudo apt-get install -y "$debfile"
            install_udev_rules
            ;;
        fedora)
            info "Building with Tauri..."
            npm ci
            npm run tauri build -- --bundles rpm
            local rpmfile=$(find src-tauri/target/release/bundle/rpm -name '*.rpm' | head -1)
            sudo dnf install -y "$rpmfile"
            install_udev_rules
            ;;
        *)
            info "Building with Tauri..."
            npm ci
            npm run tauri build -- --no-bundle
            sudo install -Dm755 src-tauri/target/release/deckmanager /usr/local/bin/deckmanager
            install_udev_rules
            ;;
    esac

    # Cleanup
    cd /
    rm -rf "$build_dir"

    success "Deck Manager installed successfully!"
}

install_udev_rules() {
    info "Installing udev rules..."

    sudo mkdir -p /etc/udev/rules.d
    curl -sSL "$UDEV_RULES_URL" | sudo tee /etc/udev/rules.d/70-streamdeck.rules > /dev/null
    sudo udevadm control --reload-rules
    sudo udevadm trigger

    success "udev rules installed"
}

setup_autostart() {
    read -p "Enable autostart on login? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if [[ -f /usr/share/deckmanager/install-autostart.sh ]]; then
            /usr/share/deckmanager/install-autostart.sh
        else
            warn "Autostart script not found. You can set it up manually later."
        fi
    fi
}

main() {
    echo ""
    echo -e "${BLUE}╔═══════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║${NC}    Deck Manager Installation Script   ${BLUE}║${NC}"
    echo -e "${BLUE}╚═══════════════════════════════════════╝${NC}"
    echo ""

    local distro=$(detect_distro)

    if [[ "$distro" == "unknown" ]]; then
        error "Unsupported distribution. Please install manually."
    fi

    case "$distro" in
        arch)
            install_arch
            ;;
        debian|fedora|opensuse)
            install_from_source
            ;;
    esac

    echo ""
    success "Installation complete!"
    echo ""
    info "Run 'deckmanager' to start the application"
    info "If your Stream Deck is plugged in, unplug and replug it"
    echo ""

    setup_autostart
}

main "$@"
