package org.horizonos.config.compiler.generators

import org.horizonos.config.dsl.CompiledConfig
import org.horizonos.config.compiler.FileType
import org.horizonos.config.compiler.GeneratedFile
import org.horizonos.config.dsl.*
import org.horizonos.config.dsl.*
import java.io.File

/**
 * Generator for automation scripts and configurations
 */
class AutomationGenerator(
    private val outputDir: File,
    private val generatedFiles: MutableList<GeneratedFile>
) {
    
    fun generate(config: CompiledConfig) {
        config.automation?.let { automation ->
            generateWorkflowScripts(automation)
        }
    }
    
    private fun generateWorkflowScripts(automation: AutomationConfig) {
        val workflowsDir = File(outputDir, "automation/workflows")
        workflowsDir.mkdirs()
        
        automation.workflows.forEach { workflow ->
            val workflowScript = File(workflowsDir, "${workflow.name}.sh")
            workflowScript.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Workflow: ${workflow.name}")
                appendLine("# Description: ${workflow.description}")
                appendLine("# Priority: ${workflow.priority}")
                appendLine()
                appendLine("set -euo pipefail")
                appendLine()
                
                if (!workflow.enabled) {
                    appendLine("echo 'Workflow is disabled: ${workflow.name}'")
                    appendLine("exit 0")
                }
                
                appendLine("echo 'Executing workflow: ${workflow.name}'")
                appendLine()
                
                // Check conditions
                workflow.conditions.forEach { condition ->
                    when (condition) {
                        is Condition.FileExists -> {
                            appendLine("# Checking if file exists: ${condition.filePath}")
                            appendLine("if [[ ! -f \"${condition.filePath}\" ]]; then")
                            appendLine("    echo 'Condition not met: File ${condition.filePath} does not exist'")
                            appendLine("    exit 1")
                            appendLine("fi")
                        }
                        is Condition.TimeRange -> {
                            appendLine("# Time-based condition: ${condition.start} - ${condition.end}")
                        }
                        is Condition.ProcessRunning -> {
                            appendLine("# Check if process is running: ${condition.processName}")
                            appendLine("if ! pgrep -x \"${condition.processName}\" > /dev/null; then")
                            appendLine("    echo 'Condition not met: Process ${condition.processName} is not running'")
                            appendLine("    exit 1")
                            appendLine("fi")
                        }
                        is Condition.NetworkConnected -> {
                            appendLine("# Check network connectivity")
                            appendLine("if ! ping -c 1 google.com > /dev/null 2>&1; then")
                            appendLine("    echo 'Condition not met: No network connection'")
                            appendLine("    exit 1")
                            appendLine("fi")
                        }
                        is Condition.DayOfWeek -> {
                            appendLine("# Day of week condition: ${condition.days}")
                        }
                        is Condition.BatteryLevel -> {
                            appendLine("# Battery level condition: ${condition.min}% - ${condition.max}%")
                        }
                        is Condition.UserIdle -> {
                            appendLine("# User idle condition: ${condition.duration}")
                        }
                    }
                    appendLine()
                }
                
                // Execute actions
                appendLine("# Executing actions")
                workflow.actions.forEach { action ->
                    when (action) {
                        is Action.Delay -> {
                            appendLine("echo 'Delaying for ${action.duration}'")
                            appendLine("sleep ${action.duration.inWholeSeconds}")
                        }
                        is Action.FileOperation -> {
                            when (action.operation) {
                                is FileOperation.Copy -> {
                                    appendLine("echo 'Copying file: ${action.operation.source} to ${action.operation.destination}'")
                                    appendLine("cp \"${action.operation.source}\" \"${action.operation.destination}\"")
                                }
                                is FileOperation.Move -> {
                                    appendLine("echo 'Moving file: ${action.operation.source} to ${action.operation.destination}'")
                                    appendLine("mv \"${action.operation.source}\" \"${action.operation.destination}\"")
                                }
                                is FileOperation.Delete -> {
                                    appendLine("echo 'Deleting file: ${action.operation.path}'")
                                    appendLine("rm -f \"${action.operation.path}\"")
                                }
                                is FileOperation.Create -> {
                                    appendLine("echo 'Creating file: ${action.operation.path}'")
                                    appendLine("cat > \"${action.operation.path}\" <<'EOF'")
                                    appendLine(action.operation.content)
                                    appendLine("EOF")
                                }
                                is FileOperation.Read -> {
                                    appendLine("echo 'Reading file: ${action.operation.path} into variable ${action.operation.variable}'")
                                    appendLine("${action.operation.variable}=\$(cat \"${action.operation.path}\")")
                                }
                                is FileOperation.Write -> {
                                    val redirect = if (action.operation.append) ">>" else ">"
                                    appendLine("echo 'Writing to file: ${action.operation.path}'")
                                    appendLine("echo \"${action.operation.content}\" $redirect \"${action.operation.path}\"")
                                }
                            }
                        }
                        is Action.RunCommand -> {
                            appendLine("echo 'Running command'")
                            appendLine(action.command)
                        }
                        is Action.RunApplication -> {
                            appendLine("echo 'Running application: ${action.app}'")
                            appendLine("${action.app} ${action.args.joinToString(" ")}")
                        }
                        is Action.Loop -> {
                            appendLine("# Loop ${action.times} times")
                            appendLine("for i in \$(seq 1 ${action.times}); do")
                            action.actions.forEach { innerAction ->
                                appendLine("    # Loop action: $innerAction")
                            }
                            appendLine("done")
                        }
                        is Action.Conditional -> {
                            appendLine("# Conditional: ${action.condition}")
                            appendLine("if ${action.condition}; then")
                            action.actions.forEach { innerAction ->
                                appendLine("    # Conditional action: $innerAction")
                            }
                            appendLine("fi")
                        }
                        is Action.Notification -> {
                            appendLine("# Send notification: ${action.title}")
                            appendLine("notify-send \"${action.title}\" \"${action.message}\"")
                        }
                        is Action.Click,
                        is Action.Type,
                        is Action.KeyPress,
                        is Action.BrowserOpen,
                        is Action.BrowserNavigate,
                        is Action.BrowserWait -> {
                            appendLine("# UI automation action: $action")
                        }
                        is Action.AITask -> {
                            appendLine("# AI task: ${action.model}")
                            appendLine("# Prompt: ${action.prompt}")
                        }
                    }
                    appendLine()
                }
                
                appendLine("echo 'Workflow completed: ${workflow.name}'")
            })
            workflowScript.setExecutable(true)
            generatedFiles.add(GeneratedFile("automation/workflows/${workflow.name}.sh", FileType.SHELL))
        }
        
        // Generate teaching mode scripts if present
        automation.teachingModes.forEach { teachingMode ->
            val teachingScript = File(workflowsDir, "teaching-${teachingMode.name}.sh")
            teachingScript.writeText(buildString {
                appendLine("#!/bin/bash")
                appendLine("# Teaching Mode: ${teachingMode.name}")
                appendLine("# Description: ${teachingMode.description}")
                appendLine()
                appendLine("echo 'Teaching mode active: ${teachingMode.name}'")
                appendLine("echo 'This mode will record user actions for automation'")
                appendLine()
                teachingMode.recordedActions.forEach { action ->
                    appendLine("# Recorded action: $action")
                }
            })
            teachingScript.setExecutable(true)
            generatedFiles.add(GeneratedFile("automation/workflows/teaching-${teachingMode.name}.sh", FileType.SHELL))
        }
    }
}