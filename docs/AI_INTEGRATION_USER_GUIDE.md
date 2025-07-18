# HorizonOS AI Integration User Guide

**Version**: 1.0  
**Last Updated**: 2025-01-18  

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [Core Features](#core-features)
4. [Configuration](#configuration)
5. [Privacy and Security](#privacy-and-security)
6. [Troubleshooting](#troubleshooting)
7. [Advanced Usage](#advanced-usage)
8. [FAQ](#faq)

## Introduction

Welcome to the HorizonOS AI Integration System! This guide will help you understand and use the AI features built into HorizonOS. Our AI system is designed with privacy-first principles, meaning all processing happens locally on your machine - your data never leaves your device.

### Key Principles

- **Local-Only Processing**: All AI models run on your hardware
- **Privacy by Design**: You control what data is collected and how it's used
- **Adaptive Intelligence**: The system learns your patterns to provide better suggestions
- **Transparent Operation**: You can see and control all AI activities

## Getting Started

### System Requirements

Before using the AI features, ensure your system meets these requirements:

- **RAM**: Minimum 8GB (16GB recommended)
- **Storage**: 50GB free space for AI models
- **GPU**: Optional but recommended for better performance
- **Network**: Internet required only for initial model downloads

### Initial Setup

1. **Enable AI Features**
   ```bash
   # Check AI status
   systemctl status horizonos-ai.target
   
   # Enable AI services
   sudo systemctl enable --now horizonos-ai.target
   ```

2. **Download AI Models**
   The system will automatically download appropriate models based on your hardware. This happens during first startup and may take 10-30 minutes depending on your internet connection.

3. **Configure Privacy Settings**
   Open the AI Settings panel to configure your privacy preferences:
   - What data to collect
   - How long to retain data
   - Which applications to monitor

## Core Features

### 1. Behavioral Learning

The AI system learns from your daily computer usage to provide intelligent suggestions and automation.

**What it learns:**
- Application usage patterns
- Common workflows
- Frequently accessed files
- Repetitive tasks

**Privacy controls:**
- Pause learning at any time
- Exclude specific applications
- Clear learning data
- Export your data

### 2. Smart Suggestions

Based on learned patterns, the system provides contextual suggestions:

- **Application Launch**: Suggests apps based on time of day and context
- **File Access**: Quick access to recently and frequently used files
- **Workflow Automation**: Detects repetitive tasks and offers to automate them
- **Command Suggestions**: Terminal command completion based on your history

### 3. Process Automation

The AI can automate repetitive tasks through multiple methods:

#### Visual Workflow Builder (n8n)
- Access at http://localhost:5678
- Create workflows visually
- Connect different applications
- Schedule automated tasks

#### Browser Automation
- Record browser actions
- Replay with modifications
- Fill forms automatically
- Extract data from websites

#### Desktop Automation
- Automate GUI interactions
- Create keyboard shortcuts
- Batch file operations
- Custom scripts

### 4. AI Agents

Specialized AI agents help with specific tasks:

- **Code Assistant**: Help with programming tasks
- **Research Assistant**: Gather and summarize information
- **Writing Assistant**: Help with documentation and emails
- **System Assistant**: Optimize system performance

## Configuration

### Using the Kotlin DSL

HorizonOS uses a type-safe configuration system. Create or edit `/etc/horizonos/ai/settings.horizonos.kts`:

```kotlin
aiSettings {
    // Master AI toggle
    enabled = true
    
    // Hardware optimization
    hardware {
        optimization = HardwareOptimization.AUTO
        preferGPU = true
        maxMemoryUsage = "4GB"
    }
    
    // Privacy settings
    privacy {
        localOnly = true
        telemetryEnabled = false
        encryptStorage = true
        dataRetention = 30.days
    }
    
    // Learning preferences
    learning {
        enabled = true
        minConfidence = 0.7
        excludeApplications = listOf("firefox-private", "signal-desktop")
        pauseDuringFocus = true
    }
    
    // Agent configuration
    agents {
        enabled = true
        maxConcurrent = 3
        defaultTimeout = 5.minutes
    }
}
```

### Configuration Options

#### Hardware Settings
- `optimization`: AUTO, PERFORMANCE, BALANCED, POWER_SAVE
- `preferGPU`: Use GPU acceleration when available
- `maxMemoryUsage`: Limit AI memory consumption
- `modelSelection`: ADAPTIVE, TINY, SMALL, MEDIUM, LARGE

#### Privacy Settings
- `localOnly`: Disable all network features
- `telemetryEnabled`: Share anonymous usage statistics
- `encryptStorage`: Encrypt AI data at rest
- `dataRetention`: How long to keep behavioral data
- `anonymizeData`: Remove identifying information

#### Learning Settings
- `enabled`: Master toggle for behavioral learning
- `minConfidence`: Minimum confidence for suggestions (0.0-1.0)
- `excludeApplications`: Apps to never monitor
- `pauseDuringFocus`: Don't learn during focus/DND mode
- `sampleInterval`: How often to sample activity (seconds)

## Privacy and Security

### Data Collection

The AI system collects:
- Application launch times and durations
- File access patterns (not content)
- Command history (with filtering)
- UI interaction patterns

The system NEVER collects:
- Passwords or credentials
- Personal messages or emails
- Browser history in private mode
- Content of your files
- Audio or video without permission

### Privacy Controls

1. **View Collected Data**
   ```bash
   horizonos-ai privacy show-data
   ```

2. **Export Your Data**
   ```bash
   horizonos-ai privacy export --output ~/my-ai-data.json
   ```

3. **Delete All AI Data**
   ```bash
   horizonos-ai privacy clear --all
   ```

4. **Pause AI Learning**
   ```bash
   horizonos-ai learning pause
   ```

### Security Features

- **Encryption**: All stored data is encrypted using AES-256
- **Access Control**: AI data accessible only to your user
- **Audit Logging**: Track all AI operations
- **Sandboxing**: Automation runs in isolated environments
- **Permission System**: Granular control over AI capabilities

## Troubleshooting

### Common Issues

**AI services not starting**
```bash
# Check service status
systemctl status horizonos-ai.service

# View logs
journalctl -u horizonos-ai.service -f

# Run diagnostics
horizonos-ai diagnose
```

**High memory usage**
```bash
# Check AI memory usage
horizonos-ai stats memory

# Limit memory usage
horizonos-ai config set maxMemoryUsage 2GB

# Switch to smaller model
horizonos-ai config set modelSelection SMALL
```

**Slow performance**
```bash
# Check if using GPU
horizonos-ai hardware info

# Enable GPU if available
horizonos-ai config set preferGPU true

# Reduce sampling frequency
horizonos-ai config set sampleInterval 10
```

### Getting Help

1. **Built-in Help**
   ```bash
   horizonos-ai help
   horizonos-ai <command> --help
   ```

2. **Diagnostics**
   ```bash
   horizonos-ai diagnose --verbose
   ```

3. **Community Support**
   - Forum: https://forum.horizonos.org/ai
   - Chat: #horizonos-ai on Matrix
   - Issues: https://github.com/horizonos/horizonos/issues

## Advanced Usage

### Custom AI Agents

Create custom agents for specific tasks:

```kotlin
// In your settings file
agents {
    custom("my-assistant") {
        model = "codellama:7b"
        systemPrompt = "You are a helpful coding assistant..."
        temperature = 0.7
        maxTokens = 2000
    }
}
```

### Workflow Automation

Create complex automations combining multiple tools:

1. **Open n8n**: http://localhost:5678
2. **Create workflow** using visual editor
3. **Add HorizonOS nodes** for system integration
4. **Test and activate** your workflow

### API Access

For developers, the AI system provides a local API:

```bash
# Query AI
curl -X POST http://localhost:11434/api/generate \
  -H "Content-Type: application/json" \
  -d '{"model": "llama3.2", "prompt": "Hello"}'

# Get suggestions
curl http://localhost:8090/api/suggestions/current
```

## FAQ

**Q: Does my data leave my computer?**
A: No, all AI processing happens locally. Only model downloads require internet.

**Q: How much disk space do AI models use?**
A: Typically 4-20GB depending on selected models. Tiny models use ~4GB, large models up to 40GB.

**Q: Can I use the AI offline?**
A: Yes, once models are downloaded, all features work offline.

**Q: How do I completely disable AI?**
A: Run `sudo systemctl disable --now horizonos-ai.target` and set `enabled = false` in your config.

**Q: Will AI slow down my system?**
A: The AI system is designed to be lightweight. It uses ~100MB RAM for monitoring and adapts to your system resources.

**Q: Can I use my own AI models?**
A: Yes, you can add custom models to Ollama and configure the system to use them.

**Q: Is the AI watching everything I do?**
A: No, you control what the AI monitors. By default, it respects private browsing, excluded apps, and focus modes.

---

## Next Steps

1. **Explore Settings**: Customize AI behavior to your preferences
2. **Try Automation**: Create your first automated workflow
3. **Enable Agents**: Try the AI assistants for specific tasks
4. **Join Community**: Share your experiences and get tips

Remember: You're in control. The AI is here to help, not to intrude. Adjust settings to find the right balance for your workflow.

For technical documentation, see the [AI Integration Technical Guide](AI_INTEGRATION_TECHNICAL_GUIDE.md).