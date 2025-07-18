# HorizonOS Auto-Update System Documentation

## Overview

HorizonOS includes a sophisticated auto-update system that provides:
- Automatic updates from GitHub releases
- Atomic updates via OSTree
- Zero-downtime updates (applied on reboot)
- Automatic rollback on failure
- Bandwidth-efficient delta updates

## Architecture

### Components

1. **horizonos-autoupdate** - Main update script
   - Location: `/usr/local/bin/horizonos-autoupdate`
   - Handles checking, downloading, and applying updates

2. **Systemd Units**
   - `horizonos-update.service` - Runs the update check
   - `horizonos-update.timer` - Schedules daily checks
   - `horizonos-update-notify.service` - Desktop notifications

3. **Configuration**
   - `/etc/horizonos/update.conf` - Update settings
   - `/etc/horizonos-release` - Current version info

4. **GitHub Integration**
   - Uses GitHub API to check for releases
   - Downloads OSTree bundles from release assets

## User Guide

### Basic Usage

Check for updates manually:
```bash
sudo horizonos-autoupdate check
```

Apply available updates:
```bash
sudo horizonos-autoupdate update
```

Configure update behavior:
```bash
sudo horizonos-autoupdate configure
```

### Configuration Options

Edit `/etc/horizonos/update.conf`:

```bash
# Update channel: stable, testing, or dev
UPDATE_CHANNEL="stable"

# Automatically download updates
AUTO_STAGE="true"

# Automatically reboot when system is idle
AUTO_REBOOT="false"

# Check interval in seconds (86400 = 24 hours)
CHECK_INTERVAL="86400"

# GitHub repository
GITHUB_REPO="codeyousef/HorizonOS"
```

### Update Process

1. **Check Phase**
   - System queries GitHub API for latest release
   - Compares with current version
   - Stores update info if available

2. **Download Phase**
   - Downloads OSTree bundle from GitHub
   - Verifies integrity
   - Extracts to temporary location

3. **Stage Phase**
   - Pulls OSTree commit
   - Creates new deployment
   - Does not affect running system

4. **Apply Phase**
   - User reboots system
   - GRUB boots into new deployment
   - Old deployment kept for rollback

### Rollback

If the new version fails to boot:
```bash
# At GRUB menu, select previous deployment
# Or after booting:
sudo ostree admin rollback
sudo systemctl reboot
```

## Developer Guide

### Creating a Release

1. Update version:
   ```bash
   ./scripts/tools/create-release.sh
   ```

2. Push tag to trigger GitHub Actions:
   ```bash
   git push origin main
   git push origin v0.2.0
   ```

3. GitHub Actions will:
   - Build the ISO
   - Create OSTree bundle
   - Upload to releases

### Manual Update Bundle

Create an update bundle locally:
```bash
./scripts/scripts/build-update-bundle.sh
```

### Testing Updates

1. Build a test system:
   ```bash
   sudo ./scripts/scripts/build-test.sh
   sudo ./scripts/scripts/build-iso.sh
   ```

2. Install in VM

3. Create a new version:
   ```bash
   # Make changes
   echo "0.2.0-test" > VERSION
   sudo ./scripts/scripts/build-test.sh
   ./scripts/scripts/build-update-bundle.sh
   ```

4. Test update flow:
   ```bash
   # In VM
   sudo horizonos-autoupdate check
   sudo horizonos-autoupdate update
   sudo reboot
   ```

## Technical Details

### OSTree Integration

Updates use OSTree's atomic deployment system:
- Each update creates a new deployment
- `/etc` is merged with three-way merge
- `/var` is shared between deployments
- Rollback is instant

### Static Deltas

The system uses OSTree static deltas for efficiency:
- Only changed files are downloaded
- Binary diff format
- Typically 10-20% of full size

### Security

Current implementation:
- HTTPS for all downloads
- SHA256 verification
- No GPG signing (planned)

Future improvements:
- GPG signature verification
- Certificate pinning
- Signed update metadata

### Update Channels

Three channels are planned:
- **stable** - Production releases
- **testing** - Beta releases
- **dev** - Nightly builds

Currently all updates come from GitHub releases.

## Troubleshooting

### Update Check Fails

```bash
# Check logs
journalctl -u horizonos-update

# Test GitHub API
curl https://api.github.com/repos/codeyousef/HorizonOS/releases/latest

# Check network
ping github.com
```

### Update Won't Apply

```bash
# Check disk space
df -h /

# Check OSTree status
ostree admin status

# Verify repo integrity
ostree fsck
```

### System Won't Boot After Update

1. At GRUB, select previous entry
2. After boot:
   ```bash
   sudo ostree admin rollback
   sudo systemctl reboot
   ```

### Manual Recovery

```bash
# List deployments
ostree admin status

# Set default deployment (0 is newest)
sudo ostree admin set-default 1

# Clean up failed deployment
sudo ostree admin undeploy 0
```

## Logs and Monitoring

View update logs:
```bash
# Service logs
journalctl -u horizonos-update

# Update script logs
sudo tail -f /var/log/horizonos-update.log

# OSTree journal
journalctl -u ostree
```

## Future Enhancements

Planned improvements:
1. P2P update distribution
2. Differential updates
3. A/B partition scheme
4. Update staging during idle
5. Bandwidth limiting
6. Update metrics/telemetry

## FAQ

**Q: How much bandwidth do updates use?**
A: Typically 50-200MB per update with static deltas.

**Q: Can I disable automatic updates?**
A: Yes, disable the timer: `sudo systemctl disable horizonos-update.timer`

**Q: How do I know an update is ready?**
A: Desktop notification appears, or run `horizonos-autoupdate check`

**Q: Is it safe to interrupt an update?**
A: Yes, updates are atomic. Interruption won't break your system.

**Q: How many old versions are kept?**
A: By default, 2 deployments (current + previous).

**Q: Can I update offline?**
A: No, internet connection is required to check GitHub.