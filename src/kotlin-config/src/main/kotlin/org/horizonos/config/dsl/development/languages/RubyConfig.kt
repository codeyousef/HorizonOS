package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Ruby Configuration =====

@HorizonOSDsl
class RubyContext {
    var rubyVersionManager: RubyVersionManager = RubyVersionManager.RBENV
    var defaultGems = mutableListOf<String>()
    var gemConfig = mutableMapOf<String, String>()
    var enableBundler: Boolean = true

    fun rbenv() {
        rubyVersionManager = RubyVersionManager.RBENV
    }

    fun rvm() {
        rubyVersionManager = RubyVersionManager.RVM
    }

    fun chruby() {
        rubyVersionManager = RubyVersionManager.CHRUBY
    }

    fun asdf() {
        rubyVersionManager = RubyVersionManager.ASDF
    }

    fun defaultGem(name: String) {
        defaultGems.add(name)
    }

    fun gemConfig(key: String, value: String) {
        gemConfig[key] = value
    }

    fun toConfig(): RubyConfig {
        return RubyConfig(
            rubyVersionManager = rubyVersionManager,
            defaultGems = defaultGems,
            gemConfig = gemConfig,
            enableBundler = enableBundler
        )
    }
}

@Serializable
data class RubyConfig(
    val rubyVersionManager: RubyVersionManager,
    val defaultGems: List<String>,
    val gemConfig: Map<String, String>,
    val enableBundler: Boolean
)

@Serializable
enum class RubyVersionManager {
    RBENV, RVM, CHRUBY, ASDF
}