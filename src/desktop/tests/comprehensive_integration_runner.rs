//! Comprehensive Integration Test Runner
//! 
//! Central test runner that orchestrates all integration tests for the
//! complete HorizonOS desktop system, providing comprehensive validation.

use std::sync::Arc;
use std::time::Instant;
use anyhow::Result;
use tokio::time::Duration;

mod graph_engine_integration;
mod multi_component_integration;
mod ai_system_integration;

use graph_engine_integration::GraphEngineIntegrationTests;
use multi_component_integration::MultiComponentIntegrationTests;
use ai_system_integration::AISystemIntegrationTests;

/// Comprehensive integration test suite for HorizonOS
pub struct ComprehensiveIntegrationRunner {
    /// Test suite configurations
    config: IntegrationTestConfig,
    
    /// Test results from all suites
    all_results: Vec<TestSuiteResult>,
}

/// Configuration for integration tests
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    /// Enable performance profiling
    pub enable_profiling: bool,
    
    /// Test timeout in seconds
    pub test_timeout: u64,
    
    /// Enable parallel test execution
    pub parallel_execution: bool,
    
    /// Skip AI tests if no LLM available
    pub skip_ai_if_unavailable: bool,
    
    /// Skip GPU tests if no GPU available
    pub skip_gpu_if_unavailable: bool,
    
    /// Enable stress testing
    pub enable_stress_tests: bool,
    
    /// Enable memory leak detection
    pub enable_memory_leak_detection: bool,
    
    /// Generate detailed reports
    pub generate_detailed_reports: bool,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            enable_profiling: true,
            test_timeout: 300, // 5 minutes
            parallel_execution: true,
            skip_ai_if_unavailable: true,
            skip_gpu_if_unavailable: true,
            enable_stress_tests: true,
            enable_memory_leak_detection: true,
            generate_detailed_reports: true,
        }
    }
}

/// Result from a complete test suite
#[derive(Debug)]
pub struct TestSuiteResult {
    pub suite_name: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub execution_time: Duration,
    pub success_rate: f64,
    pub failed_test_names: Vec<String>,
    pub performance_metrics: Option<PerformanceMetrics>,
}

/// Performance metrics for test execution
#[derive(Debug)]
pub struct PerformanceMetrics {
    pub cpu_usage_peak: f32,
    pub memory_usage_peak: u64,
    pub gpu_usage_peak: Option<f32>,
    pub disk_io_peak: u64,
    pub network_io_peak: u64,
}

/// Comprehensive test report
#[derive(Debug)]
pub struct ComprehensiveTestReport {
    pub total_suites: usize,
    pub total_tests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_skipped: usize,
    pub overall_success_rate: f64,
    pub total_execution_time: Duration,
    pub suite_results: Vec<TestSuiteResult>,
    pub system_info: SystemInfo,
    pub recommendations: Vec<String>,
}

/// System information for test context
#[derive(Debug)]
pub struct SystemInfo {
    pub os_version: String,
    pub cpu_model: String,
    pub cpu_cores: u32,
    pub memory_total: u64,
    pub gpu_model: Option<String>,
    pub graphics_driver: Option<String>,
    pub kernel_version: String,
    pub desktop_environment: String,
}

impl ComprehensiveIntegrationRunner {
    /// Create new comprehensive integration test runner
    pub fn new(config: IntegrationTestConfig) -> Self {
        Self {
            config,
            all_results: Vec::new(),
        }
    }

    /// Run all integration test suites
    pub async fn run_all_tests(&mut self) -> Result<ComprehensiveTestReport> {
        println!("🚀 Starting HorizonOS Comprehensive Integration Tests");
        println!("=" .repeat(60));
        
        let start_time = Instant::now();
        
        // Clear previous results
        self.all_results.clear();
        
        // Collect system information
        let system_info = self.collect_system_info().await?;
        println!("📊 System Information:");
        println!("  OS: {}", system_info.os_version);
        println!("  CPU: {} ({} cores)", system_info.cpu_model, system_info.cpu_cores);
        println!("  Memory: {} GB", system_info.memory_total / 1024 / 1024 / 1024);
        if let Some(gpu) = &system_info.gpu_model {
            println!("  GPU: {}", gpu);
        }
        println!();
        
        // Run test suites
        if self.config.parallel_execution {
            self.run_parallel_tests().await?;
        } else {
            self.run_sequential_tests().await?;
        }
        
        let total_execution_time = start_time.elapsed();
        
        // Generate comprehensive report
        let report = self.generate_comprehensive_report(system_info, total_execution_time).await?;
        
        // Print summary
        self.print_test_summary(&report);
        
        // Generate detailed reports if enabled
        if self.config.generate_detailed_reports {
            self.generate_detailed_reports(&report).await?;
        }
        
        Ok(report)
    }

    /// Run test suites in parallel
    async fn run_parallel_tests(&mut self) -> Result<()> {
        println!("🔄 Running test suites in parallel...");
        
        // Launch all test suites concurrently
        let handles = vec![
            tokio::spawn(async move {
                let mut suite = GraphEngineIntegrationTests::new().await?;
                let results = suite.run_all_tests().await?;
                Ok(("Graph Engine", results))
            }),
            tokio::spawn(async move {
                let mut suite = MultiComponentIntegrationTests::new().await?;
                let results = suite.run_all_tests().await?;
                Ok(("Multi-Component", results))
            }),
            tokio::spawn(async move {
                let mut suite = AISystemIntegrationTests::new().await?;
                let results = suite.run_all_tests().await?;
                Ok(("AI System", results))
            }),
        ];
        
        // Wait for all suites to complete
        let results = futures::future::join_all(handles).await;
        
        // Process results
        for result in results {
            match result {
                Ok(Ok((suite_name, test_results))) => {
                    let suite_result = self.process_test_results(suite_name, test_results).await;
                    self.all_results.push(suite_result);
                }
                Ok(Err(e)) => {
                    eprintln!("❌ Test suite failed: {}", e);
                }
                Err(e) => {
                    eprintln!("❌ Test suite panicked: {}", e);
                }
            }
        }
        
        Ok(())
    }

    /// Run test suites sequentially
    async fn run_sequential_tests(&mut self) -> Result<()> {
        println!("🔄 Running test suites sequentially...");
        
        // Run Graph Engine tests
        println!("🔧 Running Graph Engine Integration Tests...");
        let graph_start = Instant::now();
        let mut graph_suite = GraphEngineIntegrationTests::new().await?;
        let graph_results = graph_suite.run_all_tests().await?;
        let graph_suite_result = self.process_test_results_with_timing("Graph Engine", graph_results, graph_start.elapsed()).await;
        self.all_results.push(graph_suite_result);
        
        // Run Multi-Component tests
        println!("🔧 Running Multi-Component Integration Tests...");
        let multi_start = Instant::now();
        let mut multi_suite = MultiComponentIntegrationTests::new().await?;
        let multi_results = multi_suite.run_all_tests().await?;
        let multi_suite_result = self.process_test_results_with_timing("Multi-Component", multi_results, multi_start.elapsed()).await;
        self.all_results.push(multi_suite_result);
        
        // Run AI System tests
        println!("🤖 Running AI System Integration Tests...");
        let ai_start = Instant::now();
        let mut ai_suite = AISystemIntegrationTests::new().await?;
        let ai_results = ai_suite.run_all_tests().await?;
        let ai_suite_result = self.process_test_results_with_timing("AI System", ai_results, ai_start.elapsed()).await;
        self.all_results.push(ai_suite_result);
        
        Ok(())
    }

    /// Process test results into suite result
    async fn process_test_results(
        &self,
        suite_name: &str,
        results: impl TestResultCollection,
    ) -> TestSuiteResult {
        self.process_test_results_with_timing(suite_name, results, Duration::from_secs(0)).await
    }

    /// Process test results with timing information
    async fn process_test_results_with_timing(
        &self,
        suite_name: &str,
        results: impl TestResultCollection,
        execution_time: Duration,
    ) -> TestSuiteResult {
        let total_tests = results.total_count();
        let passed_tests = results.passed_count();
        let failed_tests = results.failed_count();
        let skipped_tests = results.skipped_count();
        let success_rate = results.success_rate();
        
        let failed_test_names = results.get_failed_test_names();
        
        // Collect performance metrics if enabled
        let performance_metrics = if self.config.enable_profiling {
            Some(self.collect_performance_metrics().await)
        } else {
            None
        };
        
        TestSuiteResult {
            suite_name: suite_name.to_string(),
            total_tests,
            passed_tests,
            failed_tests,
            skipped_tests,
            execution_time,
            success_rate,
            failed_test_names,
            performance_metrics,
        }
    }

    /// Generate comprehensive test report
    async fn generate_comprehensive_report(
        &self,
        system_info: SystemInfo,
        total_execution_time: Duration,
    ) -> Result<ComprehensiveTestReport> {
        let total_suites = self.all_results.len();
        let total_tests = self.all_results.iter().map(|r| r.total_tests).sum();
        let total_passed = self.all_results.iter().map(|r| r.passed_tests).sum();
        let total_failed = self.all_results.iter().map(|r| r.failed_tests).sum();
        let total_skipped = self.all_results.iter().map(|r| r.skipped_tests).sum();
        
        let overall_success_rate = if total_tests > 0 {
            total_passed as f64 / total_tests as f64
        } else {
            0.0
        };
        
        let recommendations = self.generate_recommendations().await;
        
        Ok(ComprehensiveTestReport {
            total_suites,
            total_tests,
            total_passed,
            total_failed,
            total_skipped,
            overall_success_rate,
            total_execution_time,
            suite_results: self.all_results.clone(),
            system_info,
            recommendations,
        })
    }

    /// Generate recommendations based on test results
    async fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // Overall success rate recommendations
        let total_tests: usize = self.all_results.iter().map(|r| r.total_tests).sum();
        let total_passed: usize = self.all_results.iter().map(|r| r.passed_tests).sum();
        let overall_success_rate = if total_tests > 0 {
            total_passed as f64 / total_tests as f64
        } else {
            0.0
        };
        
        if overall_success_rate < 0.7 {
            recommendations.push("🔴 Overall success rate is below 70%. Consider reviewing failing tests and system configuration.".to_string());
        } else if overall_success_rate < 0.9 {
            recommendations.push("🟡 Overall success rate is below 90%. Some optimizations may be needed.".to_string());
        } else {
            recommendations.push("🟢 Excellent overall success rate! System is performing well.".to_string());
        }
        
        // Suite-specific recommendations
        for suite_result in &self.all_results {
            if suite_result.success_rate < 0.5 {
                recommendations.push(format!(
                    "🔴 {} suite has low success rate ({:.1}%). Investigate failing tests: {}",
                    suite_result.suite_name,
                    suite_result.success_rate * 100.0,
                    suite_result.failed_test_names.join(", ")
                ));
            }
            
            if suite_result.execution_time > Duration::from_secs(60) {
                recommendations.push(format!(
                    "⏱️ {} suite took {:.1}s to execute. Consider optimizing slow tests.",
                    suite_result.suite_name,
                    suite_result.execution_time.as_secs_f64()
                ));
            }
        }
        
        // Performance recommendations
        if self.config.enable_profiling {
            for suite_result in &self.all_results {
                if let Some(metrics) = &suite_result.performance_metrics {
                    if metrics.cpu_usage_peak > 90.0 {
                        recommendations.push(format!(
                            "🔥 {} suite peaked at {:.1}% CPU usage. Consider resource optimization.",
                            suite_result.suite_name,
                            metrics.cpu_usage_peak
                        ));
                    }
                    
                    if metrics.memory_usage_peak > 8 * 1024 * 1024 * 1024 { // 8GB
                        recommendations.push(format!(
                            "💾 {} suite peaked at {:.1}GB memory usage. Monitor for memory leaks.",
                            suite_result.suite_name,
                            metrics.memory_usage_peak as f64 / 1024.0 / 1024.0 / 1024.0
                        ));
                    }
                }
            }
        }
        
        recommendations
    }

    /// Print test summary
    fn print_test_summary(&self, report: &ComprehensiveTestReport) {
        println!();
        println!("🎯 COMPREHENSIVE TEST RESULTS SUMMARY");
        println!("=" .repeat(60));
        println!("Total Test Suites: {}", report.total_suites);
        println!("Total Tests: {}", report.total_tests);
        println!("✅ Passed: {}", report.total_passed);
        println!("❌ Failed: {}", report.total_failed);
        println!("⏭️ Skipped: {}", report.total_skipped);
        println!("📊 Success Rate: {:.1}%", report.overall_success_rate * 100.0);
        println!("⏱️ Total Execution Time: {:.1}s", report.total_execution_time.as_secs_f64());
        println!();
        
        // Print suite breakdown
        println!("📋 SUITE BREAKDOWN");
        println!("-" .repeat(60));
        for suite_result in &report.suite_results {
            let status_icon = if suite_result.success_rate > 0.9 {
                "🟢"
            } else if suite_result.success_rate > 0.7 {
                "🟡"
            } else {
                "🔴"
            };
            
            println!(
                "{} {} - {}/{} passed ({:.1}%) - {:.1}s",
                status_icon,
                suite_result.suite_name,
                suite_result.passed_tests,
                suite_result.total_tests,
                suite_result.success_rate * 100.0,
                suite_result.execution_time.as_secs_f64()
            );
            
            if !suite_result.failed_test_names.is_empty() {
                println!("   Failed: {}", suite_result.failed_test_names.join(", "));
            }
        }
        
        println!();
        
        // Print recommendations
        if !report.recommendations.is_empty() {
            println!("💡 RECOMMENDATIONS");
            println!("-" .repeat(60));
            for recommendation in &report.recommendations {
                println!("{}", recommendation);
            }
            println!();
        }
    }

    /// Generate detailed reports
    async fn generate_detailed_reports(&self, report: &ComprehensiveTestReport) -> Result<()> {
        // Generate JSON report
        let json_report = serde_json::to_string_pretty(report)?;
        tokio::fs::write("integration_test_report.json", json_report).await?;
        
        // Generate HTML report
        let html_report = self.generate_html_report(report).await?;
        tokio::fs::write("integration_test_report.html", html_report).await?;
        
        // Generate CSV report
        let csv_report = self.generate_csv_report(report).await?;
        tokio::fs::write("integration_test_report.csv", csv_report).await?;
        
        println!("📄 Detailed reports generated:");
        println!("  - integration_test_report.json");
        println!("  - integration_test_report.html");
        println!("  - integration_test_report.csv");
        
        Ok(())
    }

    /// Generate HTML report
    async fn generate_html_report(&self, report: &ComprehensiveTestReport) -> Result<String> {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>HorizonOS Integration Test Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #2c3e50; color: white; padding: 20px; border-radius: 8px; }}
        .summary {{ background-color: #ecf0f1; padding: 15px; border-radius: 8px; margin: 20px 0; }}
        .suite {{ background-color: #ffffff; border: 1px solid #bdc3c7; padding: 15px; margin: 10px 0; border-radius: 8px; }}
        .passed {{ color: #27ae60; }}
        .failed {{ color: #e74c3c; }}
        .skipped {{ color: #f39c12; }}
        .recommendations {{ background-color: #fff3cd; padding: 15px; border-radius: 8px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>HorizonOS Integration Test Report</h1>
        <p>Generated on: {}</p>
    </div>
    
    <div class="summary">
        <h2>Summary</h2>
        <p><strong>Total Tests:</strong> {}</p>
        <p><strong>Passed:</strong> <span class="passed">{}</span></p>
        <p><strong>Failed:</strong> <span class="failed">{}</span></p>
        <p><strong>Skipped:</strong> <span class="skipped">{}</span></p>
        <p><strong>Success Rate:</strong> {:.1}%</p>
        <p><strong>Execution Time:</strong> {:.1}s</p>
    </div>
    
    <h2>Test Suite Results</h2>
    {}
    
    <div class="recommendations">
        <h2>Recommendations</h2>
        <ul>
            {}
        </ul>
    </div>
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            report.total_tests,
            report.total_passed,
            report.total_failed,
            report.total_skipped,
            report.overall_success_rate * 100.0,
            report.total_execution_time.as_secs_f64(),
            report.suite_results.iter().map(|suite| {
                format!(
                    r#"<div class="suite">
                        <h3>{}</h3>
                        <p><strong>Tests:</strong> {}</p>
                        <p><strong>Passed:</strong> <span class="passed">{}</span></p>
                        <p><strong>Failed:</strong> <span class="failed">{}</span></p>
                        <p><strong>Skipped:</strong> <span class="skipped">{}</span></p>
                        <p><strong>Success Rate:</strong> {:.1}%</p>
                        <p><strong>Execution Time:</strong> {:.1}s</p>
                    </div>"#,
                    suite.suite_name,
                    suite.total_tests,
                    suite.passed_tests,
                    suite.failed_tests,
                    suite.skipped_tests,
                    suite.success_rate * 100.0,
                    suite.execution_time.as_secs_f64()
                )
            }).collect::<Vec<_>>().join("\n"),
            report.recommendations.iter().map(|r| format!("<li>{}</li>", r)).collect::<Vec<_>>().join("\n")
        );
        
        Ok(html)
    }

    /// Generate CSV report
    async fn generate_csv_report(&self, report: &ComprehensiveTestReport) -> Result<String> {
        let mut csv = String::new();
        csv.push_str("Suite,Total Tests,Passed,Failed,Skipped,Success Rate,Execution Time (s)\n");
        
        for suite_result in &report.suite_results {
            csv.push_str(&format!(
                "{},{},{},{},{},{:.1},{:.1}\n",
                suite_result.suite_name,
                suite_result.total_tests,
                suite_result.passed_tests,
                suite_result.failed_tests,
                suite_result.skipped_tests,
                suite_result.success_rate * 100.0,
                suite_result.execution_time.as_secs_f64()
            ));
        }
        
        Ok(csv)
    }

    /// Collect system information
    async fn collect_system_info(&self) -> Result<SystemInfo> {
        // This is a simplified implementation
        // In a real implementation, you would use system APIs
        
        Ok(SystemInfo {
            os_version: "Linux".to_string(),
            cpu_model: "Unknown CPU".to_string(),
            cpu_cores: 4,
            memory_total: 8 * 1024 * 1024 * 1024, // 8GB
            gpu_model: None,
            graphics_driver: None,
            kernel_version: "Unknown".to_string(),
            desktop_environment: "HorizonOS".to_string(),
        })
    }

    /// Collect performance metrics
    async fn collect_performance_metrics(&self) -> PerformanceMetrics {
        // This is a simplified implementation
        // In a real implementation, you would collect actual performance data
        
        PerformanceMetrics {
            cpu_usage_peak: 45.0,
            memory_usage_peak: 2 * 1024 * 1024 * 1024, // 2GB
            gpu_usage_peak: Some(30.0),
            disk_io_peak: 1024 * 1024, // 1MB
            network_io_peak: 1024, // 1KB
        }
    }
}

/// Trait for test result collections
trait TestResultCollection {
    fn total_count(&self) -> usize;
    fn passed_count(&self) -> usize;
    fn failed_count(&self) -> usize;
    fn skipped_count(&self) -> usize;
    fn success_rate(&self) -> f64;
    fn get_failed_test_names(&self) -> Vec<String>;
}

// Implement the trait for the test result types from other modules
impl TestResultCollection for graph_engine_integration::TestResults {
    fn total_count(&self) -> usize { self.tests.len() }
    fn passed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, graph_engine_integration::TestStatus::Passed)).count() }
    fn failed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, graph_engine_integration::TestStatus::Failed)).count() }
    fn skipped_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, graph_engine_integration::TestStatus::Skipped)).count() }
    fn success_rate(&self) -> f64 { if self.tests.is_empty() { 0.0 } else { self.passed_count() as f64 / self.tests.len() as f64 } }
    fn get_failed_test_names(&self) -> Vec<String> {
        self.tests.iter()
            .filter(|t| matches!(t.status, graph_engine_integration::TestStatus::Failed))
            .map(|t| t.name.clone())
            .collect()
    }
}

impl TestResultCollection for multi_component_integration::TestResults {
    fn total_count(&self) -> usize { self.tests.len() }
    fn passed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, multi_component_integration::TestStatus::Passed)).count() }
    fn failed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, multi_component_integration::TestStatus::Failed)).count() }
    fn skipped_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, multi_component_integration::TestStatus::Skipped)).count() }
    fn success_rate(&self) -> f64 { if self.tests.is_empty() { 0.0 } else { self.passed_count() as f64 / self.tests.len() as f64 } }
    fn get_failed_test_names(&self) -> Vec<String> {
        self.tests.iter()
            .filter(|t| matches!(t.status, multi_component_integration::TestStatus::Failed))
            .map(|t| t.name.clone())
            .collect()
    }
}

impl TestResultCollection for ai_system_integration::TestResults {
    fn total_count(&self) -> usize { self.tests.len() }
    fn passed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, ai_system_integration::TestStatus::Passed)).count() }
    fn failed_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, ai_system_integration::TestStatus::Failed)).count() }
    fn skipped_count(&self) -> usize { self.tests.iter().filter(|t| matches!(t.status, ai_system_integration::TestStatus::Skipped)).count() }
    fn success_rate(&self) -> f64 { if self.tests.is_empty() { 0.0 } else { self.passed_count() as f64 / self.tests.len() as f64 } }
    fn get_failed_test_names(&self) -> Vec<String> {
        self.tests.iter()
            .filter(|t| matches!(t.status, ai_system_integration::TestStatus::Failed))
            .map(|t| t.name.clone())
            .collect()
    }
}

// Implement Clone for TestSuiteResult
impl Clone for TestSuiteResult {
    fn clone(&self) -> Self {
        Self {
            suite_name: self.suite_name.clone(),
            total_tests: self.total_tests,
            passed_tests: self.passed_tests,
            failed_tests: self.failed_tests,
            skipped_tests: self.skipped_tests,
            execution_time: self.execution_time,
            success_rate: self.success_rate,
            failed_test_names: self.failed_test_names.clone(),
            performance_metrics: self.performance_metrics.clone(),
        }
    }
}

impl Clone for PerformanceMetrics {
    fn clone(&self) -> Self {
        Self {
            cpu_usage_peak: self.cpu_usage_peak,
            memory_usage_peak: self.memory_usage_peak,
            gpu_usage_peak: self.gpu_usage_peak,
            disk_io_peak: self.disk_io_peak,
            network_io_peak: self.network_io_peak,
        }
    }
}

/// Main function to run comprehensive integration tests
pub async fn run_comprehensive_integration_tests() -> Result<()> {
    let config = IntegrationTestConfig::default();
    let mut runner = ComprehensiveIntegrationRunner::new(config);
    
    let report = runner.run_all_tests().await?;
    
    // Exit with non-zero code if tests failed
    if report.overall_success_rate < 1.0 {
        std::process::exit(1);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_comprehensive_integration() {
        let config = IntegrationTestConfig {
            test_timeout: 60, // Shorter timeout for testing
            enable_stress_tests: false, // Disable stress tests for quick testing
            ..Default::default()
        };
        
        let mut runner = ComprehensiveIntegrationRunner::new(config);
        let report = runner.run_all_tests().await.expect("Failed to run comprehensive tests");
        
        // Basic assertions
        assert!(report.total_suites > 0, "No test suites executed");
        assert!(report.total_tests > 0, "No tests executed");
        assert!(report.overall_success_rate > 0.0, "No tests passed");
        
        println!("Comprehensive integration test completed successfully!");
    }
}