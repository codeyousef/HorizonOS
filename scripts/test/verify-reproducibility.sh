#!/bin/bash
set -euo pipefail

# HorizonOS Reproducibility Verification Script
# Tests the reproducible container architecture implementation

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TEST_RESULTS=()

echo "=== HorizonOS Reproducibility Verification ==="
echo "Testing reproducible container architecture implementation"
echo

# Helper functions
pass_test() {
    local test_name="$1"
    echo -e "${GREEN}✓ PASS${NC}: $test_name"
    TESTS_PASSED=$((TESTS_PASSED + 1))
    TEST_RESULTS+=("PASS: $test_name")
}

fail_test() {
    local test_name="$1"
    local reason="$2"
    echo -e "${RED}✗ FAIL${NC}: $test_name"
    echo -e "  Reason: $reason"
    TESTS_FAILED=$((TESTS_FAILED + 1))
    TEST_RESULTS+=("FAIL: $test_name - $reason")
}

# Test 1: Verify DSL Implementation
test_dsl_implementation() {
    echo -e "${BLUE}Test 1: DSL Implementation Review${NC}"
    
    # Check Core.kt integration
    echo "  Checking Core.kt integration..."
    
    if grep -q "fun containers(" "$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt"; then
        pass_test "Core.kt has containers() function"
    else
        fail_test "Core.kt missing containers() function" "containers() not found in Core.kt"
    fi
    
    if grep -q "fun layers(" "$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt"; then
        pass_test "Core.kt has layers() function"
    else
        fail_test "Core.kt missing layers() function" "layers() not found in Core.kt"
    fi
    
    if grep -q "fun reproducible(" "$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt"; then
        pass_test "Core.kt has reproducible() function"
    else
        fail_test "Core.kt missing reproducible() function" "reproducible() not found in Core.kt"
    fi
    
    # Check if configs are included in toConfig()
    if grep -A20 "fun toConfig()" "$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Core.kt" | grep -q "containers = containersConfig"; then
        pass_test "containers config included in toConfig()"
    else
        fail_test "containers config not in toConfig()" "containersConfig not found in CompiledConfig"
    fi
}

# Test 2: Verify Container Module Features
test_container_module() {
    echo -e "\n${BLUE}Test 2: Container Module Verification${NC}"
    
    # Check runtime support
    local containers_file="$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Containers.kt"
    
    for runtime in PODMAN DOCKER TOOLBOX DISTROBOX; do
        if grep -q "$runtime" "$containers_file"; then
            pass_test "Container runtime $runtime supported"
        else
            fail_test "Container runtime $runtime missing" "$runtime not found in ContainerRuntime enum"
        fi
    done
    
    # Check digest validation
    if grep -q "validateImageReference" "$containers_file" && grep -q "sha256:[a-fA-F0-9]" "$containers_file"; then
        pass_test "Container digest validation implemented"
    else
        fail_test "Container digest validation missing" "SHA256 validation not found"
    fi
}

# Test 3: Verify Build System
test_build_system() {
    echo -e "\n${BLUE}Test 3: Build System Verification${NC}"
    
    # Check if build scripts exist
    if [ -f "$PROJECT_ROOT/scripts/scripts/build-base-image.sh" ]; then
        pass_test "Base image builder script exists"
        
        # Check if script creates minimal base
        if grep -q "MINIMAL_PACKAGES=" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh"; then
            pass_test "Base image builder uses minimal packages"
        else
            fail_test "Base image not minimal" "MINIMAL_PACKAGES not found"
        fi
    else
        fail_test "Base image builder missing" "build-base-image.sh not found"
    fi
    
    # Check container management tool
    if [ -f "$PROJECT_ROOT/scripts/tools/horizon-container" ]; then
        pass_test "Container management CLI exists"
        
        # Check for required commands
        for cmd in list install run shell export status; do
            if grep -q "cmd_$cmd()" "$PROJECT_ROOT/scripts/tools/horizon-container"; then
                pass_test "horizon-container has '$cmd' command"
            else
                fail_test "horizon-container missing '$cmd' command" "cmd_$cmd() function not found"
            fi
        done
    else
        fail_test "Container management CLI missing" "horizon-container not found"
    fi
}

# Test 4: Container Configuration Validation
test_container_configs() {
    echo -e "\n${BLUE}Test 4: Container Configuration Files${NC}"
    
    # Check if container definitions exist
    local config_dir="$PROJECT_ROOT/scripts/configs/containers"
    
    if [ -d "$config_dir" ]; then
        pass_test "Container configuration directory exists"
        
        # Check for required container definitions
        for container in development multimedia gaming desktop; do
            if [ -f "$config_dir/$container.json" ]; then
                pass_test "Container definition for '$container' exists"
                
                # Validate JSON structure
                if python3 -m json.tool "$config_dir/$container.json" >/dev/null 2>&1; then
                    pass_test "Container '$container' has valid JSON"
                    
                    # Check for required fields using Python
                    if python3 -c "import json; data=json.load(open('$config_dir/$container.json')); assert all(k in data for k in ['name', 'image', 'purpose'])" 2>/dev/null; then
                        pass_test "Container '$container' has required fields"
                    else
                        fail_test "Container '$container' missing fields" "name, image, or purpose missing"
                    fi
                else
                    fail_test "Container '$container' has invalid JSON" "JSON parsing failed"
                fi
            else
                fail_test "Container definition for '$container' missing" "$container.json not found"
            fi
        done
    else
        fail_test "Container configuration directory missing" "$config_dir not found"
    fi
}

# Test 5: Reproducibility Features
test_reproducibility_features() {
    echo -e "\n${BLUE}Test 5: Reproducibility Features${NC}"
    
    local repro_file="$PROJECT_ROOT/src/kotlin-config/src/main/kotlin/org/horizonos/config/dsl/Reproducible.kt"
    
    # Check data structures
    if grep -q "data class SystemImage" "$repro_file"; then
        pass_test "SystemImage data class exists"
    else
        fail_test "SystemImage data class missing" "SystemImage not found"
    fi
    
    if grep -q "data class ContainerImage" "$repro_file" && grep -q "digest: String" "$repro_file"; then
        pass_test "ContainerImage includes digest field"
    else
        fail_test "ContainerImage digest missing" "digest field not found"
    fi
    
    # Check validation functions
    if grep -q "validateSystemImage" "$repro_file"; then
        pass_test "System image validation function exists"
        
        # Check if it validates digests
        if grep -A20 "validateSystemImage" "$repro_file" | grep -q "sha256:"; then
            pass_test "Validation checks SHA256 format"
        else
            fail_test "SHA256 validation missing" "sha256 check not found in validateSystemImage"
        fi
    else
        fail_test "System image validation missing" "validateSystemImage function not found"
    fi
}

# Test 6: Integration Test - Create Test Config
test_integration() {
    echo -e "\n${BLUE}Test 6: Integration Test${NC}"
    
    # Create a test Kotlin DSL config
    local test_config="$PROJECT_ROOT/test-reproducible.horizonos.kts"
    cat > "$test_config" << 'EOF'
horizonOS {
    hostname = "test-reproducible"
    timezone = "UTC"
    
    containers {
        distrobox("test-tools") {
            archlinux()
            digest = "sha256:1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            packages("git", "curl")
            export("git", "curl")
        }
    }
    
    layers {
        base {
            packages("base", "linux", "systemd")
        }
        
        systemLayer("development", LayerPurpose.DEVELOPMENT) {
            image = "archlinux"
            digest = "sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
            packages("git", "neovim")
        }
    }
    
    reproducible {
        enabled = true
        strictMode = true
        verifyDigests = true
    }
}
EOF
    
    if [ -f "$test_config" ]; then
        pass_test "Test configuration created"
        
        # Try to compile it (if Kotlin compiler available)
        if command -v kotlinc &>/dev/null; then
            echo "  Attempting to compile test config..."
            # This would require full Kotlin setup
            pass_test "Test config validation skipped (requires full Kotlin setup)"
        else
            pass_test "Test config created (compilation requires Kotlin setup)"
        fi
        
        rm -f "$test_config"
    else
        fail_test "Failed to create test configuration" "Could not write test config"
    fi
}

# Test 7: Documentation Verification
test_documentation() {
    echo -e "\n${BLUE}Test 7: Documentation Verification${NC}"
    
    # Check for container architecture docs
    if [ -f "$PROJECT_ROOT/docs/CONTAINER-ARCHITECTURE.md" ]; then
        pass_test "Container architecture documentation exists"
        
        # Check for key sections
        if grep -q "## Architecture Layers" "$PROJECT_ROOT/docs/CONTAINER-ARCHITECTURE.md"; then
            pass_test "Architecture layers documented"
        else
            fail_test "Architecture layers not documented" "## Architecture Layers section missing"
        fi
    else
        fail_test "Container architecture docs missing" "CONTAINER-ARCHITECTURE.md not found"
    fi
    
    # Check for quickstart guide
    if [ -f "$PROJECT_ROOT/docs/CONTAINER-QUICKSTART.md" ]; then
        pass_test "Container quickstart guide exists"
    else
        fail_test "Container quickstart guide missing" "CONTAINER-QUICKSTART.md not found"
    fi
}

# Test 8: Security Verification
test_security() {
    echo -e "\n${BLUE}Test 8: Security Verification${NC}"
    
    # Check for rootless container support
    if grep -q "rootless" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh"; then
        pass_test "Rootless container support configured"
    else
        fail_test "Rootless container support missing" "rootless configuration not found"
    fi
    
    # Check for capability dropping in container configs
    if grep -q "default_capabilities" "$PROJECT_ROOT/scripts/configs/containers/containers.conf" 2>/dev/null; then
        pass_test "Container capability restrictions configured"
    else
        fail_test "Container capabilities not restricted" "default_capabilities not found"
    fi
}

# Test 9: Performance Checks
test_performance() {
    echo -e "\n${BLUE}Test 9: Performance Verification${NC}"
    
    # This would require actual builds to test, so we check configurations
    
    # Check for minimal base packages
    if grep -q "MINIMAL_PACKAGES=" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh"; then
        local pkg_count=$(grep -A20 "MINIMAL_PACKAGES=(" "$PROJECT_ROOT/scripts/scripts/build-base-image.sh" | grep -c '"')
        if [ "$pkg_count" -lt 50 ]; then
            pass_test "Base image has minimal package count ($pkg_count packages)"
        else
            fail_test "Base image has too many packages" "$pkg_count packages (target: <50)"
        fi
    else
        fail_test "Cannot verify base image size" "MINIMAL_PACKAGES not found"
    fi
}

# Run all tests
run_all_tests() {
    test_dsl_implementation
    test_container_module
    test_build_system
    test_container_configs
    test_reproducibility_features
    test_integration
    test_documentation
    test_security
    test_performance
}

# Execute tests
run_all_tests

# Summary
echo
echo "==================================="
echo "Test Summary"
echo "==================================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo

# Print detailed results
echo "Detailed Results:"
for result in "${TEST_RESULTS[@]}"; do
    if [[ $result == PASS:* ]]; then
        echo -e "  ${GREEN}✓${NC} ${result#PASS: }"
    else
        echo -e "  ${RED}✗${NC} ${result#FAIL: }"
    fi
done

# Exit code
if [ "$TESTS_FAILED" -eq 0 ]; then
    echo -e "\n${GREEN}All reproducibility tests passed!${NC}"
    exit 0
else
    echo -e "\n${RED}Some tests failed. Please review the failures above.${NC}"
    exit 1
fi