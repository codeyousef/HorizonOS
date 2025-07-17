//! Test module for HorizonOS Graph Desktop
//! 
//! This module provides comprehensive testing capabilities for the graph desktop system.

pub mod integration_tests;
pub mod performance_tests;
pub mod stress_tests;

pub use integration_tests::{IntegrationTestSuite, TestResult, TestResults, TestStatus};
pub use performance_tests::PerformanceTestSuite;
pub use stress_tests::StressTestSuite;

/// Main test runner for the entire graph desktop system
pub struct GraphDesktopTestRunner {
    integration_suite: IntegrationTestSuite,
    performance_suite: PerformanceTestSuite,
    stress_suite: StressTestSuite,
}

impl GraphDesktopTestRunner {
    /// Create new test runner
    pub async fn new() -> anyhow::Result<Self> {
        Ok(Self {
            integration_suite: IntegrationTestSuite::new().await?,
            performance_suite: PerformanceTestSuite::new().await?,
            stress_suite: StressTestSuite::new().await?,
        })
    }

    /// Run all test suites
    pub async fn run_all(&mut self) -> anyhow::Result<()> {
        println!("🚀 Starting HorizonOS Graph Desktop Test Suite");
        println!("=" .repeat(60));

        // Run integration tests
        println!("\n📋 Running Integration Tests...");
        let integration_results = self.integration_suite.run_all_tests().await?;
        integration_results.print_summary();

        // Run performance tests
        println!("\n⚡ Running Performance Tests...");
        let performance_results = self.performance_suite.run_all_tests().await?;
        performance_results.print_summary();

        // Run stress tests
        println!("\n🏋️ Running Stress Tests...");
        let stress_results = self.stress_suite.run_all_tests().await?;
        stress_results.print_summary();

        // Print overall summary
        self.print_overall_summary(&integration_results, &performance_results, &stress_results);

        Ok(())
    }

    /// Print overall test summary
    fn print_overall_summary(
        &self, 
        integration: &TestResults, 
        performance: &TestResults, 
        stress: &TestResults
    ) {
        let total_tests = integration.total_count() + performance.total_count() + stress.total_count();
        let total_passed = integration.passed_count() + performance.passed_count() + stress.passed_count();
        let total_failed = integration.failed_count() + performance.failed_count() + stress.failed_count();
        let total_skipped = integration.skipped_count() + performance.skipped_count() + stress.skipped_count();
        
        let overall_success_rate = if total_tests == 0 { 0.0 } else { total_passed as f64 / total_tests as f64 };

        println!("\n🎯 OVERALL TEST SUMMARY");
        println!("=" .repeat(60));
        println!("📊 Total Tests: {}", total_tests);
        println!("✅ Passed: {}", total_passed);
        println!("❌ Failed: {}", total_failed);
        println!("⏭️ Skipped: {}", total_skipped);
        println!("📈 Overall Success Rate: {:.1}%", overall_success_rate * 100.0);
        
        if overall_success_rate >= 0.9 {
            println!("🎉 EXCELLENT! Graph desktop is highly stable");
        } else if overall_success_rate >= 0.8 {
            println!("👍 GOOD! Graph desktop is stable with minor issues");
        } else if overall_success_rate >= 0.7 {
            println!("⚠️ MODERATE! Graph desktop needs attention");
        } else {
            println!("🚨 CRITICAL! Graph desktop requires immediate fixes");
        }
        
        println!("=" .repeat(60));
    }
}