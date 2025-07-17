# [[HorizonOS]] AI Integration Design Document

## Executive Summary

This document outlines the comprehensive design for HorizonOS's AI integration system, encompassing behavioral learning, local LLM processing, intelligent process automation (RPA), browser automation, AI agents, and various AI-powered services. The system prioritizes privacy, user control, and intelligent assistance while maintaining complete local operation.

## Core Principles

1. **Privacy First**: All AI processing happens locally by default, no data leaves the device
2. **User Control**: Every AI feature can be disabled, configured, or reset
3. **Non-Intrusive**: Suggestions and automations respect user preferences
4. **Transparency**: Users can see what the AI has learned and why it makes decisions
5. **Adaptive**: System learns from both acceptance and rejection of suggestions
6. **Hardware-Aware**: Automatically optimizes for available hardware resources

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    HorizonOS AI System                       │
├─────────────────────────────────────────────────────────────┤
│                    Integration Layer                         │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────────┐ │
│  │ LangChain/  │  │ Playwright/  │  │ n8n/Prefect/     │ │
│  │ LlamaIndex  │  │ Selenium     │  │ Temporal         │ │
│  └─────────────┘  └──────────────┘  └───────────────────┘ │
│                                                             │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────┐│
│  │ Continuous      │  │ Real-time Pattern │  │ Suggestion ││
│  │ Event Monitor   │──▶│ Recognition       │──▶│   Engine   ││
│  └─────────────────┘  └──────────────────┘  └────────────┘│
│                                                             │
│  ┌─────────────────┐  ┌──────────────────┐  ┌────────────┐│
│  │   LLM Runtime   │  │  UI Automation    │  │  Privacy   ││
│  │    (Ollama)     │  │  (ydotool/AT-SPI) │  │  Manager   ││
│  └─────────────────┘  └──────────────────┘  └────────────┘│
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │    Time-Series Database (InfluxDB/TimescaleDB)       │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. Local-First LLM Processing

#### Hardware Detection & Model Selection

```kotlin
data class HardwareProfile(
    val cpu: CPUInfo,
    val gpu: GPUInfo,
    val ram: MemoryInfo,
    val storage: StorageInfo
)

class ModelSelector {
    fun selectOptimalModel(
        task: AITask,
        hardware: HardwareProfile
    ): ModelConfiguration {
        val availableVRAM = hardware.gpu.vramFree
        val availableRAM = hardware.ram.free
        
        return when {
            availableVRAM >= 48_000 -> Model.LARGE_70B
            availableVRAM >= 24_000 -> Model.MEDIUM_34B
            availableVRAM >= 8_000 -> Model.SMALL_7B
            availableRAM >= 32_000 -> Model.CPU_13B
            else -> Model.TINY_3B
        }
    }
}
```

#### Ollama Integration

```yaml
# /etc/systemd/system/ollama.service
[Unit]
Description=Ollama Local LLM Service
After=network.target

[Service]
Type=simple
ExecStart=/usr/bin/ollama serve
Environment="OLLAMA_HOST=127.0.0.1:11434"
Environment="OLLAMA_MODELS=/var/lib/ollama/models"
Restart=always

[Install]
WantedBy=multi-user.target
```

### 2. Behavioral Learning System

#### Browser Extension for Web Monitoring

```typescript
// HorizonOS Browser Extension (WebExtension API)
class HorizonOSWebMonitor {
    private websocket: WebSocket
    
    constructor() {
        // Connect to local monitoring service
        this.websocket = new WebSocket('ws://localhost:8765/browser-events')
    }
    
    // Monitor navigation
    browser.webNavigation.onCompleted.addListener((details) => {
        if (!this.isPrivateMode(details.tabId)) {
            this.sendEvent({
                type: 'WEB_NAVIGATE',
                url: details.url,
                timestamp: Date.now(),
                tabId: details.tabId
            })
        }
    })
    
    // Monitor active tab time
    browser.tabs.onActivated.addListener(async (activeInfo) => {
        const tab = await browser.tabs.get(activeInfo.tabId)
        if (!tab.incognito) {
            this.sendEvent({
                type: 'TAB_FOCUS',
                url: tab.url,
                title: tab.title,
                timestamp: Date.now()
            })
        }
    })
    
    // Respect privacy
    private isPrivateMode(tabId: number): boolean {
        return browser.tabs.get(tabId).then(tab => tab.incognito)
    }
}
```

#### Continuous Event Monitoring

```kotlin
class ContinuousEventMonitor {
    private val eventStream = MutableSharedFlow<UserAction>()
    private val patternDetector = StreamingPatternDetector()
    private var isActive = false
    
    fun start() {
        isActive = true
        
        // Monitor only when user is active
        SystemIdleDetector.onActiveStateChange { active ->
            isActive = active
        }
        
        // Hook into system events
        scope.launch {
            merge(
                WaylandEventMonitor.events(),
                DBusEventMonitor.events(),
                FileSystemWatcher.events(),
                BrowserExtension.events()
            ).collect { event ->
                if (isActive && !isPrivateMode()) {
                    val action = event.toUserAction()
                    eventStream.emit(action)
                    
                    // Real-time pattern detection
                    patternDetector.processAction(action)
                }
            }
        }
    }
}

class StreamingPatternDetector {
    private val timeSeriesDB = InfluxDBClient()
    private val patternCache = SlidingWindowCache(1.hour)
    
    suspend fun processAction(action: UserAction) {
        // Write to time-series database
        timeSeriesDB.write(action)
        
        // Update pattern cache
        patternCache.add(action)
        
        // Detect patterns in real-time
        if (patternCache.hasEnoughData()) {
            val patterns = detectPatterns(patternCache.getWindow())
            patterns.forEach { pattern ->
                if (pattern.confidence > threshold) {
                    PatternRegistry.update(pattern)
                }
            }
        }
    }
}
```

#### Non-Intrusive Suggestions

```kotlin
data class Suggestion(
    val id: String,
    val pattern: Pattern,
    val action: ProposedAction,
    val confidence: Float,
    val displayMode: DisplayMode
)

enum class DisplayMode {
    TOAST,      // Small notification
    BUBBLE,     // Floating assistant
    SIDEBAR,    // Widget in sidebar
    SYSTRAY,    // System tray icon
    NONE        // Logged only
}
```

### 3. Intelligent Process Automation (RPA)

#### Integration with Existing Tools

```kotlin
// Using n8n for workflow orchestration
class N8NIntegration {
    private val n8nAPI = N8NClient("http://localhost:5678")
    
    fun createWorkflowFromTeaching(recordedActions: List<RecordedAction>): N8NWorkflow {
        val nodes = recordedActions.map { action ->
            when (action.type) {
                ActionType.BROWSER -> createBrowserNode(action)
                ActionType.APPLICATION -> createAppNode(action)
                ActionType.FILE_OPERATION -> createFileNode(action)
                else -> createCustomNode(action)
            }
        }
        
        return n8nAPI.createWorkflow(
            name = generateWorkflowName(),
            nodes = nodes,
            connections = inferConnections(nodes)
        )
    }
}

// Using ydotool for UI automation (Wayland-compatible)
class UIAutomation {
    private val ydotool = YdotoolWrapper()
    private val atSpi = ATSPIClient() // Accessibility API
    
    fun recordUserActions(): Flow<RecordedAction> = flow {
        // Use accessibility API for semantic understanding
        atSpi.onUIEvent { event ->
            emit(RecordedAction(
                type = event.type,
                target = event.accessibleName,
                position = event.screenCoordinates,
                screenshot = captureScreen(),
                semanticContext = event.accessibilityTree
            ))
        }
    }
    
    fun replayAction(action: RecordedAction) {
        when (action.type) {
            UIActionType.CLICK -> ydotool.click(action.position)
            UIActionType.TYPE -> ydotool.type(action.text)
            UIActionType.KEY -> ydotool.key(action.keyCombo)
        }
    }
}
```

#### Semantic Understanding

```kotlin
class SemanticAnalyzer {
    fun analyzeIntent(actions: List<RecordedAction>): UserIntent {
        // Use LLM to understand the user's goal
        val prompt = buildPrompt(actions)
        return llmService.analyze(prompt).toUserIntent()
    }
    
    fun generateWorkflow(intent: UserIntent): Workflow {
        return Workflow(
            name = intent.suggestedName,
            description = intent.description,
            triggers = intent.detectTriggers(),
            actions = intent.toExecutableActions()
        )
    }
}
```

### 4. Browser Automation

```kotlin
// Using Playwright for browser automation
class BrowserAutomation {
    private val playwright = Playwright.create()
    
    fun createAutomation(config: BrowserConfig): PlaywrightSession {
        val browser = when (config.browser) {
            BrowserType.CHROMIUM -> playwright.chromium()
            BrowserType.FIREFOX -> playwright.firefox()
            BrowserType.WEBKIT -> playwright.webkit()
        }
        
        return browser.launch(LaunchOptions().apply {
            headless = config.headless
            devtools = config.debug
        }).let { browserInstance ->
            PlaywrightSession(browserInstance, config)
        }
    }
}

// Integration with existing web scraping tools
class WebScrapingService {
    private val playwright = PlaywrightIntegration()
    private val beautifulSoup = PythonBridge("beautifulsoup4")
    private val scrapy = ScrapyIntegration()
    
    suspend fun scrapeWebsite(config: ScrapeConfig): ScrapeResult {
        return when (config.complexity) {
            Complexity.SIMPLE -> beautifulSoup.scrape(config)
            Complexity.DYNAMIC -> playwright.scrape(config)
            Complexity.LARGE_SCALE -> scrapy.scrape(config)
        }
    }
}

// Example automation using Playwright
automation {
    browser("firefox") {
        val page = newPage()
        page.goto("https://example.com")
        page.waitForSelector("#login-form")
        page.fill("#username", "user@example.com")
        page.fill("#password", getSecurePassword())
        page.click("#submit")
        page.waitForNavigation()
        
        val data = page.evaluate("""
            () => {
                return Array.from(document.querySelectorAll('.data-row'))
                    .map(row => ({
                        title: row.querySelector('.title').innerText,
                        value: row.querySelector('.value').innerText
                    }));
            }
        """)
        
        processExtractedData(data)
    }
}
```

### 5. AI Agent Framework

```kotlin
// Using LangChain for agent orchestration
class AIAgentFramework {
    private val langchain = LangChainKotlin()
    private val llamaIndex = LlamaIndexIntegration()
    private val ollama = OllamaService()
    
    fun createAgent(config: AgentConfig): LangChainAgent {
        return langchain.agent {
            llm = ollama.model(config.model)
            
            // Add tools
            tools {
                add(WebSearchTool())
                add(FileSystemTool())
                add(BashTool())
                add(PlaywrightTool())
                add(GraphDatabaseTool())
            }
            
            // Memory
            memory = when (config.memoryType) {
                MemoryType.CONVERSATION -> ConversationBufferMemory()
                MemoryType.SUMMARY -> ConversationSummaryMemory(llm)
                MemoryType.VECTOR -> VectorStoreMemory(llamaIndex)
            }
            
            // Custom prompt
            systemPrompt = config.systemPrompt
        }
    }
}

// Multi-agent orchestration using LangGraph
class MultiAgentOrchestrator {
    private val langGraph = LangGraphBuilder()
    
    fun buildAgentGraph(): AgentGraph {
        return langGraph.build {
            // Define agents
            val codeAgent = agent("code_assistant") {
                model = "codellama:34b"
                tools = listOf(CodeAnalysisTool(), RefactoringTool())
            }
            
            val researchAgent = agent("researcher") {
                model = "llama3.2:7b"
                tools = listOf(WebSearchTool(), ArxivTool(), WikipediaTool())
            }
            
            val plannerAgent = agent("planner") {
                model = "llama3.2:7b"
                role = "Decompose tasks and coordinate other agents"
            }
            
            // Define workflow
            workflow {
                start -> plannerAgent
                plannerAgent -> conditionalRouter {
                    when (taskType) {
                        TaskType.CODE -> codeAgent
                        TaskType.RESEARCH -> researchAgent
                        TaskType.COMPLEX -> parallelExecution(codeAgent, researchAgent)
                    }
                }
                anyAgent -> aggregator -> end
            }
        }
    }
}
```

### 6. AI Services

#### Service Definitions

```kotlin
enum class AIServiceType {
    CODE_ASSISTANT,
    DOCUMENT_SUMMARIZER,
    EMAIL_ASSISTANT,
    BROWSER_ASSISTANT,
    FILE_ORGANIZER,
    MEETING_ASSISTANT,
    RESEARCH_ASSISTANT,
    SECURITY_MONITOR
}

interface AIService {
    val type: AIServiceType
    val name: String
    val description: String
    val model: String
    val enabled: Boolean
    
    suspend fun execute(request: ServiceRequest): ServiceResponse
}
```

#### Example Service Implementation

```kotlin
class EmailAssistant : AIService {
    override val type = AIServiceType.EMAIL_ASSISTANT
    override val name = "Smart Email Assistant"
    override val description = "Filters, summarizes, and drafts email responses"
    override val model = "llama3.2:7b"
    override var enabled = true
    
    suspend fun execute(request: ServiceRequest): ServiceResponse {
        return when (request.action) {
            "summarize" -> summarizeEmails(request.data)
            "draft" -> draftResponse(request.data)
            "filter" -> intelligentFilter(request.data)
            else -> ServiceResponse.error("Unknown action")
        }
    }
}
```

### 7. Automation Creation Methods

#### Visual Workflow Builder (n8n Integration)

```kotlin
class VisualWorkflowBuilder {
    private val n8n = N8NEmbedded()
    
    fun launchBuilder(context: BuilderContext): WorkflowEditor {
        return n8n.createEditor {
            // Custom nodes for HorizonOS
            registerNode("horizonos.ai.query") { config ->
                OllamaQueryNode(config)
            }
            
            registerNode("horizonos.browser.scrape") { config ->
                PlaywrightScrapeNode(config)
            }
            
            registerNode("horizonos.system.command") { config ->
                SystemCommandNode(config)
            }
            
            registerNode("horizonos.graph.query") { config ->
                GraphDatabaseNode(config)
            }
            
            // Enable webhook triggers
            enableWebhooks = true
            
            // Enable cron triggers
            enableCron = true
            
            // Custom UI theme
            theme = HorizonOSTheme()
        }
    }
}

// Alternative: Prefect for Python-based workflows
class PrefectIntegration {
    private val prefect = PrefectClient()
    
    fun createFlow(name: String, description: String): PrefectFlow {
        return prefect.flow(name) {
            // Define tasks
            val fetchData = task("fetch_data") {
                playwright.scrape(url)
            }
            
            val processWithAI = task("process_ai") {
                ollama.process(fetchData.result)
            }
            
            val saveResults = task("save_results") {
                database.save(processWithAI.result)
            }
            
            // Define dependencies
            fetchData >> processWithAI >> saveResults
        }
    }
}

// Alternative: Temporal for complex, long-running workflows
class TemporalIntegration {
    private val temporal = TemporalClient()
    
    @WorkflowInterface
    interface DataProcessingWorkflow {
        @WorkflowMethod
        fun processData(input: DataInput): DataOutput
    }
    
    class DataProcessingWorkflowImpl : DataProcessingWorkflow {
        override fun processData(input: DataInput): DataOutput {
            val activities = Workflow.newActivityStub<DataActivities>()
            
            // Long-running workflow with retries and timeouts
            val scraped = activities.scrapeWebsite(input.url)
            val processed = activities.processWithAI(scraped)
            val validated = activities.validateResults(processed)
            
            return DataOutput(validated)
        }
    }
}
```

#### Text-to-Workflow

```kotlin
class TextToWorkflow {
    suspend fun generate(description: String): Workflow {
        val prompt = """
        Generate a workflow for the following task:
        $description
        
        Output as structured JSON with triggers and actions.
        """
        
        val response = llmService.generate(prompt)
        return WorkflowParser.parse(response)
    }
}

// Example usage
val workflow = textToWorkflow.generate(
    "Every morning at 9 AM, check my emails and summarize important ones"
)
```

### 8. Privacy and Control System

#### Comprehensive Settings

```kotlin
ai {
    // Master AI toggle
    enabled = true
    
    // Behavioral learning controls
    learning {
        enabled = true
        applications = true
        documents = true
        websites = true
        workflows = true
        
        // Data retention
        retentionDays = 30
        minConfidence = 0.7
        minOccurrences = 5
        
        // Learning exclusions
        excludedApps = ["1password", "banking-app"]
        excludedPaths = ["~/private", "~/secure"]
        excludedDomains = ["*.bank.com", "*.health.gov"]
    }
    
    // Suggestion controls
    suggestions {
        enabled = true
        displayMode = DisplayMode.TOAST
        maxPerHour = 3
        quietHours = TimeRange("22:00", "08:00")
        
        // Types of suggestions
        appLaunch = true
        documentOpen = true
        websiteVisit = true
        workflowAutomation = true
        
        interruptionLevel = InterruptionLevel.GENTLE
    }
    
    // LLM configuration
    llm {
        provider = "ollama"
        defaultModel = "llama3.2:7b"
        
        // Hardware optimization
        hardware {
            gpuAcceleration = true
            cpuThreads = 8
            memoryLimit = "16GB"
            optimization = HardwareOptimization.AUTO
        }
    }
    
    // RPA settings
    automation {
        teachingMode = true
        browserAutomation = true
        
        // Security
        requireConfirmation = true
        sandboxed = true
        
        // Sharing
        enableMarketplace = true
        shareAnonymousStats = false
    }
    
    // AI services
    services {
        codeAssistant {
            enabled = true
            model = "codellama:34b"
            contextLines = 100
        }
        
        emailAssistant {
            enabled = true
            model = "llama3.2:7b"
            autoSummarize = true
            draftSuggestions = true
        }
        
        documentSummarizer {
            enabled = true
            model = "llama3.2:7b"
            maxLength = 500
        }
        
        meetingAssistant {
            enabled = false
            model = "whisper:large"
            transcribe = true
            extractActionItems = true
        }
    }
    
    // Privacy controls
    privacy {
        localOnly = true
        telemetryEnabled = false
        dataRetention = DataRetention.SESSION_ONLY
        encryptStorage = true
        
        // Network restrictions
        allowedNetworkAccess = ["localhost", "127.0.0.1"]
        
        // Sensitive data handling
        sensitiveDataFilter = true
        piiDetection = true
    }
}
```

### 9. Settings UI Design

```
AI & Automation Settings
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🤖 AI Features                           [ON]
├─ 📊 Behavioral Learning               [ON]
│  ├─ Learn from apps                   [✓]
│  ├─ Learn from documents              [✓]
│  ├─ Learn from websites               [✓]
│  ├─ Learn from workflows              [✓]
│  ├─ Data retention                 [30 days]
│  └─ [View learned patterns...]
│
├─ 💡 Smart Suggestions                 [ON]
│  ├─ Suggest app launches              [✓]
│  ├─ Suggest documents                 [✓]
│  ├─ Suggest websites                  [✓]
│  ├─ Max suggestions/hour              [3]
│  └─ Quiet hours                   [22-08]
│
├─ 🧠 LLM Processing                    [ON]
│  ├─ Provider                     [Ollama]
│  ├─ Default model          [llama3.2:7b]
│  ├─ GPU acceleration                  [✓]
│  └─ [Hardware settings...]
│
├─ 🎯 Process Automation (RPA)          [ON]
│  ├─ Teaching mode                     [✓]
│  ├─ Browser automation                [✓]
│  ├─ Require confirmation              [✓]
│  └─ [Automation library...]
│
├─ 🛠️ AI Services
│  ├─ Code Assistant                    [✓]
│  ├─ Email Assistant                   [✓]
│  ├─ Document Summarizer               [✓]
│  ├─ Meeting Assistant                 [✗]
│  ├─ File Organizer                    [✓]
│  ├─ Research Assistant                [✓]
│  └─ [Configure services...]
│
└─ 🔒 Privacy & Security
   ├─ Local processing only             [✓]
   ├─ Encrypt AI storage                [✓]
   ├─ Filter sensitive data             [✓]
   └─ [Advanced privacy...]
```

## Implementation Details

### Technology Stack

#### Core Libraries & Tools

```kotlin
// Build configuration
dependencies {
    // LLM & AI Orchestration
    implementation("com.langchain:langchain-kotlin:0.1.0")
    implementation("com.llamaindex:llamaindex-kotlin:0.1.0")
    implementation("ai.ollama:ollama-kotlin:0.1.0")
    
    // Browser Automation
    implementation("com.microsoft.playwright:playwright:1.40.0")
    implementation("org.seleniumhq.selenium:selenium-java:4.16.0")
    
    // Workflow Orchestration
    implementation("io.n8n:n8n-embedded:1.0.0")
    implementation("io.prefect:prefect-kotlin:2.0.0")
    implementation("io.temporal:temporal-sdk:1.20.0")
    
    // UI Automation
    implementation("tools.ydotool:ydotool-kotlin:0.1.0")
    implementation("org.gnome:atspi-kotlin:2.0.0")
    
    // Time-Series Database
    implementation("com.timescale:timescaledb-jdbc:2.0.0")
    implementation("com.influxdb:influxdb-client-kotlin:6.0.0")
    
    // Event Streaming
    implementation("org.apache.kafka:kafka-clients:3.6.0")
    implementation("io.projectreactor:reactor-core:3.5.0")
    
    // Web Scraping
    implementation("com.github.kittinunf.fuel:fuel:2.3.1")
    implementation("org.jsoup:jsoup:1.17.0")
}
```

#### External Tools Integration

```yaml
# Docker Compose for development environment
version: '3.8'
services:
  ollama:
    image: ollama/ollama:latest
    volumes:
      - ollama_models:/root/.ollama
    ports:
      - "11434:11434"
  
  n8n:
    image: n8nio/n8n:latest
    environment:
      - N8N_BASIC_AUTH_ACTIVE=false
      - N8N_EXECUTIONS_PROCESS=main
    volumes:
      - n8n_data:/home/node/.n8n
    ports:
      - "5678:5678"
  
  timescaledb:
    image: timescale/timescaledb:latest-pg15
    environment:
      - POSTGRES_PASSWORD=horizonos
      - POSTGRES_DB=ai_patterns
    volumes:
      - timescale_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"
  
  temporal:
    image: temporalio/auto-setup:latest
    ports:
      - "7233:7233"
    environment:
      - DB=postgresql
      - DB_PORT=5432
      - POSTGRES_USER=temporal
      - POSTGRES_PWD=temporal
```

### Database Schema

```sql
-- Using TimescaleDB (PostgreSQL extension) for time-series data
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Continuous user actions table
CREATE TABLE user_actions (
    time TIMESTAMPTZ NOT NULL,
    action_type TEXT NOT NULL,
    target TEXT NOT NULL,
    context JSONB,
    duration_ms INTEGER,
    user_id INTEGER DEFAULT current_user_id()
);

-- Convert to hypertable for time-series optimization
SELECT create_hypertable('user_actions', 'time');

-- Create indexes for fast queries
CREATE INDEX idx_action_type_time ON user_actions (action_type, time DESC);
CREATE INDEX idx_target_time ON user_actions (target, time DESC);

-- Continuous aggregate for pattern detection
CREATE MATERIALIZED VIEW action_patterns_15min
WITH (timescaledb.continuous) AS
SELECT 
    time_bucket('15 minutes', time) AS bucket,
    action_type,
    target,
    COUNT(*) as occurrence_count,
    AVG(duration_ms) as avg_duration
FROM user_actions
GROUP BY bucket, action_type, target;

-- Retention policy (keep detailed data for 30 days)
SELECT add_retention_policy('user_actions', INTERVAL '30 days');

-- Keep aggregated patterns for 1 year
CREATE TABLE learned_patterns (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pattern_type TEXT NOT NULL,
    pattern_data JSONB NOT NULL,
    confidence REAL NOT NULL,
    first_seen TIMESTAMPTZ NOT NULL,
    last_seen TIMESTAMPTZ NOT NULL,
    occurrence_count INTEGER NOT NULL,
    user_feedback JSONB,
    enabled BOOLEAN DEFAULT true
);

-- Workflow storage remains in PostgreSQL
CREATE TABLE workflows (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT,
    workflow_definition JSONB NOT NULL, -- n8n/Prefect/Temporal format
    created_at TIMESTAMPTZ DEFAULT NOW(),
    modified_at TIMESTAMPTZ DEFAULT NOW(),
    execution_count INTEGER DEFAULT 0,
    enabled BOOLEAN DEFAULT true
);
```

### System Services

```yaml
# horizon-ai-monitor.service
[Unit]
Description=HorizonOS AI Continuous Monitor
After=graphical.target

[Service]
Type=simple
ExecStart=/usr/bin/horizon-ai-monitor
Restart=always
Nice=10
MemoryLimit=100M
CPUQuota=20%
# Only runs when user is logged in
Requisite=graphical-session.target

# horizon-ai-pattern-detector.service
[Unit]
Description=HorizonOS AI Real-time Pattern Detection
After=horizon-ai-monitor.service
Requires=horizon-ai-monitor.service

[Service]
Type=simple
ExecStart=/usr/bin/horizon-ai-pattern-detector
Restart=always
Nice=15
MemoryLimit=200M

# horizon-ai-suggester.service
[Unit]
Description=HorizonOS AI Suggestion Engine
After=horizon-ai-pattern-detector.service

[Service]
Type=simple
ExecStart=/usr/bin/horizon-ai-suggester
Restart=always
Nice=19
MemoryLimit=50M
```

## Performance Optimization

### Resource Management

```kotlin
class AIResourceManager {
    private val cpuThreshold = 80 // percent
    private val memoryThreshold = 85 // percent
    private val batteryThreshold = 20 // percent
    
    fun shouldPauseMonitoring(): Boolean {
        return when {
            !userIsActive() -> true // Don't monitor when user is away
            onBattery() && batteryLevel() < batteryThreshold -> true
            cpuUsage() > cpuThreshold -> true
            memoryUsage() > memoryThreshold -> true
            else -> false
        }
    }
    
    fun optimizeForCurrentState() {
        when {
            onBattery() && batteryLevel() < 50 -> {
                // Reduce AI activity
                setMonitoringSampleRate(SampleRate.LOW) // 1Hz instead of 10Hz
                disableNonEssentialAgents()
                reduceLLMBatchSize()
            }
            cpuUsage() > 90 -> {
                // Throttle monitoring
                pauseNonCriticalStreams()
                increaseAggregationWindow()
            }
            else -> {
                // Normal operation
                setMonitoringSampleRate(SampleRate.NORMAL) // 10Hz
                enableAllFeatures()
            }
        }
    }
}

class EfficientEventStreaming {
    private val eventBuffer = CircularBuffer<UserAction>(1000)
    private val batchProcessor = BatchProcessor(
        batchSize = 100,
        maxLatency = 1.second
    )
    
    suspend fun processEventStream(events: Flow<UserAction>) {
        events
            .buffer(Channel.CONFLATED) // Drop events if processing can't keep up
            .batch(100, 1.second) // Batch for efficiency
            .collect { batch ->
                // Write to TimescaleDB in batches
                timeSeriesDB.writeBatch(batch)
                
                // Process patterns asynchronously
                scope.launch {
                    patternDetector.processBatch(batch)
                }
            }
    }
}
```

### Caching Strategy

```kotlin
class AICache {
    private val patternCache = LRUCache<String, Pattern>(1000)
    private val suggestionCache = TTLCache<String, Suggestion>(
        maxSize = 100,
        ttl = Duration.minutes(30)
    )
    private val llmCache = SemanticCache(
        maxSize = 10_000,
        similarity = 0.95
    )
}
```

## Security Considerations

### Sandboxing

```kotlin
class AutomationSandbox {
    fun executeWorkflow(workflow: Workflow) {
        val sandbox = createSandbox(
            allowedPaths = workflow.declaredPaths,
            allowedApps = workflow.declaredApps,
            networkAccess = workflow.requiresNetwork,
            maxDuration = workflow.timeout
        )
        
        sandbox.execute {
            workflow.run()
        }
    }
}
```

### Encryption

```kotlin
class AIDataEncryption {
    private val key = deriveKeyFromUserPassword()
    
    fun encryptPattern(pattern: Pattern): EncryptedData {
        return AES256.encrypt(
            data = pattern.toByteArray(),
            key = key,
            mode = GCM
        )
    }
}
```

## Multi-Agent Coordination

```kotlin
class AgentOrchestrator {
    private val agents = mutableListOf<AIAgent>()
    
    suspend fun processComplexTask(task: ComplexTask): TaskResult {
        // Decompose task
        val subtasks = TaskDecomposer.decompose(task)
        
        // Assign to capable agents
        val assignments = subtasks.map { subtask ->
            val agent = agents.find { it.canHandle(subtask) }
                ?: throw NoCapableAgentException(subtask)
            subtask to agent
        }
        
        // Execute in parallel where possible
        val results = coroutineScope {
            assignments.map { (subtask, agent) ->
                async {
                    agent.process(
                        input = subtask.toAgentInput(),
                        context = buildContext()
                    )
                }
            }.awaitAll()
        }
        
        // Combine results
        return ResultCombiner.combine(results)
    }
}
```

## Testing Strategy

### Unit Tests

- Pattern detection algorithms
- Model selection logic
- Privacy filter effectiveness
- Automation sandbox security

### Integration Tests

- Service communication
- Database operations
- LLM integration
- UI notification system

### End-to-End Tests

- Complete automation workflows
- Learning and suggestion cycle
- Multi-agent task processing
- Privacy control enforcement

### User Testing

- Suggestion timing effectiveness
- UI/UX of different display modes
- Privacy concern validation
- Performance impact assessment

## Future Enhancements

### Phase 1 (6 months)

- Voice command integration
- Gesture-based automation triggers
- Enhanced semantic understanding
- Workflow marketplace

### Phase 2 (12 months)

- Distributed agent processing
- Federated learning (privacy-preserving)
- AR/VR automation interfaces
- Predictive system optimization

### Phase 3 (18+ months)

- Quantum-resistant encryption
- Neural interface support
- Autonomous system management
- AGI integration preparation

---

## Claude Code Implementation Prompt

```
I need you to implement the comprehensive HorizonOS AI integration system using existing tools and libraries. The implementation should include:

1. **Core AI Infrastructure**:
   - Ollama integration service with hardware detection
   - Model selection algorithm based on available VRAM/RAM
   - System-wide AI API using D-Bus
   - LangChain/LlamaIndex for agent orchestration

2. **Continuous Behavioral Learning**:
   - Real-time event monitoring (only when user is active)
   - TimescaleDB for efficient time-series storage
   - Streaming pattern detection using Kafka/Reactor
   - Event collection from Wayland, D-Bus, filesystem, and browser extension
   - Non-intrusive toast notifications using existing notification daemon

3. **RPA Teaching Mode with n8n**:
   - ydotool for Wayland-compatible UI automation
   - AT-SPI for semantic UI understanding
   - n8n embedded for visual workflow creation
   - Integration with Playwright for browser automation
   - Export to n8n, Prefect, or Temporal formats

4. **Browser Automation with Playwright**:
   - Playwright integration for cross-browser support
   - Selenium as fallback option
   - Beautiful Soup for simple scraping
   - Scrapy for large-scale scraping
   - Cookie and session persistence

5. **AI Agent Framework with LangChain**:
   - LangChain agents with custom tools
   - LangGraph for multi-agent orchestration
   - LlamaIndex for knowledge management
   - Vector stores for long-term memory
   - Integration with graph desktop data

6. **AI Services Implementation**:
   - Each service as a LangChain agent
   - Code Assistant using CodeLlama
   - Email Assistant with IMAP integration
   - Document Summarizer supporting multiple formats
   - Meeting Assistant with Whisper integration
   - File Organizer with semantic understanding
   - Research Assistant using web tools
   - Security Monitor analyzing system logs

7. **Workflow Creation Tools**:
   - n8n embedded editor for visual workflows
   - Prefect for Python-based workflows
   - Temporal for long-running workflows
   - Text-to-workflow using LLM
   - Import/export in multiple formats

8. **Privacy & Settings System**:
   - Comprehensive Kotlin DSL configuration
   - GUI settings using existing KDE/GNOME frameworks
   - Per-pattern privacy controls in TimescaleDB
   - Encrypted storage using system keyring
   - Network isolation via firewall rules

9. **Performance Optimization**:
   - Pause monitoring when user is idle
   - Adaptive sampling rates based on resources
   - Batch processing for database writes
   - Conflated channels for backpressure handling
   - Time-series aggregations for efficient queries

10. **Integration Points**:
    - D-Bus service for system-wide access
    - Browser extension for web monitoring
    - Wayland protocol extension for UI events
    - systemd services with proper dependencies
    - Docker Compose for development setup

Key implementation requirements:
- Use TimescaleDB instead of SQLite for efficient time-series data
- Continuous monitoring only when user is active (not periodic)
- Leverage n8n, Playwright, LangChain instead of custom implementations
- All services communicate via D-Bus
- Respect system idle state and battery levels
- Maximum 100MB RAM for monitoring service

Start with setting up the Docker Compose environment, then implement the Kotlin DSL extensions, create the continuous monitoring service, and finally integrate all the existing tools and libraries.
```