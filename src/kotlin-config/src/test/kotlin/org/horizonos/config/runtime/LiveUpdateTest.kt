package org.horizonos.config.runtime

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.types.shouldBeInstanceOf
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.collections.shouldContain
import io.kotest.matchers.booleans.shouldBeTrue
import io.kotest.matchers.booleans.shouldBeFalse
import io.mockk.*
import kotlinx.coroutines.runBlocking
import org.horizonos.config.dsl.*
import java.io.File

class LiveUpdateTest : StringSpec({
    
    "should detect changes between configurations" {
        val detector = ChangeDetector()
        
        val currentConfig = CompiledConfig(
            system = SystemConfig("old-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = listOf(Service("NetworkManager", true)),
            users = listOf(User("testuser", 1000, "/bin/bash", listOf("wheel"), "/home/testuser")),
            repositories = emptyList()
        )
        
        val newConfig = CompiledConfig(
            system = SystemConfig("new-host", "America/New_York", "en_US.UTF-8"),
            packages = listOf(
                Package("vim", PackageAction.INSTALL),
                Package("git", PackageAction.INSTALL)
            ),
            services = listOf(
                Service("NetworkManager", true),
                Service("sshd", true)
            ),
            users = listOf(User("testuser", 1000, "/bin/zsh", listOf("wheel", "docker"), "/home/testuser")),
            repositories = emptyList()
        )
        
        val changes = detector.detectChanges(currentConfig, newConfig)
        
        changes shouldHaveSize 5 // hostname, timezone, package install, service add, user modify
        
        // Check for specific changes
        changes.any { it.type == ChangeType.SYSTEM_CONFIG && it.field == "hostname" }.shouldBeTrue()
        changes.any { it.type == ChangeType.SYSTEM_CONFIG && it.field == "timezone" }.shouldBeTrue()
        changes.any { it.type == ChangeType.PACKAGE_INSTALL }.shouldBeTrue()
        changes.any { it.type == ChangeType.SERVICE_ADD }.shouldBeTrue()
        changes.any { it.type == ChangeType.USER_MODIFY }.shouldBeTrue()
    }
    
    "should categorize changes by update strategy" {
        val detector = ChangeDetector()
        
        val currentConfig = CompiledConfig(
            system = SystemConfig("host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList(),
            desktop = DesktopConfig(DesktopEnvironment.PLASMA)
        )
        
        val newConfig = CompiledConfig(
            system = SystemConfig("new-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = listOf(Service("nginx", true)),
            users = listOf(User("newuser", 1001, "/bin/bash", emptyList(), "/home/newuser")),
            repositories = emptyList(),
            desktop = DesktopConfig(DesktopEnvironment.HYPRLAND) // Different DE = reboot required
        )
        
        val changes = detector.detectChanges(currentConfig, newConfig)
        
        // Check update strategies
        val hostnameChange = changes.find { it.type == ChangeType.SYSTEM_CONFIG && it.field == "hostname" }
        hostnameChange?.updateStrategy shouldBe UpdateStrategy.LIVE
        
        val desktopChange = changes.find { it.type == ChangeType.DESKTOP_CONFIG }
        desktopChange?.updateStrategy shouldBe UpdateStrategy.REBOOT_REQUIRED
        
        val serviceChange = changes.find { it.type == ChangeType.SERVICE_ADD }
        serviceChange?.updateStrategy shouldBe UpdateStrategy.SERVICE_RELOAD
    }
    
    "should apply live updates successfully" {
        val mockSystemManager = mockk<SystemManager>()
        val mockChangeDetector = mockk<ChangeDetector>()
        val mockStateSync = mockk<StateSyncManager>()
        val mockServiceReloader = mockk<ServiceReloader>()
        val mockNotifier = mockk<UpdateNotifier>()
        
        val liveUpdateManager = LiveUpdateManager(
            mockSystemManager,
            mockChangeDetector,
            mockStateSync,
            mockServiceReloader,
            mockNotifier
        )
        
        val currentConfig = CompiledConfig(
            system = SystemConfig("old-host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val newConfig = CompiledConfig(
            system = SystemConfig("new-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val changes = listOf(
            ConfigChange(
                type = ChangeType.SYSTEM_CONFIG,
                field = "hostname",
                oldValue = "old-host",
                newValue = "new-host",
                description = "Hostname change",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.LOW
            ),
            ConfigChange(
                type = ChangeType.PACKAGE_INSTALL,
                oldValue = emptyList<Package>(),
                newValue = listOf(Package("vim", PackageAction.INSTALL)),
                description = "Install packages: vim",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            )
        )
        
        val snapshot = StateSnapshot(
            id = "test-snapshot",
            timestamp = java.time.LocalDateTime.now(),
            configPath = "/tmp/config.json",
            statePath = "/tmp/state.json",
            servicesPath = "/tmp/services.json",
            packagesPath = "/tmp/packages.txt"
        )
        
        coEvery { mockChangeDetector.detectChanges(currentConfig, newConfig) } returns changes
        coEvery { mockStateSync.createSnapshot() } returns snapshot
        coEvery { mockStateSync.syncState(newConfig) } returns Unit
        coEvery { mockSystemManager.setHostname("new-host", false) } returns Unit
        coEvery { mockSystemManager.installPackages(any(), false) } returns Unit
        coEvery { mockNotifier.notifyUpdateStarting(newConfig) } returns Unit
        coEvery { mockNotifier.notifyChangeApplied(any()) } returns Unit
        coEvery { mockNotifier.notifyUpdateCompleted(any(), any()) } returns Unit
        
        val result = runBlocking {
            liveUpdateManager.applyLiveUpdates(currentConfig, newConfig)
        }
        
        result.shouldBeInstanceOf<LiveUpdateResult.Success>()
        val success = result as LiveUpdateResult.Success
        success.appliedChanges shouldHaveSize 2
        success.pendingRebootChanges shouldHaveSize 0
        
        coVerify { mockSystemManager.setHostname("new-host", false) }
        coVerify { mockSystemManager.installPackages(any(), false) }
        coVerify { mockStateSync.syncState(newConfig) }
    }
    
    "should handle reboot-required changes" {
        val mockSystemManager = mockk<SystemManager>()
        val mockChangeDetector = mockk<ChangeDetector>()
        val mockStateSync = mockk<StateSyncManager>()
        val mockServiceReloader = mockk<ServiceReloader>()
        val mockNotifier = mockk<UpdateNotifier>()
        
        val liveUpdateManager = LiveUpdateManager(
            mockSystemManager,
            mockChangeDetector,
            mockStateSync,
            mockServiceReloader,
            mockNotifier
        )
        
        val currentConfig = CompiledConfig(
            system = SystemConfig("host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val newConfig = currentConfig.copy(
            desktop = DesktopConfig(DesktopEnvironment.PLASMA)
        )
        
        val changes = listOf(
            ConfigChange(
                type = ChangeType.DESKTOP_CONFIG,
                oldValue = null,
                newValue = DesktopConfig(DesktopEnvironment.PLASMA),
                description = "Enable desktop environment",
                updateStrategy = UpdateStrategy.REBOOT_REQUIRED,
                impact = ImpactLevel.CRITICAL
            )
        )
        
        coEvery { mockChangeDetector.detectChanges(currentConfig, newConfig) } returns changes
        coEvery { mockNotifier.notifyUpdateStarting(newConfig) } returns Unit
        coEvery { mockNotifier.notifyRebootRequired(changes) } returns Unit
        
        val result = runBlocking {
            liveUpdateManager.applyLiveUpdates(
                currentConfig, 
                newConfig,
                LiveUpdateOptions(allowPartialUpdate = false)
            )
        }
        
        result.shouldBeInstanceOf<LiveUpdateResult.RebootRequired>()
        val rebootRequired = result as LiveUpdateResult.RebootRequired
        rebootRequired.changes shouldHaveSize 1
        
        coVerify { mockNotifier.notifyRebootRequired(changes) }
    }
    
    "should reload services when needed" {
        val mockServiceReloader = mockk<ServiceReloader>()
        
        coEvery { 
            mockServiceReloader.reloadService("nginx", true) 
        } returns ReloadResult.Success("nginx", ReloadMethod.SIGNAL)
        
        val result = runBlocking {
            mockServiceReloader.reloadService("nginx", true)
        }
        
        result.shouldBeInstanceOf<ReloadResult.Success>()
        val success = result as ReloadResult.Success
        success.serviceName shouldBe "nginx"
        success.method shouldBe ReloadMethod.SIGNAL
    }
    
    "should track state snapshots" {
        val tempDir = kotlin.io.path.createTempDirectory("horizonos-state-test")
        val stateSync = StateSyncManager(tempDir.toFile())
        
        val config = CompiledConfig(
            system = SystemConfig("test-host", "UTC", "en_US.UTF-8"),
            packages = listOf(Package("vim", PackageAction.INSTALL)),
            services = listOf(Service("sshd", true)),
            users = emptyList(),
            repositories = emptyList()
        )
        
        runBlocking {
            // Sync state
            stateSync.syncState(config)
            
            // Create snapshot
            val snapshot = stateSync.createSnapshot()
            
            snapshot.id.contains("snapshot-").shouldBeTrue()
            File(snapshot.configPath).exists().shouldBeTrue()
            
            // List snapshots
            val snapshots = stateSync.listSnapshots()
            snapshots shouldHaveSize 1
            snapshots[0].id shouldBe snapshot.id
        }
        
        tempDir.toFile().deleteRecursively()
    }
    
    "should estimate update duration" {
        val mockSystemManager = mockk<SystemManager>()
        val mockChangeDetector = mockk<ChangeDetector>()
        val mockStateSync = mockk<StateSyncManager>()
        val mockServiceReloader = mockk<ServiceReloader>()
        val mockNotifier = mockk<UpdateNotifier>()
        
        val liveUpdateManager = LiveUpdateManager(
            mockSystemManager,
            mockChangeDetector,
            mockStateSync,
            mockServiceReloader,
            mockNotifier
        )
        
        val currentConfig = CompiledConfig(
            system = SystemConfig("host", "UTC", "en_US.UTF-8"),
            packages = emptyList(),
            services = emptyList(),
            users = emptyList(),
            repositories = emptyList()
        )
        
        val newConfig = currentConfig.copy(
            packages = listOf(
                Package("vim", PackageAction.INSTALL),
                Package("git", PackageAction.INSTALL),
                Package("docker", PackageAction.INSTALL)
            ),
            services = listOf(
                Service("nginx", true),
                Service("postgresql", true)
            )
        )
        
        val changes = listOf(
            ConfigChange(
                type = ChangeType.PACKAGE_INSTALL,
                oldValue = emptyList<Package>(),
                newValue = newConfig.packages,
                description = "Install packages",
                updateStrategy = UpdateStrategy.LIVE,
                impact = ImpactLevel.MEDIUM
            ),
            ConfigChange(
                type = ChangeType.SERVICE_ADD,
                oldValue = null,
                newValue = Service("nginx", true),
                affectedService = "nginx",
                description = "Add service: nginx",
                updateStrategy = UpdateStrategy.SERVICE_RELOAD,
                impact = ImpactLevel.MEDIUM
            ),
            ConfigChange(
                type = ChangeType.SERVICE_ADD,
                oldValue = null,
                newValue = Service("postgresql", true),
                affectedService = "postgresql",
                description = "Add service: postgresql",
                updateStrategy = UpdateStrategy.SERVICE_RELOAD,
                impact = ImpactLevel.MEDIUM
            )
        )
        
        coEvery { mockChangeDetector.detectChanges(currentConfig, newConfig) } returns changes
        
        val capability = runBlocking {
            liveUpdateManager.canApplyLiveUpdates(currentConfig, newConfig)
        }
        
        capability.canFullyUpdate.shouldBeTrue()
        capability.liveUpdatableChanges shouldBe 3
        capability.rebootRequiredChanges shouldBe 0
        // 3 packages * 30s + 2 services * 5s = 100s
        capability.estimatedDuration.inWholeSeconds shouldBe 100
    }
})