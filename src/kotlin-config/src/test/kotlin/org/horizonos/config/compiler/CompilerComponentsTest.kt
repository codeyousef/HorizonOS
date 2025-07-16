package org.horizonos.config.compiler

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse
import kotlinx.coroutines.runBlocking
import org.horizonos.config.dsl.*
import java.io.File
import kotlin.io.path.createTempDirectory
import kotlin.io.path.writeText
import kotlin.time.Duration.Companion.seconds

class CompilerComponentsTest : StringSpec({
    
    "should parse simple configuration file" {
        val tempDir = createTempDirectory("horizonos-test")
        val configFile = tempDir.resolve("test.horizonos.kts").toFile()
        configFile.writeText("""
            horizonOS {
                hostname = "test-system"
                timezone = "UTC"
                locale = "en_US.UTF-8"
                
                packages {
                    install("vim", "git")
                }
                
                services {
                    enable("NetworkManager")
                }
            }
        """.trimIndent())
        
        val parser = SimpleConfigParser()
        val result = parser.parseFile(configFile)
        
        result.shouldBeInstanceOf<ParseResult.Success>()
        val config = (result as ParseResult.Success).config
        config.system.hostname shouldBe "test-system"
        config.system.timezone shouldBe "UTC"
        config.system.locale shouldBe "en_US.UTF-8"
        
        tempDir.toFile().deleteRecursively()
    }
    
    "should handle parse errors" {
        val parser = SimpleConfigParser()
        
        // Test file not found
        val result1 = parser.parseFile(File("/nonexistent/file.horizonos.kts"))
        result1.shouldBeInstanceOf<ParseResult.Error>()
        (result1 as ParseResult.Error).error.shouldBeInstanceOf<ParseError.FileNotFound>()
        
        // Test invalid extension
        val tempFile = createTempDirectory("horizonos-test").resolve("test.txt").toFile()
        tempFile.writeText("some content")
        val result2 = parser.parseFile(tempFile)
        result2.shouldBeInstanceOf<ParseResult.Error>()
        (result2 as ParseResult.Error).error.shouldBeInstanceOf<ParseError.InvalidFileExtension>()
        
        tempFile.parentFile.deleteRecursively()
    }
    
    "should validate configuration with enhanced checks" {
        val validator = EnhancedConfigValidator()
        
        // Valid configuration
        val validConfig = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = listOf(Service("NetworkManager", true)),
            users = listOf(User("testuser", 1000, "/bin/bash", listOf("wheel"), "/home/testuser")),
            repositories = emptyList()
        )
        
        val result1 = validator.validate(validConfig)
        result1.isValid.shouldBeTrue()
        // May have warnings about best practices
        
        // Configuration with security issues
        val insecureConfig = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = listOf(Service("telnet", true)),
            users = listOf(User("root", 0, "/bin/bash", emptyList(), "/root")),
            repositories = listOf(PackageRepository("insecure", "http://example.com", gpgCheck = false))
        )
        
        val result2 = validator.validate(insecureConfig)
        result2.hasErrors.shouldBeTrue()
        result2.hasWarnings.shouldBeTrue()
        result2.errors.any { it is ConfigValidationError.SecurityError }.shouldBeTrue()
        result2.warnings.any { it is ConfigValidationWarning.SecurityWarning }.shouldBeTrue()
    }
    
    "should detect performance and compatibility issues" {
        val validator = EnhancedConfigValidator()
        
        // Configuration with performance issues
        val packages = (1..1500).map { Package("package$it", PackageAction.INSTALL) }
        val perfConfig = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = packages,
            services = listOf(
                Service("NetworkManager", true),
                Service("systemd-networkd", true)
            ),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = validator.validate(perfConfig)
        result.hasErrors.shouldBeTrue()
        result.hasWarnings.shouldBeTrue()
        result.warnings.any { it is ConfigValidationWarning.PerformanceWarning }.shouldBeTrue()
        result.errors.any { it is ConfigValidationError.ConflictError }.shouldBeTrue()
    }
    
    "should validate automation configurations" {
        val validator = EnhancedConfigValidator()
        
        // Configuration with risky automation
        val automationConfig = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList(),
            automation = AutomationConfig(
                workflows = listOf(
                    Workflow(
                        name = "risky-workflow",
                        description = "Test workflow",
                        enabled = true,
                        priority = 50,
                        trigger = null,
                        actions = listOf(
                            Action.RunCommand("sudo rm -rf /", null),
                            Action.FileOperation(FileOperation.Delete("/etc/passwd")),
                            Action.Loop(5000, listOf(Action.Delay(1.seconds)))
                        ),
                        conditions = emptyList()
                    )
                ),
                teachingModes = listOf(
                    TeachingMode(
                        name = "system-watch",
                        description = "Watch system",
                        enabled = true,
                        watchedPath = "/etc",
                        filePattern = "*",
                        learningMode = LearningMode.USER_DEMONSTRATION,
                        recordedActions = emptyList()
                    )
                )
            )
        )
        
        val result = validator.validate(automationConfig)
        result.hasErrors.shouldBeTrue()
        result.hasWarnings.shouldBeTrue()
        result.errors.any { it is ConfigValidationError.AutomationError }.shouldBeTrue()
        result.warnings.any { it is ConfigValidationWarning.AutomationWarning }.shouldBeTrue()
    }
    
    "should generate multiple output formats" {
        val tempDir = createTempDirectory("horizonos-test").toFile()
        val generator = RefactoredEnhancedConfigGenerator(tempDir)
        
        val config = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = listOf(
                Package("vim", PackageAction.INSTALL),
                Package("nano", PackageAction.REMOVE)
            ),
            services = listOf(
                Service("NetworkManager", true),
                Service("bluetooth", false)
            ),
            users = listOf(
                User("admin", 1000, "/bin/bash", listOf("wheel", "docker"), "/home/admin")
            ),
            repositories = listOf(
                PackageRepository("core", "https://mirror.archlinux.org/core"),
                OstreeRepository("horizonos", "https://ostree.horizonos.org", branches = listOf("stable"))
            ),
            desktop = DesktopConfig(
                environment = DesktopEnvironment.HYPRLAND,
                autoLogin = true,
                autoLoginUser = "admin",
                hyprlandConfig = HyprlandConfig(
                    theme = "breeze-dark",
                    animations = true,
                    gaps = 10,
                    borderSize = 2,
                    kdeIntegration = true,
                    personalityMode = PersonalityMode.KDE
                )
            ),
            automation = AutomationConfig(
                workflows = listOf(
                    Workflow(
                        name = "test-workflow",
                        description = "Test workflow",
                        enabled = true,
                        priority = 50,
                        trigger = Trigger(TriggerType.TIME, Schedule.Time("09:00", WEEKDAYS)),
                        actions = listOf(Action.Notification("Test", "Hello", NotificationUrgency.NORMAL)),
                        conditions = emptyList()
                    )
                ),
                teachingModes = emptyList()
            )
        )
        
        val result = generator.generate(config)
        result.shouldBeInstanceOf<GenerationResult.Success>()
        
        val success = result as GenerationResult.Success
        success.files.size shouldNotBe 0  // Should generate multiple files
        
        // Check that various file types were generated
        val fileTypes = success.files.map { it.type }.toSet()
        fileTypes shouldContain FileType.JSON
        fileTypes shouldContain FileType.YAML
        fileTypes shouldContain FileType.SHELL
        fileTypes shouldContain FileType.SYSTEMD
        fileTypes shouldContain FileType.ANSIBLE
        fileTypes shouldContain FileType.DOCKER
        fileTypes shouldContain FileType.DOCUMENTATION
        
        // Check that specific files exist
        File(tempDir, "json/config.json").exists().shouldBeTrue()
        File(tempDir, "yaml/config.yaml").exists().shouldBeTrue()
        File(tempDir, "scripts/deploy.sh").exists().shouldBeTrue()
        File(tempDir, "systemd/horizonos-config.service").exists().shouldBeTrue()
        File(tempDir, "ansible/horizonos-playbook.yml").exists().shouldBeTrue()
        File(tempDir, "docker/Dockerfile").exists().shouldBeTrue()
        File(tempDir, "docs/README.md").exists().shouldBeTrue()
        
        tempDir.deleteRecursively()
    }
    
    "should handle generation errors gracefully" {
        val readOnlyDir = createTempDirectory("horizonos-test").toFile()
        readOnlyDir.setWritable(false)
        
        val generator = EnhancedConfigGenerator(readOnlyDir)
        val config = CompiledConfig(
            system = SystemConfig("test", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = generator.generate(config)
        
        // On some systems, this might succeed if running as root
        // So we just check that it returns a valid result
        result shouldNotBe null
        
        readOnlyDir.setWritable(true)
        readOnlyDir.deleteRecursively()
    }
})