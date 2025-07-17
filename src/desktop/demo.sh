#!/bin/bash
# Demo script for HorizonOS graph desktop

echo "=== HorizonOS Graph Desktop Demo ==="
echo ""
echo "This demo showcases the key features of the graph desktop:"
echo "1. Graph-based window management"
echo "2. AI-powered assistance"
echo "3. Workspace management"
echo "4. Node clustering"
echo "5. Advanced interactions"
echo ""

# Function to print colored output
print_status() {
    echo -e "\033[1;32m✓\033[0m $1"
}

print_info() {
    echo -e "\033[1;34mℹ\033[0m $1"
}

print_warning() {
    echo -e "\033[1;33m⚠\033[0m $1"
}

# Check environment
echo "Checking environment..."
if command -v cargo >/dev/null 2>&1; then
    print_status "Rust toolchain found"
else
    print_warning "Rust toolchain not found. Please install Rust."
    exit 1
fi

# Build status
echo ""
echo "Build Status:"
if [ -f "./target/release/horizonos-compositor" ]; then
    print_status "Compositor binary built"
else
    print_info "Compositor binary not found. Building..."
    cargo build --release -p horizonos-graph-compositor --bin horizonos-compositor
fi

# Feature showcase
echo ""
echo "Key Features Implemented:"
print_status "Graph Engine - 3D scene management with WebGPU"
print_status "Node System - 12 node types (App, File, Person, Task, etc.)"
print_status "Edge System - Relationships, dependencies, data flow"
print_status "Clustering - Automatic node grouping by type/relationship"
print_status "AI Integration - Ollama support with hardware detection"
print_status "Workspace Management - Multiple workspaces with templates"
print_status "Visual Design - Icons, thumbnails, edge styles"
print_status "Accessibility - Screen reader, keyboard nav, magnification"
print_status "Performance - LOD, frustum culling, GPU instancing"
print_status "Wayland Compositor - Native Wayland support"

# System requirements
echo ""
echo "System Requirements:"
print_info "GPU: WebGPU compatible (most modern GPUs)"
print_info "Memory: 4GB RAM minimum, 8GB recommended"
print_info "Display: Wayland session preferred"
print_info "AI: Ollama installed for AI features (optional)"

# Test results
echo ""
echo "Test Results:"
cargo test -p horizonos-graph-engine -p horizonos-graph-nodes -p horizonos-graph-workspaces --quiet
if [ $? -eq 0 ]; then
    print_status "Core module tests passed"
else
    print_warning "Some tests failed"
fi

# Running instructions
echo ""
echo "To run the compositor:"
echo "  1. Ensure you're in a Wayland session"
echo "  2. Run: ./test-compositor.sh"
echo "  3. Or directly: RUST_LOG=info ./target/release/horizonos-compositor"
echo ""
echo "Keyboard shortcuts (when running):"
echo "  - Super+Q: Quit"
echo "  - Super+Space: Open launcher"
echo "  - Super+Tab: Switch workspaces"
echo "  - Super+1-9: Switch to workspace N"
echo ""

# Architecture summary
echo "Architecture Highlights:"
print_info "Modular design with 17 specialized crates"
print_info "Graph-based scene representation"
print_info "AI-first with local LLM integration"
print_info "Hardware-adaptive performance"
print_info "Privacy-focused design"
echo ""

# Development stats
echo "Development Statistics:"
TOTAL_RUST_FILES=$(find . -name "*.rs" | wc -l)
TOTAL_LINES=$(find . -name "*.rs" -exec wc -l {} + | tail -1 | awk '{print $1}')
print_info "Rust files: $TOTAL_RUST_FILES"
print_info "Lines of code: ~$TOTAL_LINES"
print_info "Modules: 17 specialized crates"
echo ""

echo "Demo complete! The HorizonOS graph desktop is ready for testing."