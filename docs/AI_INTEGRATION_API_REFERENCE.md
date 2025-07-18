# HorizonOS AI Integration API Reference

**Version**: 1.0  
**Last Updated**: 2025-01-18  

## Overview

The HorizonOS AI Integration provides several APIs for interacting with the AI system:

1. **REST API** - HTTP-based API for all services
2. **D-Bus API** - System integration for desktop components  
3. **Rust API** - Native library interface for Rust applications
4. **WebSocket API** - Real-time event streaming

## REST API

### Base URLs

- Main AI Service: `http://localhost:8090`
- Monitor Service: `http://localhost:8091`
- Agent Service: `http://localhost:8092`

### Authentication

Local APIs don't require authentication by default. For multi-user systems, enable token authentication:

```bash
# Generate API token
horizonos-ai auth generate-token --name "my-app"

# Use token in requests
Authorization: Bearer <token>
```

### Main AI Service API

#### Generate Completion

Generate text using the AI model.

**Endpoint:** `POST /api/generate`

**Request:**
```json
{
    "prompt": "Explain quantum computing",
    "model": "llama3.2:latest",
    "options": {
        "temperature": 0.7,
        "max_tokens": 500,
        "stream": false
    },
    "context": {
        "session_id": "uuid-string",
        "user_context": {}
    }
}
```

**Response:**
```json
{
    "response": "Quantum computing is...",
    "model": "llama3.2:latest",
    "created_at": "2025-01-18T10:30:00Z",
    "eval_count": 150,
    "eval_duration": 2500000000,
    "total_duration": 3000000000
}
```

**Streaming Response:**
```
data: {"response": "Quantum", "done": false}
data: {"response": " computing", "done": false}
data: {"response": " is", "done": false}
data: {"done": true, "total_duration": 3000000000}
```

#### Get Current Suggestions

Get AI-generated suggestions based on current context.

**Endpoint:** `GET /api/suggestions/current`

**Query Parameters:**
- `type` - Filter by suggestion type (app, file, command, workflow)
- `limit` - Maximum number of suggestions (default: 10)
- `min_confidence` - Minimum confidence threshold (0.0-1.0)

**Response:**
```json
{
    "suggestions": [
        {
            "id": "sugg_123",
            "type": "app",
            "content": "firefox",
            "display_name": "Firefox Web Browser",
            "confidence": 0.92,
            "reason": "Usually opened at this time",
            "metadata": {
                "icon": "firefox",
                "launch_count": 156,
                "last_used": "2025-01-18T09:45:00Z"
            }
        }
    ],
    "context": {
        "time_of_day": "morning",
        "active_app": "terminal",
        "idle_time": 30
    }
}
```

#### Record User Action

Record a user action for learning.

**Endpoint:** `POST /api/actions/record`

**Request:**
```json
{
    "action_type": "app_launch",
    "target": "vscode",
    "context": {
        "triggered_by": "suggestion",
        "suggestion_id": "sugg_123"
    },
    "timestamp": "2025-01-18T10:30:00Z"
}
```

#### Privacy Controls

**Pause Learning:** `POST /api/privacy/pause`

**Resume Learning:** `POST /api/privacy/resume`

**Clear Data:** `DELETE /api/privacy/data`
```json
{
    "scope": "all|recent|pattern",
    "before_date": "2025-01-01T00:00:00Z"
}
```

**Export Data:** `GET /api/privacy/export`
```json
{
    "format": "json|csv",
    "include": ["actions", "patterns", "suggestions"]
}
```

### Monitor Service API

#### Get Monitoring Status

**Endpoint:** `GET /api/monitor/status`

**Response:**
```json
{
    "enabled": true,
    "uptime": 3600,
    "events_collected": 1523,
    "events_filtered": 203,
    "active_sources": [
        "wayland",
        "dbus",
        "filesystem"
    ],
    "resource_usage": {
        "cpu_percent": 1.2,
        "memory_mb": 84,
        "events_per_second": 2.3
    }
}
```

#### Configure Monitoring

**Endpoint:** `POST /api/monitor/config`

**Request:**
```json
{
    "sample_interval": 5,
    "idle_threshold": 300,
    "event_sources": {
        "wayland": true,
        "dbus": true,
        "filesystem": true,
        "browser": false
    },
    "exclude_applications": [
        "keepassxc",
        "signal-desktop"
    ],
    "privacy_filters": {
        "filter_passwords": true,
        "filter_private_browsing": true,
        "anonymize_paths": true
    }
}
```

#### Get Detected Patterns

**Endpoint:** `GET /api/patterns/detected`

**Query Parameters:**
- `type` - Pattern type (temporal, sequence, contextual)
- `min_confidence` - Minimum confidence (0.0-1.0)
- `days` - Look back N days (default: 7)

**Response:**
```json
{
    "patterns": [
        {
            "id": "pat_789",
            "type": "temporal",
            "description": "Opens email client every morning at 9 AM",
            "confidence": 0.87,
            "occurrences": 23,
            "last_seen": "2025-01-18T09:00:00Z",
            "actions": [
                {"action": "app_launch", "target": "thunderbird"}
            ]
        }
    ]
}
```

### Agent Service API

#### Create Agent Task

**Endpoint:** `POST /api/agents/task`

**Request:**
```json
{
    "type": "automation",
    "priority": "high",
    "content": "Create a daily backup of my documents",
    "requirements": {
        "capabilities": ["filesystem", "scheduling"],
        "max_duration": 300
    },
    "metadata": {
        "user_description": "Backup important files daily"
    }
}
```

**Response:**
```json
{
    "task_id": "task_456",
    "status": "assigned",
    "assigned_agent": "automation-agent-1",
    "estimated_duration": 45,
    "created_at": "2025-01-18T10:30:00Z"
}
```

#### Get Task Status

**Endpoint:** `GET /api/agents/task/{task_id}`

**Response:**
```json
{
    "task_id": "task_456",
    "status": "in_progress",
    "progress": 0.6,
    "assigned_agent": "automation-agent-1",
    "started_at": "2025-01-18T10:31:00Z",
    "subtasks": [
        {
            "id": "sub_1",
            "description": "Identify files to backup",
            "status": "completed"
        },
        {
            "id": "sub_2", 
            "description": "Create backup script",
            "status": "in_progress"
        }
    ],
    "logs": [
        {
            "timestamp": "2025-01-18T10:31:00Z",
            "level": "info",
            "message": "Started task execution"
        }
    ]
}
```

#### List Available Agents

**Endpoint:** `GET /api/agents/list`

**Response:**
```json
{
    "agents": [
        {
            "id": "conv-agent-1",
            "type": "conversational",
            "status": "idle",
            "capabilities": ["chat", "qa", "summarization"],
            "model": "llama3.2:latest",
            "load": 0.0
        },
        {
            "id": "automation-agent-1",
            "type": "automation",
            "status": "busy",
            "capabilities": ["workflow", "scripting", "scheduling"],
            "current_tasks": 2,
            "load": 0.7
        }
    ]
}
```

## D-Bus API

### Service Name
`org.horizonos.AI`

### Object Paths
- `/org/horizonos/AI` - Main AI service
- `/org/horizonos/AI/Monitor` - Monitoring service
- `/org/horizonos/AI/Agents` - Agent coordinator

### Main AI Interface

```xml
<interface name="org.horizonos.AI">
    <!-- Get suggestions for current context -->
    <method name="GetSuggestions">
        <arg name="context" type="s" direction="in"/>
        <arg name="suggestions" type="a(ssd)" direction="out"/>
        <!-- Returns array of (type, content, confidence) -->
    </method>
    
    <!-- Record user action -->
    <method name="RecordAction">
        <arg name="action_type" type="s" direction="in"/>
        <arg name="target" type="s" direction="in"/>
        <arg name="context" type="a{sv}" direction="in"/>
        <arg name="success" type="b" direction="out"/>
    </method>
    
    <!-- Generate text completion -->
    <method name="Generate">
        <arg name="prompt" type="s" direction="in"/>
        <arg name="options" type="a{sv}" direction="in"/>
        <arg name="response" type="s" direction="out"/>
    </method>
    
    <!-- Signals -->
    <signal name="SuggestionReady">
        <arg name="type" type="s"/>
        <arg name="content" type="s"/>
        <arg name="confidence" type="d"/>
    </signal>
    
    <signal name="LearningPaused">
        <arg name="reason" type="s"/>
    </signal>
    
    <signal name="LearningResumed"/>
</interface>
```

### Monitor Interface

```xml
<interface name="org.horizonos.AI.Monitor">
    <!-- Control monitoring -->
    <method name="SetEnabled">
        <arg name="enabled" type="b" direction="in"/>
    </method>
    
    <!-- Get monitoring statistics -->
    <method name="GetStats">
        <arg name="stats" type="a{sv}" direction="out"/>
    </method>
    
    <!-- Configure exclusions -->
    <method name="AddExclusion">
        <arg name="app_name" type="s" direction="in"/>
    </method>
    
    <!-- Signals -->
    <signal name="PatternDetected">
        <arg name="pattern_type" type="s"/>
        <arg name="confidence" type="d"/>
        <arg name="description" type="s"/>
    </signal>
</interface>
```

### Example D-Bus Usage

```python
import dbus

# Connect to session bus
bus = dbus.SessionBus()

# Get AI service
ai_service = bus.get_object('org.horizonos.AI', '/org/horizonos/AI')
ai_interface = dbus.Interface(ai_service, 'org.horizonos.AI')

# Get suggestions
suggestions = ai_interface.GetSuggestions("current")
for suggestion_type, content, confidence in suggestions:
    print(f"{suggestion_type}: {content} (confidence: {confidence})")

# Record an action
success = ai_interface.RecordAction(
    "app_launch",
    "firefox",
    {"triggered_by": "manual"}
)
```

## WebSocket API

### Connection

Connect to real-time event stream:

```javascript
const ws = new WebSocket('ws://localhost:8090/api/events');

ws.onopen = () => {
    // Subscribe to event types
    ws.send(JSON.stringify({
        type: 'subscribe',
        events: ['suggestions', 'patterns', 'agent_updates']
    }));
};

ws.onmessage = (event) => {
    const data = JSON.parse(event.data);
    console.log('Event:', data);
};
```

### Event Types

#### Suggestion Event
```json
{
    "type": "suggestion",
    "data": {
        "id": "sugg_999",
        "suggestion_type": "workflow",
        "content": "Run daily standup workflow",
        "confidence": 0.89,
        "trigger": "time_based"
    }
}
```

#### Pattern Detected Event
```json
{
    "type": "pattern_detected",
    "data": {
        "pattern_id": "pat_555",
        "pattern_type": "sequence",
        "description": "Always checks email after opening browser",
        "confidence": 0.92
    }
}
```

#### Agent Update Event
```json
{
    "type": "agent_update",
    "data": {
        "task_id": "task_777",
        "agent_id": "auto-agent-2",
        "status": "completed",
        "result": {
            "success": true,
            "output": "Workflow created successfully"
        }
    }
}
```

## Rust API

### Core Types

```rust
use horizonos_ai::{AIClient, Suggestion, Pattern, Agent};

// Initialize client
let client = AIClient::new()?;

// Get suggestions
let suggestions: Vec<Suggestion> = client.get_suggestions(
    SuggestionContext::current()
).await?;

// Generate completion
let response = client.generate(
    "Explain rust ownership",
    GenerateOptions::default()
).await?;

// Record action
client.record_action(
    ActionType::AppLaunch,
    "vscode",
    ActionContext::default()
).await?;
```

### Agent Framework

```rust
use horizonos_ai::agents::{AgentBuilder, AgentType, Task};

// Create custom agent
let agent = AgentBuilder::new()
    .agent_type(AgentType::Custom)
    .name("my-assistant")
    .capabilities(vec!["code_analysis", "refactoring"])
    .model("codellama:7b")
    .build()?;

// Submit task
let task = Task::new()
    .content("Refactor this function for better performance")
    .code_context(source_code)
    .priority(Priority::High);

let result = agent.execute(task).await?;
```

### Event Monitoring

```rust
use horizonos_ai::monitoring::{EventMonitor, EventFilter};

let monitor = EventMonitor::new()
    .add_filter(EventFilter::exclude_app("keepassxc"))
    .add_filter(EventFilter::private_browsing())
    .sample_interval(Duration::from_secs(5));

// Subscribe to events
let mut events = monitor.subscribe().await?;

while let Some(event) = events.next().await {
    match event {
        Event::AppLaunch { app, .. } => {
            println!("App launched: {}", app);
        }
        Event::FileAccess { path, .. } => {
            println!("File accessed: {:?}", path);
        }
        _ => {}
    }
}
```

## Error Handling

All APIs use consistent error codes:

| Code | Name | Description |
|------|------|-------------|
| 400 | Bad Request | Invalid parameters |
| 401 | Unauthorized | Missing or invalid auth token |
| 404 | Not Found | Resource not found |
| 429 | Too Many Requests | Rate limit exceeded |
| 500 | Internal Error | Server error |
| 503 | Service Unavailable | Service temporarily down |

### Error Response Format

```json
{
    "error": {
        "code": 400,
        "type": "validation_error",
        "message": "Invalid model specified",
        "details": {
            "field": "model",
            "value": "invalid-model",
            "allowed": ["llama3.2", "codellama", "mistral"]
        }
    }
}
```

## Rate Limiting

Default rate limits:
- Generate API: 60 requests/minute
- Suggestions API: 120 requests/minute  
- Record Action: 600 requests/minute
- WebSocket: 1000 messages/minute

Configure in `/etc/horizonos/ai/config.toml`:
```toml
[api.rate_limits]
generate = 60
suggestions = 120
record_action = 600
websocket = 1000
```

## Best Practices

1. **Use Streaming** for long generations to improve UX
2. **Batch Operations** when recording multiple actions
3. **Cache Suggestions** client-side for 30-60 seconds
4. **Handle Errors** gracefully with exponential backoff
5. **Monitor Rate Limits** via `X-RateLimit-*` headers
6. **Use WebSockets** for real-time updates instead of polling
7. **Validate Input** before sending to avoid errors
8. **Set Timeouts** appropriate for your use case

---

For more details, see:
- [Technical Guide](AI_INTEGRATION_TECHNICAL_GUIDE.md)
- [User Guide](AI_INTEGRATION_USER_GUIDE.md)
- [OpenAPI Spec](../api/openapi.yaml)