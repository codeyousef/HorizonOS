# HorizonOS Release Channels Guide

## Overview

HorizonOS uses a three-channel release strategy to provide different levels of stability and features to users based on their needs.

## Release Channels

### 1. Stable Channel
- **Tag format**: `v0.1.0`
- **Target audience**: Production systems, general users
- **Update frequency**: Monthly or when critical fixes are needed
- **Testing**: Fully tested, no known critical issues
- **Features**: Only stable, well-tested features

### 2. Testing Channel
- **Tag formats**: 
  - Beta: `v0.1.0-beta.1`, `v0.1.0-beta.2`
  - Release Candidate: `v0.1.0-rc.1`, `v0.1.0-rc.2`
- **Target audience**: Early adopters, testers
- **Update frequency**: Weekly during active development
- **Testing**: Feature complete, may have minor bugs
- **Features**: New features ready for wider testing

### 3. Dev Channel
- **Tag format**: `v0.1.0-dev.20231201` (includes date)
- **Target audience**: Developers, contributors
- **Update frequency**: Daily or per-commit
- **Testing**: May have bugs, experimental features
- **Features**: Bleeding edge, experimental

## Creating Releases

### Using the Release Script

```bash
# Interactive release creation
./scripts/tools/create-release.sh

# Choose:
# - Version number (e.g., 0.2.0)
# - Channel (stable/testing/dev)
# - Enter release notes
```

### Manual Tagging

```bash
# Stable release
git tag -a v0.2.0 -m "Release 0.2.0"

# Beta release
git tag -a v0.2.0-beta.1 -m "Beta release 0.2.0-beta.1"

# Dev build
git tag -a v0.2.0-dev.20231201 -m "Dev build"

# Push tags
git push origin --tags
```

## Configuring Update Channel

Users can configure their preferred channel in `/etc/horizonos/update.conf`:

```bash
# Default: stable
UPDATE_CHANNEL="stable"

# For beta testing
UPDATE_CHANNEL="testing"

# For development
UPDATE_CHANNEL="dev"
```

## Channel Promotion

Typical release flow:

1. **Dev builds** → Daily automated builds
2. **Beta release** → When features are complete
3. **RC release** → After beta testing
4. **Stable release** → After RC validation

Example progression:
- `v0.2.0-dev.20231201` → Dev build
- `v0.2.0-dev.20231202` → Dev build with fixes
- `v0.2.0-beta.1` → First beta
- `v0.2.0-beta.2` → Beta with fixes
- `v0.2.0-rc.1` → Release candidate
- `v0.2.0` → Stable release

## Update Behavior by Channel

### Stable Channel Users
- Only receive stable releases
- Updates are thoroughly tested
- Minimal risk of breakage

### Testing Channel Users
- Receive beta and RC releases
- Help test new features
- May encounter minor issues

### Dev Channel Users
- Receive all updates including daily builds
- First to get new features
- Should expect occasional breakage

## GitHub Actions Integration

The release workflow automatically:
1. Detects channel from tag format
2. Marks prereleases appropriately
3. Includes channel info in release notes
4. Builds ISOs and update bundles

## Best Practices

1. **Version Numbering**
   - Use semantic versioning: MAJOR.MINOR.PATCH
   - Increment PATCH for fixes
   - Increment MINOR for features
   - Increment MAJOR for breaking changes

2. **Testing Flow**
   - All features start in dev
   - Promote to beta when ready
   - RC for final testing
   - Stable only after validation

3. **Release Notes**
   - List all changes clearly
   - Highlight breaking changes
   - Include upgrade instructions

4. **Rollback Plan**
   - Keep previous stable version available
   - Document rollback procedures
   - Test rollback before releasing

## Examples

### Creating a dev build:
```bash
echo "0.2.0" > VERSION
./scripts/tools/create-release.sh
# Select: dev channel
git push origin main
git push origin v0.2.0-dev.20231201
```

### Creating a beta release:
```bash
./scripts/tools/create-release.sh
# Select: testing channel
# Tag will be v0.2.0-beta.1
```

### Creating a stable release:
```bash
./scripts/tools/create-release.sh  
# Select: stable channel
# Tag will be v0.2.0
```

## Channel Migration

Users can switch channels:

```bash
# Switch to testing
sudo sed -i 's/UPDATE_CHANNEL=.*/UPDATE_CHANNEL="testing"/' /etc/horizonos/update.conf
sudo horizonos-autoupdate check

# Switch back to stable
sudo sed -i 's/UPDATE_CHANNEL=.*/UPDATE_CHANNEL="stable"/' /etc/horizonos/update.conf
```

Note: Downgrading requires manual intervention with OSTree rollback.