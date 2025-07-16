package org.horizonos.config.dsl

import io.kotest.core.spec.style.StringSpec
import io.kotest.matchers.shouldBe
import io.kotest.matchers.shouldNotBe
import io.kotest.matchers.collections.shouldContainExactly
import io.kotest.matchers.collections.shouldHaveSize
import io.kotest.matchers.types.shouldBeInstanceOf
import kotlin.time.Duration.Companion.minutes
import kotlin.time.Duration.Companion.seconds

class AutomationTest : StringSpec({
    
    "should create basic workflow" {
        val config = horizonOS {
            automation {
                workflow("test-workflow") {
                    description = "A test workflow"
                    priority = 100
                    
                    trigger {
                        time("09:00", WEEKDAYS)
                    }
                    
                    actions {
                        notification("Test", "Workflow started")
                        delay(5.seconds)
                        runCommand("echo 'Hello World'")
                    }
                }
            }
        }
        
        config.automation shouldNotBe null
        config.automation!!.workflows shouldHaveSize 1
        
        val workflow = config.automation!!.workflows[0]
        workflow.name shouldBe "test-workflow"
        workflow.description shouldBe "A test workflow"
        workflow.priority shouldBe 100
        workflow.enabled shouldBe true
        
        workflow.trigger shouldNotBe null
        workflow.trigger!!.type shouldBe TriggerType.TIME
        
        workflow.actions shouldHaveSize 3
        workflow.actions[0].shouldBeInstanceOf<Action.Notification>()
        workflow.actions[1].shouldBeInstanceOf<Action.Delay>()
        workflow.actions[2].shouldBeInstanceOf<Action.RunCommand>()
    }
    
    "should create complex workflow with conditions" {
        val config = horizonOS {
            automation {
                workflow("complex-workflow") {
                    trigger {
                        fileCreated("*.pdf", "/home/user/documents")
                    }
                    
                    conditions {
                        timeRange("09:00", "17:00")
                        dayOfWeek(DayOfWeek.MONDAY, DayOfWeek.TUESDAY, DayOfWeek.WEDNESDAY)
                        processRunning("okular")
                    }
                    
                    actions {
                        aiTask {
                            model = "llama3.2:3b"
                            prompt = "Summarize this document"
                            inputSource = InputSource.File("/tmp/current_document.pdf")
                            outputDestination = OutputDestination.Notification("Document Summary")
                        }
                        
                        fileOperation {
                            copy("/tmp/current_document.pdf", "/home/user/processed/")
                        }
                        
                        conditional("file_size > 1MB") {
                            notification("Large File", "Processing large document")
                            delay(10.seconds)
                        }
                        
                        loop(3) {
                            delay(1.seconds)
                            notification("Loop", "Processing step")
                        }
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.name shouldBe "complex-workflow"
        workflow.trigger!!.type shouldBe TriggerType.FILE_CREATED
        workflow.trigger!!.filePattern shouldBe "*.pdf"
        workflow.trigger!!.directoryPath shouldBe "/home/user/documents"
        
        workflow.conditions shouldHaveSize 3
        workflow.conditions[0].shouldBeInstanceOf<Condition.TimeRange>()
        workflow.conditions[1].shouldBeInstanceOf<Condition.DayOfWeek>()
        workflow.conditions[2].shouldBeInstanceOf<Condition.ProcessRunning>()
        
        workflow.actions shouldHaveSize 4
        workflow.actions[0].shouldBeInstanceOf<Action.AITask>()
        workflow.actions[1].shouldBeInstanceOf<Action.FileOperation>()
        workflow.actions[2].shouldBeInstanceOf<Action.Conditional>()
        workflow.actions[3].shouldBeInstanceOf<Action.Loop>()
    }
    
    "should create browser automation workflow" {
        val config = horizonOS {
            automation {
                workflow("browser-automation") {
                    trigger {
                        hotkey("Ctrl+Alt+B")
                    }
                    
                    actions {
                        browserOpen("https://github.com")
                        browserWait(BrowserCondition.PAGE_LOADED)
                        click("#search-input")
                        type("HorizonOS")
                        keyPress("Enter")
                        browserWait(BrowserCondition.ELEMENT_VISIBLE)
                        click("a[href*='horizonos']")
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.trigger!!.type shouldBe TriggerType.HOTKEY
        workflow.trigger!!.hotkey shouldBe "Ctrl+Alt+B"
        
        workflow.actions shouldHaveSize 7
        workflow.actions[0].shouldBeInstanceOf<Action.BrowserOpen>()
        workflow.actions[1].shouldBeInstanceOf<Action.BrowserWait>()
        workflow.actions[2].shouldBeInstanceOf<Action.Click>()
        workflow.actions[3].shouldBeInstanceOf<Action.Type>()
        workflow.actions[4].shouldBeInstanceOf<Action.KeyPress>()
        workflow.actions[5].shouldBeInstanceOf<Action.BrowserWait>()
        workflow.actions[6].shouldBeInstanceOf<Action.Click>()
    }
    
    "should create teaching mode configuration" {
        val config = horizonOS {
            automation {
                teaching("invoice-processing") {
                    description = "Learn to process invoices"
                    watchFolder("/home/user/invoices")
                    filePattern("*.pdf")
                    learnFrom(LearningMode.USER_DEMONSTRATION)
                    
                    recordedActions {
                        runApplication("okular")
                        delay(2.seconds)
                        aiTask {
                            prompt = "Extract invoice details"
                            model = "llama3.2:7b"
                        }
                        fileOperation {
                            move("/home/user/invoices/processed/", "/home/user/invoices/archive/")
                        }
                    }
                }
            }
        }
        
        config.automation!!.teachingModes shouldHaveSize 1
        
        val teaching = config.automation!!.teachingModes[0]
        teaching.name shouldBe "invoice-processing"
        teaching.description shouldBe "Learn to process invoices"
        teaching.watchedPath shouldBe "/home/user/invoices"
        teaching.filePattern shouldBe "*.pdf"
        teaching.learningMode shouldBe LearningMode.USER_DEMONSTRATION
        teaching.enabled shouldBe true
        
        teaching.recordedActions shouldHaveSize 4
        teaching.recordedActions[0].shouldBeInstanceOf<Action.RunApplication>()
        teaching.recordedActions[1].shouldBeInstanceOf<Action.Delay>()
        teaching.recordedActions[2].shouldBeInstanceOf<Action.AITask>()
        teaching.recordedActions[3].shouldBeInstanceOf<Action.FileOperation>()
    }
    
    "should create scheduled workflow" {
        val config = horizonOS {
            automation {
                workflow("daily-backup") {
                    trigger {
                        time("02:00", ALL_DAYS)
                    }
                    
                    conditions {
                        networkConnected()
                        batteryLevel(20)
                    }
                    
                    actions {
                        runCommand("rsync -av /home/user/ /backup/")
                        notification("Backup", "Daily backup completed")
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.trigger!!.type shouldBe TriggerType.TIME
        
        val schedule = workflow.trigger!!.schedule
        schedule.shouldBeInstanceOf<Schedule.Time>()
        schedule as Schedule.Time
        schedule.timeSpec shouldBe "02:00"
        schedule.days shouldContainExactly ALL_DAYS
        
        workflow.conditions shouldHaveSize 2
        workflow.conditions[0].shouldBeInstanceOf<Condition.NetworkConnected>()
        workflow.conditions[1].shouldBeInstanceOf<Condition.BatteryLevel>()
    }
    
    "should create interval-based workflow" {
        val config = horizonOS {
            automation {
                workflow("periodic-check") {
                    trigger {
                        interval(15.minutes)
                    }
                    
                    actions {
                        runCommand("systemctl status NetworkManager")
                        conditional("exit_code != 0") {
                            notification("Service Alert", "NetworkManager is not running", NotificationUrgency.HIGH)
                            runCommand("systemctl restart NetworkManager")
                        }
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.trigger!!.type shouldBe TriggerType.INTERVAL
        
        val schedule = workflow.trigger!!.schedule
        schedule.shouldBeInstanceOf<Schedule.Interval>()
        schedule as Schedule.Interval
        schedule.duration shouldBe 15.minutes
    }
    
    "should create system event workflow" {
        val config = horizonOS {
            automation {
                workflow("suspend-actions") {
                    trigger {
                        systemEvent(SystemEvent.SUSPEND)
                    }
                    
                    actions {
                        runCommand("sync")
                        notification("System", "Preparing for suspend")
                        runCommand("nmcli radio wifi off")
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.trigger!!.type shouldBe TriggerType.SYSTEM_EVENT
        workflow.trigger!!.systemEvent shouldBe SystemEvent.SUSPEND
    }
    
    "should create multiple workflows and teaching modes" {
        val config = horizonOS {
            automation {
                workflow("workflow1") {
                    trigger { time("09:00") }
                    actions { notification("Test", "Workflow 1") }
                }
                
                workflow("workflow2") {
                    trigger { hotkey("Ctrl+Alt+2") }
                    actions { runCommand("echo 'Workflow 2'") }
                }
                
                teaching("teaching1") {
                    watchFolder("/test1")
                    learnFrom(LearningMode.SCREEN_RECORDING)
                }
                
                teaching("teaching2") {
                    watchFolder("/test2")
                    learnFrom(LearningMode.API_MONITORING)
                }
            }
        }
        
        config.automation!!.workflows shouldHaveSize 2
        config.automation!!.teachingModes shouldHaveSize 2
        
        config.automation!!.workflows.map { it.name } shouldContainExactly listOf("workflow1", "workflow2")
        config.automation!!.teachingModes.map { it.name } shouldContainExactly listOf("teaching1", "teaching2")
    }
    
    "should handle file operations correctly" {
        val config = horizonOS {
            automation {
                workflow("file-ops") {
                    trigger { time("10:00") }
                    
                    actions {
                        fileOperation {
                            copy("/source/file.txt", "/dest/file.txt")
                        }
                        
                        fileOperation {
                            move("/temp/data.json", "/processed/data.json")
                        }
                        
                        fileOperation {
                            delete("/tmp/cache.tmp")
                        }
                        
                        fileOperation {
                            create("/new/file.txt", "Hello World")
                        }
                        
                        fileOperation {
                            write("/log/app.log", "Application started", append = true)
                        }
                        
                        fileOperation {
                            read("/config/settings.json", "config_data")
                        }
                    }
                }
            }
        }
        
        val workflow = config.automation!!.workflows[0]
        workflow.actions shouldHaveSize 6
        workflow.actions.all { it is Action.FileOperation } shouldBe true
        
        val fileOps = workflow.actions.map { (it as Action.FileOperation).operation }
        fileOps[0].shouldBeInstanceOf<FileOperation.Copy>()
        fileOps[1].shouldBeInstanceOf<FileOperation.Move>()
        fileOps[2].shouldBeInstanceOf<FileOperation.Delete>()
        fileOps[3].shouldBeInstanceOf<FileOperation.Create>()
        fileOps[4].shouldBeInstanceOf<FileOperation.Write>()
        fileOps[5].shouldBeInstanceOf<FileOperation.Read>()
    }
})