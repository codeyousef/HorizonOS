# HorizonOS Auto-Update Implementation Guide

## Overview

This guide sets up a complete auto-update system for HorizonOS that:

- Builds ISOs automatically on GitHub releases
- Provides downloadable ISOs from GitHub
- Updates the OS from within itself using OSTree
- Handles ongoing development changes

## 1. GitHub Actions Setup

### Create Release Workflow

1. Create `.github/workflows/release.yml` with the workflow from the first artifact
2. This workflow will:
    - Build the ISO when you push a tag
    - Create OSTree bundles for updates
    - Upload everything to GitHub releases

### Manual Release Process

```bash
# Tag a new version
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0

# Or trigger manually from GitHub Actions UI
```

## 2. Update System Integration

### Add Update Scripts to Repository

1. Create `scripts/tools/horizonos-autoupdate` (from artifact 2)
2. Make it executable: `chmod +x scripts/tools/horizonos-autoupdate`

### Modify Build Scripts

Add to `scripts/scripts/build-test.sh` after creating the rootfs:

```bash
# Install update system
echo "Installing update system..."
sudo cp scripts/tools/horizonos-autoupdate "$ROOTFS_DIR/usr/local/bin/"
sudo chmod +x "$ROOTFS_DIR/usr/local/bin/horizonos-autoupdate"

# Install systemd units (from artifact 3)
# ... copy systemd service and timer files ...

# Enable auto-updates by default
sudo ln -sf /etc/systemd/system/horizonos-update.timer \
    "$ROOTFS_DIR/etc/systemd/system/timers.target.wants/horizonos-update.timer"
```

## 3. OSTree Remote Configuration

### Set Up Update Server

For development, you can use GitHub releases as your "update server":

```bash
# In the installed system, add GitHub as remote
ostree remote add horizonos-github \
    https://github.com/codeyousef/HorizonOS/releases/download/latest/ostree-repo \
    --no-gpg-verify

# Or use a dedicated server
python3 scripts/scripts/serve-ostree.sh 8080
```

### Production Setup

For production, consider:

1. Setting up a CDN for OSTree content
2. Implementing GPG signing for security
3. Creating multiple update channels (stable, testing, dev)

## 4. Testing the Update System

### Local Testing

```bash
# Build and install HorizonOS
sudo ./scripts/scripts/build-iso.sh
# Install to a VM or test machine

# In the installed system
sudo horizonos-autoupdate check    # Check for updates
sudo horizonos-autoupdate update   # Apply updates
sudo horizonos-autoupdate configure # Configure auto-updates
```

### Update Flow

1. **Check Phase**: System checks GitHub API for new releases
2. **Download Phase**: Downloads OSTree bundle if update available
3. **Stage Phase**: Applies update to alternate OSTree deployment
4. **Reboot Phase**: User reboots to activate new version
5. **Rollback**: If boot fails, automatically rolls back

## 5. Development Workflow

### For Ongoing Development

```bash
# 1. Make changes to HorizonOS
vim src/some-file.kt

# 2. Test locally
sudo ./scripts/scripts/build-test.sh

# 3. Commit and push
git add .
git commit -m "Add new feature"
git push

# 4. Create release when ready
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0

# 5. Users automatically get update
# Their systems will check GitHub and update
```

## 6. User Experience

### For End Users

Once installed, HorizonOS will:

1. **Automatically check** for updates daily
2. **Download updates** in the background
3. **Stage updates** without disrupting current system
4. **Notify users** when updates are ready
5. **Apply on reboot** or immediately if configured

### Manual Control

Users can also:

```bash
# Check for updates manually
horizonos-autoupdate check

# Apply staged updates
horizonos-autoupdate update

# Configure update behavior
horizonos-autoupdate configure

# View update history
ostree admin status
```

## 7. Advanced Features

### Differential Updates

OSTree supports binary deltas to minimize download size:

```bash
# Generate static deltas in your build
ostree static-delta generate --repo=repo \
    --from=OLD_COMMIT --to=NEW_COMMIT
```

### A/B Updates

The system maintains two deployments:

- Current running system
- Staged update or previous version

This enables instant rollback if issues occur.

### Update Policies

Configure via `/etc/horizonos/update.conf`:

```bash
UPDATE_CHANNEL="stable"     # or "testing", "dev"
AUTO_STAGE="true"          # Download automatically
AUTO_REBOOT="false"        # Reboot automatically
REBOOT_TIME="03:00"        # When to reboot if auto
```

## 8. Security Considerations

### Implement GPG Signing

```bash
# Generate signing key
gpg --gen-key

# Sign OSTree commits
ostree commit --repo=repo \
    --gpg-sign=YOUR_KEY_ID \
    --gpg-homedir=/path/to/keys
```

### Secure Transport

- Use HTTPS for all downloads
- Verify checksums before applying updates
- Implement certificate pinning for production

## 9. Monitoring and Telemetry

### Update Metrics

Track (with user consent):

- Update success/failure rates
- Download times
- Common rollback reasons

### Health Checks

Implement post-update verification:

```bash
# In update script
verify_update() {
    # Check critical services
    systemctl is-active NetworkManager
    systemctl is-active sshd
    
    # Verify kernel modules
    lsmod | grep -q btrfs
    
    # Test network connectivity
    ping -c 1 8.8.8.8
}
```

## 10. Troubleshooting

### Common Issues

1. **Updates not downloading**: Check network, GitHub API limits
2. **Updates failing to apply**: Check disk space, OSTree repo integrity
3. **System not booting after update**: Automatic rollback should occur

### Debug Commands

```bash
# View update logs
journalctl -u horizonos-update

# Check OSTree status
ostree admin status

# Manually rollback
ostree admin set-default 1
systemctl reboot
```

## Summary

With this system in place:

- ✅ ISOs are automatically built and published to GitHub releases
- ✅ Users can download and install HorizonOS
- ✅ The OS updates itself automatically from GitHub
- ✅ Updates are atomic and can be rolled back
- ✅ Development changes are distributed seamlessly

The key is leveraging OSTree's atomic update capabilities with GitHub as your distribution channel, creating a modern, reliable update system for your rolling-release distribution.