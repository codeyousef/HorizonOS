//! Ollama integration for local LLM processing

use crate::{AIError, Message, MessageRole, HardwareOptimization};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use futures::stream::Stream;
// use futures::StreamExt; // Unused import
use std::pin::Pin;
use async_stream;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::Mutex;
use log::{info, debug};

/// Ollama client for interacting with the local LLM server
pub struct OllamaClient {
    /// HTTP client
    client: Client,
    /// Base URL for Ollama API
    base_url: String,
    /// Connection pool for concurrent requests
    connection_pool: Arc<RwLock<ConnectionPool>>,
    /// Model cache for fast access
    model_cache: DashMap<String, CachedModel>,
    /// Performance metrics
    metrics: Arc<Mutex<PerformanceMetrics>>,
    /// Hardware optimization settings
    hardware_optimization: HardwareOptimization,
}

/// Connection pool for managing concurrent requests
#[derive(Debug)]
struct ConnectionPool {
    /// Active connections
    active_connections: HashMap<String, DateTime<Utc>>,
    /// Maximum concurrent connections
    max_connections: usize,
    /// Connection timeout
    timeout: Duration,
}

/// Cached model information
#[derive(Debug, Clone)]
pub struct CachedModel {
    /// Model information
    info: ModelInfo,
    /// Last access time
    last_accessed: DateTime<Utc>,
    /// Performance data
    performance: ModelPerformance,
    /// Whether the model is currently loaded
    loaded: bool,
}

/// Model performance metrics
#[derive(Debug, Clone)]
pub struct ModelPerformance {
    /// Average inference time in milliseconds
    avg_inference_time: f64,
    /// Tokens per second
    tokens_per_second: f64,
    /// Memory usage in MB
    memory_usage: u64,
    /// Number of requests served
    request_count: u64,
    /// Success rate (0.0 to 1.0)
    success_rate: f64,
}

/// Overall performance metrics
#[derive(Debug, Default, Clone)]
pub struct PerformanceMetrics {
    /// Total requests
    total_requests: u64,
    /// Successful requests
    successful_requests: u64,
    /// Failed requests
    failed_requests: u64,
    /// Total inference time
    total_inference_time: Duration,
    /// Average response time
    avg_response_time: Duration,
    /// Peak memory usage
    peak_memory_usage: u64,
    /// Last reset time
    last_reset: DateTime<Utc>,
}

/// Model recommendation based on hardware
#[derive(Debug, Clone)]
pub struct ModelRecommendation {
    /// Recommended model name
    pub model_name: String,
    /// Confidence in recommendation (0.0 to 1.0)
    pub confidence: f32,
    /// Expected performance
    pub expected_performance: ModelPerformance,
    /// Hardware requirements
    pub hardware_requirements: HardwareRequirements,
    /// Reason for recommendation
    pub reason: String,
}

/// Hardware requirements for a model
#[derive(Debug, Clone)]
pub struct HardwareRequirements {
    /// Minimum RAM in MB
    pub min_ram: u64,
    /// Minimum VRAM in MB (if GPU acceleration)
    pub min_vram: Option<u64>,
    /// CPU cores recommended
    pub recommended_cores: u32,
    /// GPU acceleration recommended
    pub gpu_recommended: bool,
}

/// Model download progress
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// Model name
    pub model_name: String,
    /// Download progress (0.0 to 1.0)
    pub progress: f32,
    /// Downloaded bytes
    pub downloaded_bytes: u64,
    /// Total bytes
    pub total_bytes: u64,
    /// Download speed in bytes/second
    pub download_speed: u64,
    /// Status message
    pub status: String,
    /// Whether download is complete
    pub complete: bool,
}

/// Streaming response with token information
#[derive(Debug, Clone)]
pub struct StreamingToken {
    /// Token text
    pub text: String,
    /// Token index in the response
    pub index: usize,
    /// Processing time for this token
    pub processing_time: Duration,
    /// Whether this is the final token
    pub is_final: bool,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: &str) -> Self {
        Self::new_with_optimization(base_url, HardwareOptimization::Auto)
    }

    /// Create a new Ollama client with hardware optimization
    pub fn new_with_optimization(base_url: &str, hardware_optimization: HardwareOptimization) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for long responses
            .pool_max_idle_per_host(10) // Connection pooling
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        let connection_pool = Arc::new(RwLock::new(ConnectionPool {
            active_connections: HashMap::new(),
            max_connections: 20,
            timeout: Duration::from_secs(30),
        }));

        Self {
            client,
            base_url: base_url.to_string(),
            connection_pool,
            model_cache: DashMap::new(),
            metrics: Arc::new(Mutex::new(PerformanceMetrics::default())),
            hardware_optimization,
        }
    }

    /// Get hardware optimization mode
    pub fn hardware_optimization(&self) -> HardwareOptimization {
        self.hardware_optimization
    }

    /// Update hardware optimization mode
    pub fn set_hardware_optimization(&mut self, optimization: HardwareOptimization) {
        self.hardware_optimization = optimization;
    }

    /// Test connection to Ollama server
    pub async fn test_connection(&self) -> Result<(), AIError> {
        let url = format!("{}/api/tags", self.base_url);
        
        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(AIError::OllamaConnection(format!(
                        "Ollama server returned status: {}",
                        response.status()
                    )))
                }
            }
            Err(e) => Err(AIError::OllamaConnection(format!(
                "Failed to connect to Ollama: {}",
                e
            ))),
        }
    }

    /// List available models with caching
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, AIError> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::OllamaConnection(format!(
                "Failed to list models: {}",
                response.status()
            )));
        }

        let data: ListModelsResponse = response
            .json()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        // Update model cache
        let now = Utc::now();
        for model in &data.models {
            self.model_cache.insert(model.name.clone(), CachedModel {
                info: model.clone(),
                last_accessed: now,
                performance: ModelPerformance {
                    avg_inference_time: 0.0,
                    tokens_per_second: 0.0,
                    memory_usage: 0,
                    request_count: 0,
                    success_rate: 1.0,
                },
                loaded: true,
            });
        }

        Ok(data.models)
    }

    /// Get cached model information
    pub fn get_cached_model(&self, model_name: &str) -> Option<CachedModel> {
        self.model_cache.get(model_name).map(|entry| entry.clone())
    }

    /// Get model performance metrics
    pub fn get_model_performance(&self, model_name: &str) -> Option<ModelPerformance> {
        self.model_cache.get(model_name).map(|entry| entry.performance.clone())
    }

    /// Get overall performance metrics
    pub fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.metrics.lock().clone()
    }

    /// Reset performance metrics
    pub fn reset_performance_metrics(&self) {
        let mut metrics = self.metrics.lock();
        *metrics = PerformanceMetrics::default();
        metrics.last_reset = Utc::now();
    }

    /// Pull a model if not already available
    pub async fn pull_model(&self, model_name: &str) -> Result<(), AIError> {
        info!("Pulling model: {}", model_name);
        
        let url = format!("{}/api/pull", self.base_url);
        
        let request = PullRequest {
            name: model_name.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::ModelNotAvailable(format!(
                "Failed to pull model {}: {}",
                model_name,
                response.status()
            )));
        }

        info!("Successfully pulled model: {}", model_name);
        Ok(())
    }

    /// Pull model with progress tracking
    pub async fn pull_model_with_progress(
        &self,
        model_name: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<DownloadProgress, AIError>> + Send>>, AIError> {
        info!("Pulling model with progress: {}", model_name);
        
        let url = format!("{}/api/pull", self.base_url);
        
        let request = PullRequest {
            name: model_name.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::ModelNotAvailable(format!(
                "Failed to pull model {}: {}",
                model_name,
                response.status()
            )));
        }

        let model_name = model_name.to_string();
        let stream = async_stream::stream! {
            let body = response.text().await.map_err(|e| AIError::OllamaConnection(e.to_string()))?;
            
            for line in body.lines() {
                if let Ok(pull_response) = serde_json::from_str::<PullResponse>(line) {
                    let progress = DownloadProgress {
                        model_name: model_name.clone(),
                        progress: pull_response.completed as f32 / pull_response.total as f32,
                        downloaded_bytes: pull_response.completed,
                        total_bytes: pull_response.total,
                        download_speed: 0, // TODO: Calculate download speed
                        status: pull_response.status,
                        complete: pull_response.completed == pull_response.total,
                    };
                    yield Ok(progress);
                }
            }
        };

        Ok(Box::pin(stream))
    }

    /// Remove a model
    pub async fn remove_model(&self, model_name: &str) -> Result<(), AIError> {
        info!("Removing model: {}", model_name);
        
        let url = format!("{}/api/delete", self.base_url);
        
        let request = DeleteRequest {
            name: model_name.to_string(),
        };

        let response = self.client
            .delete(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::ModelNotAvailable(format!(
                "Failed to remove model {}: {}",
                model_name,
                response.status()
            )));
        }

        // Remove from cache
        self.model_cache.remove(model_name);
        
        info!("Successfully removed model: {}", model_name);
        Ok(())
    }

    /// Check if a model is available
    pub async fn is_model_available(&self, model_name: &str) -> Result<bool, AIError> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name == model_name))
    }

    /// Get model recommendations based on hardware
    pub async fn get_model_recommendations(&self) -> Result<Vec<ModelRecommendation>, AIError> {
        let hardware_profile = crate::hardware::detect_hardware_profile()
            .map_err(|e| AIError::HardwareDetection(e.to_string()))?;
        
        let mut recommendations = Vec::new();
        
        // Recommend models based on available VRAM/RAM
        if let Some(vram) = hardware_profile.gpu.vram_total {
            if vram >= 48_000 {
                recommendations.push(ModelRecommendation {
                    model_name: "llama3.2:70b".to_string(),
                    confidence: 0.95,
                    expected_performance: ModelPerformance {
                        avg_inference_time: 2000.0,
                        tokens_per_second: 10.0,
                        memory_usage: 45_000,
                        request_count: 0,
                        success_rate: 1.0,
                    },
                    hardware_requirements: HardwareRequirements {
                        min_ram: 32_000,
                        min_vram: Some(48_000),
                        recommended_cores: 16,
                        gpu_recommended: true,
                    },
                    reason: "Large VRAM available for high-quality model".to_string(),
                });
            } else if vram >= 24_000 {
                recommendations.push(ModelRecommendation {
                    model_name: "llama3.2:34b".to_string(),
                    confidence: 0.90,
                    expected_performance: ModelPerformance {
                        avg_inference_time: 1500.0,
                        tokens_per_second: 15.0,
                        memory_usage: 22_000,
                        request_count: 0,
                        success_rate: 1.0,
                    },
                    hardware_requirements: HardwareRequirements {
                        min_ram: 16_000,
                        min_vram: Some(24_000),
                        recommended_cores: 8,
                        gpu_recommended: true,
                    },
                    reason: "Medium VRAM suitable for balanced performance".to_string(),
                });
            } else if vram >= 8_000 {
                recommendations.push(ModelRecommendation {
                    model_name: "llama3.2:7b".to_string(),
                    confidence: 0.85,
                    expected_performance: ModelPerformance {
                        avg_inference_time: 800.0,
                        tokens_per_second: 25.0,
                        memory_usage: 7_000,
                        request_count: 0,
                        success_rate: 1.0,
                    },
                    hardware_requirements: HardwareRequirements {
                        min_ram: 8_000,
                        min_vram: Some(8_000),
                        recommended_cores: 4,
                        gpu_recommended: true,
                    },
                    reason: "Small VRAM suitable for efficient inference".to_string(),
                });
            }
        }
        
        // CPU-only recommendations
        if hardware_profile.memory.total >= 32_000 {
            recommendations.push(ModelRecommendation {
                model_name: "llama3.2:13b".to_string(),
                confidence: 0.70,
                expected_performance: ModelPerformance {
                    avg_inference_time: 3000.0,
                    tokens_per_second: 8.0,
                    memory_usage: 30_000,
                    request_count: 0,
                    success_rate: 1.0,
                },
                hardware_requirements: HardwareRequirements {
                    min_ram: 32_000,
                    min_vram: None,
                    recommended_cores: 8,
                    gpu_recommended: false,
                },
                reason: "CPU-only inference with large RAM".to_string(),
            });
        }
        
        // Fallback recommendation
        recommendations.push(ModelRecommendation {
            model_name: "llama3.2:3b".to_string(),
            confidence: 0.60,
            expected_performance: ModelPerformance {
                avg_inference_time: 1200.0,
                tokens_per_second: 12.0,
                memory_usage: 3_000,
                request_count: 0,
                success_rate: 1.0,
            },
            hardware_requirements: HardwareRequirements {
                min_ram: 4_000,
                min_vram: None,
                recommended_cores: 2,
                gpu_recommended: false,
            },
            reason: "Lightweight model for limited hardware".to_string(),
        });
        
        // Sort by confidence
        recommendations.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        Ok(recommendations)
    }

    /// Generate a completion with performance tracking
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<GenerateOptions>,
    ) -> Result<String, AIError> {
        let start_time = std::time::Instant::now();
        
        log::debug!("Generating completion with model: {}", model);
        
        let url = format!("{}/api/generate", self.base_url);
        
        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: false,
            options,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                self.update_metrics(false, start_time.elapsed());
                AIError::OllamaConnection(e.to_string())
            })?;

        if !response.status().is_success() {
            self.update_metrics(false, start_time.elapsed());
            return Err(AIError::OllamaConnection(format!(
                "Generation failed: {}",
                response.status()
            )));
        }

        let data: GenerateResponse = response
            .json()
            .await
            .map_err(|e| {
                self.update_metrics(false, start_time.elapsed());
                AIError::OllamaConnection(e.to_string())
            })?;

        let elapsed = start_time.elapsed();
        self.update_metrics(true, elapsed);
        self.update_model_performance(model, &data, elapsed);
        
        log::debug!("Generation completed in {:?}", elapsed);
        
        Ok(data.response)
    }

    /// Update performance metrics
    fn update_metrics(&self, success: bool, duration: Duration) {
        let mut metrics = self.metrics.lock();
        metrics.total_requests += 1;
        metrics.total_inference_time += duration;
        
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // Update average response time
        metrics.avg_response_time = metrics.total_inference_time / metrics.total_requests as u32;
    }
    
    /// Update model performance metrics
    fn update_model_performance(&self, model: &str, response: &GenerateResponse, duration: Duration) {
        if let Some(mut cached_model) = self.model_cache.get_mut(model) {
            let perf = &mut cached_model.performance;
            perf.request_count += 1;
            
            // Update average inference time
            let old_avg = perf.avg_inference_time;
            perf.avg_inference_time = (old_avg * (perf.request_count - 1) as f64 + duration.as_millis() as f64) / perf.request_count as f64;
            
            // Calculate tokens per second
            if let Some(eval_count) = response.eval_count {
                if let Some(eval_duration) = response.eval_duration {
                    let tokens_per_ns = eval_count as f64 / eval_duration as f64;
                    perf.tokens_per_second = tokens_per_ns * 1_000_000_000.0; // Convert to tokens per second
                }
            }
            
            cached_model.last_accessed = Utc::now();
        }
    }

    /// Generate a streaming completion
    pub async fn generate_stream(
        &self,
        model: &str,
        prompt: &str,
        options: Option<GenerateOptions>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, AIError>> + Send>>, AIError> {
        let url = format!("{}/api/generate", self.base_url);
        
        let request = GenerateRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
            stream: true,
            options,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::OllamaConnection(format!(
                "Generation failed: {}",
                response.status()
            )));
        }

        // Create a stream that reads the response body line by line
        let stream = async_stream::stream! {
            let body = response.text().await.map_err(|e| AIError::OllamaConnection(e.to_string()))?;
            
            for line in body.lines() {
                if let Ok(response) = serde_json::from_str::<GenerateStreamResponse>(line) {
                    yield Ok(response.response);
                }
            }
        };

        Ok(Box::pin(stream))
    }

    /// Chat with the model
    pub async fn chat(
        &self,
        model: &str,
        messages: &[Message],
        options: Option<GenerateOptions>,
    ) -> Result<String, AIError> {
        let url = format!("{}/api/chat", self.base_url);
        
        let ollama_messages: Vec<OllamaMessage> = messages
            .iter()
            .map(|msg| OllamaMessage {
                role: match msg.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let request = ChatRequest {
            model: model.to_string(),
            messages: ollama_messages,
            stream: false,
            options,
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::OllamaConnection(format!(
                "Chat failed: {}",
                response.status()
            )));
        }

        let data: ChatResponse = response
            .json()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        Ok(data.message.content)
    }

    /// Get embeddings for text
    pub async fn embeddings(
        &self,
        model: &str,
        prompt: &str,
    ) -> Result<Vec<f32>, AIError> {
        let url = format!("{}/api/embeddings", self.base_url);
        
        let request = EmbeddingsRequest {
            model: model.to_string(),
            prompt: prompt.to_string(),
        };

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::OllamaConnection(format!(
                "Embeddings failed: {}",
                response.status()
            )));
        }

        let data: EmbeddingsResponse = response
            .json()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        Ok(data.embedding)
    }
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub modified_at: String,
    pub size: u64,
    pub digest: String,
}

/// Response from list models API
#[derive(Debug, Deserialize)]
struct ListModelsResponse {
    models: Vec<ModelInfo>,
}

/// Request to pull a model
#[derive(Debug, Serialize)]
struct PullRequest {
    name: String,
}

/// Response from pull API
#[derive(Debug, Deserialize)]
struct PullResponse {
    status: String,
    completed: u64,
    total: u64,
    #[serde(default)]
    digest: String,
}

/// Request to delete a model
#[derive(Debug, Serialize)]
struct DeleteRequest {
    name: String,
}

/// Generate request
#[derive(Debug, Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

/// Generate options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    /// Temperature for sampling (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Number of tokens to predict
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Seed for deterministic generation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            temperature: Some(0.7),
            top_p: None,
            top_k: None,
            num_predict: None,
            stop: None,
            seed: None,
        }
    }
}

/// Generate response
#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: String,
    done: bool,
    context: Option<Vec<u32>>,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u64>,
}

/// Streaming generate response
#[derive(Debug, Deserialize)]
struct GenerateStreamResponse {
    response: String,
    done: bool,
}

/// Ollama message format
#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Chat request
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

/// Chat response
#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: OllamaMessage,
    done: bool,
    total_duration: Option<u64>,
    load_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    prompt_eval_duration: Option<u64>,
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
}

/// Embeddings request
#[derive(Debug, Serialize)]
struct EmbeddingsRequest {
    model: String,
    prompt: String,
}

/// Embeddings response
#[derive(Debug, Deserialize)]
struct EmbeddingsResponse {
    embedding: Vec<f32>,
}

/// Chat stream response
#[derive(Debug, Deserialize)]
struct ChatStreamResponse {
    message: OllamaMessage,
    done: bool,
}

/// Server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub version: String,
    #[serde(default)]
    pub build_date: String,
    #[serde(default)]
    pub commit: String,
}

/// Health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub response_time: Duration,
    pub models_available: usize,
    pub error_message: Option<String>,
    pub last_check: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_options_default() {
        let options = GenerateOptions::default();
        assert_eq!(options.temperature, Some(0.7));
        assert!(options.top_p.is_none());
        assert!(options.top_k.is_none());
    }
}