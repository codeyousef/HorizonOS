package org.horizonos.config.dsl.development.vcs

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Version Control Configuration =====

@HorizonOSDsl
class VCSConfigurationContext(private val type: VCSType) {
    var enabled: Boolean = true
    val globalConfig = mutableMapOf<String, String>()
    val aliases = mutableMapOf<String, String>()
    val hooks = mutableMapOf<String, String>()
    
    // VCS-specific configurations
    var gitConfig: GitConfig? = null
    var mercurialConfig: MercurialConfig? = null
    var svnConfig: SVNConfig? = null

    fun config(key: String, value: String) {
        globalConfig[key] = value
    }

    fun alias(name: String, command: String) {
        aliases[name] = command
    }

    fun hook(name: String, script: String) {
        hooks[name] = script
    }

    fun git(block: GitContext.() -> Unit) {
        gitConfig = GitContext().apply(block).toConfig()
    }

    fun mercurial(block: MercurialContext.() -> Unit) {
        mercurialConfig = MercurialContext().apply(block).toConfig()
    }

    fun svn(block: SVNContext.() -> Unit) {
        svnConfig = SVNContext().apply(block).toConfig()
    }

    fun toConfiguration(): VCSConfiguration {
        return VCSConfiguration(
            type = type,
            enabled = enabled,
            globalConfig = globalConfig,
            aliases = aliases,
            hooks = hooks,
            gitConfig = gitConfig,
            mercurialConfig = mercurialConfig,
            svnConfig = svnConfig
        )
    }
}

@HorizonOSDsl
class GitContext {
    var userName: String? = null
    var userEmail: String? = null
    var defaultBranch: String = "main"
    var autoSetupRemote: Boolean = true
    var autoStash: Boolean = true
    var rerereEnabled: Boolean = true
    val ignorePatterns = mutableListOf<String>()
    val lfsPatterns = mutableListOf<String>()
    val gitFlowConfig = mutableMapOf<String, String>()

    fun ignore(pattern: String) {
        ignorePatterns.add(pattern)
    }

    fun lfs(pattern: String) {
        lfsPatterns.add(pattern)
    }

    fun gitFlow(key: String, value: String) {
        gitFlowConfig[key] = value
    }

    fun toConfig(): GitConfig {
        return GitConfig(
            userName = userName,
            userEmail = userEmail,
            defaultBranch = defaultBranch,
            autoSetupRemote = autoSetupRemote,
            autoStash = autoStash,
            rerereEnabled = rerereEnabled,
            ignorePatterns = ignorePatterns,
            lfsPatterns = lfsPatterns,
            gitFlowConfig = gitFlowConfig
        )
    }
}

@HorizonOSDsl
class MercurialContext {
    var userName: String? = null
    var userEmail: String? = null
    val extensions = mutableListOf<String>()
    val aliases = mutableMapOf<String, String>()

    fun extension(name: String) {
        extensions.add(name)
    }

    fun alias(name: String, command: String) {
        aliases[name] = command
    }

    fun toConfig(): MercurialConfig {
        return MercurialConfig(
            userName = userName,
            userEmail = userEmail,
            extensions = extensions,
            aliases = aliases
        )
    }
}

@HorizonOSDsl
class SVNContext {
    var storePasswords: Boolean = false
    var storeAuthCreds: Boolean = true
    val globalIgnores = mutableListOf<String>()

    fun ignore(pattern: String) {
        globalIgnores.add(pattern)
    }

    fun toConfig(): SVNConfig {
        return SVNConfig(
            storePasswords = storePasswords,
            storeAuthCreds = storeAuthCreds,
            globalIgnores = globalIgnores
        )
    }
}

// Data Classes
@Serializable
data class VCSConfiguration(
    val type: VCSType,
    val enabled: Boolean,
    val globalConfig: Map<String, String>,
    val aliases: Map<String, String>,
    val hooks: Map<String, String>,
    val gitConfig: GitConfig?,
    val mercurialConfig: MercurialConfig?,
    val svnConfig: SVNConfig?
)

@Serializable
data class GitConfig(
    val userName: String?,
    val userEmail: String?,
    val defaultBranch: String,
    val autoSetupRemote: Boolean,
    val autoStash: Boolean,
    val rerereEnabled: Boolean,
    val ignorePatterns: List<String>,
    val lfsPatterns: List<String>,
    val gitFlowConfig: Map<String, String>
)

@Serializable
data class MercurialConfig(
    val userName: String?,
    val userEmail: String?,
    val extensions: List<String>,
    val aliases: Map<String, String>
)

@Serializable
data class SVNConfig(
    val storePasswords: Boolean,
    val storeAuthCreds: Boolean,
    val globalIgnores: List<String>
)

@Serializable
enum class VCSType {
    GIT, MERCURIAL, SVN, FOSSIL, DARCS, BAZAAR, PERFORCE, CVS
}