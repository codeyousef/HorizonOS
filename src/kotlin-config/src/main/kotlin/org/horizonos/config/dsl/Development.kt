package org.horizonos.config.dsl

import kotlinx.serialization.Serializable

// ===== Development Environment Configuration DSL =====

@HorizonOSDsl
class DevelopmentContext {
    internal val languages = mutableListOf<LanguageRuntime>()
    internal val ides = mutableListOf<IDEConfiguration>()
    internal val editors = mutableListOf<EditorConfiguration>()
    internal val tools = mutableListOf<DevelopmentTool>()
    internal val packageManagers = mutableListOf<PackageManagerConfig>()
    internal val containerDev = mutableListOf<ContainerDevEnvironment>()
    internal val versionControl = mutableListOf<VCSConfiguration>()

    fun language(type: LanguageType, block: LanguageRuntimeContext.() -> Unit) {
        languages.add(LanguageRuntimeContext(type).apply(block).toRuntime())
    }

    fun ide(type: IDEType, block: IDEConfigurationContext.() -> Unit) {
        ides.add(IDEConfigurationContext(type).apply(block).toConfiguration())
    }

    fun editor(type: EditorType, block: EditorConfigurationContext.() -> Unit) {
        editors.add(EditorConfigurationContext(type).apply(block).toConfiguration())
    }

    fun tool(name: String, block: DevelopmentToolContext.() -> Unit) {
        tools.add(DevelopmentToolContext(name).apply(block).toTool())
    }

    fun packageManager(type: PackageManagerType, block: PackageManagerContext.() -> Unit) {
        packageManagers.add(PackageManagerContext(type).apply(block).toConfig())
    }

    fun containerDev(name: String, block: ContainerDevContext.() -> Unit) {
        containerDev.add(ContainerDevContext(name).apply(block).toEnvironment())
    }

    fun versionControl(type: VCSType, block: VCSConfigurationContext.() -> Unit) {
        versionControl.add(VCSConfigurationContext(type).apply(block).toConfiguration())
    }

    fun toConfig(): DevelopmentConfig {
        return DevelopmentConfig(
            languages = languages,
            ides = ides,
            editors = editors,
            tools = tools,
            packageManagers = packageManagers,
            containerDev = containerDev,
            versionControl = versionControl
        )
    }
}

// ===== Language Runtime Configuration =====

@HorizonOSDsl
class LanguageRuntimeContext(private val type: LanguageType) {
    var enabled: Boolean = true
    var version: String? = null
    var defaultVersion: String? = null
    var globalPackages = mutableListOf<String>()
    var environmentVariables = mutableMapOf<String, String>()
    var configOverrides = mutableMapOf<String, String>()
    
    // Language-specific configurations
    var nodeConfig: NodeJSConfig? = null
    var pythonConfig: PythonConfig? = null
    var javaConfig: JavaConfig? = null
    var rustConfig: RustConfig? = null
    var goConfig: GoConfig? = null
    var rubyConfig: RubyConfig? = null

    fun nodejs(block: NodeJSContext.() -> Unit) {
        nodeConfig = NodeJSContext().apply(block).toConfig()
    }

    fun python(block: PythonContext.() -> Unit) {
        pythonConfig = PythonContext().apply(block).toConfig()
    }

    fun java(block: JavaContext.() -> Unit) {
        javaConfig = JavaContext().apply(block).toConfig()
    }

    fun rust(block: RustContext.() -> Unit) {
        rustConfig = RustContext().apply(block).toConfig()
    }

    fun go(block: GoContext.() -> Unit) {
        goConfig = GoContext().apply(block).toConfig()
    }

    fun ruby(block: RubyContext.() -> Unit) {
        rubyConfig = RubyContext().apply(block).toConfig()
    }

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun env(key: String, value: String) {
        environmentVariables[key] = value
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toRuntime(): LanguageRuntime {
        return LanguageRuntime(
            type = type,
            enabled = enabled,
            version = version,
            defaultVersion = defaultVersion,
            globalPackages = globalPackages,
            environmentVariables = environmentVariables.toMap(),
            configOverrides = configOverrides.toMap(),
            nodeConfig = nodeConfig,
            pythonConfig = pythonConfig,
            javaConfig = javaConfig,
            rustConfig = rustConfig,
            goConfig = goConfig,
            rubyConfig = rubyConfig
        )
    }
}

@HorizonOSDsl
class NodeJSContext {
    var packageManager: NodePackageManager = NodePackageManager.NPM
    var yarnVersion: String = "latest"
    var enableCorepack: Boolean = true
    var globalPackages = mutableListOf<String>()
    var registries = mutableMapOf<String, String>()

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun registry(scope: String, url: String) {
        registries[scope] = url
    }

    fun toConfig(): NodeJSConfig {
        return NodeJSConfig(
            packageManager = packageManager,
            yarnVersion = yarnVersion,
            enableCorepack = enableCorepack,
            globalPackages = globalPackages,
            registries = registries.toMap()
        )
    }
}

@HorizonOSDsl
class PythonContext {
    var packageManager: PythonPackageManager = PythonPackageManager.PIP
    var virtualenvTool: PythonVirtualenvTool = PythonVirtualenvTool.VENV
    var globalPackages = mutableListOf<String>()
    var pipConfig = mutableMapOf<String, String>()
    var enableUvLoop: Boolean = false

    fun globalPackage(name: String) {
        globalPackages.add(name)
    }

    fun pipConfig(key: String, value: String) {
        pipConfig[key] = value
    }

    fun toConfig(): PythonConfig {
        return PythonConfig(
            packageManager = packageManager,
            virtualenvTool = virtualenvTool,
            globalPackages = globalPackages,
            pipConfig = pipConfig.toMap(),
            enableUvLoop = enableUvLoop
        )
    }
}

@HorizonOSDsl
class JavaContext {
    var jvmImplementation: JVMImplementation = JVMImplementation.OPENJDK
    var enableJavaFX: Boolean = false
    var jvmArgs = mutableListOf<String>()
    var mavenConfig: MavenConfig? = null
    var gradleConfig: GradleConfig? = null

    fun maven(block: MavenContext.() -> Unit) {
        mavenConfig = MavenContext().apply(block).toConfig()
    }

    fun gradle(block: GradleContext.() -> Unit) {
        gradleConfig = GradleContext().apply(block).toConfig()
    }

    fun jvmArg(arg: String) {
        jvmArgs.add(arg)
    }

    fun toConfig(): JavaConfig {
        return JavaConfig(
            jvmImplementation = jvmImplementation,
            enableJavaFX = enableJavaFX,
            jvmArgs = jvmArgs,
            mavenConfig = mavenConfig,
            gradleConfig = gradleConfig
        )
    }
}

@HorizonOSDsl
class MavenContext {
    var localRepository: String = "\${user.home}/.m2/repository"
    var mirrors = mutableListOf<MavenMirror>()
    var profiles = mutableListOf<MavenProfile>()

    fun mirror(id: String, url: String, mirrorOf: String = "*") {
        mirrors.add(MavenMirror(id, url, mirrorOf))
    }

    fun profile(id: String, block: MavenProfileContext.() -> Unit) {
        profiles.add(MavenProfileContext(id).apply(block).toProfile())
    }

    fun toConfig(): MavenConfig {
        return MavenConfig(
            localRepository = localRepository,
            mirrors = mirrors,
            profiles = profiles
        )
    }
}

@HorizonOSDsl
class MavenProfileContext(private val id: String) {
    var activeByDefault: Boolean = false
    var repositories = mutableListOf<MavenRepository>()

    fun repository(id: String, url: String, releases: Boolean = true, snapshots: Boolean = false) {
        repositories.add(MavenRepository(id, url, releases, snapshots))
    }

    fun toProfile(): MavenProfile {
        return MavenProfile(
            id = id,
            activeByDefault = activeByDefault,
            repositories = repositories
        )
    }
}

@HorizonOSDsl
class GradleContext {
    var distributionUrl: String = "https://services.gradle.org/distributions/gradle-8.5-bin.zip"
    var enableDaemon: Boolean = true
    var jvmArgs = mutableListOf<String>()
    var systemProperties = mutableMapOf<String, String>()

    fun jvmArg(arg: String) {
        jvmArgs.add(arg)
    }

    fun systemProperty(key: String, value: String) {
        systemProperties[key] = value
    }

    fun toConfig(): GradleConfig {
        return GradleConfig(
            distributionUrl = distributionUrl,
            enableDaemon = enableDaemon,
            jvmArgs = jvmArgs,
            systemProperties = systemProperties.toMap()
        )
    }
}

@HorizonOSDsl
class RustContext {
    var toolchain: RustToolchain = RustToolchain.STABLE
    var targets = mutableListOf<String>()
    var components = mutableListOf<String>()
    var cargoConfig = mutableMapOf<String, String>()

    fun target(target: String) {
        targets.add(target)
    }

    fun component(component: String) {
        components.add(component)
    }

    fun cargoConfig(key: String, value: String) {
        cargoConfig[key] = value
    }

    fun toConfig(): RustConfig {
        return RustConfig(
            toolchain = toolchain,
            targets = targets,
            components = components,
            cargoConfig = cargoConfig.toMap()
        )
    }
}

@HorizonOSDsl
class GoContext {
    var goPath: String = "\${HOME}/go"
    var goProxy: String = "https://proxy.golang.org,direct"
    var goSumDb: String = "sum.golang.org"
    var enableModules: Boolean = true
    var privateRepos = mutableListOf<String>()

    fun privateRepo(repo: String) {
        privateRepos.add(repo)
    }

    fun toConfig(): GoConfig {
        return GoConfig(
            goPath = goPath,
            goProxy = goProxy,
            goSumDb = goSumDb,
            enableModules = enableModules,
            privateRepos = privateRepos
        )
    }
}

@HorizonOSDsl
class RubyContext {
    var rubyManager: RubyManager = RubyManager.RVM
    var defaultGems = mutableListOf<String>()
    var gemSources = mutableListOf<String>()

    fun defaultGem(name: String) {
        defaultGems.add(name)
    }

    fun gemSource(url: String) {
        gemSources.add(url)
    }

    fun toConfig(): RubyConfig {
        return RubyConfig(
            rubyManager = rubyManager,
            defaultGems = defaultGems,
            gemSources = gemSources
        )
    }
}

// ===== IDE Configuration =====

@HorizonOSDsl
class IDEConfigurationContext(private val type: IDEType) {
    var enabled: Boolean = true
    var version: String? = null
    var extensions = mutableListOf<String>()
    var themes = mutableListOf<String>()
    var settings = mutableMapOf<String, String>()
    var keybindings = mutableListOf<Keybinding>()

    // IDE-specific configurations
    var vscodeConfig: VSCodeConfig? = null
    var intellijConfig: IntelliJConfig? = null
    var vimConfig: VimConfig? = null

    fun vscode(block: VSCodeContext.() -> Unit) {
        vscodeConfig = VSCodeContext().apply(block).toConfig()
    }

    fun intellij(block: IntelliJContext.() -> Unit) {
        intellijConfig = IntelliJContext().apply(block).toConfig()
    }

    fun vim(block: VimContext.() -> Unit) {
        vimConfig = VimContext().apply(block).toConfig()
    }

    fun extension(name: String) {
        extensions.add(name)
    }

    fun theme(name: String) {
        themes.add(name)
    }

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun keybinding(key: String, command: String, when_: String? = null) {
        keybindings.add(Keybinding(key, command, when_))
    }

    fun toConfiguration(): IDEConfiguration {
        return IDEConfiguration(
            type = type,
            enabled = enabled,
            version = version,
            extensions = extensions,
            themes = themes,
            settings = settings.toMap(),
            keybindings = keybindings,
            vscodeConfig = vscodeConfig,
            intellijConfig = intellijConfig,
            vimConfig = vimConfig
        )
    }
}

@HorizonOSDsl
class VSCodeContext {
    var enableTelemetry: Boolean = false
    var enableAutoUpdate: Boolean = true
    var workbenchTheme: String = "Default Dark+"
    var editorFontFamily: String = "Consolas, 'Courier New', monospace"
    var editorFontSize: Int = 14
    var enableMinimap: Boolean = true
    var enableWordWrap: Boolean = false
    var tabSize: Int = 4
    var insertSpaces: Boolean = true

    fun toConfig(): VSCodeConfig {
        return VSCodeConfig(
            enableTelemetry = enableTelemetry,
            enableAutoUpdate = enableAutoUpdate,
            workbenchTheme = workbenchTheme,
            editorFontFamily = editorFontFamily,
            editorFontSize = editorFontSize,
            enableMinimap = enableMinimap,
            enableWordWrap = enableWordWrap,
            tabSize = tabSize,
            insertSpaces = insertSpaces
        )
    }
}

@HorizonOSDsl
class IntelliJContext {
    var vmOptions = mutableListOf<String>()
    var plugins = mutableListOf<String>()
    var codeStyle: String = "Default"
    var keymap: String = "Default"

    fun vmOption(option: String) {
        vmOptions.add(option)
    }

    fun plugin(name: String) {
        plugins.add(name)
    }

    fun toConfig(): IntelliJConfig {
        return IntelliJConfig(
            vmOptions = vmOptions,
            plugins = plugins,
            codeStyle = codeStyle,
            keymap = keymap
        )
    }
}

@HorizonOSDsl
class VimContext {
    var enableSyntaxHighlighting: Boolean = true
    var enableLineNumbers: Boolean = true
    var tabWidth: Int = 4
    var expandTabs: Boolean = true
    var plugins = mutableListOf<VimPlugin>()
    var colorScheme: String = "default"

    fun plugin(name: String, source: String? = null) {
        plugins.add(VimPlugin(name, source))
    }

    fun toConfig(): VimConfig {
        return VimConfig(
            enableSyntaxHighlighting = enableSyntaxHighlighting,
            enableLineNumbers = enableLineNumbers,
            tabWidth = tabWidth,
            expandTabs = expandTabs,
            plugins = plugins,
            colorScheme = colorScheme
        )
    }
}

// ===== Editor Configuration =====

@HorizonOSDsl
class EditorConfigurationContext(private val type: EditorType) {
    var enabled: Boolean = true
    var version: String? = null
    var configFile: String? = null
    var settings = mutableMapOf<String, String>()

    fun setting(key: String, value: String) {
        settings[key] = value
    }

    fun toConfiguration(): EditorConfiguration {
        return EditorConfiguration(
            type = type,
            enabled = enabled,
            version = version,
            configFile = configFile,
            settings = settings.toMap()
        )
    }
}

// ===== Development Tools =====

@HorizonOSDsl
class DevelopmentToolContext(private val name: String) {
    var category: ToolCategory = ToolCategory.GENERAL
    var version: String? = null
    var enabled: Boolean = true
    var autoInstall: Boolean = true
    var configuration = mutableMapOf<String, String>()
    var dependencies = mutableListOf<String>()

    fun config(key: String, value: String) {
        configuration[key] = value
    }

    fun dependency(name: String) {
        dependencies.add(name)
    }

    fun toTool(): DevelopmentTool {
        return DevelopmentTool(
            name = name,
            category = category,
            version = version,
            enabled = enabled,
            autoInstall = autoInstall,
            configuration = configuration.toMap(),
            dependencies = dependencies
        )
    }
}

// ===== Package Manager Configuration =====

@HorizonOSDsl
class PackageManagerContext(private val type: PackageManagerType) {
    var enabled: Boolean = true
    var autoUpdate: Boolean = false
    var registries = mutableListOf<PackageRegistry>()
    var authentication = mutableMapOf<String, PackageAuth>()
    var globalSettings = mutableMapOf<String, String>()

    fun registry(name: String, url: String, priority: Int = 50) {
        registries.add(PackageRegistry(name, url, priority))
    }

    fun auth(registry: String, username: String, token: String) {
        authentication[registry] = PackageAuth(username, token)
    }

    fun setting(key: String, value: String) {
        globalSettings[key] = value
    }

    fun toConfig(): PackageManagerConfig {
        return PackageManagerConfig(
            type = type,
            enabled = enabled,
            autoUpdate = autoUpdate,
            registries = registries,
            authentication = authentication.toMap(),
            globalSettings = globalSettings.toMap()
        )
    }
}

// ===== Container Development Environment =====

@HorizonOSDsl
class ContainerDevContext(private val name: String) {
    var image: String = ""
    var tag: String = "latest"
    var workingDirectory: String = "/workspace"
    var ports = mutableListOf<PortMapping>()
    var volumes = mutableListOf<VolumeMount>()
    var environment = mutableMapOf<String, String>()
    var postCreateCommand: String? = null
    var extensions = mutableListOf<String>()
    var features = mutableListOf<DevContainerFeature>()

    fun port(host: Int, container: Int) {
        ports.add(PortMapping(host, container, "tcp"))
    }

    fun volume(source: String, target: String, readOnly: Boolean = false) {
        volumes.add(VolumeMount(source, target, readOnly))
    }

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun extension(name: String) {
        extensions.add(name)
    }

    fun feature(name: String, options: Map<String, String> = emptyMap()) {
        features.add(DevContainerFeature(name, options))
    }

    fun toEnvironment(): ContainerDevEnvironment {
        return ContainerDevEnvironment(
            name = name,
            image = image,
            tag = tag,
            workingDirectory = workingDirectory,
            ports = ports,
            volumes = volumes,
            environment = environment.toMap(),
            postCreateCommand = postCreateCommand,
            extensions = extensions,
            features = features
        )
    }
}

// ===== Version Control Configuration =====

@HorizonOSDsl
class VCSConfigurationContext(private val type: VCSType) {
    var enabled: Boolean = true
    var globalConfig = mutableMapOf<String, String>()
    var aliases = mutableMapOf<String, String>()
    var hooks = mutableListOf<VCSHook>()

    // Git-specific configuration
    var gitConfig: GitConfig? = null

    fun git(block: GitContext.() -> Unit) {
        gitConfig = GitContext().apply(block).toConfig()
    }

    fun config(key: String, value: String) {
        globalConfig[key] = value
    }

    fun alias(name: String, command: String) {
        aliases[name] = command
    }

    fun hook(type: VCSHookType, script: String) {
        hooks.add(VCSHook(type, script))
    }

    fun toConfiguration(): VCSConfiguration {
        return VCSConfiguration(
            type = type,
            enabled = enabled,
            globalConfig = globalConfig.toMap(),
            aliases = aliases.toMap(),
            hooks = hooks,
            gitConfig = gitConfig
        )
    }
}

@HorizonOSDsl
class GitContext {
    var userName: String = ""
    var userEmail: String = ""
    var defaultBranch: String = "main"
    var enableGPGSigning: Boolean = false
    var gpgKeyId: String? = null
    var enableLFS: Boolean = false
    var lfsConfig: GitLFSConfig? = null

    fun lfs(block: GitLFSContext.() -> Unit) {
        enableLFS = true
        lfsConfig = GitLFSContext().apply(block).toConfig()
    }

    fun toConfig(): GitConfig {
        return GitConfig(
            userName = userName,
            userEmail = userEmail,
            defaultBranch = defaultBranch,
            enableGPGSigning = enableGPGSigning,
            gpgKeyId = gpgKeyId,
            enableLFS = enableLFS,
            lfsConfig = lfsConfig
        )
    }
}

@HorizonOSDsl
class GitLFSContext {
    var trackPatterns = mutableListOf<String>()
    var fetchInclude: String? = null
    var fetchExclude: String? = null

    fun track(pattern: String) {
        trackPatterns.add(pattern)
    }

    fun toConfig(): GitLFSConfig {
        return GitLFSConfig(
            trackPatterns = trackPatterns,
            fetchInclude = fetchInclude,
            fetchExclude = fetchExclude
        )
    }
}

// ===== Enums =====

@Serializable
enum class LanguageType {
    NODEJS, PYTHON, JAVA, RUST, GO, RUBY, PHP, CSHARP, CPP, KOTLIN
}

@Serializable
enum class NodePackageManager {
    NPM, YARN, PNPM
}

@Serializable
enum class PythonPackageManager {
    PIP, CONDA, POETRY, PIPENV
}

@Serializable
enum class PythonVirtualenvTool {
    VENV, VIRTUALENV, CONDA, PYENV
}

@Serializable
enum class JVMImplementation {
    OPENJDK, ORACLE_JDK, GRAALVM, AZUL_ZULU, ADOPTIUM
}

@Serializable
enum class RustToolchain {
    STABLE, BETA, NIGHTLY
}

@Serializable
enum class RubyManager {
    RVM, RBENV, CHRUBY
}

@Serializable
enum class IDEType {
    VSCODE, INTELLIJ_IDEA, INTELLIJ_COMMUNITY, PYCHARM, WEBSTORM, ANDROID_STUDIO, VIM, NEOVIM, EMACS
}

@Serializable
enum class EditorType {
    NANO, GEDIT, SUBLIME_TEXT, ATOM, BRACKETS
}

@Serializable
enum class ToolCategory {
    GENERAL, BUILD, DEBUG, TEST, DEPLOY, MONITOR, DATABASE, NETWORK
}

@Serializable
enum class PackageManagerType {
    NPM, YARN, PIP, CARGO, GO_MODULES, MAVEN, GRADLE, COMPOSER, NUGET
}

@Serializable
enum class VCSType {
    GIT, SVN, MERCURIAL
}

@Serializable
enum class VCSHookType {
    PRE_COMMIT, POST_COMMIT, PRE_PUSH, POST_RECEIVE, PRE_RECEIVE
}

// ===== Data Classes =====

@Serializable
data class DevelopmentConfig(
    val languages: List<LanguageRuntime> = emptyList(),
    val ides: List<IDEConfiguration> = emptyList(),
    val editors: List<EditorConfiguration> = emptyList(),
    val tools: List<DevelopmentTool> = emptyList(),
    val packageManagers: List<PackageManagerConfig> = emptyList(),
    val containerDev: List<ContainerDevEnvironment> = emptyList(),
    val versionControl: List<VCSConfiguration> = emptyList()
)

// Language Runtime Data Classes
@Serializable
data class LanguageRuntime(
    val type: LanguageType,
    val enabled: Boolean,
    val version: String?,
    val defaultVersion: String?,
    val globalPackages: List<String>,
    val environmentVariables: Map<String, String>,
    val configOverrides: Map<String, String>,
    val nodeConfig: NodeJSConfig?,
    val pythonConfig: PythonConfig?,
    val javaConfig: JavaConfig?,
    val rustConfig: RustConfig?,
    val goConfig: GoConfig?,
    val rubyConfig: RubyConfig?
)

@Serializable
data class NodeJSConfig(
    val packageManager: NodePackageManager,
    val yarnVersion: String,
    val enableCorepack: Boolean,
    val globalPackages: List<String>,
    val registries: Map<String, String>
)

@Serializable
data class PythonConfig(
    val packageManager: PythonPackageManager,
    val virtualenvTool: PythonVirtualenvTool,
    val globalPackages: List<String>,
    val pipConfig: Map<String, String>,
    val enableUvLoop: Boolean
)

@Serializable
data class JavaConfig(
    val jvmImplementation: JVMImplementation,
    val enableJavaFX: Boolean,
    val jvmArgs: List<String>,
    val mavenConfig: MavenConfig?,
    val gradleConfig: GradleConfig?
)

@Serializable
data class MavenConfig(
    val localRepository: String,
    val mirrors: List<MavenMirror>,
    val profiles: List<MavenProfile>
)

@Serializable
data class MavenMirror(
    val id: String,
    val url: String,
    val mirrorOf: String
)

@Serializable
data class MavenProfile(
    val id: String,
    val activeByDefault: Boolean,
    val repositories: List<MavenRepository>
)

@Serializable
data class MavenRepository(
    val id: String,
    val url: String,
    val releases: Boolean,
    val snapshots: Boolean
)

@Serializable
data class GradleConfig(
    val distributionUrl: String,
    val enableDaemon: Boolean,
    val jvmArgs: List<String>,
    val systemProperties: Map<String, String>
)

@Serializable
data class RustConfig(
    val toolchain: RustToolchain,
    val targets: List<String>,
    val components: List<String>,
    val cargoConfig: Map<String, String>
)

@Serializable
data class GoConfig(
    val goPath: String,
    val goProxy: String,
    val goSumDb: String,
    val enableModules: Boolean,
    val privateRepos: List<String>
)

@Serializable
data class RubyConfig(
    val rubyManager: RubyManager,
    val defaultGems: List<String>,
    val gemSources: List<String>
)

// IDE Configuration Data Classes
@Serializable
data class IDEConfiguration(
    val type: IDEType,
    val enabled: Boolean,
    val version: String?,
    val extensions: List<String>,
    val themes: List<String>,
    val settings: Map<String, String>,
    val keybindings: List<Keybinding>,
    val vscodeConfig: VSCodeConfig?,
    val intellijConfig: IntelliJConfig?,
    val vimConfig: VimConfig?
)

@Serializable
data class VSCodeConfig(
    val enableTelemetry: Boolean,
    val enableAutoUpdate: Boolean,
    val workbenchTheme: String,
    val editorFontFamily: String,
    val editorFontSize: Int,
    val enableMinimap: Boolean,
    val enableWordWrap: Boolean,
    val tabSize: Int,
    val insertSpaces: Boolean
)

@Serializable
data class IntelliJConfig(
    val vmOptions: List<String>,
    val plugins: List<String>,
    val codeStyle: String,
    val keymap: String
)

@Serializable
data class VimConfig(
    val enableSyntaxHighlighting: Boolean,
    val enableLineNumbers: Boolean,
    val tabWidth: Int,
    val expandTabs: Boolean,
    val plugins: List<VimPlugin>,
    val colorScheme: String
)

@Serializable
data class VimPlugin(
    val name: String,
    val source: String?
)

@Serializable
data class Keybinding(
    val key: String,
    val command: String,
    val when_: String?
)

// Editor Configuration Data Classes
@Serializable
data class EditorConfiguration(
    val type: EditorType,
    val enabled: Boolean,
    val version: String?,
    val configFile: String?,
    val settings: Map<String, String>
)

// Development Tools Data Classes
@Serializable
data class DevelopmentTool(
    val name: String,
    val category: ToolCategory,
    val version: String?,
    val enabled: Boolean,
    val autoInstall: Boolean,
    val configuration: Map<String, String>,
    val dependencies: List<String>
)

// Package Manager Data Classes
@Serializable
data class PackageManagerConfig(
    val type: PackageManagerType,
    val enabled: Boolean,
    val autoUpdate: Boolean,
    val registries: List<PackageRegistry>,
    val authentication: Map<String, PackageAuth>,
    val globalSettings: Map<String, String>
)

@Serializable
data class PackageRegistry(
    val name: String,
    val url: String,
    val priority: Int
)

@Serializable
data class PackageAuth(
    val username: String,
    val token: String
)

// Container Development Data Classes
@Serializable
data class ContainerDevEnvironment(
    val name: String,
    val image: String,
    val tag: String,
    val workingDirectory: String,
    val ports: List<PortMapping>,
    val volumes: List<VolumeMount>,
    val environment: Map<String, String>,
    val postCreateCommand: String?,
    val extensions: List<String>,
    val features: List<DevContainerFeature>
)

@Serializable
data class DevContainerFeature(
    val name: String,
    val options: Map<String, String>
)

// Version Control Data Classes
@Serializable
data class VCSConfiguration(
    val type: VCSType,
    val enabled: Boolean,
    val globalConfig: Map<String, String>,
    val aliases: Map<String, String>,
    val hooks: List<VCSHook>,
    val gitConfig: GitConfig?
)

@Serializable
data class GitConfig(
    val userName: String,
    val userEmail: String,
    val defaultBranch: String,
    val enableGPGSigning: Boolean,
    val gpgKeyId: String?,
    val enableLFS: Boolean,
    val lfsConfig: GitLFSConfig?
)

@Serializable
data class GitLFSConfig(
    val trackPatterns: List<String>,
    val fetchInclude: String?,
    val fetchExclude: String?
)

@Serializable
data class VCSHook(
    val type: VCSHookType,
    val script: String
)