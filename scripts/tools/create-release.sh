#!/bin/bash
# Script to create a new HorizonOS release

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}HorizonOS Release Creator${NC}"
echo "=========================="
echo ""

# Check if we're in the project root
if [ ! -f "VERSION" ] || [ ! -d "scripts" ]; then
    echo -e "${RED}Error: This script must be run from the HorizonOS project root${NC}"
    exit 1
fi

# Get current version
CURRENT_VERSION=$(cat VERSION)
echo "Current version: $CURRENT_VERSION"
echo ""

# Ask for new version
read -p "Enter new version (e.g., 0.2.0): " NEW_VERSION

if [ -z "$NEW_VERSION" ]; then
    echo -e "${RED}Error: Version cannot be empty${NC}"
    exit 1
fi

# Ask for release type
echo ""
echo "Release type:"
echo "1) stable"
echo "2) testing"
echo "3) dev"
read -p "Select release type [1-3]: " RELEASE_TYPE

case $RELEASE_TYPE in
    1) CHANNEL="stable" ;;
    2) CHANNEL="testing" ;;
    3) CHANNEL="dev" ;;
    *) echo -e "${RED}Invalid selection${NC}"; exit 1 ;;
esac

# Update VERSION file
echo "$NEW_VERSION" > VERSION

# Update dev.conf
sed -i "s/HORIZONOS_VERSION=.*/HORIZONOS_VERSION=\"$NEW_VERSION\"/" config/dev.conf

# Commit version bump
echo ""
echo -e "${YELLOW}Committing version bump...${NC}"
git add VERSION config/dev.conf
git commit -m "Bump version to $NEW_VERSION" || true

# Create tag based on channel
case $CHANNEL in
    "stable") TAG="v$NEW_VERSION" ;;
    "testing") TAG="v$NEW_VERSION-beta.1" ;;
    "dev") TAG="v$NEW_VERSION-dev.$(date +%Y%m%d)" ;;
esac

echo ""
echo -e "${YELLOW}Creating tag $TAG...${NC}"

# Get release notes for this version
echo ""
echo "Enter release notes (press Ctrl+D when done):"
RELEASE_NOTES=$(cat)

# Create annotated tag
git tag -a "$TAG" -m "Release $NEW_VERSION

$RELEASE_NOTES"

echo ""
echo -e "${GREEN}Release preparation complete!${NC}"
echo ""
echo "Next steps:"
echo "1. Review the changes: git show $TAG"
echo "2. Push to GitHub: git push origin main && git push origin $TAG"
echo "3. GitHub Actions will automatically build and release"
echo ""
echo "To undo this release:"
echo "  git tag -d $TAG"
echo "  git reset --hard HEAD~1"