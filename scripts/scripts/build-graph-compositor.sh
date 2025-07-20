#!/bin/bash
# Build HorizonOS Graph Desktop Compositor

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
DESKTOP_DIR="$PROJECT_ROOT/src/desktop"
BUILD_DIR="$PROJECT_ROOT/build/graph-compositor"
TARGET_DIR="$PROJECT_ROOT/scripts/archiso/airootfs/usr/share/horizonos/desktop/graph"

echo "=== Building HorizonOS Graph Desktop Compositor ==="

# Create directories
mkdir -p "$BUILD_DIR"
mkdir -p "$TARGET_DIR"

# Check if running in CI environment
if [ -n "$CI" ] || [ -n "$GITHUB_ACTIONS" ]; then
    echo "Running in CI environment - skipping Rust build"
    SKIP_RUST_BUILD=true
else
    # Check if Rust is installed
    if ! command -v cargo &> /dev/null; then
        echo "Warning: Rust is not installed. Creating placeholder compositor."
        SKIP_RUST_BUILD=true
    else
        SKIP_RUST_BUILD=false
    fi
fi

# Navigate to desktop directory
cd "$DESKTOP_DIR"

# Build the compositor if not skipping
if [ "$SKIP_RUST_BUILD" = "false" ] && [ -f "Cargo.toml" ]; then
    echo "Building graph compositor..."
    cargo build --release --package horizonos-graph-compositor --bin horizonos-compositor
    
    # Copy the binary
    if [ -f "target/release/horizonos-compositor" ]; then
        echo "Copying compositor binary to ISO..."
        cp "target/release/horizonos-compositor" "$TARGET_DIR/"
        chmod +x "$TARGET_DIR/horizonos-compositor"
        echo "Graph compositor built and copied successfully!"
    else
        echo "Warning: Compositor binary not found. Creating placeholder..."
        # Create a placeholder script that shows a message
        cat > "$TARGET_DIR/horizonos-compositor" << 'EOF'
#!/bin/bash
echo "=================================="
echo "HorizonOS Graph Desktop Compositor"
echo "=================================="
echo ""
echo "The graph desktop compositor is not yet built."
echo "This is a placeholder for the experimental 3D semantic desktop."
echo ""
echo "To build the real compositor:"
echo "1. Install Rust toolchain"
echo "2. Navigate to src/desktop/"
echo "3. Run: cargo build --release"
echo ""
echo "Press Ctrl+C to exit..."
sleep infinity
EOF
        chmod +x "$TARGET_DIR/horizonos-compositor"
    fi
else
    echo "Creating placeholder compositor for ISO..."
    # Create placeholder
    cat > "$TARGET_DIR/horizonos-compositor" << 'EOF'
#!/bin/bash
echo "=================================="
echo "HorizonOS Graph Desktop Compositor"
echo "=================================="
echo ""
echo "The graph desktop source code is not found."
echo "This is a placeholder for the experimental 3D semantic desktop."
echo ""
echo "For more information, visit:"
echo "https://github.com/codeyousef/HorizonOS"
echo ""
echo "Press Ctrl+C to exit..."
sleep infinity
EOF
    chmod +x "$TARGET_DIR/horizonos-compositor"
fi

echo "Graph compositor setup complete!"