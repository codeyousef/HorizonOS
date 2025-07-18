# HorizonOS AI Integration

Welcome to the HorizonOS AI Integration system! This directory contains the implementation of our privacy-first, local AI system that brings intelligent automation and assistance to HorizonOS.

## 🚀 Quick Start

### For Users

1. **Enable AI Features**
   ```bash
   sudo systemctl enable --now horizonos-ai.target
   ```

2. **Check Status**
   ```bash
   horizonos-ai status
   ```

3. **Configure Settings**
   Edit `/etc/horizonos/ai/settings.horizonos.kts` or use the Settings app

### For Developers

1. **Setup Development Environment**
   ```bash
   ./scripts/setup-ai-dev.sh
   ```

2. **Run Tests**
   ```bash
   cd src/desktop/graph-ai && cargo test
   ```

3. **Access Services**
   - Ollama API: http://localhost:11434
   - n8n Workflows: http://localhost:5678
   - Grafana Metrics: http://localhost:3000

## 📁 Project Structure

```
horizonos/
├── docs/
│   ├── AI Integration Design Document.md    # Original design specification
│   ├── AI_INTEGRATION_IMPLEMENTATION_PLAN.md # Development roadmap
│   ├── AI_INTEGRATION_USER_GUIDE.md         # End-user documentation
│   ├── AI_INTEGRATION_TECHNICAL_GUIDE.md    # Developer documentation
│   └── AI_INTEGRATION_API_REFERENCE.md      # API documentation
│
├── src/
│   ├── desktop/graph-ai/                    # Core AI implementation (Rust)
│   │   ├── src/
│   │   │   ├── agents/                      # AI agent framework
│   │   │   ├── automation/                  # RPA and automation
│   │   │   ├── monitoring/                  # Event monitoring
│   │   │   ├── patterns/                    # Pattern detection
│   │   │   ├── privacy/                     # Privacy controls
│   │   │   ├── storage/                     # Data persistence
│   │   │   ├── suggestions/                 # Suggestion engine
│   │   │   ├── hardware.rs                  # Hardware detection
│   │   │   ├── ollama.rs                    # LLM integration
│   │   │   └── lib.rs                       # Main library
│   │   └── Cargo.toml
│   │
│   └── kotlin-config/                       # Configuration DSL
│       └── src/main/kotlin/org/horizonos/
│           └── config/dsl/ai/               # AI settings DSL
│
├── scripts/
│   ├── setup-ai-dev.sh                      # Development setup
│   ├── deploy-ai-system.sh                  # Production deployment
│   ├── sql/
│   │   └── init-timescaledb.sql             # Database schema
│   └── systemd/                             # Service definitions
│       ├── horizonos-ai.service
│       ├── horizonos-ai-monitor.service
│       ├── horizonos-ai-agents.service
│       └── horizonos-ai.target
│
├── tests/
│   ├── integration/                         # Integration tests
│   └── benchmarks/                          # Performance tests
│
└── docker-compose.yml                       # Container orchestration
```

## 🎯 Key Features

### 1. **Privacy-First Design**
- All AI processing happens locally
- No data leaves your machine
- Full control over what's monitored
- Encrypted storage with user-controlled keys

### 2. **Intelligent Automation**
- Learn from your workflows
- Suggest optimizations
- Automate repetitive tasks
- Visual workflow builder (n8n)

### 3. **Hardware Optimization**
- Automatic GPU detection
- Adaptive model selection
- Resource-aware processing
- Battery-conscious operation

### 4. **Multi-Agent System**
- Specialized AI assistants
- Task decomposition
- Parallel processing
- Context-aware responses

## 🛠️ Technical Stack

- **Language Models**: Ollama (local LLM runtime)
- **AI Framework**: LangChain for agent orchestration
- **Time-Series DB**: TimescaleDB for behavioral data
- **Automation**: n8n workflows, Playwright, ydotool
- **Languages**: Rust (core), Kotlin (config DSL)
- **Container**: Docker Compose for services

## 📊 Architecture Overview

```
User Input → Privacy Filter → Event Monitor → Pattern Detection
                                    ↓
                            TimescaleDB Storage
                                    ↓
                        AI Analysis (Ollama/LangChain)
                                    ↓
                        Suggestion/Automation Engine
                                    ↓
                            User Action/Notification
```

## 🔧 Configuration

The AI system uses Kotlin DSL for type-safe configuration:

```kotlin
aiSettings {
    enabled = true
    
    hardware {
        optimization = HardwareOptimization.AUTO
        preferGPU = true
    }
    
    privacy {
        localOnly = true
        encryptStorage = true
        dataRetention = 30.days
    }
    
    learning {
        enabled = true
        excludeApplications = listOf("keepassxc", "signal-desktop")
    }
}
```

## 📡 API Access

### REST API
```bash
# Generate completion
curl -X POST http://localhost:8090/api/generate \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Hello", "model": "llama3.2"}'

# Get suggestions  
curl http://localhost:8090/api/suggestions/current
```

### D-Bus Integration
```python
import dbus
bus = dbus.SessionBus()
ai = bus.get_object('org.horizonos.AI', '/org/horizonos/AI')
suggestions = ai.GetSuggestions("current")
```

## 🧪 Testing

```bash
# Unit tests
cargo test

# Integration tests
cargo test --features integration-tests

# Performance benchmarks
cargo bench

# Run specific test
cargo test test_ollama_client
```

## 🚀 Deployment

### Development
```bash
./scripts/setup-ai-dev.sh
```

### Production
```bash
sudo ./scripts/deploy-ai-system.sh --production
```

## 📈 Performance

- **Memory**: ~100MB for monitoring service
- **CPU**: <5% during normal operation
- **Response Time**: <500ms for suggestions
- **Model Loading**: 2-10s depending on size

## 🔒 Security

- Local-only processing by default
- Encrypted storage (AES-256-GCM)
- Sandboxed automation execution
- Granular permission system
- Complete audit logging

## 🤝 Contributing

1. Read the [Design Document](docs/AI%20Integration%20Design%20Document.md)
2. Check the [Implementation Plan](docs/AI_INTEGRATION_IMPLEMENTATION_PLAN.md)
3. Follow the [Technical Guide](docs/AI_INTEGRATION_TECHNICAL_GUIDE.md)
4. Submit PRs with tests and documentation

## 📚 Documentation

- **Users**: [AI Integration User Guide](docs/AI_INTEGRATION_USER_GUIDE.md)
- **Developers**: [AI Integration Technical Guide](docs/AI_INTEGRATION_TECHNICAL_GUIDE.md)
- **API**: [AI Integration API Reference](docs/AI_INTEGRATION_API_REFERENCE.md)

## 🐛 Troubleshooting

```bash
# Check service status
systemctl status horizonos-ai.target

# View logs
journalctl -u horizonos-ai.service -f

# Run diagnostics
horizonos-ai diagnose --verbose

# Get help
horizonos-ai help
```

## 📜 License

This project is part of HorizonOS and follows the same license terms.

---

Built with ❤️ for privacy, powered by local AI 🤖