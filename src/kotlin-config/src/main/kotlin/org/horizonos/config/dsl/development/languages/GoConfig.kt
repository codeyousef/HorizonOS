package org.horizonos.config.dsl.development.languages

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Go Configuration =====

@HorizonOSDsl
class GoContext {
    var enableGoModules: Boolean = true
    var goProxy: String = "https://proxy.golang.org"
    var goSumDB: String = "sum.golang.org"
    var goPrivate: String? = null
    val goBin = mutableListOf<String>()
    val goEnv = mutableMapOf<String, String>()

    fun goBin(tool: String) {
        goBin.add(tool)
    }

    fun goEnv(key: String, value: String) {
        goEnv[key] = value
    }

    fun toConfig(): GoConfig {
        return GoConfig(
            enableGoModules = enableGoModules,
            goProxy = goProxy,
            goSumDB = goSumDB,
            goPrivate = goPrivate,
            goBin = goBin,
            goEnv = goEnv
        )
    }
}

@Serializable
data class GoConfig(
    val enableGoModules: Boolean,
    val goProxy: String,
    val goSumDB: String,
    val goPrivate: String?,
    val goBin: List<String>,
    val goEnv: Map<String, String>
)