# Maintainer: Travis <your@email.com>
pkgname=deckmanager
pkgver=0.1.0
pkgrel=1
pkgdesc="Open-source Stream Deck software for Linux"
arch=('x86_64')
url="https://github.com/yourusername/deckmanager"
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

    # Install frontend dependencies
    npm ci

    # Build the Tauri app - generates .deb in target/release/bundle/deb/
    npm run tauri build -- --bundles deb
}

package() {
    cd "$srcdir/../"

    # Extract the .deb that Tauri built (like OpenDeck does)
    local debfile=$(find src-tauri/target/release/bundle/deb -name '*.deb' | head -1)

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

    # Install udev rules (not included in .deb by default)
    install -Dm644 "src-tauri/scripts/70-streamdeck.rules" "$pkgdir/usr/lib/udev/rules.d/70-streamdeck.rules"

    # Install helper scripts
    install -Dm755 "src-tauri/scripts/install-autostart.sh" "$pkgdir/usr/share/deckmanager/install-autostart.sh"
    install -Dm755 "src-tauri/scripts/uninstall-autostart.sh" "$pkgdir/usr/share/deckmanager/uninstall-autostart.sh"
}
