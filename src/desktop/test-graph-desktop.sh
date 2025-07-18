#!/bin/bash
# Test HorizonOS Graph Desktop locally

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}HorizonOS Graph Desktop Test Suite${NC}"
echo "===================================="
echo ""

# Check prerequisites
echo -e "${YELLOW}Checking prerequisites...${NC}"

# Check for Wayland
if [ -z "$WAYLAND_DISPLAY" ]; then
    echo -e "${YELLOW}Warning: Not running under Wayland. Some features may not work.${NC}"
    echo "Tip: Run this from a Wayland session (GNOME, KDE on Wayland, Sway, etc.)"
fi

# Check for required tools
MISSING_DEPS=""
command -v cargo >/dev/null 2>&1 || MISSING_DEPS="$MISSING_DEPS cargo"
command -v pkg-config >/dev/null 2>&1 || MISSING_DEPS="$MISSING_DEPS pkg-config"

if [ -n "$MISSING_DEPS" ]; then
    echo -e "${RED}Missing dependencies:$MISSING_DEPS${NC}"
    echo "Install with: sudo pacman -S rust pkg-config"
    exit 1
fi

# Build options
echo ""
echo -e "${YELLOW}Select test mode:${NC}"
echo "1) Quick test - Run pre-built compositor"
echo "2) Full build - Build all components first"
echo "3) Examples only - Run graph engine examples"
echo "4) Unit tests - Run test suite"
echo "5) Demo - Run feature demonstration"
read -p "Choice [1-5]: " CHOICE

case $CHOICE in
    1)
        echo -e "\n${GREEN}Running quick test...${NC}"
        
        # Check if already built
        if [ ! -f "target/release/horizonos-compositor" ]; then
            echo "Compositor not built yet. Building now..."
            cargo build --release -p horizonos-graph-compositor --bin horizonos-compositor
        fi
        
        echo -e "\n${GREEN}Starting HorizonOS compositor...${NC}"
        echo "Keyboard shortcuts:"
        echo "  Super+Q: Quit"
        echo "  Super+Space: Open launcher"
        echo "  Super+Tab: Switch workspaces"
        echo "  Super+1-9: Switch to workspace N"
        echo ""
        
        RUST_LOG=info ./target/release/horizonos-compositor
        ;;
        
    2)
        echo -e "\n${GREEN}Building all components...${NC}"
        
        # Build everything
        cargo build --release --workspace
        
        echo -e "\n${GREEN}Build complete! Starting compositor...${NC}"
        RUST_LOG=info ./target/release/horizonos-compositor
        ;;
        
    3)
        echo -e "\n${GREEN}Running graph engine examples...${NC}"
        
        # Basic graph visualization
        echo -e "\n${YELLOW}1. Basic Graph Visualization${NC}"
        cargo run --release --example basic_graph -p horizonos-graph-engine &
        GRAPH_PID=$!
        
        sleep 5
        
        # Layout algorithm demo
        echo -e "\n${YELLOW}2. Layout Algorithm Demo${NC}"
        cargo run --release --example layout_demo -p horizonos-graph-layout &
        LAYOUT_PID=$!
        
        sleep 5
        
        # Node system demo
        echo -e "\n${YELLOW}3. Node System Demo${NC}"
        cargo run --release --example node_demo -p horizonos-graph-nodes &
        NODE_PID=$!
        
        echo -e "\n${GREEN}Examples running. Press Enter to stop all.${NC}"
        read
        
        kill $GRAPH_PID $LAYOUT_PID $NODE_PID 2>/dev/null || true
        ;;
        
    4)
        echo -e "\n${GREEN}Running test suite...${NC}"
        
        # Run tests for each component
        echo -e "\n${YELLOW}Testing graph-engine...${NC}"
        cargo test -p horizonos-graph-engine
        
        echo -e "\n${YELLOW}Testing graph-nodes...${NC}"
        cargo test -p horizonos-graph-nodes
        
        echo -e "\n${YELLOW}Testing graph-layout...${NC}"
        cargo test -p horizonos-graph-layout
        
        echo -e "\n${YELLOW}Testing graph-ai...${NC}"
        cargo test -p horizonos-graph-ai
        
        echo -e "\n${GREEN}All tests complete!${NC}"
        ;;
        
    5)
        echo -e "\n${GREEN}Running feature demonstration...${NC}"
        
        if [ -f "./demo.sh" ]; then
            ./demo.sh
        else
            echo "Creating demo script..."
            cat > demo.sh << 'EOF'
#!/bin/bash
# HorizonOS Graph Desktop Demo

echo "HorizonOS Graph Desktop Feature Demo"
echo "===================================="
echo ""
echo "Features demonstrated:"
echo "✓ 3D graph visualization with WebGPU"
echo "✓ Force-directed layout algorithms"
echo "✓ AI-powered node suggestions"
echo "✓ Wayland compositor with gesture support"
echo "✓ D-Bus system integration"
echo ""

# Start compositor with demo workspace
HORIZONOS_DEMO_MODE=1 RUST_LOG=info ./target/release/horizonos-compositor &
COMPOSITOR_PID=$!

sleep 3

# Show some example notifications
dbus-send --session --type=method_call \
    --dest=org.horizonos.GraphDesktop \
    /org/horizonos/GraphDesktop \
    org.horizonos.GraphDesktop.ShowNotification \
    string:"Welcome to HorizonOS" \
    string:"Graph Desktop is running"

echo "Demo running. Press Enter to stop."
read

kill $COMPOSITOR_PID 2>/dev/null || true
EOF
            chmod +x demo.sh
            ./demo.sh
        fi
        ;;
        
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo -e "\n${GREEN}Test complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Report any issues found"
echo "2. Try different test modes"
echo "3. Check logs in target/ directory"