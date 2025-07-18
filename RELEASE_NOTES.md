# HorizonOS Release Notes

## Version 0.1.0-dev (Genesis)
*Release Date: TBD*

### 🎉 Initial Release

HorizonOS is an Arch Linux-based immutable distribution that brings together modern technologies for a next-generation desktop experience.

### ✨ Key Features

#### System Architecture
- **OSTree-based immutable system** - Atomic updates and rollbacks
- **Btrfs with automatic snapshots** - Data protection and recovery
- **Container-based architecture** - Applications run in isolated environments
- **Auto-update system** - Seamless updates from GitHub releases

#### Desktop Environment
- **Graph-based window management** - Revolutionary UI where everything is a node
- **AI-powered desktop** - Local LLM integration with Ollama
- **Advanced visual effects** - GPU-accelerated rendering with WebGPU
- **Comprehensive theming** - Dark/light modes with smooth transitions

#### Developer Experience
- **Kotlin DSL configuration** - Type-safe system configuration
- **Multi-language support** - Rust, Kotlin, TypeScript development
- **Integrated AI assistance** - Code suggestions and automation
- **Graph-based development** - Visual programming paradigms

#### AI Integration
- **Local-first AI** - Privacy-preserving LLM with Ollama
- **Multi-agent system** - Specialized AI agents for different tasks
- **Hardware optimization** - Automatic GPU/CPU selection
- **Pattern recognition** - Learn from user behavior
- **Workflow automation** - n8n integration for complex tasks

### 📦 Included Software

#### Base System
- Linux kernel (latest stable)
- systemd init system
- Btrfs filesystem
- OSTree for atomic updates
- Podman container runtime

#### Development Tools (via containers)
- Git, Vim, Neovim
- Rust toolchain
- Node.js and npm
- Python 3.x
- Go compiler

#### Desktop Components
- Graph compositor (custom Wayland compositor)
- Graph engine (WebGPU-based renderer)
- Notification system
- Accessibility features
- Multi-user workspaces

### 🚀 Installation

1. Download the ISO from GitHub releases
2. Write to USB: `sudo dd if=horizonos-*.iso of=/dev/sdX bs=4M status=progress`
3. Boot from USB
4. Run `horizonos-install` and follow the prompts

### 🔄 Updates

HorizonOS includes an automatic update system that:
- Checks for updates daily
- Downloads updates in the background
- Applies updates atomically on reboot
- Allows rollback if issues occur

To manually check for updates:
```bash
horizonos-autoupdate check
horizonos-autoupdate update
```

### 🐛 Known Issues

- Graph desktop is experimental and may have rendering issues
- AI features require significant RAM (8GB+ recommended)
- Some container applications may need manual configuration

### 🔮 Future Plans

- Mobile companion app (Flutter-based)
- Enhanced AI capabilities
- Distributed computing support
- Advanced security features
- Package manager integration

### 📝 Notes

This is an early development release. Please report bugs and feedback at:
https://github.com/codeyousef/HorizonOS/issues

---

For more information, visit: https://github.com/codeyousef/HorizonOS