package org.horizonos.config.runtime

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.matchers.collections.shouldHaveSize
import kotlinx.coroutines.runBlocking
import org.horizonos.config.dsl.*
import java.nio.file.Path
import java.nio.file.Paths
import kotlin.io.path.createTempDirectory

class ExecutionEngineTest : StringSpec({
    
    "should create execution engine with default paths" {
        val engine = ExecutionEngine(dryRun = true)
        engine shouldNotBe null
    }
    
    "should create execution engine with custom paths" {
        val tempDir = createTempDirectory("horizonos-test")
        val engine = ExecutionEngine(
            ostreeRepo = tempDir.resolve("repo"),
            systemRoot = tempDir.resolve("system"),
            configRoot = tempDir.resolve("config"),
            dryRun = true
        )
        
        engine shouldNotBe null
    }
    
    "should validate configuration successfully" {
        val tempDir = createTempDirectory("horizonos-test")
        val engine = ExecutionEngine(
            ostreeRepo = tempDir.resolve("repo"),
            systemRoot = tempDir.resolve("system"),
            configRoot = tempDir.resolve("config"),
            dryRun = true
        )
        
        // Create mock OSTree repo
        tempDir.resolve("repo").toFile().mkdirs()
        
        val config = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = listOf(Service("NetworkManager", true)),
            users = listOf(User("testuser", null, "/bin/bash", emptyList(), "/home/testuser")),
            repositories = emptyList()
        )
        
        runBlocking {
            val result = engine.validateConfiguration(config)
            result.shouldBeInstanceOf<ValidationResult.Valid>()
        }
    }
    
    "should detect validation errors" {
        val tempDir = createTempDirectory("horizonos-test")
        val engine = ExecutionEngine(
            ostreeRepo = tempDir.resolve("nonexistent"),
            systemRoot = tempDir.resolve("system"),
            configRoot = tempDir.resolve("config"),
            dryRun = true
        )
        
        val config = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        runBlocking {
            val result = engine.validateConfiguration(config)
            result.shouldBeInstanceOf<ValidationResult.Invalid>()
            val invalid = result as ValidationResult.Invalid
            invalid.errors shouldHaveSize 1
            invalid.errors[0].shouldBeInstanceOf<ExecutionError.OSTreeError>()
        }
    }
    
    "should handle command execution in dry run mode" {
        val executor = CommandExecutor(dryRun = true)
        
        runBlocking {
            val result = executor.execute("echo", "test")
            result shouldBe ""
        }
    }
    
    "should create ostree manager" {
        val tempDir = createTempDirectory("horizonos-test")
        val executor = CommandExecutor(dryRun = true)
        val manager = OstreeManager(tempDir.resolve("repo"), executor)
        
        manager shouldNotBe null
    }
    
    "should create system manager" {
        val tempDir = createTempDirectory("horizonos-test")
        val executor = CommandExecutor(dryRun = true)
        val manager = SystemManager(tempDir.resolve("system"), executor)
        
        manager shouldNotBe null
    }
    
    "should create config manager" {
        val tempDir = createTempDirectory("horizonos-test")
        val executor = CommandExecutor(dryRun = true)
        val manager = ConfigManager(tempDir.resolve("config"), executor)
        
        manager shouldNotBe null
    }
    
    "should handle execution results" {
        val operations = listOf(
            ExecutionOperation.OSTreeCommit("Test commit"),
            ExecutionOperation.SystemConfig("Test config")
        )
        
        val successResult = ExecutionResult.Success(operations, "commit123")
        successResult.operations shouldHaveSize 2
        successResult.commitId shouldBe "commit123"
        
        val errors = listOf(ExecutionError.OSTreeError("Test error"))
        val failureResult = ExecutionResult.Failure(operations, errors)
        failureResult.operations shouldHaveSize 2
        failureResult.errors shouldHaveSize 1
    }
    
    "should handle execution operations" {
        val commitOp = ExecutionOperation.OSTreeCommit("Creating commit")
        commitOp.description shouldBe "Creating commit"
        
        val deployOp = ExecutionOperation.OSTreeDeploy("Deploying commit")
        deployOp.description shouldBe "Deploying commit"
        
        val rollbackOp = ExecutionOperation.OSTreeRollback("Rolling back")
        rollbackOp.description shouldBe "Rolling back"
        
        val systemOp = ExecutionOperation.SystemConfig("System config")
        systemOp.description shouldBe "System config"
        
        val packageOp = ExecutionOperation.PackageManagement("Package management")
        packageOp.description shouldBe "Package management"
        
        val serviceOp = ExecutionOperation.ServiceConfig("Service config")
        serviceOp.description shouldBe "Service config"
        
        val userOp = ExecutionOperation.UserManagement("User management")
        userOp.description shouldBe "User management"
        
        val repoOp = ExecutionOperation.RepositoryConfig("Repository config")
        repoOp.description shouldBe "Repository config"
        
        val desktopOp = ExecutionOperation.DesktopConfig("Desktop config")
        desktopOp.description shouldBe "Desktop config"
        
        val automationOp = ExecutionOperation.AutomationConfig("Automation config")
        automationOp.description shouldBe "Automation config"
    }
    
    "should handle execution errors" {
        val ostreeError = ExecutionError.OSTreeError("OSTree failed")
        ostreeError.message shouldBe "OSTree error: OSTree failed"
        
        val packageError = ExecutionError.PackageNotFound("missing-package")
        packageError.message shouldBe "Package not found: missing-package"
        
        val permissionError = ExecutionError.PermissionError("Access denied")
        permissionError.message shouldBe "Permission error: Access denied"
        
        val rollbackError = ExecutionError.RollbackFailed("Cannot rollback")
        rollbackError.message shouldBe "Rollback failed: Cannot rollback"
        
        val unexpectedError = ExecutionError.UnexpectedError("Something went wrong")
        unexpectedError.message shouldBe "Unexpected error: Something went wrong"
    }
    
    "should handle system status" {
        val systemInfo = SystemInfo(
            uptime = "up 1 day, 2 hours",
            kernelVersion = "6.1.0-arch1-1",
            memoryInfo = "Total: 16GB, Used: 8GB"
        )
        
        val status = SystemStatus(
            currentCommit = "abc123",
            availableCommits = listOf("abc123", "def456", "ghi789"),
            systemInfo = systemInfo
        )
        
        status.currentCommit shouldBe "abc123"
        status.availableCommits shouldHaveSize 3
        status.systemInfo.uptime shouldBe "up 1 day, 2 hours"
        status.systemInfo.kernelVersion shouldBe "6.1.0-arch1-1"
        status.systemInfo.memoryInfo shouldBe "Total: 16GB, Used: 8GB"
    }
    
    "should handle validation results" {
        val validResult = ValidationResult.Valid
        validResult.shouldBeInstanceOf<ValidationResult.Valid>()
        
        val errors = listOf(ExecutionError.OSTreeError("Test error"))
        val invalidResult = ValidationResult.Invalid(errors)
        invalidResult.shouldBeInstanceOf<ValidationResult.Invalid>()
        invalidResult.errors shouldHaveSize 1
    }
})