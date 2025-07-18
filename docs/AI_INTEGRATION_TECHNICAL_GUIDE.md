# HorizonOS AI Integration Technical Guide

**Version**: 1.0  
**Last Updated**: 2025-01-18  

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Component Details](#component-details)
3. [API Reference](#api-reference)
4. [Development Guide](#development-guide)
5. [Deployment](#deployment)
6. [Performance Tuning](#performance-tuning)
7. [Security Implementation](#security-implementation)
8. [Monitoring and Debugging](#monitoring-and-debugging)

## Architecture Overview

### System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Interface Layer                       │
│  ┌──────────────┐  ┌───────────────┐  ┌────────────────────┐   │
│  │ Graph Desktop│  │ Settings DSL   │  │ Web UI (n8n)     │   │
│  └──────────────┘  └───────────────┘  └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────────────────────────────────────┐
│                         Service Layer                             │
│  ┌──────────────┐  ┌───────────────┐  ┌────────────────────┐   │
│  │ AI Service   │  │ Monitor Service│  │ Agent Service    │   │
│  │ (Main)       │  │ (Events)      │  │ (Coordination)   │   │
│  └──────────────┘  └───────────────┘  └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
                                 │
┌─────────────────────────────────────────────────────────────────┐
│                        Core Components                            │
│  ┌──────────────┐  ┌───────────────┐  ┌────────────────────┐   │
│  │ Ollama       │  │ LangChain     │  │ Privacy Manager  │   │
│  │ Integration  │  │ Framework     │  │                  │   │
│  ├──────────────┤  ├───────────────┤  ├────────────────────┤   │
│  │ Hardware     │  │ Automation    │  │ Storage Layer    │   │
│  │ Detection    │  │ Engine        │  │ (TimescaleDB)    │   │
│  └──────────────┘  └───────────────┘  └────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

```
User Action → Event Monitor → Privacy Filter → Pattern Detection
                                    ↓
                            TimescaleDB Storage
                                    ↓
                        AI Analysis (Ollama/LangChain)
                                    ↓
                        Suggestion/Automation Engine
                                    ↓
                            User Notification/Action
```

## Component Details

### 1. Ollama Integration (`src/desktop/graph-ai/src/ollama.rs`)

The Ollama integration provides local LLM inference with hardware optimization.

**Key Features:**
- Automatic hardware detection and model selection
- Connection pooling for concurrent requests
- Streaming response support
- Performance metrics tracking

**Architecture:**
```rust
pub struct OllamaClient {
    client: Client,                    // HTTP client
    base_url: String,                  // Ollama endpoint
    connection_pool: Arc<RwLock<ConnectionPool>>,
    model_cache: DashMap<String, CachedModel>,
    metrics: Arc<Mutex<PerformanceMetrics>>,
    hardware_optimization: HardwareOptimization,
}
```

**Model Selection Algorithm:**
```rust
match vram_gb {
    vram if vram >= 48 => "llama3:70b",
    vram if vram >= 24 => "llama3:34b",
    vram if vram >= 8 => "llama3:7b",
    _ => match total_ram_gb {
        ram if ram >= 32 => "llama3:13b-cpu",
        _ => "tinyllama:latest"
    }
}
```

### 2. Hardware Detection (`src/desktop/graph-ai/src/hardware.rs`)

Comprehensive hardware profiling for optimal AI performance.

**Detected Components:**
- CPU: Cores, frequency, architecture
- GPU: Vendor, model, VRAM, compute capability
- Memory: Total, available, swap
- Storage: Available space, I/O performance
- Thermal: Temperature monitoring
- Power: Battery status, power profile

**Usage:**
```rust
let profile = HardwareDetector::new().detect_hardware().await?;
let optimal_model = profile.select_optimal_model();
```

### 3. Storage Layer (`src/desktop/graph-ai/src/storage/`)

TimescaleDB-based storage for time-series behavioral data.

**Schema Design:**
```sql
-- Main events table
CREATE TABLE user_actions (
    time TIMESTAMPTZ NOT NULL,
    user_id TEXT NOT NULL,
    action_type TEXT NOT NULL,
    target TEXT NOT NULL,
    context JSONB,
    embedding vector(384),
    PRIMARY KEY (time, user_id, action_type)
);

-- Continuous aggregates for performance
CREATE MATERIALIZED VIEW hourly_action_summary AS
SELECT 
    time_bucket('1 hour', time) AS hour,
    action_type,
    COUNT(*) as count,
    AVG(duration_ms) as avg_duration
FROM user_actions
GROUP BY hour, action_type;
```

### 4. Monitoring System (`src/desktop/graph-ai/src/monitoring/`)

Multi-source event collection with privacy filtering.

**Event Sources:**
- Wayland compositor events
- D-Bus system/session messages
- File system notifications (inotify)
- Browser extension API
- Application-specific APIs

**Privacy Filter Pipeline:**
```rust
event -> PII Detection -> Path Sanitization -> URL Filtering 
      -> Private Mode Check -> User Exclusions -> Storage
```

### 5. Agent Framework (`src/desktop/graph-ai/src/agents/`)

LangChain-based multi-agent system with specialized capabilities.

**Agent Types:**
- **Conversational**: Natural language interaction
- **Automation**: Task automation and RPA
- **Research**: Information gathering and synthesis
- **Code**: Programming assistance
- **System**: System optimization and management

**Agent Coordination:**
```rust
pub struct AgentCoordinator {
    agents: HashMap<String, Box<dyn Agent>>,
    task_queue: Arc<Mutex<PriorityQueue<AgentTask>>>,
    load_balancer: LoadBalancer,
    health_monitor: HealthMonitor,
}
```

### 6. Automation Engine (`src/desktop/graph-ai/src/automation/`)

Multi-modal automation supporting various interaction types.

**Automation Modes:**
- **n8n Workflows**: Visual workflow automation
- **Browser Automation**: Playwright-based web automation
- **UI Automation**: ydotool for Wayland desktop automation
- **Script Execution**: Safe script execution environment

## API Reference

### REST API Endpoints

#### Main AI Service (Port 8090)

```bash
# Generate completion
POST /api/generate
{
    "prompt": "string",
    "model": "string (optional)",
    "stream": "boolean (optional)"
}

# Get suggestions
GET /api/suggestions/current
Response: {
    "suggestions": [{
        "type": "app|file|command",
        "content": "string",
        "confidence": 0.0-1.0,
        "reason": "string"
    }]
}

# Privacy controls
POST /api/privacy/pause
POST /api/privacy/resume
DELETE /api/privacy/data
GET /api/privacy/export
```

#### Monitor Service (Port 8091)

```bash
# Get monitoring status
GET /api/monitor/status

# Configure monitoring
POST /api/monitor/config
{
    "sampleInterval": 5,
    "idleThreshold": 300,
    "excludeApps": ["string"]
}

# Get patterns
GET /api/patterns/recent
GET /api/patterns/frequent
```

#### Agent Service (Port 8092)

```bash
# Create task
POST /api/agents/task
{
    "type": "conversation|automation|research",
    "content": "string",
    "priority": "low|medium|high"
}

# Get task status
GET /api/agents/task/{id}

# List agents
GET /api/agents/list
```

### D-Bus Interface

```xml
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
    "http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node name="/org/horizonos/AI">
    <interface name="org.horizonos.AI">
        <method name="GetSuggestions">
            <arg name="context" type="s" direction="in"/>
            <arg name="suggestions" type="a(ssd)" direction="out"/>
        </method>
        <method name="RecordAction">
            <arg name="action" type="s" direction="in"/>
            <arg name="target" type="s" direction="in"/>
            <arg name="success" type="b" direction="out"/>
        </method>
        <signal name="SuggestionReady">
            <arg name="type" type="s"/>
            <arg name="content" type="s"/>
        </signal>
    </interface>
</node>
```

## Development Guide

### Setting Up Development Environment

1. **Clone Repository**
   ```bash
   git clone https://github.com/horizonos/horizonos.git
   cd horizonos
   ```

2. **Run Setup Script**
   ```bash
   ./scripts/setup-ai-dev.sh
   ```

3. **Start Development Services**
   ```bash
   docker-compose -f docker-compose.yml -f docker-compose.dev.yml up
   ```

### Building Components

```bash
# Build Rust components
cd src/desktop/graph-ai
cargo build --release

# Build Kotlin DSL
cd src/kotlin-config
./gradlew build

# Run tests
cargo test
./gradlew test
```

### Creating Custom Agents

```rust
use horizonos_ai::agents::{Agent, AgentTask, AgentResult};

pub struct CustomAgent {
    name: String,
    capabilities: Vec<String>,
}

#[async_trait]
impl Agent for CustomAgent {
    async fn can_handle(&self, task: &AgentTask) -> bool {
        // Implement capability checking
    }
    
    async fn execute(&self, task: AgentTask) -> Result<AgentResult> {
        // Implement task execution
    }
}
```

### Adding Event Sources

```rust
use horizonos_ai::monitoring::{EventSource, Event};

pub struct CustomEventSource {
    name: String,
}

#[async_trait]
impl EventSource for CustomEventSource {
    async fn start(&self) -> Result<()> {
        // Initialize event source
    }
    
    async fn events(&self) -> impl Stream<Item = Event> {
        // Return event stream
    }
}
```

## Deployment

### Production Deployment

1. **Run Deployment Script**
   ```bash
   sudo ./scripts/deploy-ai-system.sh --production
   ```

2. **Verify Installation**
   ```bash
   systemctl status horizonos-ai.target
   horizonos-ai diagnose
   ```

### Configuration Management

```bash
# Generate config from DSL
cd /etc/horizonos/ai
horizonos-ai-config generate settings.horizonos.kts > config.toml

# Validate configuration
horizonos-ai-config validate config.toml
```

### Service Management

```bash
# Start all AI services
systemctl start horizonos-ai.target

# Stop all AI services
systemctl stop horizonos-ai.target

# View logs
journalctl -u horizonos-ai.service -f
journalctl -u horizonos-ai-monitor.service -f
journalctl -u horizonos-ai-agents.service -f
```

## Performance Tuning

### Memory Optimization

```toml
# In config.toml
[performance]
# Limit total AI memory usage
max_memory = "4GB"

# Configure model memory
[performance.models]
max_loaded_models = 2
model_cache_ttl = 3600

# Optimize database
[performance.database]
connection_pool_size = 10
query_cache_size = "100MB"
```

### GPU Optimization

```toml
[hardware]
# Force GPU usage
prefer_gpu = true

# Set GPU memory limit
gpu_memory_limit = "6GB"

# Enable mixed precision
mixed_precision = true
```

### Monitoring Performance

```bash
# Real-time metrics
horizonos-ai stats --live

# Generate performance report
horizonos-ai benchmark --full > performance-report.txt

# Profile specific component
horizonos-ai profile --component ollama --duration 60
```

## Security Implementation

### Encryption

All sensitive data is encrypted using AES-256-GCM:

```rust
let encrypted = encryption_manager.encrypt(data, EncryptionContext {
    purpose: Purpose::Storage,
    key_id: Some("user-data"),
})?;
```

### Access Control

```rust
// Check permissions before operations
if !permission_manager.check(user, Permission::ReadAIData) {
    return Err(PermissionDenied);
}
```

### Sandboxing

Automation scripts run in isolated environments:

```rust
let sandbox = Sandbox::new()
    .memory_limit("500MB")
    .cpu_limit(0.5)
    .network_access(false)
    .filesystem_access(vec!["/tmp/sandbox"]);

sandbox.execute(script).await?;
```

## Monitoring and Debugging

### Health Checks

```bash
# Component health
curl http://localhost:8090/health
curl http://localhost:8091/health
curl http://localhost:8092/health

# Detailed diagnostics
horizonos-ai diagnose --verbose
```

### Debug Mode

```bash
# Enable debug logging
export RUST_LOG=horizonos_ai=debug
systemctl restart horizonos-ai.service

# Enable trace logging for specific module
export RUST_LOG=horizonos_ai::agents=trace
```

### Performance Monitoring

```bash
# Grafana dashboards
http://localhost:3000

# Default credentials: admin/admin
# Pre-configured dashboards:
# - AI System Overview
# - Model Performance
# - Agent Activity
# - Resource Usage
```

### Troubleshooting Tools

```bash
# Analyze patterns
horizonos-ai analyze patterns --days 7

# Debug specific agent
horizonos-ai debug agent --name code-assistant

# Trace event flow
horizonos-ai trace event --follow

# Export debug bundle
horizonos-ai debug export --output debug-bundle.tar.gz
```

---

## Additional Resources

- [Architecture Decision Records](./adr/)
- [API Documentation](https://docs.horizonos.org/ai/api)
- [Contributing Guide](../CONTRIBUTING.md)
- [Security Policy](../SECURITY.md)

For user-facing documentation, see the [AI Integration User Guide](AI_INTEGRATION_USER_GUIDE.md).