#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
REPO_DIR="$PROJECT_ROOT/repo"
PORT=${1:-8080}

echo "=== HorizonOS OSTree Server ==="
echo "Repository: $REPO_DIR"
echo "Port: $PORT"
echo ""

# Check if repo exists
if [ ! -d "$REPO_DIR" ]; then
    echo "Error: OSTree repository not found at $REPO_DIR"
    echo "Run build-test.sh first to create an OSTree repository"
    exit 1
fi

# Update repository summary for HTTP serving
echo "Updating repository summary..."
ostree summary --repo="$REPO_DIR" --update

# Create a simple repository config if it doesn't exist
if [ ! -f "$REPO_DIR/config" ]; then
    cat > "$REPO_DIR/config" << EOF
[core]
repo_version=1
mode=archive-z2

[remote "origin"]
url=http://localhost:$PORT
EOF
fi

echo "Starting HTTP server..."
echo "Repository will be available at: http://localhost:$PORT"
echo ""
echo "To pull from this repository on another system:"
echo "  ostree remote add horizonos http://localhost:$PORT --no-gpg-verify"
echo "  ostree pull horizonos horizonos/test/x86_64"
echo ""
echo "Press Ctrl+C to stop the server"
echo ""

# Start a simple HTTP server
cd "$REPO_DIR"
python3 -m http.server $PORT