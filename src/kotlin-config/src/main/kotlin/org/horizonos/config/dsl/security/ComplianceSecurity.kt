package org.horizonos.config.dsl.security

import kotlinx.serialization.Serializable
import kotlin.time.Duration
import kotlin.time.Duration.Companion.days

// ===== Compliance Configuration =====

@Serializable
data class ComplianceConfig(
    val enabled: Boolean = false,
    val frameworks: List<ComplianceFramework> = emptyList(),
    val scanning: ComplianceScanConfig = ComplianceScanConfig(),
    val reporting: ComplianceReportConfig = ComplianceReportConfig(),
    val remediation: ComplianceRemediationConfig = ComplianceRemediationConfig()
)

@Serializable
data class ComplianceFramework(
    val name: String,
    val version: String,
    val enabled: Boolean = true,
    val profile: String = "default"
)

@Serializable
data class ComplianceScanConfig(
    val enabled: Boolean = true,
    val schedule: String = "weekly",
    val profiles: List<String> = emptyList(),
    val excludeRules: List<String> = emptyList()
)

@Serializable
data class ComplianceReportConfig(
    val enabled: Boolean = true,
    val format: ReportFormat = ReportFormat.HTML,
    val outputPath: String = "/var/log/compliance",
    val retentionDays: Duration = 90.days
)

@Serializable
data class ComplianceRemediationConfig(
    val enabled: Boolean = false,
    val autoRemediate: Boolean = false,
    val backupConfig: Boolean = true,
    val confirmationRequired: Boolean = true
)

// ===== Enums =====

@Serializable
enum class ReportFormat {
    HTML,         // HTML report format
    XML,          // XML report format
    JSON,         // JSON report format
    PDF,          // PDF report format
    CSV           // CSV report format
}