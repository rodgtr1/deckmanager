#!/bin/bash
# Deck Manager launcher wrapper
# Applies workarounds for Wayland + Nvidia setups

# Detect Wayland + Nvidia and apply workarounds
if [[ -n "$WAYLAND_DISPLAY" ]] && lsmod | grep nvidia > /dev/null 2>&1; then
    export GDK_BACKEND="${GDK_BACKEND:-x11}"
    export WEBKIT_DISABLE_COMPOSITING_MODE="${WEBKIT_DISABLE_COMPOSITING_MODE:-1}"
    export GBM_BACKEND="${GBM_BACKEND:-nvidia-drm}"
fi

# Handle HiDPI if not already set
if [[ -z "$GDK_SCALE" && -n "$WAYLAND_DISPLAY" ]]; then
    # Let the system handle scaling by default
    export GDK_SCALE="${GDK_SCALE:-1}"
fi

exec /usr/lib/deckmanager/deckmanager-bin "$@"
