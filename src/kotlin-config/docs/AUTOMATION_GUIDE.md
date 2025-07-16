# HorizonOS Automation Guide

This guide covers the automation capabilities of the HorizonOS Kotlin DSL, including workflows, teaching modes, and advanced automation patterns.

## Overview

The HorizonOS automation system enables you to create sophisticated workflows that respond to system events, schedule tasks, and learn from user behavior. It's designed for both simple scheduled tasks and complex RPA (Robotic Process Automation) scenarios.

## Basic Concepts

### Workflows

Workflows are automated sequences of actions triggered by specific events:

```kotlin
automation {
    workflow("daily-update") {
        description = "Update system packages daily"
        priority = 50
        
        trigger {
            time("02:00")
            onDays(DAILY)
        }
        
        actions {
            runCommand("pacman -Syu --noconfirm")
            notification("Update Complete", "System updated successfully")
        }
    }
}
```

### Triggers

Triggers define when workflows execute:

- **Time-based**: `time("HH:mm")`, `interval(duration)`
- **Event-based**: `fileModified()`, `processStarted()`, `systemEvent()`
- **Conditional**: `diskSpaceBelow()`, `batteryLevel()`, `systemIdle()`

### Actions

Actions define what the workflow does:

- **System commands**: `runCommand()`, `scriptBlock()`
- **File operations**: `fileOperation()`, `move()`, `copy()`, `delete()`
- **Notifications**: `notification()`, `email()`
- **Browser automation**: `browserOpen()`, `browserClick()`
- **AI tasks**: `aiTask()`

## Trigger Types

### Time-Based Triggers

```kotlin
trigger {
    // Specific time daily
    time("09:00")
    onDays(WEEKDAYS)
    
    // Multiple times
    times("08:00", "12:00", "18:00")
    
    // Intervals
    interval(30.minutes)
    
    // Complex schedules
    schedule {
        monthly(dayOfMonth = 1, time = "00:00")
        yearly(month = 12, dayOfMonth = 31, time = "23:59")
    }
}
```

### File System Triggers

```kotlin
trigger {
    // File modifications
    fileModified("/home/user/Documents/config.txt")
    fileCreated("~/Downloads/*.pdf")
    fileDeleted("~/Trash/*")
    
    // Directory changes
    directoryModified("/etc/nginx/sites-enabled/")
    
    // Pattern matching
    fileModified("**/*.log")  // Any .log file recursively
}
```

### System Event Triggers

```kotlin
trigger {
    // System lifecycle
    systemEvent(SystemEvent.BOOT_COMPLETE)
    systemEvent(SystemEvent.SHUTDOWN_INITIATED)
    systemEvent(SystemEvent.USER_LOGIN)
    systemEvent(SystemEvent.USER_LOGOUT)
    systemEvent(SystemEvent.SUSPEND)
    systemEvent(SystemEvent.RESUME)
    
    // Process events
    processStarted("firefox", "steam")
    processExited("steam")
    
    // Hardware events
    deviceConnected("usb")
    networkConnected("WiFi")
    
    // Calendar integration
    calendarEvent("meeting", 5.minutes.before)
}
```

### Conditional Triggers

```kotlin
trigger {
    // System resources
    diskSpaceBelow("/", 10.GB)
    memoryUsageAbove(90.percent)
    cpuUsageAbove(80.percent, duration = 5.minutes)
    
    // Power management
    batteryLevel(20.percent)
    powerConnected()
    powerDisconnected()
    
    // User activity
    systemIdle(30.minutes)
    userActive()
    
    // Network conditions
    internetConnected()
    wifiConnected("HomeNetwork")
}
```

## Action Types

### System Commands

```kotlin
actions {
    // Simple commands
    runCommand("pacman -Syu --noconfirm")
    runCommand("systemctl restart nginx")
    
    // Commands with environment
    runCommand("docker build -t myapp .") {
        workingDirectory = "/home/user/projects/myapp"
        environment["DOCKER_BUILDKIT"] = "1"
        timeout = 10.minutes
    }
    
    // Script blocks for complex logic
    scriptBlock {
        """
        #!/bin/bash
        if [ -f /tmp/update.lock ]; then
            echo "Update already running"
            exit 1
        fi
        
        touch /tmp/update.lock
        pacman -Syu --noconfirm
        rm /tmp/update.lock
        """
    }
    
    // Conditional execution
    conditional {
        if (fileExists("/etc/docker/daemon.json")) {
            runCommand("systemctl restart docker")
        }
    }
}
```

### File Operations

```kotlin
actions {
    fileOperation {
        // Copy files
        copy {
            from = "~/Documents/important/*"
            to = "/backup/documents/"
            preserveTimestamps = true
            createTargetDir = true
        }
        
        // Move with pattern replacement
        move {
            from = "~/Screenshots/*.png"
            to = "~/Pictures/Screenshots/{{date:yyyy-MM}}/{{filename}}"
            createTargetDir = true
        }
        
        // Delete old files
        delete {
            path = "~/Downloads/*"
            olderThan = 30.days
            exclude = listOf("*.pdf", "*.txt")
        }
        
        // Create directories
        createDirectory("~/Projects/{{date:yyyy}}/{{project_name}}")
        
        // Compress archives
        compress {
            source = "~/Documents/project"
            target = "~/Backups/project-{{date:yyyyMMdd}}.tar.gz"
            format = CompressionFormat.TAR_GZ
        }
    }
}
```

### Browser Automation

```kotlin
actions {
    browserAutomation {
        // Open URLs
        browserOpen("https://github.com")
        
        // Navigate and interact
        browser("firefox") {
            navigate("https://example.com")
            
            // Wait for elements
            waitForElement("input[name='username']")
            
            // Fill forms
            fillText("input[name='username']", "myusername")
            fillText("input[name='password']", "mypassword")
            
            // Click elements
            click("button[type='submit']")
            
            // Wait for navigation
            waitForUrl("https://example.com/dashboard")
            
            // Extract data
            extractText(".welcome-message") { text ->
                notification("Login Success", text)
            }
            
            // Take screenshot
            screenshot("~/Pictures/login-success.png")
        }
    }
}
```

### AI Integration

```kotlin
actions {
    aiTask {
        model = ModelSize.MEDIUM
        prompt = """
            Analyze the following log file and summarize any errors:
            ${fileContent("/var/log/system.log")}
        """
        
        onResult { summary ->
            if (summary.contains("ERROR")) {
                email("admin@example.com", "System Errors Detected", summary)
            }
        }
    }
    
    // AI-powered file organization
    aiFileOrganizer {
        sourcePath = "~/Downloads"
        prompt = "Organize these files by type and purpose"
        
        onSuggestion { suggestions ->
            // Review suggestions before applying
            notification("File Organization", "AI suggests: ${suggestions.size} changes")
        }
    }
}
```

### Notifications and Communication

```kotlin
actions {
    // Desktop notifications
    notification("Title", "Message") {
        urgency = NotificationUrgency.NORMAL
        timeout = 5.seconds
        icon = "/path/to/icon.png"
    }
    
    // Email notifications
    email("user@example.com", "Subject", "Body") {
        attachments = listOf("~/report.pdf")
        importance = EmailImportance.HIGH
    }
    
    // SMS (if configured)
    sms("+1234567890", "Important system alert")
    
    // Slack/Discord webhooks
    webhook("https://hooks.slack.com/...") {
        payload = mapOf(
            "text" to "System update completed",
            "channel" to "#notifications"
        )
    }
}
```

## Conditions

Add conditions to control workflow execution:

```kotlin
workflow("backup") {
    trigger {
        time("23:00")
        onDays(DAILY)
    }
    
    conditions {
        // System conditions
        diskSpaceAvailable("/backup", 50.GB)
        batteryLevel(30.percent)
        systemIdle(10.minutes)
        
        // Time windows
        timeWindow("22:00", "06:00")
        
        // File conditions
        fileExists("/backup/config.json")
        pathExists("/mnt/external-backup")
        
        // Process conditions
        processNotRunning("backup-tool")
        
        // Network conditions
        internetConnected()
        pingSuccessful("backup-server.com")
        
        // Custom conditions
        customCondition { 
            // Return true/false based on custom logic
            System.currentTimeMillis() % 2 == 0L
        }
    }
    
    actions {
        // Backup logic
    }
}
```

## Teaching Mode

The teaching mode allows the system to learn from user demonstrations:

```kotlin
automation {
    teach("email-processing") {
        description = "Learn email processing workflow"
        enabled = true
        
        // What to watch
        watchPath = "/home/user/.thunderbird"
        watchApplications = listOf("thunderbird", "firefox")
        
        // Learning configuration
        learningMode = LearningMode.USER_DEMONSTRATION
        maxRecordingTime = 30.minutes
        
        // Actions to track
        trackActions = listOf(
            ActionType.MOUSE_CLICK,
            ActionType.KEYBOARD_INPUT,
            ActionType.WINDOW_SWITCH,
            ActionType.FILE_OPERATION
        )
        
        // When learning is complete
        onLearned { workflow ->
            workflow.name = "auto-email-processing"
            workflow.description = "Automatically process emails based on learned behavior"
            
            // Set up trigger for the learned workflow
            workflow.trigger {
                fileModified("/home/user/.thunderbird/*/INBOX")
            }
            
            // The system automatically generates actions based on observed behavior
        }
        
        // Manual teaching triggers
        startTrigger {
            keyboardShortcut("Ctrl+Alt+T")
        }
        
        stopTrigger {
            keyboardShortcut("Ctrl+Alt+S")
        }
    }
}
```

## Advanced Patterns

### Workflow Chaining

```kotlin
workflow("build-and-deploy") {
    description = "Build application and deploy"
    
    trigger {
        fileModified("src/**/*.kt")
        delay(30.seconds)  // Debounce multiple changes
    }
    
    actions {
        // Build
        runCommand("./gradlew build") {
            onSuccess {
                // Only deploy if build succeeds
                triggerWorkflow("deploy-to-staging")
            }
            onFailure {
                notification("Build Failed", "Check build logs")
            }
        }
    }
}

workflow("deploy-to-staging") {
    actions {
        runCommand("docker build -t myapp:latest .")
        runCommand("docker push myapp:latest")
        notification("Deploy Complete", "Deployed to staging")
    }
}
```

### State Management

```kotlin
workflow("backup-rotation") {
    trigger {
        time("01:00")
        onDays(DAILY)
    }
    
    actions {
        // Read state
        val lastBackupDate = readState("last_backup_date")
        val backupCount = readState("backup_count", "0").toInt()
        
        if (shouldCreateBackup(lastBackupDate)) {
            runCommand("create-backup.sh")
            
            // Update state
            setState("last_backup_date", currentDate())
            setState("backup_count", (backupCount + 1).toString())
            
            // Clean old backups
            if (backupCount > 7) {
                runCommand("clean-old-backups.sh")
                setState("backup_count", "7")
            }
        }
    }
}
```

### Error Handling

```kotlin
workflow("system-update") {
    actions {
        runCommand("pacman -Syu --noconfirm")
    }
    
    onError { error ->
        // Retry logic
        if (error.isRecoverable) {
            delay(5.minutes)
            retry(maxAttempts = 3)
        } else {
            // Escalate
            email("admin@example.com", "Update Failed", error.message)
            notification("Update Error", "Manual intervention required")
        }
    }
    
    onSuccess {
        notification("Update Complete", "System updated successfully")
        
        // Reboot if kernel updated
        if (kernelUpdated()) {
            runCommand("systemctl reboot")
        }
    }
}
```

### Parallel Execution

```kotlin
workflow("multi-site-backup") {
    actions {
        parallel {
            // These run simultaneously
            task("backup-site-1") {
                runCommand("backup-site1.sh")
            }
            
            task("backup-site-2") {
                runCommand("backup-site2.sh")
            }
            
            task("backup-database") {
                runCommand("backup-db.sh")
            }
        }
        
        // This runs after all parallel tasks complete
        runCommand("verify-backups.sh")
        notification("All Backups Complete", "All sites backed up successfully")
    }
}
```

## Security Considerations

### Secure Secret Management

```kotlin
workflow("deploy-app") {
    actions {
        // Use system keyring
        val apiKey = getSecret("deployment.api_key")
        
        runCommand("deploy.sh") {
            environment["API_KEY"] = apiKey
            // API key is not logged or stored in plain text
        }
    }
}
```

### Permission Management

```kotlin
workflow("system-maintenance") {
    // Specify required permissions
    requiredPermissions = listOf(
        Permission.SYSTEM_ADMIN,
        Permission.FILE_WRITE("/var/log"),
        Permission.COMMAND_EXECUTION("systemctl")
    )
    
    actions {
        // Actions that require elevated permissions
    }
}
```

### Sandboxing

```kotlin
workflow("untrusted-script") {
    actions {
        runCommand("./user-script.sh") {
            sandbox = true
            allowedPaths = listOf("/tmp", "/home/user/sandbox")
            allowedCommands = listOf("ls", "cat", "grep")
            networkAccess = false
        }
    }
}
```

## Debugging and Monitoring

### Logging

```kotlin
workflow("debug-example") {
    enableLogging = true
    logLevel = LogLevel.DEBUG
    
    actions {
        log("Starting workflow execution")
        
        runCommand("complex-command.sh") {
            logOutput = true
            logLevel = LogLevel.INFO
        }
        
        log("Workflow completed")
    }
}
```

### Performance Monitoring

```kotlin
workflow("performance-tracking") {
    measurePerformance = true
    
    actions {
        timed("database-backup") {
            runCommand("pg_dump database > backup.sql")
        }
        
        timed("compression") {
            runCommand("gzip backup.sql")
        }
    }
    
    onComplete { metrics ->
        if (metrics.totalDuration > 10.minutes) {
            notification("Slow Backup", "Backup took ${metrics.totalDuration}")
        }
    }
}
```

## Best Practices

### 1. Design for Reliability

- Always include error handling
- Use timeouts for long-running commands
- Implement retry logic for transient failures
- Test workflows in dry-run mode

### 2. Resource Management

- Avoid resource-intensive operations during peak hours
- Use conditions to check system resources
- Implement proper cleanup in error scenarios
- Monitor workflow execution frequency

### 3. Security

- Minimize required permissions
- Use secure secret management
- Validate all user inputs
- Sandbox untrusted operations

### 4. Maintainability

- Use descriptive names and documentation
- Break complex workflows into smaller ones
- Version control your configurations
- Monitor and log workflow execution

### 5. Testing

- Test workflows with dry-run mode
- Use staging environments
- Implement health checks
- Monitor for regressions

## Example Use Cases

See the [examples](../examples/) directory for complete configurations demonstrating:

- **Development Workflow**: Automated testing, building, and deployment
- **System Maintenance**: Updates, cleanup, and monitoring
- **Content Management**: File organization and media processing
- **Security Monitoring**: Intrusion detection and alerting
- **Backup Automation**: Multi-tier backup strategies
- **Desktop Productivity**: Email processing and task automation

## Troubleshooting

### Common Issues

1. **Workflow not triggering**: Check trigger conditions and permissions
2. **Commands failing**: Verify paths, permissions, and environment
3. **High resource usage**: Review scheduling and resource conditions
4. **Security errors**: Check required permissions and sandboxing

### Debug Commands

```bash
# List active workflows
horizonos-automation list

# Show workflow status
horizonos-automation status <workflow-name>

# Test workflow
horizonos-automation test <workflow-name> --dry-run

# View logs
journalctl -u horizonos-automation

# Enable debug mode
export HORIZONOS_AUTOMATION_DEBUG=true
```