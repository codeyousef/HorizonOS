package org.horizonos.config.validation

import org.horizonos.config.validation.validators.*
import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.collections.shouldBeEmpty
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.assertions.throwables.shouldThrow
import org.horizonos.config.dsl.*

class ValidatorsTest : StringSpec({
    
    "should validate correct system configuration" {
        val config = CompiledConfig(
            system = SystemConfig("valid-hostname", "America/New_York", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe true
        result.errors.shouldBeEmpty()
    }
    
    "should reject invalid hostname" {
        val config = CompiledConfig(
            system = SystemConfig("invalid.hostname!", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.InvalidHostname>()
    }
    
    "should reject invalid timezone" {
        val config = CompiledConfig(
            system = SystemConfig("valid-hostname", "Invalid/Timezone!", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.InvalidTimezone>()
    }
    
    "should reject invalid locale" {
        val config = CompiledConfig(
            system = SystemConfig("valid-hostname", "UTC", "invalid-locale"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.InvalidLocale>()
    }
    
    "should detect conflicting package actions" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = listOf(
                Package("vim", PackageAction.INSTALL),
                Package("vim", PackageAction.REMOVE)
            ),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.ConflictingPackages>()
    }
    
    "should reject invalid package names" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = listOf(
                Package("invalid package name!", PackageAction.INSTALL),
                Package("", PackageAction.INSTALL)
            ),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidPackageName } shouldBe true
    }
    
    "should detect duplicate services" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = listOf(
                Service("NetworkManager", true),
                Service("NetworkManager", false)
            ),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.DuplicateService>()
    }
    
    "should reject invalid service names" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = listOf(
                Service("invalid service name!", true),
                Service("", false)
            ),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidServiceName } shouldBe true
    }
    
    "should detect duplicate users" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("admin", 1000, "/bin/bash", listOf("wheel"), "/home/admin"),
                User("admin", 1001, "/bin/zsh", listOf("users"), "/home/admin")
            ),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.DuplicateUser>()
    }
    
    "should reject invalid usernames" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("Invalid User!", null, "/bin/bash", emptyList(), "/home/invalid"),
                User("123user", null, "/bin/bash", emptyList(), "/home/123user")
            ),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidUsername } shouldBe true
    }
    
    "should reject invalid UIDs" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("user1", 0, "/bin/bash", emptyList(), "/home/user1"),
                User("user2", 70000, "/bin/bash", emptyList(), "/home/user2")
            ),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidUID } shouldBe true
    }
    
    "should reject invalid shells" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("user1", null, "bash", emptyList(), "/home/user1"),
                User("user2", null, "", emptyList(), "/home/user2")
            ),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidShell } shouldBe true
    }
    
    "should reject invalid group names" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("user1", null, "/bin/bash", listOf("Invalid Group!", "123group"), "/home/user1")
            ),
            repositories = emptyList()
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidGroupName } shouldBe true
    }
    
    "should detect duplicate repositories" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = listOf(
                PackageRepository("core", "https://example.com/core"),
                PackageRepository("core", "https://example.com/core2")
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.DuplicateRepository>()
    }
    
    "should reject invalid repository names" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = listOf(
                PackageRepository("invalid repo!", "https://example.com"),
                PackageRepository("", "https://example.com")
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidRepositoryName } shouldBe true
    }
    
    "should reject invalid repository URLs" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = listOf(
                PackageRepository("repo1", "invalid-url"),
                PackageRepository("repo2", "ftp://example.com")
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidUrl } shouldBe true
    }
    
    "should reject invalid branch names" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = listOf(
                OstreeRepository("ostree", "https://example.com", branches = listOf("invalid branch!", ""))
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 2
        result.errors.all { it is ValidationError.InvalidBranch } shouldBe true
    }
    
    "should detect missing auto-login user" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("user1", null, "/bin/bash", emptyList(), "/home/user1")
            ),
            repositories = emptyList(),
            desktop = DesktopConfig(
                environment = DesktopEnvironment.PLASMA,
                autoLogin = true,
                autoLoginUser = "nonexistent"
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe false
        result.errors shouldHaveSize 1
        result.errors[0].shouldBeInstanceOf<ValidationError.MissingAutoLoginUser>()
    }
    
    "should validate desktop configuration correctly" {
        val config = CompiledConfig(
            system = SystemConfig("hostname", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = listOf(
                User("admin", null, "/bin/bash", emptyList(), "/home/admin")
            ),
            repositories = emptyList(),
            desktop = DesktopConfig(
                environment = DesktopEnvironment.PLASMA,
                autoLogin = true,
                autoLoginUser = "admin",
                plasmaConfig = PlasmaConfig("breeze", "org.kde.breeze", emptyList())
            )
        )
        
        val result = ConfigurationValidator.validate(config)
        result.isValid shouldBe true
        result.errors.shouldBeEmpty()
    }
    
    "should throw ValidationException when using horizonOS DSL with invalid config" {
        shouldThrow<ValidationException> {
            horizonOS {
                hostname = "invalid.hostname!"
                timezone = "UTC"
                locale = "en_US.UTF-8"
            }
        }
    }
    
    "should include multiple errors in validation exception" {
        val exception = shouldThrow<ValidationException> {
            horizonOS {
                hostname = "invalid.hostname!"
                timezone = "Invalid/Timezone!"
                locale = "invalid-locale"
            }
        }
        
        exception.errors shouldHaveSize 3
        exception.errors.map { it::class } shouldContain ValidationError.InvalidHostname::class
        exception.errors.map { it::class } shouldContain ValidationError.InvalidTimezone::class
        exception.errors.map { it::class } shouldContain ValidationError.InvalidLocale::class
    }
})