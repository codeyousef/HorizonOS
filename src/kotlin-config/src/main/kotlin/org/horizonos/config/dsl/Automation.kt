package org.horizonos.config.dsl

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.seconds

// ===== Automation DSL =====

@HorizonOSDsl
class AutomationContext {
    private val workflows = mutableListOf<Workflow>()
    private val teachingModes = mutableListOf<TeachingMode>()
    
    fun workflow(name: String, block: WorkflowContext.() -> Unit) {
        val context = WorkflowContext(name).apply(block)
        workflows.add(context.toWorkflow())
    }
    
    fun teaching(name: String, block: TeachingContext.() -> Unit) {
        val context = TeachingContext(name).apply(block)
        teachingModes.add(context.toTeachingMode())
    }
    
    fun toConfig(): AutomationConfig {
        return AutomationConfig(
            workflows = workflows,
            teachingModes = teachingModes
        )
    }
}

@HorizonOSDsl
class WorkflowContext(private val name: String) {
    private var trigger: Trigger? = null
    private var actions: List<Action> = emptyList()
    private var conditions: List<Condition> = emptyList()
    var description: String = ""
    var enabled: Boolean = true
    var priority: Int = 50
    
    fun trigger(block: TriggerContext.() -> Unit) {
        trigger = TriggerContext().apply(block).toTrigger()
    }
    
    fun actions(block: ActionsContext.() -> Unit) {
        actions = ActionsContext().apply(block).actions
    }
    
    fun conditions(block: ConditionsContext.() -> Unit) {
        conditions = ConditionsContext().apply(block).conditions
    }
    
    fun toWorkflow(): Workflow {
        return Workflow(
            name = name,
            description = description,
            enabled = enabled,
            priority = priority,
            trigger = trigger,
            actions = actions,
            conditions = conditions
        )
    }
}

@HorizonOSDsl
class TriggerContext {
    private var triggerType: TriggerType? = null
    private var schedule: Schedule? = null
    private var filePattern: String? = null
    private var directoryPath: String? = null
    private var hotkey: String? = null
    private var systemEvent: SystemEvent? = null
    
    fun time(timeSpec: String, days: Set<DayOfWeek> = setOf()) {
        triggerType = TriggerType.TIME
        schedule = Schedule.Time(timeSpec, days)
    }
    
    fun interval(duration: Duration) {
        triggerType = TriggerType.INTERVAL
        schedule = Schedule.Interval(duration)
    }
    
    fun fileCreated(pattern: String, directory: String) {
        triggerType = TriggerType.FILE_CREATED
        filePattern = pattern
        directoryPath = directory
    }
    
    fun fileModified(pattern: String, directory: String) {
        triggerType = TriggerType.FILE_MODIFIED
        filePattern = pattern
        directoryPath = directory
    }
    
    fun hotkey(keys: String) {
        triggerType = TriggerType.HOTKEY
        hotkey = keys
    }
    
    fun systemEvent(event: SystemEvent) {
        triggerType = TriggerType.SYSTEM_EVENT
        systemEvent = event
    }
    
    fun toTrigger(): Trigger {
        return Trigger(
            type = triggerType ?: TriggerType.MANUAL,
            schedule = schedule,
            filePattern = filePattern,
            directoryPath = directoryPath,
            hotkey = hotkey,
            systemEvent = systemEvent
        )
    }
}

@HorizonOSDsl
class ActionsContext {
    internal val actions = mutableListOf<Action>()
    
    fun delay(duration: Duration) {
        actions.add(Action.Delay(duration))
    }
    
    fun click(selector: String, button: MouseButton = MouseButton.LEFT) {
        actions.add(Action.Click(selector, button))
    }
    
    fun type(text: String) {
        actions.add(Action.Type(text))
    }
    
    fun keyPress(key: String) {
        actions.add(Action.KeyPress(key))
    }
    
    fun browserOpen(url: String) {
        actions.add(Action.BrowserOpen(url))
    }
    
    fun browserNavigate(url: String) {
        actions.add(Action.BrowserNavigate(url))
    }
    
    fun browserWait(condition: BrowserCondition) {
        actions.add(Action.BrowserWait(condition))
    }
    
    fun runCommand(command: String, workingDir: String? = null) {
        actions.add(Action.RunCommand(command, workingDir))
    }
    
    fun runApplication(app: String, args: List<String> = emptyList()) {
        actions.add(Action.RunApplication(app, args))
    }
    
    fun notification(title: String, message: String, urgency: NotificationUrgency = NotificationUrgency.NORMAL) {
        actions.add(Action.Notification(title, message, urgency))
    }
    
    fun aiTask(block: AITaskContext.() -> Unit) {
        val context = AITaskContext().apply(block)
        actions.add(context.toAction())
    }
    
    fun fileOperation(block: FileOperationContext.() -> Unit) {
        val context = FileOperationContext().apply(block)
        actions.add(context.toAction())
    }
    
    fun conditional(condition: String, block: ActionsContext.() -> Unit) {
        val nestedActions = ActionsContext().apply(block).actions
        actions.add(Action.Conditional(condition, nestedActions))
    }
    
    fun loop(times: Int, block: ActionsContext.() -> Unit) {
        val nestedActions = ActionsContext().apply(block).actions
        actions.add(Action.Loop(times, nestedActions))
    }
}

@HorizonOSDsl
class ConditionsContext {
    internal val conditions = mutableListOf<Condition>()
    
    fun timeRange(start: String, end: String) {
        conditions.add(Condition.TimeRange(start, end))
    }
    
    fun dayOfWeek(vararg days: DayOfWeek) {
        conditions.add(Condition.DayOfWeek(days.toSet()))
    }
    
    fun processRunning(processName: String) {
        conditions.add(Condition.ProcessRunning(processName))
    }
    
    fun fileExists(filePath: String) {
        conditions.add(Condition.FileExists(filePath))
    }
    
    fun networkConnected() {
        conditions.add(Condition.NetworkConnected)
    }
    
    fun batteryLevel(min: Int, max: Int = 100) {
        conditions.add(Condition.BatteryLevel(min, max))
    }
    
    fun userIdle(duration: Duration) {
        conditions.add(Condition.UserIdle(duration))
    }
}

@HorizonOSDsl
class TeachingContext(private val name: String) {
    private var watchedPath: String? = null
    private var filePattern: String? = null
    private var learningMode: LearningMode = LearningMode.USER_DEMONSTRATION
    private var actions: List<Action> = emptyList()
    var description: String = ""
    var enabled: Boolean = true
    
    fun watchFolder(path: String) {
        watchedPath = path
    }
    
    fun filePattern(pattern: String) {
        filePattern = pattern
    }
    
    fun learnFrom(mode: LearningMode) {
        learningMode = mode
    }
    
    fun recordedActions(block: ActionsContext.() -> Unit) {
        actions = ActionsContext().apply(block).actions
    }
    
    fun toTeachingMode(): TeachingMode {
        return TeachingMode(
            name = name,
            description = description,
            enabled = enabled,
            watchedPath = watchedPath,
            filePattern = filePattern,
            learningMode = learningMode,
            recordedActions = actions
        )
    }
}

@HorizonOSDsl
class AITaskContext {
    var model: String = "llama3.2:3b"
    var prompt: String = ""
    var systemPrompt: String = ""
    var temperature: Float = 0.7f
    var maxTokens: Int = 1000
    var timeout: Duration = 30.seconds
    var inputSource: InputSource = InputSource.None
    var outputDestination: OutputDestination = OutputDestination.Variable("ai_result")
    
    fun toAction(): Action.AITask {
        return Action.AITask(
            model = model,
            prompt = prompt,
            systemPrompt = systemPrompt,
            temperature = temperature,
            maxTokens = maxTokens,
            timeout = timeout,
            inputSource = inputSource,
            outputDestination = outputDestination
        )
    }
}

@HorizonOSDsl
class FileOperationContext {
    private var operation: FileOperation? = null
    
    fun copy(source: String, destination: String) {
        operation = FileOperation.Copy(source, destination)
    }
    
    fun move(source: String, destination: String) {
        operation = FileOperation.Move(source, destination)
    }
    
    fun delete(path: String) {
        operation = FileOperation.Delete(path)
    }
    
    fun create(path: String, content: String = "") {
        operation = FileOperation.Create(path, content)
    }
    
    fun read(path: String, variable: String) {
        operation = FileOperation.Read(path, variable)
    }
    
    fun write(path: String, content: String, append: Boolean = false) {
        operation = FileOperation.Write(path, content, append)
    }
    
    fun toAction(): Action.FileOperation {
        return Action.FileOperation(operation ?: FileOperation.Create("", ""))
    }
}

// ===== Data Classes =====

@Serializable
data class AutomationConfig(
    val workflows: List<Workflow>,
    val teachingModes: List<TeachingMode>
)

@Serializable
data class Workflow(
    val name: String,
    val description: String,
    val enabled: Boolean,
    val priority: Int,
    val trigger: Trigger?,
    val actions: List<Action>,
    val conditions: List<Condition>
)

@Serializable
data class TeachingMode(
    val name: String,
    val description: String,
    val enabled: Boolean,
    val watchedPath: String?,
    val filePattern: String?,
    val learningMode: LearningMode,
    val recordedActions: List<Action>
)

@Serializable
data class Trigger(
    val type: TriggerType,
    val schedule: Schedule? = null,
    val filePattern: String? = null,
    val directoryPath: String? = null,
    val hotkey: String? = null,
    val systemEvent: SystemEvent? = null
)

@Serializable
sealed class Schedule {
    @Serializable
    data class Time(val timeSpec: String, val days: Set<DayOfWeek>) : Schedule()
    
    @Serializable
    data class Interval(val duration: Duration) : Schedule()
}

@Serializable
sealed class Action {
    @Serializable
    data class Delay(val duration: Duration) : Action()
    
    @Serializable
    data class Click(val selector: String, val button: MouseButton) : Action()
    
    @Serializable
    data class Type(val text: String) : Action()
    
    @Serializable
    data class KeyPress(val key: String) : Action()
    
    @Serializable
    data class BrowserOpen(val url: String) : Action()
    
    @Serializable
    data class BrowserNavigate(val url: String) : Action()
    
    @Serializable
    data class BrowserWait(val condition: BrowserCondition) : Action()
    
    @Serializable
    data class RunCommand(val command: String, val workingDir: String?) : Action()
    
    @Serializable
    data class RunApplication(val app: String, val args: List<String>) : Action()
    
    @Serializable
    data class Notification(val title: String, val message: String, val urgency: NotificationUrgency) : Action()
    
    @Serializable
    data class AITask(
        val model: String,
        val prompt: String,
        val systemPrompt: String,
        val temperature: Float,
        val maxTokens: Int,
        val timeout: Duration,
        val inputSource: InputSource,
        val outputDestination: OutputDestination
    ) : Action()
    
    @Serializable
    data class FileOperation(val operation: org.horizonos.config.dsl.FileOperation) : Action()
    
    @Serializable
    data class Conditional(val condition: String, val actions: List<Action>) : Action()
    
    @Serializable
    data class Loop(val times: Int, val actions: List<Action>) : Action()
}

@Serializable
sealed class Condition {
    @Serializable
    data class TimeRange(val start: String, val end: String) : Condition()
    
    @Serializable
    data class DayOfWeek(val days: Set<org.horizonos.config.dsl.DayOfWeek>) : Condition()
    
    @Serializable
    data class ProcessRunning(val processName: String) : Condition()
    
    @Serializable
    data class FileExists(val filePath: String) : Condition()
    
    @Serializable
    object NetworkConnected : Condition()
    
    @Serializable
    data class BatteryLevel(val min: Int, val max: Int) : Condition()
    
    @Serializable
    data class UserIdle(val duration: Duration) : Condition()
}

@Serializable
sealed class FileOperation {
    @Serializable
    data class Copy(val source: String, val destination: String) : FileOperation()
    
    @Serializable
    data class Move(val source: String, val destination: String) : FileOperation()
    
    @Serializable
    data class Delete(val path: String) : FileOperation()
    
    @Serializable
    data class Create(val path: String, val content: String) : FileOperation()
    
    @Serializable
    data class Read(val path: String, val variable: String) : FileOperation()
    
    @Serializable
    data class Write(val path: String, val content: String, val append: Boolean) : FileOperation()
}

@Serializable
sealed class InputSource {
    @Serializable
    object None : InputSource()
    
    @Serializable
    data class File(val path: String) : InputSource()
    
    @Serializable
    data class Clipboard(val format: String = "text") : InputSource()
    
    @Serializable
    data class Variable(val name: String) : InputSource()
    
    @Serializable
    data class UserInput(val prompt: String) : InputSource()
}

@Serializable
sealed class OutputDestination {
    @Serializable
    data class Variable(val name: String) : OutputDestination()
    
    @Serializable
    data class File(val path: String) : OutputDestination()
    
    @Serializable
    data class Clipboard(val format: String = "text") : OutputDestination()
    
    @Serializable
    data class Notification(val title: String = "AI Task Complete") : OutputDestination()
}

// ===== Enums =====

@Serializable
enum class TriggerType {
    MANUAL, TIME, INTERVAL, FILE_CREATED, FILE_MODIFIED, HOTKEY, SYSTEM_EVENT
}

@Serializable
enum class DayOfWeek {
    MONDAY, TUESDAY, WEDNESDAY, THURSDAY, FRIDAY, SATURDAY, SUNDAY
}

@Serializable
enum class MouseButton {
    LEFT, RIGHT, MIDDLE
}

@Serializable
enum class BrowserCondition {
    PAGE_LOADED, ELEMENT_VISIBLE, ELEMENT_CLICKABLE, URL_CONTAINS, TITLE_CONTAINS
}

@Serializable
enum class NotificationUrgency {
    LOW, NORMAL, HIGH, CRITICAL
}

@Serializable
enum class LearningMode {
    USER_DEMONSTRATION, SCREEN_RECORDING, API_MONITORING, HYBRID
}

@Serializable
enum class SystemEvent {
    STARTUP, SHUTDOWN, SUSPEND, RESUME, NETWORK_CONNECTED, NETWORK_DISCONNECTED,
    USB_CONNECTED, USB_DISCONNECTED, BATTERY_LOW, BATTERY_CRITICAL
}

// ===== Convenience Constants =====

val WEEKDAYS = setOf(DayOfWeek.MONDAY, DayOfWeek.TUESDAY, DayOfWeek.WEDNESDAY, DayOfWeek.THURSDAY, DayOfWeek.FRIDAY)
val WEEKENDS = setOf(DayOfWeek.SATURDAY, DayOfWeek.SUNDAY)
val ALL_DAYS = DayOfWeek.values().toSet()

// ===== Extension Functions =====

// The automation DSL is integrated directly into SystemConfiguration in Core.kt