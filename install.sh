#!/bin/sh
# One-time installer: builds, installs udev rule + binaries.
# Asks for root only for the udev step — never again afterwards.
set -e
cd "$(dirname "$0")"

if [ ! -x target/release/attackshark-rs ] || [ ! -x target/release/ui ]; then
    if command -v cargo >/dev/null 2>&1; then
        echo "Building release binaries (one time, takes a few minutes)..."
        cargo build --release
    else
        echo "ERROR: no release binaries and no cargo. Install Rust, then re-run." >&2
        exit 1
    fi
fi

echo "Installing udev rule for the ATTACK SHARK receiver..."
sudo install -m 644 99-attackshark.rules /etc/udev/rules.d/99-attackshark.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw

echo "Installing binaries to ~/.local/bin..."
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/attackshark-rs "$HOME/.local/bin/"
install -m 755 target/release/ui "$HOME/.local/bin/attackshark-ui"

case ":$PATH:" in
    *":$HOME/.local/bin:"*) ;;
    *) echo "NOTE: ~/.local/bin is not on your PATH — add it to your shell rc." ;;
esac

echo "Done. Unplug the receiver and plug it back in once, then run: attackshark-ui"
