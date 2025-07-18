#!/bin/bash
# Unit tests for build configuration
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "=== Build Configuration Tests ==="
echo ""

TESTS_PASSED=0
TESTS_FAILED=0

# Function to run a test
run_test() {
    local test_name="$1"
    local test_command="$2"
    
    echo -n "Testing $test_name... "
    if eval "$test_command"; then
        echo -e "${GREEN}PASS${NC}"
        ((TESTS_PASSED++))
    else
        echo -e "${RED}FAIL${NC}"
        ((TESTS_FAILED++))
    fi
}

# Test 1: Check if build scripts exist
run_test "build-iso.sh exists" "[ -f '$PROJECT_ROOT/scripts/scripts/build-iso.sh' ]"
run_test "build-base-image.sh exists" "[ -f '$PROJECT_ROOT/scripts/scripts/build-base-image.sh' ]"
run_test "build-test.sh exists" "[ -f '$PROJECT_ROOT/scripts/scripts/build-test.sh' ]"

# Test 2: Check if build scripts are executable
run_test "build-iso.sh is executable" "[ -x '$PROJECT_ROOT/scripts/scripts/build-iso.sh' ]"
run_test "build-base-image.sh is executable" "[ -x '$PROJECT_ROOT/scripts/scripts/build-base-image.sh' ]"

# Test 3: Check configuration files
run_test "dev.conf exists" "[ -f '$PROJECT_ROOT/config/dev.conf' ]"

# Test 4: Check for problematic patterns
echo -e "\n${YELLOW}Checking for problematic patterns...${NC}"

# Check for duplicate autologin configurations
if grep -r "autologin.conf" "$PROJECT_ROOT/scripts/scripts/" | grep -v "^Binary" | grep -v ".sh:" > /tmp/autologin_refs.txt; then
    echo "References to autologin.conf:"
    cat /tmp/autologin_refs.txt
    
    # Count how many times we're creating autologin.conf
    CREATE_COUNT=$(grep -c "cat.*autologin.conf" /tmp/autologin_refs.txt || echo 0)
    if [ "$CREATE_COUNT" -gt 0 ]; then
        echo -e "${YELLOW}⚠ Found $CREATE_COUNT instances of creating autologin.conf${NC}"
    fi
fi

# Check for /sbin/agetty references
if grep -r "/sbin/agetty" "$PROJECT_ROOT/scripts/" | grep -v "^Binary" | grep -v "sed.*sbin/agetty" > /tmp/sbin_refs.txt; then
    echo -e "\n${YELLOW}References to /sbin/agetty (should be /usr/bin/agetty):${NC}"
    cat /tmp/sbin_refs.txt
fi

# Test 5: Verify no getty configs in base image
echo -e "\n${YELLOW}Checking base image for getty configs...${NC}"
if grep -q "getty@" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh"; then
    echo -e "${YELLOW}⚠ Found getty configuration in base image (should only be in ISO)${NC}"
    grep -n "getty@" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh"
else
    echo -e "${GREEN}✓ No getty configuration in base image${NC}"
fi

# Test 6: Check ISO customization approach
echo -e "\n${YELLOW}Checking ISO customization approach...${NC}"
if grep -q "cp -r /usr/share/archiso/configs/releng" "$PROJECT_ROOT/scripts/scripts/build-iso.sh"; then
    echo -e "${GREEN}✓ Using archiso releng profile as base${NC}"
    
    # Check if we're properly fixing the agetty path
    if grep -q "sed.*sbin/agetty.*usr/bin/agetty" "$PROJECT_ROOT/scripts/scripts/build-iso.sh"; then
        echo -e "${GREEN}✓ Fixing agetty path from /sbin to /usr/bin${NC}"
    else
        echo -e "${YELLOW}⚠ Not fixing agetty path${NC}"
    fi
fi

# Summary
echo -e "\n${GREEN}=== Test Summary ===${NC}"
echo "Tests passed: $TESTS_PASSED"
echo "Tests failed: $TESTS_FAILED"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
fi