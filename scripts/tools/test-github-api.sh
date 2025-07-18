#!/bin/bash
# Test GitHub API integration locally

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${GREEN}Testing GitHub API Integration${NC}"
echo "=============================="
echo ""

# Test 1: Check API connectivity
echo -e "${YELLOW}Test 1: Checking GitHub API connectivity...${NC}"
RESPONSE=$(curl -s -w "\n%{http_code}" https://api.github.com/rate_limit)
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
if [ "$HTTP_CODE" = "200" ]; then
    echo -e "${GREEN}✓ GitHub API is accessible${NC}"
    RATE_LIMIT=$(echo "$RESPONSE" | head -n -1 | grep -o '"remaining":[0-9]*' | cut -d: -f2)
    echo "  API rate limit remaining: $RATE_LIMIT"
else
    echo -e "${RED}✗ Cannot reach GitHub API (HTTP $HTTP_CODE)${NC}"
fi

# Test 2: Check repository
echo -e "\n${YELLOW}Test 2: Checking HorizonOS repository...${NC}"
REPO_INFO=$(curl -s https://api.github.com/repos/codeyousef/HorizonOS)
if echo "$REPO_INFO" | grep -q '"full_name"'; then
    echo -e "${GREEN}✓ Repository found${NC}"
    echo "  Name: $(echo "$REPO_INFO" | grep -o '"full_name":"[^"]*"' | cut -d'"' -f4)"
    echo "  Description: $(echo "$REPO_INFO" | grep -o '"description":"[^"]*"' | cut -d'"' -f4)"
else
    echo -e "${YELLOW}! Repository not found (expected for private repos)${NC}"
fi

# Test 3: Check releases endpoint
echo -e "\n${YELLOW}Test 3: Checking releases endpoint...${NC}"
RELEASES=$(curl -s https://api.github.com/repos/codeyousef/HorizonOS/releases)
if echo "$RELEASES" | grep -q '\['; then
    RELEASE_COUNT=$(echo "$RELEASES" | grep -c '"tag_name"' || echo "0")
    echo -e "${GREEN}✓ Releases endpoint accessible${NC}"
    echo "  Number of releases: $RELEASE_COUNT"
    
    if [ "$RELEASE_COUNT" -gt 0 ]; then
        LATEST_TAG=$(echo "$RELEASES" | grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4)
        echo "  Latest release: $LATEST_TAG"
    fi
else
    echo -e "${YELLOW}! No releases found yet${NC}"
fi

# Test 4: Simulate update check
echo -e "\n${YELLOW}Test 4: Simulating update check...${NC}"
cat > /tmp/test-update-check.sh << 'EOF'
#!/bin/bash
GITHUB_REPO="codeyousef/HorizonOS"
CURRENT_VERSION="0.1.0-dev"

# Check for updates
API_URL="https://api.github.com/repos/$GITHUB_REPO/releases/latest"
RESPONSE=$(curl -s -w "\n%{http_code}" "$API_URL")
HTTP_CODE=$(echo "$RESPONSE" | tail -1)
RELEASE_INFO=$(echo "$RESPONSE" | head -n -1)

if [ "$HTTP_CODE" = "200" ]; then
    TAG_NAME=$(echo "$RELEASE_INFO" | grep -o '"tag_name":"[^"]*"' | cut -d'"' -f4)
    VERSION="${TAG_NAME#v}"
    echo "Latest version: $VERSION"
    echo "Current version: $CURRENT_VERSION"
    
    if [ "$VERSION" != "$CURRENT_VERSION" ]; then
        echo "Update available!"
    else
        echo "System is up to date"
    fi
elif [ "$HTTP_CODE" = "404" ]; then
    echo "No releases found (this is normal if you haven't created any releases yet)"
else
    echo "API request failed with code: $HTTP_CODE"
fi
EOF

chmod +x /tmp/test-update-check.sh
/tmp/test-update-check.sh
rm /tmp/test-update-check.sh

echo -e "\n${GREEN}API Testing Complete!${NC}"
echo ""
echo "Note: If the repository is private or has no releases yet,"
echo "some tests may show warnings. This is normal."