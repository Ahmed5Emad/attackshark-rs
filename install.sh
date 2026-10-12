#!/bin/sh
# One-time installer: device access + binaries. Asks for root once, ever.
set -e
cd "$(dirname "$0")"
echo "Installing udev rule for the ATTACK SHARK receiver..."
sudo install -m 644 99-attackshark.rules /etc/udev/rules.d/99-attackshark.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=hidraw
echo "Installing binaries to ~/.local/bin..."
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/attackshark-rs "$HOME/.local/bin/" 2>/dev/null || echo "(skip: build first with: cargo build --release)"
install -m 755 target/release/ui "$HOME/.local/bin/attackshark-ui" 2>/dev/null || echo "(skip: build first with: cargo build --release --bin ui)"
echo "Done. Unplug the receiver and plug it back in once."
