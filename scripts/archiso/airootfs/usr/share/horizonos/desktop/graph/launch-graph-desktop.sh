#!/bin/bash
# HorizonOS Graph Desktop Launcher
# Detects GPU capabilities and launches with appropriate backend

set -e

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if running in Wayland
check_wayland() {
    if [ -z "$WAYLAND_DISPLAY" ]; then
        print_error "Not running in a Wayland session"
        print_info "The Graph Desktop requires Wayland. Please log into a Wayland session."
        exit 1
    fi
}

# Detect GPU capabilities
detect_gpu() {
    print_info "Detecting GPU capabilities..."
    
    # Check for NVIDIA
    if lspci | grep -i nvidia > /dev/null; then
        if command -v nvidia-smi &> /dev/null; then
            print_info "NVIDIA GPU detected with drivers installed"
            export WLR_RENDERER=vulkan
            export __GLX_VENDOR_LIBRARY_NAME=nvidia
            GPU_TYPE="nvidia"
        else
            print_warning "NVIDIA GPU detected but drivers not installed"
            GPU_TYPE="software"
        fi
    # Check for AMD
    elif lspci | grep -E "(AMD|ATI)" | grep -E "(VGA|3D)" > /dev/null; then
        print_info "AMD GPU detected"
        export WLR_RENDERER=vulkan
        GPU_TYPE="amd"
    # Check for Intel
    elif lspci | grep -i intel | grep -E "(VGA|3D)" > /dev/null; then
        print_info "Intel GPU detected"
        export WLR_RENDERER=gles2
        GPU_TYPE="intel"
    else
        print_warning "No dedicated GPU detected"
        GPU_TYPE="software"
    fi
    
    # Check for hardware acceleration
    if [ "$GPU_TYPE" = "software" ]; then
        print_warning "Falling back to software rendering"
        print_info "Performance may be limited. The Graph Desktop will use CPU rendering."
        export WLR_RENDERER=pixman
        export LIBGL_ALWAYS_SOFTWARE=1
        
        # Ask user for confirmation
        read -p "Continue with software rendering? (y/N): " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_info "Launch cancelled"
            exit 0
        fi
    else
        print_success "Hardware acceleration available via $GPU_TYPE"
    fi
}

# Check system resources
check_resources() {
    print_info "Checking system resources..."
    
    # Check RAM
    total_ram=$(free -m | awk '/^Mem:/{print $2}')
    if [ "$total_ram" -lt 4096 ]; then
        print_warning "System has less than 4GB RAM ($total_ram MB)"
        print_warning "Graph Desktop may run slowly"
    else
        print_success "RAM: ${total_ram}MB"
    fi
    
    # Check CPU cores
    cpu_cores=$(nproc)
    print_info "CPU cores: $cpu_cores"
}

# Launch Graph Desktop
launch_graph_desktop() {
    print_info "Launching HorizonOS Graph Desktop..."
    
    # Set environment variables
    export HORIZONOS_GRAPH_MODE=1
    export XDG_CURRENT_DESKTOP=horizonos-graph
    export XDG_SESSION_TYPE=wayland
    
    # Set rendering backend based on GPU
    case "$GPU_TYPE" in
        nvidia)
            export WLR_NO_HARDWARE_CURSORS=1
            ;;
        software)
            export WLR_RENDERER_ALLOW_SOFTWARE=1
            ;;
    esac
    
    # Create runtime directory
    mkdir -p /tmp/horizonos-graph
    
    # Launch compositor
    COMPOSITOR="/usr/share/horizonos/desktop/graph/horizonos-compositor"
    
    if [ ! -f "$COMPOSITOR" ]; then
        print_error "Graph compositor not found at: $COMPOSITOR"
        print_info "Please ensure HorizonOS Graph Desktop is properly installed"
        exit 1
    fi
    
    # Make sure it's executable
    chmod +x "$COMPOSITOR"
    
    # Launch with appropriate settings
    if [ "$GPU_TYPE" = "software" ]; then
        print_info "Starting with software rendering..."
        exec env WLR_RENDERER=pixman LIBGL_ALWAYS_SOFTWARE=1 "$COMPOSITOR"
    else
        print_info "Starting with hardware acceleration..."
        exec "$COMPOSITOR"
    fi
}

# Show intro message
show_intro() {
    clear
    echo "╔══════════════════════════════════════════════════════╗"
    echo "║          HorizonOS Graph Desktop Launcher            ║"
    echo "╠══════════════════════════════════════════════════════╣"
    echo "║                                                      ║"
    echo "║  The Graph Desktop is an experimental 3D semantic   ║"
    echo "║  workspace where everything exists as interconnected ║"
    echo "║  nodes in space.                                     ║"
    echo "║                                                      ║"
    echo "║  Requirements:                                       ║"
    echo "║  • Wayland session                                   ║"
    echo "║  • 4GB+ RAM recommended                              ║"
    echo "║  • GPU recommended (software rendering available)    ║"
    echo "║                                                      ║"
    echo "╚══════════════════════════════════════════════════════╝"
    echo
    read -p "Press Enter to continue..."
}

# Main execution
main() {
    show_intro
    check_wayland
    detect_gpu
    check_resources
    
    echo
    print_info "Ready to launch Graph Desktop"
    print_info "Press Ctrl+C to cancel"
    sleep 2
    
    launch_graph_desktop
}

# Handle errors
trap 'print_error "An error occurred. Exiting..."; exit 1' ERR

# Run main function
main "$@"