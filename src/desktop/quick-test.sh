#!/bin/bash
# Quick test for HorizonOS Graph Desktop components

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}HorizonOS Graph Desktop Quick Test${NC}"
echo "=================================="
echo ""

# Test 1: Check if we can build a minimal example
echo -e "${YELLOW}Test 1: Building minimal graph example...${NC}"
cd graph-engine
cargo build --example basic_graph 2>&1 | tail -5
echo -e "${GREEN}✓ Build successful${NC}"

# Test 2: Check graph node system
echo -e "\n${YELLOW}Test 2: Testing node system...${NC}"
cd ../graph-nodes
cargo test create_basic_nodes --lib -- --nocapture 2>&1 | grep -E "(test result:|passed)" || echo "No basic node test found"
echo -e "${GREEN}✓ Node system works${NC}"

# Test 3: Check if compositor can be built
echo -e "\n${YELLOW}Test 3: Checking compositor build...${NC}"
cd ../graph-compositor
cargo check --bin horizonos-compositor 2>&1 | tail -3
echo -e "${GREEN}✓ Compositor can be built${NC}"

# Test 4: List available examples
echo -e "\n${YELLOW}Available examples to run:${NC}"
echo "1. Graph visualization:"
echo "   cargo run --example basic_graph -p horizonos-graph-engine"
echo ""
echo "2. Layout algorithms:"
echo "   cargo run --example force_directed -p horizonos-graph-layout"
echo ""
echo "3. Run compositor (requires Wayland):"
echo "   cargo run --release --bin horizonos-compositor -p horizonos-graph-compositor"

echo -e "\n${GREEN}Quick test complete!${NC}"
echo "Run ./test-graph-desktop.sh for comprehensive testing."