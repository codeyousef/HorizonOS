#!/bin/bash
# Test script for HorizonOS graph desktop compositor

echo "=== HorizonOS Graph Desktop Compositor Test ==="
echo "This will launch the compositor in a nested Wayland session"
echo ""

# Check if running under Wayland
if [ -z "$WAYLAND_DISPLAY" ]; then
    echo "Warning: Not running under Wayland. The compositor may not work properly."
    echo "Consider running this under a Wayland session (e.g., GNOME, KDE Plasma on Wayland)"
fi

# Set environment variables for testing
export RUST_LOG=info,horizonos=debug
export RUST_BACKTRACE=1

# Path to the compositor binary
COMPOSITOR_BIN="./target/release/horizonos-compositor"

if [ ! -f "$COMPOSITOR_BIN" ]; then
    echo "Error: Compositor binary not found at $COMPOSITOR_BIN"
    echo "Please build it first with: cargo build --release -p horizonos-graph-compositor --bin horizonos-compositor"
    exit 1
fi

echo "Starting compositor with debug logging..."
echo "Press Ctrl+C to exit"
echo ""

# Run the compositor
exec $COMPOSITOR_BIN