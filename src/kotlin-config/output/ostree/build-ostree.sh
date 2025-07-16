#!/bin/bash
# Build OSTree commit from configuration

set -euo pipefail

REPO_PATH="${REPO_PATH:-/ostree/repo}"
BRANCH="horizonos/stable/x86_64"

# Create temporary root
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Apply configuration
../scripts/deploy.sh

# Create OSTree commit
ostree commit \
    --repo=$REPO_PATH \
    --branch=$BRANCH \
    --subject="HorizonOS configuration update" \
    --tree=dir=$TMPDIR

echo "OSTree commit created successfully"