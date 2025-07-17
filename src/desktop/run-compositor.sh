#!/bin/bash

# Run the HorizonOS Graph Desktop Compositor

echo "Starting HorizonOS Graph Desktop Compositor..."

# Set environment variables
export RUST_LOG=info
export WAYLAND_DEBUG=1

# Make sure we're in the right directory
cd "$(dirname "$0")"

# Build the compositor
echo "Building compositor..."
cargo build --release

# Run the compositor
echo "Running compositor..."
exec target/release/horizonos-compositor "$@"