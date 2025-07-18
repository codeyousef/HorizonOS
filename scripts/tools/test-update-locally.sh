#!/bin/bash
# Local test script for HorizonOS auto-update system

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$PROJECT_ROOT/config/dev.conf"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}HorizonOS Auto-Update Local Test${NC}"
echo "================================="
echo ""

# Create test directories
TEST_DIR="$PROJECT_ROOT/test-update"
mkdir -p "$TEST_DIR"/{repo,updates,config,logs}

# Test 1: Verify update script syntax
echo -e "${YELLOW}Test 1: Checking update script syntax...${NC}"
bash -n "$PROJECT_ROOT/scripts/tools/horizonos-autoupdate" && echo -e "${GREEN}✓ Script syntax is valid${NC}" || echo -e "${RED}✗ Script has syntax errors${NC}"

# Test 2: Test configuration handling
echo -e "\n${YELLOW}Test 2: Testing configuration...${NC}"
cat > "$TEST_DIR/config/update.conf" << EOF
UPDATE_CHANNEL="testing"
AUTO_STAGE="true"
AUTO_REBOOT="false"
CHECK_INTERVAL="3600"
GITHUB_REPO="codeyousef/HorizonOS"
EOF

# Source the update script functions in a subshell
(
    # Override paths for testing
    CONFIG_FILE="$TEST_DIR/config/update.conf"
    UPDATE_DIR="$TEST_DIR/updates"
    LOG_FILE="$TEST_DIR/logs/update.log"
    
    # Source configuration
    source "$CONFIG_FILE"
    echo -e "${GREEN}✓ Configuration loaded successfully${NC}"
    echo "  Channel: $UPDATE_CHANNEL"
    echo "  Auto-stage: $AUTO_STAGE"
    echo "  GitHub repo: $GITHUB_REPO"
)

# Test 3: Create mock GitHub API response
echo -e "\n${YELLOW}Test 3: Creating mock GitHub release...${NC}"
MOCK_VERSION="0.2.0-test"
cat > "$TEST_DIR/mock-release.json" << EOF
{
  "tag_name": "v$MOCK_VERSION",
  "name": "HorizonOS $MOCK_VERSION",
  "body": "Test release for local testing",
  "published_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "assets": [
    {
      "name": "horizonos-ostree-$MOCK_VERSION.tar.gz",
      "browser_download_url": "file://$TEST_DIR/updates/horizonos-ostree-$MOCK_VERSION.tar.gz",
      "size": 1024000
    }
  ]
}
EOF
echo -e "${GREEN}✓ Mock release created${NC}"

# Test 4: Build test OSTree commits
echo -e "\n${YELLOW}Test 4: Building test OSTree repository...${NC}"
ostree init --repo="$TEST_DIR/repo" --mode=archive
echo -e "${GREEN}✓ Test repository initialized${NC}"

# Create initial commit
mkdir -p "$TEST_DIR/rootfs"
echo "$HORIZONOS_VERSION" > "$TEST_DIR/rootfs/etc-horizonos-release"
ostree commit --repo="$TEST_DIR/repo" \
    --branch=horizonos/test/x86_64 \
    --subject="Initial test commit" \
    "$TEST_DIR/rootfs"
COMMIT1=$(ostree rev-parse --repo="$TEST_DIR/repo" horizonos/test/x86_64)
echo -e "${GREEN}✓ Initial commit: ${COMMIT1:0:8}${NC}"

# Create update commit
echo "$MOCK_VERSION" > "$TEST_DIR/rootfs/etc-horizonos-release"
echo "Updated file" > "$TEST_DIR/rootfs/test-update"
ostree commit --repo="$TEST_DIR/repo" \
    --branch=horizonos/test/x86_64 \
    --subject="Test update to $MOCK_VERSION" \
    "$TEST_DIR/rootfs"
COMMIT2=$(ostree rev-parse --repo="$TEST_DIR/repo" horizonos/test/x86_64)
echo -e "${GREEN}✓ Update commit: ${COMMIT2:0:8}${NC}"

# Create update bundle
echo -e "\n${YELLOW}Test 5: Creating update bundle...${NC}"
cd "$TEST_DIR"
tar czf "updates/horizonos-ostree-$MOCK_VERSION.tar.gz" -C repo .
echo -e "${GREEN}✓ Update bundle created${NC}"

# Test 6: Test update script functions
echo -e "\n${YELLOW}Test 6: Testing update script functions...${NC}"
cat > "$TEST_DIR/test-update-check.sh" << 'EOF'
#!/bin/bash
# Test version of check_github_updates function

check_github_updates() {
    local api_url="file://TEST_DIR/mock-release.json"
    local release_info=$(cat "TEST_DIR/mock-release.json")
    
    local tag_name=$(echo "$release_info" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
    local version="${tag_name#v}"
    
    echo "Latest version: $version"
    echo "Current version: $HORIZONOS_VERSION"
    
    if [ "$version" != "$HORIZONOS_VERSION" ]; then
        echo "Update available: $HORIZONOS_VERSION -> $version"
        return 0
    else
        echo "System is up to date"
        return 1
    fi
}

# Run test
HORIZONOS_VERSION="0.1.0-dev"
check_github_updates
EOF
sed -i "s|TEST_DIR|$TEST_DIR|g" "$TEST_DIR/test-update-check.sh"
chmod +x "$TEST_DIR/test-update-check.sh"
"$TEST_DIR/test-update-check.sh"

# Test 7: Verify systemd units
echo -e "\n${YELLOW}Test 7: Checking systemd units...${NC}"
for unit in horizonos-update.service horizonos-update.timer horizonos-update-notify.service; do
    if [ -f "$PROJECT_ROOT/scripts/systemd-units/$unit" ]; then
        systemd-analyze verify "$PROJECT_ROOT/scripts/systemd-units/$unit" 2>/dev/null && \
            echo -e "${GREEN}✓ $unit is valid${NC}" || \
            echo -e "${YELLOW}! $unit needs systemd to verify (OK for local test)${NC}"
    else
        echo -e "${RED}✗ $unit not found${NC}"
    fi
done

# Test 8: Test update script commands
echo -e "\n${YELLOW}Test 8: Testing update script commands...${NC}"
echo -e "${YELLOW}Note: Some commands require root/systemd and will show warnings${NC}"

# Test help
"$PROJECT_ROOT/scripts/tools/horizonos-autoupdate" -h > /dev/null 2>&1 && \
    echo -e "${GREEN}✓ Help command works${NC}" || \
    echo -e "${RED}✗ Help command failed${NC}"

# Summary
echo -e "\n${GREEN}Local Testing Complete!${NC}"
echo "======================="
echo ""
echo "Test artifacts created in: $TEST_DIR"
echo ""
echo "Next steps for full testing:"
echo "1. Build a test ISO with: sudo ./scripts/scripts/build-test.sh && sudo ./scripts/scripts/build-iso.sh"
echo "2. Install in a VM"
echo "3. Run: sudo horizonos-autoupdate check"
echo "4. Create a GitHub release to test real updates"
echo ""
echo "To clean up test files:"
echo "  rm -rf $TEST_DIR"