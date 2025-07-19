# Getty Configuration for HorizonOS ISO

## Problem
The previous HorizonOS ISO builds were experiencing a getty loop issue where the system would repeatedly try to start getty services but fail, preventing the boot process from completing properly.

## Root Cause
1. **Incorrect agetty path**: The configuration was using `/sbin/agetty`, but in modern Arch Linux systems, agetty is located at `/usr/bin/agetty`.
2. **Overly complex configuration**: Multiple redundant services and configurations were conflicting with each other.
3. **Deviation from standard archiso**: The configuration didn't follow the standard archiso approach used by official Arch Linux ISOs.

## Solution
Based on research of BlendOS, EndeavourOS, and official archiso configurations, we've implemented a minimal, standard-compliant getty configuration.

### Key Changes
1. **Correct agetty path**: Now using `/usr/bin/agetty`
2. **Minimal configuration**: Only configuring autologin for tty1, letting systemd handle the rest
3. **Standard approach**: Following the exact pattern used by official Arch Linux ISOs

### Configuration File
Located at: `/etc/systemd/system/getty@tty1.service.d/autologin.conf`

```ini
[Service]
ExecStart=
ExecStart=-/usr/bin/agetty --autologin root --noclear %I 38400 linux
```

## Debugging
If getty issues occur, use the included `debug-getty` tool:
```bash
debug-getty
```

This tool will:
- Check agetty binary location
- Display getty configuration files
- Show systemd target and service status
- List failed services
- Display getty-related journal entries

## Build Process
The getty configuration is applied during ISO build by:
1. `scripts/scripts/boot-fixes/getty-archiso-standard.sh` - Contains the standard configuration
2. `scripts/scripts/build-iso.sh` - Applies the configuration during ISO creation

## Testing
Before building an ISO, test the getty configuration:
```bash
./scripts/scripts/test-getty-config.sh
```

## References
- [Arch Linux archiso](https://gitlab.archlinux.org/archlinux/archiso)
- [BlendOS ISO build scripts](https://github.com/blend-os/blendiso)
- [EndeavourOS ISO framework](https://github.com/endeavouros-team/EndeavourOS-ISO)
- [Getty - ArchWiki](https://wiki.archlinux.org/title/Getty)