# Maintainer: Travis <your@email.com>
pkgname=deckmanager
pkgver=0.1.0
pkgrel=1
pkgdesc="Open-source Stream Deck software for Linux"
arch=('x86_64')
url="https://github.com/rodgtr1/deckmanager"
license=('MIT')
depends=(
    'webkit2gtk-4.1'
    'gtk3'
    'hidapi'
)
optdepends=(
    'playerctl: media control support'
    'pipewire: audio control support'
)
makedepends=(
    'rust'
    'cargo'
    'npm'
    'hidapi'
)
options=('!lto')
source=()
install=deckmanager.install

build() {
    cd "$srcdir/../"

    # Clean stale build artifacts to ensure fresh build
    rm -rf src-tauri/target/release/bundle

    # Install frontend dependencies
    npm ci

    # Build the Tauri app - generates .deb in target/release/bundle/deb/
    npm run tauri build -- --bundles deb
}

package() {
    cd "$srcdir/../"

    # Extract the .deb that Tauri built (use newest file in case of stale builds)
    local debfile=$(find src-tauri/target/release/bundle/deb -name '*.deb' -printf '%T@\t%p\n' | sort -rn | head -1 | cut -f2)

    if [[ -z "$debfile" ]]; then
        echo "Error: No .deb file found in bundle output"
        exit 1
    fi

    # Extract .deb contents to temp location (not srcdir to avoid polluting source)
    local tmpdir=$(mktemp -d)
    bsdtar -xf "$debfile" -C "$tmpdir"
    bsdtar -xf "$tmpdir/data.tar.gz" -C "$pkgdir"
    rm -rf "$tmpdir"

    # Remove debian metadata
    rm -rf "$pkgdir/usr/share/doc"

    # Move binary to /usr/lib and install wrapper script
    # This allows the wrapper to apply Wayland/Nvidia workarounds
    install -dm755 "$pkgdir/usr/lib/deckmanager"
    mv "$pkgdir/usr/bin/deckmanager" "$pkgdir/usr/lib/deckmanager/deckmanager-bin"
    install -Dm755 "src-tauri/scripts/deckmanager-wrapper.sh" "$pkgdir/usr/bin/deckmanager"

    # Install udev rules (not included in .deb by default)
    install -Dm644 "src-tauri/scripts/70-streamdeck.rules" "$pkgdir/usr/lib/udev/rules.d/70-streamdeck.rules"

    # Install helper scripts
    install -Dm755 "src-tauri/scripts/install-autostart.sh" "$pkgdir/usr/share/deckmanager/install-autostart.sh"
    install -Dm755 "src-tauri/scripts/uninstall-autostart.sh" "$pkgdir/usr/share/deckmanager/uninstall-autostart.sh"
}
