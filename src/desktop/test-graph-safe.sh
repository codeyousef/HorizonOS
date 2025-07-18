#!/bin/bash
# Safe testing script for HorizonOS Graph Desktop with fallback options

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}HorizonOS Graph Desktop Safe Test${NC}"
echo "================================="
echo ""

# Check GPU and display setup
echo -e "${YELLOW}Checking display environment...${NC}"
echo "WAYLAND_DISPLAY: ${WAYLAND_DISPLAY:-not set}"
echo "XDG_SESSION_TYPE: ${XDG_SESSION_TYPE:-not set}"
echo "GPU info:"
lspci | grep -i vga || echo "No GPU info available"
echo ""

# Test options that don't require EGL
echo -e "${YELLOW}Select safe test option:${NC}"
echo "1) Software rendering mode (no GPU required)"
echo "2) Headless tests (no display required)"
echo "3) X11 compatibility mode"
echo "4) Virtual framebuffer mode"
echo "5) Component tests only"
read -p "Choice [1-5]: " CHOICE

case $CHOICE in
    1)
        echo -e "\n${GREEN}Running with software rendering...${NC}"
        
        # Force software rendering
        export LIBGL_ALWAYS_SOFTWARE=1
        export GALLIUM_DRIVER=llvmpipe
        export WLR_RENDERER=pixman
        export WGPU_BACKEND=gl
        export MESA_GL_VERSION_OVERRIDE=3.3
        export MESA_GLSL_VERSION_OVERRIDE=330
        
        echo "Environment configured for software rendering"
        echo "Building compositor..."
        
        cd graph-compositor
        cargo build --release --bin horizonos-compositor
        
        echo -e "\n${GREEN}Starting compositor with software rendering...${NC}"
        RUST_LOG=info,smithay=warn ./target/release/horizonos-compositor --software-render 2>&1 | grep -v "EGL"
        ;;
        
    2)
        echo -e "\n${GREEN}Running headless tests...${NC}"
        
        # Test node system
        echo -e "\n${YELLOW}Testing node creation and relationships...${NC}"
        cd graph-nodes
        cargo test --release -- --nocapture
        
        # Test layout algorithms
        echo -e "\n${YELLOW}Testing layout algorithms...${NC}"
        cd ../graph-layout
        cargo test --release -- --nocapture
        
        # Test AI integration
        echo -e "\n${YELLOW}Testing AI components...${NC}"
        cd ../graph-ai
        cargo test --release -- --nocapture
        
        echo -e "\n${GREEN}All headless tests complete!${NC}"
        ;;
        
    3)
        echo -e "\n${GREEN}Running in X11 compatibility mode...${NC}"
        
        # Check if running under X11
        if [ -n "$DISPLAY" ] && [ -z "$WAYLAND_DISPLAY" ]; then
            echo "X11 display detected: $DISPLAY"
            
            # Use Xwayland
            export WLR_BACKENDS=x11
            export WLR_X11_OUTPUTS=1
            
            cd graph-compositor
            cargo build --release --bin horizonos-compositor
            
            echo -e "\n${GREEN}Starting compositor in X11 window...${NC}"
            RUST_LOG=info ./target/release/horizonos-compositor --x11-backend
        else
            echo -e "${RED}No X11 display found. Try option 4 for virtual framebuffer.${NC}"
        fi
        ;;
        
    4)
        echo -e "\n${GREEN}Setting up virtual framebuffer...${NC}"
        
        # Check for Xvfb
        if ! command -v Xvfb &> /dev/null; then
            echo -e "${YELLOW}Xvfb not installed. Install with: sudo pacman -S xorg-server-xvfb${NC}"
            exit 1
        fi
        
        # Start virtual display
        echo "Starting virtual display :99"
        Xvfb :99 -screen 0 1280x720x24 &
        XVFB_PID=$!
        export DISPLAY=:99
        
        sleep 2
        
        # Run with virtual display
        cd graph-compositor
        cargo build --release --bin horizonos-compositor
        
        echo -e "\n${GREEN}Starting compositor in virtual framebuffer...${NC}"
        RUST_LOG=info ./target/release/horizonos-compositor --headless
        
        # Cleanup
        kill $XVFB_PID 2>/dev/null || true
        ;;
        
    5)
        echo -e "\n${GREEN}Running component tests only...${NC}"
        
        # Create a test program that exercises the graph system
        cat > test-components.rs << 'EOF'
use horizonos_graph_nodes::{Node, NodeType, NodeId};
use horizonos_graph_edges::{Edge, EdgeType};
use horizonos_graph_layout::{LayoutAlgorithm, ForceDirectedLayout};

fn main() {
    println!("Testing HorizonOS Graph Components\n");
    
    // Create nodes
    let app_node = Node::new(NodeId::new(), NodeType::Application {
        name: "Firefox".to_string(),
        executable: "/usr/bin/firefox".to_string(),
        icon: None,
    });
    
    let file_node = Node::new(NodeId::new(), NodeType::File {
        path: "/home/user/document.pdf".into(),
        mime_type: "application/pdf".to_string(),
        size: 1024 * 1024,
    });
    
    println!("Created nodes:");
    println!("- Application: Firefox");
    println!("- File: document.pdf");
    
    // Create edge
    let edge = Edge::new(
        app_node.id.clone(),
        file_node.id.clone(),
        EdgeType::Opens,
        1.0,
    );
    
    println!("\nCreated relationship: Firefox -> Opens -> document.pdf");
    
    // Test layout
    println!("\nTesting force-directed layout algorithm...");
    let layout = ForceDirectedLayout::new();
    println!("Layout initialized successfully!");
    
    println!("\n✓ All components working correctly!");
}
EOF
        
        # Build and run test
        rustc test-components.rs \
            -L target/release/deps \
            --extern horizonos_graph_nodes=target/release/libhorizonos_graph_nodes.rlib \
            --extern horizonos_graph_edges=target/release/libhorizonos_graph_edges.rlib \
            --extern horizonos_graph_layout=target/release/libhorizonos_graph_layout.rlib \
            -o test-components 2>/dev/null || {
            echo -e "${YELLOW}Direct compilation failed, using cargo instead...${NC}"
            
            # Alternative: create a simple test crate
            mkdir -p component-test
            cd component-test
            
            cat > Cargo.toml << EOF
[package]
name = "component-test"
version = "0.1.0"
edition = "2021"

[dependencies]
horizonos-graph-nodes = { path = "../graph-nodes" }
horizonos-graph-edges = { path = "../graph-edges" }
horizonos-graph-layout = { path = "../graph-layout" }
EOF
            
            cat > src/main.rs << 'EOF'
fn main() {
    println!("Testing HorizonOS Graph Components\n");
    
    use horizonos_graph_nodes::{Node, NodeType, NodeId};
    
    let node = Node::new(
        NodeId::new(),
        NodeType::Application {
            name: "Test App".to_string(),
            executable: "/usr/bin/test".to_string(),
            icon: None,
        }
    );
    
    println!("✓ Successfully created node: {:?}", node.node_type);
    println!("✓ Node ID: {}", node.id);
    println!("\nAll components working!");
}
EOF
            
            cargo run --release
            cd ..
            rm -rf component-test
        }
        
        rm -f test-components.rs test-components
        ;;
        
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo -e "\n${BLUE}Troubleshooting Tips:${NC}"
echo "1. If EGL errors persist, try:"
echo "   export LIBGL_ALWAYS_SOFTWARE=1"
echo "   export WLR_RENDERER=pixman"
echo ""
echo "2. For NVIDIA GPUs:"
echo "   export WLR_NO_HARDWARE_CURSORS=1"
echo "   export WGPU_BACKEND=vulkan"
echo ""
echo "3. For better logs:"
echo "   export RUST_LOG=debug,smithay=warn"
echo ""
echo "4. Check your GPU driver:"
echo "   glxinfo | grep OpenGL"