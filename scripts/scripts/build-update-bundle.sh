#!/bin/bash
# Create an OSTree update bundle for distribution

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$PROJECT_ROOT/config/dev.conf"

echo "Creating OSTree update bundle..."
echo "Version: $HORIZONOS_VERSION"

# Check if repo exists
if [ ! -d "$PROJECT_ROOT/repo" ]; then
    echo "Error: OSTree repository not found"
    exit 1
fi

# Get latest commit
COMMIT=$(ostree log --repo="$PROJECT_ROOT/repo" horizonos/test/x86_64 | head -1 | cut -d' ' -f2)
echo "Latest commit: $COMMIT"

# Create output directory
mkdir -p "$PROJECT_ROOT/build/updates"

# Generate static deltas for efficient updates
echo "Generating static deltas..."
ostree --repo="$PROJECT_ROOT/repo" static-delta generate horizonos/test/x86_64

# Create update bundle
BUNDLE_NAME="horizonos-ostree-$HORIZONOS_VERSION.tar.gz"
echo "Creating bundle: $BUNDLE_NAME"

cd "$PROJECT_ROOT"
tar czf "build/updates/$BUNDLE_NAME" -C repo .

# Create metadata file
cat > "build/updates/horizonos-update-$HORIZONOS_VERSION.json" << EOF
{
  "version": "$HORIZONOS_VERSION",
  "channel": "stable",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "commit": "$COMMIT",
  "size": $(stat -c%s "build/updates/$BUNDLE_NAME"),
  "sha256": "$(sha256sum "build/updates/$BUNDLE_NAME" | cut -d' ' -f1)",
  "changes": [
    "See RELEASE_NOTES.md for details"
  ]
}
EOF

echo ""
echo "Update bundle created:"
echo "  Bundle: build/updates/$BUNDLE_NAME"
echo "  Metadata: build/updates/horizonos-update-$HORIZONOS_VERSION.json"
echo ""
echo "Upload these files to your GitHub release."