package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Systemd Configuration =====

@HorizonOSDsl
class SystemdUnitContext(private val name: String, private val unitType: SystemdUnitType) {
    var description: String = ""
    var after = mutableListOf<String>()
    var requires = mutableListOf<String>()
    var wants = mutableListOf<String>()
    var conflicts = mutableListOf<String>()
    
    // Service section
    var execStart: String? = null
    var execStop: String? = null
    var execReload: String? = null
    var workingDirectory: String? = null
    var user: String? = null
    var group: String? = null
    var environment = mutableMapOf<String, String>()
    var restart: SystemdRestartPolicy = SystemdRestartPolicy.ON_FAILURE
    var restartSec: Int = 10
    var type: SystemdServiceType = SystemdServiceType.SIMPLE
    
    // Install section
    var wantedBy = mutableListOf<String>()
    var requiredBy = mutableListOf<String>()

    fun env(key: String, value: String) {
        environment[key] = value
    }

    fun toUnit(): SystemdUnit {
        return SystemdUnit(
            name = name,
            unitType = unitType,
            description = description,
            after = after,
            requires = requires,
            wants = wants,
            conflicts = conflicts,
            execStart = execStart,
            execStop = execStop,
            execReload = execReload,
            workingDirectory = workingDirectory,
            user = user,
            group = group,
            environment = environment.toMap(),
            restart = restart,
            restartSec = restartSec,
            type = type,
            wantedBy = wantedBy,
            requiredBy = requiredBy
        )
    }
}

// ===== Enums =====

@Serializable
enum class SystemdUnitType {
    SERVICE, SOCKET, TIMER, MOUNT, TARGET
}

@Serializable
enum class SystemdRestartPolicy {
    NO, ON_SUCCESS, ON_FAILURE, ON_ABNORMAL, ON_ABORT, ON_WATCHDOG, ALWAYS
}

@Serializable
enum class SystemdServiceType {
    SIMPLE, FORKING, ONESHOT, DBUS, NOTIFY, IDLE
}

// ===== Data Classes =====

@Serializable
data class SystemdUnit(
    val name: String,
    val unitType: SystemdUnitType,
    val description: String,
    val after: List<String>,
    val requires: List<String>,
    val wants: List<String>,
    val conflicts: List<String>,
    val execStart: String?,
    val execStop: String?,
    val execReload: String?,
    val workingDirectory: String?,
    val user: String?,
    val group: String?,
    val environment: Map<String, String>,
    val restart: SystemdRestartPolicy,
    val restartSec: Int,
    val type: SystemdServiceType,
    val wantedBy: List<String>,
    val requiredBy: List<String>
)