# HorizonOS Kotlin Configuration DSL

A type-safe, declarative configuration system for HorizonOS using Kotlin DSL. This system provides compile-time validation, atomic updates via OSTree, and live system updates without reboots.

## Features

- **Type-Safe Configuration**: Leverage Kotlin's type system for compile-time validation
- **Declarative Syntax**: Clean, readable configuration files using Kotlin DSL
- **Comprehensive Validation**: Built-in validators for hostnames, timezones, packages, services, and more
- **Live Updates**: Apply configuration changes to running systems without reboots
- **Atomic Updates**: Integration with OSTree for safe, atomic system updates with rollback capability
- **Multi-Format Output**: Generate JSON, YAML, Shell scripts, Systemd units, Ansible playbooks, and more
- **Automation Support**: Built-in RPA/workflow DSL for system automation
- **State Management**: Snapshot and restore system states for recovery

## Quick Start

### Writing a Configuration

Create a file named `system.horizonos.kts`:

```kotlin
horizonOS {
    // System configuration
    hostname = "my-desktop"
    timezone = "America/New_York"
    locale = "en_US.UTF-8"
    
    // Package management
    packages {
        install("firefox", "thunderbird", "libreoffice")
        install("development-tools") {
            group = "Development"
        }
        remove("nano")  // Prefer vim
    }
    
    // Service management
    services {
        enable("NetworkManager")
        enable("bluetooth")
        disable("cups")  // No printing needed
    }
    
    // User management
    users {
        user("john") {
            uid = 1000
            shell = "/usr/bin/zsh"
            groups("wheel", "video", "audio")
        }
    }
    
    // Desktop environment
    desktop {
        environment = DesktopEnvironment.PLASMA
        autoLogin = true
        autoLoginUser = "john"
        
        plasma {
            theme = "breeze-dark"
            lookAndFeel = "org.kde.breezedark.desktop"
        }
    }
}
```

### Compiling a Configuration

```bash
# Compile to various output formats
./gradlew compileConfig -PconfigFile=system.horizonos.kts -PoutputDir=output/

# This generates:
# - output/json/config.json          # Complete configuration in JSON
# - output/yaml/config.yaml          # YAML representation
# - output/scripts/deploy.sh         # Deployment script
# - output/systemd/*.service         # Systemd unit files
# - output/ansible/*.yml             # Ansible playbooks
# - output/docs/README.md            # Documentation
```

## Configuration Reference

### System Configuration

```kotlin
horizonOS {
    hostname = "my-system"              // Required: Valid hostname
    timezone = "UTC"                    // Required: Valid timezone
    locale = "en_US.UTF-8"             // Required: Valid locale
}
```

### Package Management

```kotlin
packages {
    // Install packages
    install("package1", "package2", "package3")
    
    // Install with options
    install("docker") {
        group = "Containers"
        version = "latest"
    }
    
    // Remove packages
    remove("unwanted-package")
}
```

### Service Management

```kotlin
services {
    // Enable and start services
    enable("sshd", "NetworkManager")
    
    // Enable with configuration
    enable("nginx") {
        environment["NGINX_WORKER_PROCESSES"] = "auto"
        environment["NGINX_ERROR_LOG"] = "/var/log/nginx/error.log"
    }
    
    // Disable services
    disable("bluetooth", "cups")
}
```

### User Management

```kotlin
users {
    user("alice") {
        uid = 1000                      // Optional: Specific UID
        shell = "/usr/bin/fish"         // Default: /bin/bash
        groups("wheel", "docker")       // Additional groups
        homeDir = "/home/alice"         // Default: /home/<username>
    }
}
```

### Repository Configuration

```kotlin
repositories {
    // Package repository
    add("multilib", "https://mirror.archlinux.org/multilib") {
        gpgCheck = true
        priority = 10
    }
    
    // OSTree repository
    ostree("horizonos", "https://ostree.horizonos.org") {
        branch("stable", "testing")
        gpgVerify = true
    }
}
```

### Desktop Environment Configuration

```kotlin
desktop {
    environment = DesktopEnvironment.HYPRLAND
    
    hyprland {
        theme = "catppuccin-mocha"
        animations = true
        gaps = 10
        borderSize = 2
        
        // KDE integration
        kdeIntegration = true
        personalityMode = PersonalityMode.MACOS
    }
}
```

### Automation Workflows

```kotlin
automation {
    // Time-based workflow
    workflow("daily-update") {
        description = "Daily system update"
        
        trigger {
            time("02:00")
            onDays(WEEKDAYS)
        }
        
        actions {
            runCommand("pacman -Syu --noconfirm")
            notification("System Update", "Daily update completed")
        }
    }
    
    // File monitoring workflow
    workflow("config-sync") {
        trigger {
            fileModified("/etc/horizonos/config.kts")
        }
        
        actions {
            runCommand("horizonos-apply /etc/horizonos/config.kts")
        }
    }
    
    // Teaching mode for user demonstrations
    teach("browser-cleanup") {
        description = "Learn browser cleanup routine"
        watchPath = "/home/user/.mozilla"
        
        // System learns from user actions and creates workflow
    }
}
```

## Advanced Features

### Validation

The DSL includes comprehensive validation:

- **Hostname validation**: RFC 1123 compliant
- **Timezone validation**: Against system timezone database
- **Locale validation**: Valid locale format
- **Package validation**: Checks against package repositories
- **Service validation**: Verifies service unit files exist
- **User validation**: Ensures no conflicts with existing users

### Live Updates

Apply configuration changes without rebooting:

```kotlin
// In your config file, mark updates as live-applicable
services {
    enable("nginx")  // Can be applied live
}

packages {
    install("htop")  // Can be installed without reboot
}

// Desktop environment changes typically require reboot
desktop {
    environment = DesktopEnvironment.PLASMA  // Requires reboot
}
```

### State Management

The system automatically creates snapshots before applying changes:

```bash
# List snapshots
horizonos-state list

# Restore to a previous state
horizonos-state restore <snapshot-id>

# Manual snapshot
horizonos-state snapshot "Before major update"
```

## Integration

### With OSTree

HorizonOS uses OSTree for atomic updates:

```bash
# Apply configuration (creates OSTree commit)
horizonos-apply system.horizonos.kts

# Rollback to previous configuration
horizonos-rollback

# List available commits
ostree log horizonos/stable/x86_64
```

### With CI/CD

Example GitHub Actions workflow:

```yaml
name: Deploy Configuration
on:
  push:
    paths:
      - 'config/**.horizonos.kts'

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Validate Configuration
        run: |
          ./gradlew compileConfig -PconfigFile=config/prod.horizonos.kts
```

## Examples

### Minimal Configuration

```kotlin
horizonOS {
    hostname = "minimal"
    timezone = "UTC"
    locale = "en_US.UTF-8"
}
```

### Development Workstation

```kotlin
horizonOS {
    hostname = "dev-workstation"
    timezone = "America/Los_Angeles"
    locale = "en_US.UTF-8"
    
    packages {
        // Development tools
        install("base-devel", "git", "neovim")
        install("docker", "docker-compose")
        install("rust", "go", "nodejs", "npm")
        
        // IDEs
        install("intellij-idea-community-edition")
        install("visual-studio-code-bin")
    }
    
    services {
        enable("docker")
        enable("sshd")
    }
    
    users {
        user("developer") {
            uid = 1000
            shell = "/usr/bin/zsh"
            groups("wheel", "docker", "video", "audio")
        }
    }
    
    desktop {
        environment = DesktopEnvironment.HYPRLAND
        hyprland {
            animations = true
            gaps = 8
            kdeIntegration = true
        }
    }
}
```

### Server Configuration

```kotlin
horizonOS {
    hostname = "web-server"
    timezone = "UTC"
    locale = "en_US.UTF-8"
    
    packages {
        install("nginx", "postgresql", "redis")
        install("certbot", "python-certbot-nginx")
        install("fail2ban", "ufw")
    }
    
    services {
        enable("nginx", "postgresql", "redis")
        enable("fail2ban", "ufw")
        enable("sshd") {
            environment["PORT"] = "2222"
        }
        
        disable("bluetooth", "cups", "avahi-daemon")
    }
    
    users {
        user("webadmin") {
            uid = 1000
            groups("wheel")
        }
        
        user("postgres") {
            uid = 999
            shell = "/bin/false"
            homeDir = "/var/lib/postgresql"
        }
    }
}
```

## Best Practices

1. **Version Control**: Keep your configuration files in Git
2. **Test First**: Use dry-run mode to test changes
3. **Document Changes**: Use descriptive commit messages
4. **Modularize**: Split large configurations into multiple files
5. **Validate Early**: Run validation before deploying
6. **Backup State**: Ensure snapshots are created before major changes

## Troubleshooting

### Validation Errors

```
Error: Invalid hostname 'my_host'
  Hostnames cannot contain underscores
  
Fix: Use 'my-host' instead
```

### Live Update Failures

```
Warning: Changes require reboot:
  - Desktop environment change
  - Kernel update
  
Run 'systemctl reboot' when convenient
```

### State Recovery

```bash
# If system is in inconsistent state
horizonos-state check

# Restore last known good configuration
horizonos-state restore --last-good
```

## Contributing

See [CONTRIBUTING.md](../../../CONTRIBUTING.md) for development setup and guidelines.

## License

HorizonOS Kotlin Configuration DSL is part of the HorizonOS project and is licensed under the MIT License.