package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

// ===== Shell and Environment Configuration DSL =====

@HorizonOSDsl
class EnvironmentContext {
    internal val shells = mutableListOf<ShellConfiguration>()
    internal val envVars = mutableMapOf<String, EnvironmentVariable>()
    internal val pathEntries = mutableListOf<PathEntry>()
    internal val dotfiles = mutableListOf<DotfileConfiguration>()
    internal val terminalConfigs = mutableListOf<TerminalConfiguration>()
    internal val promptConfigs = mutableListOf<PromptConfiguration>()

    fun shell(type: ShellType, block: ShellConfigurationContext.() -> Unit) {
        shells.add(ShellConfigurationContext(type).apply(block).toConfiguration())
    }

    fun environmentVariable(name: String, block: EnvironmentVariableContext.() -> Unit) {
        envVars[name] = EnvironmentVariableContext(name).apply(block).toVariable()
    }

    fun pathEntry(path: String, block: PathEntryContext.() -> Unit = {}) {
        pathEntries.add(PathEntryContext(path).apply(block).toEntry())
    }

    fun dotfile(name: String, block: DotfileContext.() -> Unit) {
        dotfiles.add(DotfileContext(name).apply(block).toConfiguration())
    }

    fun terminal(type: TerminalType, block: TerminalConfigurationContext.() -> Unit) {
        terminalConfigs.add(TerminalConfigurationContext(type).apply(block).toConfiguration())
    }

    fun prompt(type: PromptType, block: PromptConfigurationContext.() -> Unit) {
        promptConfigs.add(PromptConfigurationContext(type).apply(block).toConfiguration())
    }

    fun toConfig(): EnvironmentConfig {
        return EnvironmentConfig(
            shells = shells,
            environmentVariables = envVars.toMap(),
            pathEntries = pathEntries,
            dotfiles = dotfiles,
            terminals = terminalConfigs,
            prompts = promptConfigs
        )
    }
}

// ===== Shell Configuration =====

@HorizonOSDsl
class ShellConfigurationContext(private val type: ShellType) {
    var enabled: Boolean = true
    var defaultShell: Boolean = false
    var configFile: String? = null
    var aliases = mutableMapOf<String, String>()
    var functions = mutableMapOf<String, String>()
    var completions = mutableListOf<String>()
    var plugins = mutableListOf<ShellPlugin>()
    var settings = mutableMapOf<String, String>()
    
    // Shell-specific configurations
    var bashConfig: BashConfig? = null
    var zshConfig: ZshConfig? = null
    var fishConfig: FishConfig? = null

    fun bash(block: BashContext.() -> Unit) {
        bashConfig = BashContext().apply(block).toConfig()
    }

    fun zsh(block: ZshContext.() -> Unit) {
        zshConfig = ZshContext().apply(block).toConfig()
    }

    fun fish(block: FishContext.() -> Unit) {
        fishConfig = FishContext().apply(block).toConfig()
    }

    fun alias(name: String, command: String) {
        aliases[name] = command
    }

    fun function(name: String, body: String) {
        functions[name] = body
    }

    fun completion(name: String) {
        completions.add(name)
    }

    fun plugin(name: String, source: String? = null, enabled: Boolean = true) {
        plugins.add(ShellPlugin(name, source, enabled))
    }

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfiguration(): ShellConfiguration {
        return ShellConfiguration(
            type = type,
            enabled = enabled,
            defaultShell = defaultShell,
            configFile = configFile ?: getDefaultConfigFile(type),
            aliases = aliases.toMap(),
            functions = functions.toMap(),
            completions = completions,
            plugins = plugins,
            settings = settings.toMap(),
            bashConfig = bashConfig,
            zshConfig = zshConfig,
            fishConfig = fishConfig
        )
    }

    private fun getDefaultConfigFile(type: ShellType): String = when (type) {
        ShellType.BASH -> "~/.bashrc"
        ShellType.ZSH -> "~/.zshrc"
        ShellType.FISH -> "~/.config/fish/config.fish"
        ShellType.DASH -> "~/.profile"
        ShellType.TCSH -> "~/.tcshrc"
        ShellType.KSH -> "~/.kshrc"
    }
}

@HorizonOSDsl
class BashContext {
    var historySize: Int = 1000
    var historyFileSize: Int = 2000
    var historyControl: String = "ignoredups"
    var enableColorPrompt: Boolean = true
    var enableCompletion: Boolean = true
    var enableGlobstar: Boolean = true
    var checkWinSize: Boolean = true

    fun toConfig(): BashConfig {
        return BashConfig(
            historySize = historySize,
            historyFileSize = historyFileSize,
            historyControl = historyControl,
            enableColorPrompt = enableColorPrompt,
            enableCompletion = enableCompletion,
            enableGlobstar = enableGlobstar,
            checkWinSize = checkWinSize
        )
    }
}

@HorizonOSDsl
class ZshContext {
    var theme: String = "robbyrussell"
    var enableOhMyZsh: Boolean = true
    var ohMyZshPlugins = mutableListOf<String>()
    var historySize: Int = 10000
    var saveHistory: Int = 10000
    var shareHistory: Boolean = true
    var autoCorrect: Boolean = true
    var enableCompletion: Boolean = true

    fun ohMyZshPlugin(name: String) {
        ohMyZshPlugins.add(name)
    }

    fun toConfig(): ZshConfig {
        return ZshConfig(
            theme = theme,
            enableOhMyZsh = enableOhMyZsh,
            ohMyZshPlugins = ohMyZshPlugins,
            historySize = historySize,
            saveHistory = saveHistory,
            shareHistory = shareHistory,
            autoCorrect = autoCorrect,
            enableCompletion = enableCompletion
        )
    }
}

@HorizonOSDsl
class FishContext {
    var theme: String = "default"
    var enableGreeting: Boolean = true
    var enableAbbreviations: Boolean = true
    var abbreviations = mutableMapOf<String, String>()
    var variables = mutableMapOf<String, String>()

    fun abbreviation(abbr: String, expansion: String) {
        abbreviations[abbr] = expansion
    }

    fun variable(name: String, value: String) {
        variables[name] = value
    }

    fun toConfig(): FishConfig {
        return FishConfig(
            theme = theme,
            enableGreeting = enableGreeting,
            enableAbbreviations = enableAbbreviations,
            abbreviations = abbreviations.toMap(),
            variables = variables.toMap()
        )
    }
}

// ===== Environment Variables =====

@HorizonOSDsl
class EnvironmentVariableContext(private val name: String) {
    var value: String = ""
    var scope: VariableScope = VariableScope.USER
    var persistent: Boolean = true
    var overwrite: Boolean = false
    var description: String? = null

    fun toVariable(): EnvironmentVariable {
        return EnvironmentVariable(
            name = name,
            value = value,
            scope = scope,
            persistent = persistent,
            overwrite = overwrite,
            description = description
        )
    }
}

// ===== PATH Management =====

@HorizonOSDsl
class PathEntryContext(private val path: String) {
    var priority: Int = 50
    var condition: String? = null
    var createIfMissing: Boolean = false

    fun toEntry(): PathEntry {
        return PathEntry(
            path = path,
            priority = priority,
            condition = condition,
            createIfMissing = createIfMissing
        )
    }
}

// ===== Dotfiles Management =====

@HorizonOSDsl
class DotfileContext(private val name: String) {
    var sourcePath: String? = null
    var targetPath: String? = null
    var template: Boolean = false
    var variables = mutableMapOf<String, String>()
    var backup: Boolean = true
    var overwrite: Boolean = false
    var executable: Boolean = false
    var owner: String? = null
    var group: String? = null
    var permissions: String? = null

    fun variable(key: String, value: String) {
        variables[key] = value
    }

    fun toConfiguration(): DotfileConfiguration {
        return DotfileConfiguration(
            name = name,
            sourcePath = sourcePath,
            targetPath = targetPath ?: getDefaultTargetPath(name),
            template = template,
            variables = variables.toMap(),
            backup = backup,
            overwrite = overwrite,
            executable = executable,
            owner = owner,
            group = group,
            permissions = permissions
        )
    }

    private fun getDefaultTargetPath(name: String): String {
        return if (name.startsWith(".")) "~/$name" else "~/.$name"
    }
}

// ===== Terminal Configuration =====

@HorizonOSDsl
class TerminalConfigurationContext(private val type: TerminalType) {
    var enabled: Boolean = true
    var defaultTerminal: Boolean = false
    var theme: String? = null
    var colorScheme: String? = null
    var fontSize: Int = 12
    var fontFamily: String = "monospace"
    var enableBell: Boolean = false
    var enableBlinking: Boolean = true
    var scrollbackLines: Int = 1000
    var settings = mutableMapOf<String, String>()
    
    // Terminal-specific configurations
    var gnomeTerminalConfig: GnomeTerminalConfig? = null
    var konsoleConfig: KonsoleConfig? = null
    var alacrittyConfig: AlacrittyConfig? = null

    fun gnomeTerminal(block: GnomeTerminalContext.() -> Unit) {
        gnomeTerminalConfig = GnomeTerminalContext().apply(block).toConfig()
    }

    fun konsole(block: KonsoleContext.() -> Unit) {
        konsoleConfig = KonsoleContext().apply(block).toConfig()
    }

    fun alacritty(block: AlacrittyContext.() -> Unit) {
        alacrittyConfig = AlacrittyContext().apply(block).toConfig()
    }

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfiguration(): TerminalConfiguration {
        return TerminalConfiguration(
            type = type,
            enabled = enabled,
            defaultTerminal = defaultTerminal,
            theme = theme,
            colorScheme = colorScheme,
            fontSize = fontSize,
            fontFamily = fontFamily,
            enableBell = enableBell,
            enableBlinking = enableBlinking,
            scrollbackLines = scrollbackLines,
            settings = settings.toMap(),
            gnomeTerminalConfig = gnomeTerminalConfig,
            konsoleConfig = konsoleConfig,
            alacrittyConfig = alacrittyConfig
        )
    }
}

@HorizonOSDsl
class GnomeTerminalContext {
    var profileName: String = "default"
    var enableTransparency: Boolean = false
    var transparency: Double = 0.9
    var enableAudibleBell: Boolean = false
    var enableVisualBell: Boolean = true

    fun toConfig(): GnomeTerminalConfig {
        return GnomeTerminalConfig(
            profileName = profileName,
            enableTransparency = enableTransparency,
            transparency = transparency,
            enableAudibleBell = enableAudibleBell,
            enableVisualBell = enableVisualBell
        )
    }
}

@HorizonOSDsl
class KonsoleContext {
    var profileName: String = "default"
    var enableTranslucency: Boolean = false
    var enableBlur: Boolean = false
    var tabPosition: String = "top"

    fun toConfig(): KonsoleConfig {
        return KonsoleConfig(
            profileName = profileName,
            enableTranslucency = enableTranslucency,
            enableBlur = enableBlur,
            tabPosition = tabPosition
        )
    }
}

@HorizonOSDsl
class AlacrittyContext {
    var enableLiveConfigReload: Boolean = true
    var enableTabSpaces: Boolean = true
    var tabSpaces: Int = 8
    var drawBoldTextWithBrightColors: Boolean = true

    fun toConfig(): AlacrittyConfig {
        return AlacrittyConfig(
            enableLiveConfigReload = enableLiveConfigReload,
            enableTabSpaces = enableTabSpaces,
            tabSpaces = tabSpaces,
            drawBoldTextWithBrightColors = drawBoldTextWithBrightColors
        )
    }
}

// ===== Prompt Configuration =====

@HorizonOSDsl
class PromptConfigurationContext(private val type: PromptType) {
    var enabled: Boolean = true
    var theme: String? = null
    var showGitBranch: Boolean = true
    var showTimestamp: Boolean = false
    var showPath: Boolean = true
    var showUser: Boolean = true
    var showHost: Boolean = true
    var customFormat: String? = null
    var colors = mutableMapOf<String, String>()
    
    // Prompt-specific configurations
    var starshipConfig: StarshipConfig? = null
    var powerlevel10kConfig: Powerlevel10kConfig? = null

    fun starship(block: StarshipContext.() -> Unit) {
        starshipConfig = StarshipContext().apply(block).toConfig()
    }

    fun powerlevel10k(block: Powerlevel10kContext.() -> Unit) {
        powerlevel10kConfig = Powerlevel10kContext().apply(block).toConfig()
    }

    fun color(element: String, color: String) {
        colors[element] = color
    }

    fun toConfiguration(): PromptConfiguration {
        return PromptConfiguration(
            type = type,
            enabled = enabled,
            theme = theme,
            showGitBranch = showGitBranch,
            showTimestamp = showTimestamp,
            showPath = showPath,
            showUser = showUser,
            showHost = showHost,
            customFormat = customFormat,
            colors = colors.toMap(),
            starshipConfig = starshipConfig,
            powerlevel10kConfig = powerlevel10kConfig
        )
    }
}

@HorizonOSDsl
class StarshipContext {
    var configFile: String = "~/.config/starship.toml"
    var enabledModules = mutableListOf<String>()
    var disabledModules = mutableListOf<String>()
    var customModules = mutableMapOf<String, String>()

    fun enableModule(name: String) {
        enabledModules.add(name)
    }

    fun disableModule(name: String) {
        disabledModules.add(name)
    }

    fun customModule(name: String, config: String) {
        customModules[name] = config
    }

    fun toConfig(): StarshipConfig {
        return StarshipConfig(
            configFile = configFile,
            enabledModules = enabledModules,
            disabledModules = disabledModules,
            customModules = customModules.toMap()
        )
    }
}

@HorizonOSDsl
class Powerlevel10kContext {
    var instantPrompt: Boolean = true
    var mode: String = "classic"
    var enabledElements = mutableListOf<String>()
    var colors = mutableMapOf<String, String>()

    fun enableElement(name: String) {
        enabledElements.add(name)
    }

    fun color(element: String, color: String) {
        colors[element] = color
    }

    fun toConfig(): Powerlevel10kConfig {
        return Powerlevel10kConfig(
            instantPrompt = instantPrompt,
            mode = mode,
            enabledElements = enabledElements,
            colors = colors.toMap()
        )
    }
}

// ===== Enums =====

@Serializable
enum class ShellType {
    BASH, ZSH, FISH, DASH, TCSH, KSH
}

@Serializable
enum class VariableScope {
    SYSTEM, USER, SESSION
}

@Serializable
enum class TerminalType {
    GNOME_TERMINAL, KONSOLE, ALACRITTY, KITTY, XTERM, TERMINATOR, TILIX
}

@Serializable
enum class PromptType {
    DEFAULT, STARSHIP, POWERLEVEL10K, OH_MY_POSH, PURE
}

// ===== Data Classes =====

@Serializable
data class EnvironmentConfig(
    val shells: List<ShellConfiguration> = emptyList(),
    val environmentVariables: Map<String, EnvironmentVariable> = emptyMap(),
    val pathEntries: List<PathEntry> = emptyList(),
    val dotfiles: List<DotfileConfiguration> = emptyList(),
    val terminals: List<TerminalConfiguration> = emptyList(),
    val prompts: List<PromptConfiguration> = emptyList()
)

// Shell Configuration Data Classes
@Serializable
data class ShellConfiguration(
    val type: ShellType,
    val enabled: Boolean,
    val defaultShell: Boolean,
    val configFile: String,
    val aliases: Map<String, String>,
    val functions: Map<String, String>,
    val completions: List<String>,
    val plugins: List<ShellPlugin>,
    val settings: Map<String, String>,
    val bashConfig: BashConfig?,
    val zshConfig: ZshConfig?,
    val fishConfig: FishConfig?
)

@Serializable
data class BashConfig(
    val historySize: Int,
    val historyFileSize: Int,
    val historyControl: String,
    val enableColorPrompt: Boolean,
    val enableCompletion: Boolean,
    val enableGlobstar: Boolean,
    val checkWinSize: Boolean
)

@Serializable
data class ZshConfig(
    val theme: String,
    val enableOhMyZsh: Boolean,
    val ohMyZshPlugins: List<String>,
    val historySize: Int,
    val saveHistory: Int,
    val shareHistory: Boolean,
    val autoCorrect: Boolean,
    val enableCompletion: Boolean
)

@Serializable
data class FishConfig(
    val theme: String,
    val enableGreeting: Boolean,
    val enableAbbreviations: Boolean,
    val abbreviations: Map<String, String>,
    val variables: Map<String, String>
)

@Serializable
data class ShellPlugin(
    val name: String,
    val source: String?,
    val enabled: Boolean
)

// Environment Variables Data Classes
@Serializable
data class EnvironmentVariable(
    val name: String,
    val value: String,
    val scope: VariableScope,
    val persistent: Boolean,
    val overwrite: Boolean,
    val description: String?
)

@Serializable
data class PathEntry(
    val path: String,
    val priority: Int,
    val condition: String?,
    val createIfMissing: Boolean
)

// Dotfiles Data Classes
@Serializable
data class DotfileConfiguration(
    val name: String,
    val sourcePath: String?,
    val targetPath: String,
    val template: Boolean,
    val variables: Map<String, String>,
    val backup: Boolean,
    val overwrite: Boolean,
    val executable: Boolean,
    val owner: String?,
    val group: String?,
    val permissions: String?
)

// Terminal Configuration Data Classes
@Serializable
data class TerminalConfiguration(
    val type: TerminalType,
    val enabled: Boolean,
    val defaultTerminal: Boolean,
    val theme: String?,
    val colorScheme: String?,
    val fontSize: Int,
    val fontFamily: String,
    val enableBell: Boolean,
    val enableBlinking: Boolean,
    val scrollbackLines: Int,
    val settings: Map<String, String>,
    val gnomeTerminalConfig: GnomeTerminalConfig?,
    val konsoleConfig: KonsoleConfig?,
    val alacrittyConfig: AlacrittyConfig?
)

@Serializable
data class GnomeTerminalConfig(
    val profileName: String,
    val enableTransparency: Boolean,
    val transparency: Double,
    val enableAudibleBell: Boolean,
    val enableVisualBell: Boolean
)

@Serializable
data class KonsoleConfig(
    val profileName: String,
    val enableTranslucency: Boolean,
    val enableBlur: Boolean,
    val tabPosition: String
)

@Serializable
data class AlacrittyConfig(
    val enableLiveConfigReload: Boolean,
    val enableTabSpaces: Boolean,
    val tabSpaces: Int,
    val drawBoldTextWithBrightColors: Boolean
)

// Prompt Configuration Data Classes
@Serializable
data class PromptConfiguration(
    val type: PromptType,
    val enabled: Boolean,
    val theme: String?,
    val showGitBranch: Boolean,
    val showTimestamp: Boolean,
    val showPath: Boolean,
    val showUser: Boolean,
    val showHost: Boolean,
    val customFormat: String?,
    val colors: Map<String, String>,
    val starshipConfig: StarshipConfig?,
    val powerlevel10kConfig: Powerlevel10kConfig?
)

@Serializable
data class StarshipConfig(
    val configFile: String,
    val enabledModules: List<String>,
    val disabledModules: List<String>,
    val customModules: Map<String, String>
)

@Serializable
data class Powerlevel10kConfig(
    val instantPrompt: Boolean,
    val mode: String,
    val enabledElements: List<String>,
    val colors: Map<String, String>
)