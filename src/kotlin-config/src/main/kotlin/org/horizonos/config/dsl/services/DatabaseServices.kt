package org.horizonos.config.dsl.services

import kotlinx.serialization.Serializable
import org.horizonos.config.dsl.HorizonOSDsl

// ===== Database Configuration =====

@HorizonOSDsl
class DatabaseContext(private val type: DatabaseType) {
    var enabled: Boolean = true
    var autoStart: Boolean = true
    var version: String? = null
    var port: Int? = null
    var dataDirectory: String? = null
    var configOverrides = mutableMapOf<String, String>()
    var users = mutableListOf<DatabaseUser>()
    var databases = mutableListOf<DatabaseSchema>()
    
    // Database-specific configurations
    var postgresConfig: PostgresConfig? = null
    var mysqlConfig: MySQLConfig? = null
    var redisConfig: RedisConfig? = null

    fun postgres(block: PostgresContext.() -> Unit) {
        postgresConfig = PostgresContext().apply(block).toConfig()
    }

    fun mysql(block: MySQLContext.() -> Unit) {
        mysqlConfig = MySQLContext().apply(block).toConfig()
    }

    fun redis(block: RedisContext.() -> Unit) {
        redisConfig = RedisContext().apply(block).toConfig()
    }

    fun user(name: String, block: DatabaseUserContext.() -> Unit) {
        users.add(DatabaseUserContext(name).apply(block).toUser())
    }

    fun database(name: String, block: DatabaseSchemaContext.() -> Unit) {
        databases.add(DatabaseSchemaContext(name).apply(block).toSchema())
    }

    fun config(key: String, value: String) {
        configOverrides[key] = value
    }

    fun toService(): DatabaseService {
        return DatabaseService(
            type = type,
            enabled = enabled,
            autoStart = autoStart,
            version = version,
            port = port ?: getDefaultPort(type),
            dataDirectory = dataDirectory ?: getDefaultDataDirectory(type),
            configOverrides = configOverrides.toMap(),
            users = users,
            databases = databases,
            postgresConfig = postgresConfig,
            mysqlConfig = mysqlConfig,
            redisConfig = redisConfig
        )
    }

    private fun getDefaultPort(type: DatabaseType): Int = when (type) {
        DatabaseType.POSTGRESQL -> 5432
        DatabaseType.MYSQL -> 3306
        DatabaseType.REDIS -> 6379
        DatabaseType.MONGODB -> 27017
        DatabaseType.SQLITE -> 0
    }

    private fun getDefaultDataDirectory(type: DatabaseType): String = when (type) {
        DatabaseType.POSTGRESQL -> "/var/lib/postgresql/data"
        DatabaseType.MYSQL -> "/var/lib/mysql"
        DatabaseType.REDIS -> "/var/lib/redis"
        DatabaseType.MONGODB -> "/var/lib/mongodb"
        DatabaseType.SQLITE -> "/var/lib/sqlite"
    }
}

// ===== PostgreSQL Configuration =====

@HorizonOSDsl
class PostgresContext {
    var maxConnections: Int = 100
    var sharedBuffers: String = "128MB"
    var effectiveCacheSize: String = "4GB"
    var maintenanceWorkMem: String = "64MB"
    var checkpointSegments: Int = 32
    var checkpointCompletionTarget: Double = 0.7
    var walBuffers: String = "16MB"
    var defaultStatisticsTarget: Int = 100
    var logMinDurationStatement: Int = -1
    var logConnections: Boolean = false
    var logDisconnections: Boolean = false
    var logLockWaits: Boolean = false
    var logStatement: String = "none"
    var logLinePrefix: String = "%t "
    var extensions = mutableListOf<String>()

    fun extension(name: String) {
        extensions.add(name)
    }

    fun toConfig(): PostgresConfig {
        return PostgresConfig(
            maxConnections = maxConnections,
            sharedBuffers = sharedBuffers,
            effectiveCacheSize = effectiveCacheSize,
            maintenanceWorkMem = maintenanceWorkMem,
            checkpointSegments = checkpointSegments,
            checkpointCompletionTarget = checkpointCompletionTarget,
            walBuffers = walBuffers,
            defaultStatisticsTarget = defaultStatisticsTarget,
            logMinDurationStatement = logMinDurationStatement,
            logConnections = logConnections,
            logDisconnections = logDisconnections,
            logLockWaits = logLockWaits,
            logStatement = logStatement,
            logLinePrefix = logLinePrefix,
            extensions = extensions
        )
    }
}

// ===== MySQL Configuration =====

@HorizonOSDsl
class MySQLContext {
    var maxConnections: Int = 151
    var innodbBufferPoolSize: String = "128MB"
    var innodbLogFileSize: String = "48MB"
    var innodbFlushLogAtTrxCommit: Int = 1
    var innodbFlushMethod: String = "O_DIRECT"
    var queryCache: Boolean = true
    var queryCacheSize: String = "16MB"
    var tmpTableSize: String = "16MB"
    var maxHeapTableSize: String = "16MB"

    fun toConfig(): MySQLConfig {
        return MySQLConfig(
            maxConnections = maxConnections,
            innodbBufferPoolSize = innodbBufferPoolSize,
            innodbLogFileSize = innodbLogFileSize,
            innodbFlushLogAtTrxCommit = innodbFlushLogAtTrxCommit,
            innodbFlushMethod = innodbFlushMethod,
            queryCache = queryCache,
            queryCacheSize = queryCacheSize,
            tmpTableSize = tmpTableSize,
            maxHeapTableSize = maxHeapTableSize
        )
    }
}

// ===== Redis Configuration =====

@HorizonOSDsl
class RedisContext {
    var maxMemory: String = "128MB"
    var maxMemoryPolicy: String = "allkeys-lru"
    var persistenceMode: RedisPersistenceMode = RedisPersistenceMode.RDB
    var savePolicy: String = "900 1 300 10 60 10000"
    var appendOnly: Boolean = false
    var appendFsync: String = "everysec"
    var clustering: Boolean = false
    var clusterNodes = mutableListOf<String>()

    fun clusterNode(node: String) {
        clusterNodes.add(node)
    }

    fun toConfig(): RedisConfig {
        return RedisConfig(
            maxMemory = maxMemory,
            maxMemoryPolicy = maxMemoryPolicy,
            persistenceMode = persistenceMode,
            savePolicy = savePolicy,
            appendOnly = appendOnly,
            appendFsync = appendFsync,
            clustering = clustering,
            clusterNodes = clusterNodes
        )
    }
}

// ===== Database User Configuration =====

@HorizonOSDsl
class DatabaseUserContext(private val name: String) {
    var password: String? = null
    var privileges = mutableListOf<String>()
    var databases = mutableListOf<String>()

    fun privilege(privilege: String) {
        privileges.add(privilege)
    }

    fun database(database: String) {
        databases.add(database)
    }

    fun toUser(): DatabaseUser {
        return DatabaseUser(
            name = name,
            password = password,
            privileges = privileges,
            databases = databases
        )
    }
}

// ===== Database Schema Configuration =====

@HorizonOSDsl
class DatabaseSchemaContext(private val name: String) {
    var owner: String? = null
    var encoding: String = "UTF8"
    var collation: String? = null
    var template: String? = null

    fun toSchema(): DatabaseSchema {
        return DatabaseSchema(
            name = name,
            owner = owner,
            encoding = encoding,
            collation = collation,
            template = template
        )
    }
}

// ===== Enums =====

@Serializable
enum class DatabaseType {
    POSTGRESQL, MYSQL, REDIS, MONGODB, SQLITE
}

@Serializable
enum class RedisPersistenceMode {
    RDB, AOF, BOTH, NONE
}

// ===== Data Classes =====

@Serializable
data class DatabaseService(
    val type: DatabaseType,
    val enabled: Boolean,
    val autoStart: Boolean,
    val version: String?,
    val port: Int,
    val dataDirectory: String,
    val configOverrides: Map<String, String>,
    val users: List<DatabaseUser>,
    val databases: List<DatabaseSchema>,
    val postgresConfig: PostgresConfig?,
    val mysqlConfig: MySQLConfig?,
    val redisConfig: RedisConfig?
)

@Serializable
data class PostgresConfig(
    val maxConnections: Int,
    val sharedBuffers: String,
    val effectiveCacheSize: String,
    val maintenanceWorkMem: String,
    val checkpointSegments: Int,
    val checkpointCompletionTarget: Double,
    val walBuffers: String,
    val defaultStatisticsTarget: Int,
    val logMinDurationStatement: Int,
    val logConnections: Boolean,
    val logDisconnections: Boolean,
    val logLockWaits: Boolean,
    val logStatement: String,
    val logLinePrefix: String,
    val extensions: List<String>
)

@Serializable
data class MySQLConfig(
    val maxConnections: Int,
    val innodbBufferPoolSize: String,
    val innodbLogFileSize: String,
    val innodbFlushLogAtTrxCommit: Int,
    val innodbFlushMethod: String,
    val queryCache: Boolean,
    val queryCacheSize: String,
    val tmpTableSize: String,
    val maxHeapTableSize: String
)

@Serializable
data class RedisConfig(
    val maxMemory: String,
    val maxMemoryPolicy: String,
    val persistenceMode: RedisPersistenceMode,
    val savePolicy: String,
    val appendOnly: Boolean,
    val appendFsync: String,
    val clustering: Boolean,
    val clusterNodes: List<String>
)

@Serializable
data class DatabaseUser(
    val name: String,
    val password: String?,
    val privileges: List<String>,
    val databases: List<String>
)

@Serializable
data class DatabaseSchema(
    val name: String,
    val owner: String?,
    val encoding: String,
    val collation: String?,
    val template: String?
)