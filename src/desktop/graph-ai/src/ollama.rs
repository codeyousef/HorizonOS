//! Ollama integration for local LLM processing

use crate::{AIError, Message, MessageRole};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use futures::stream::Stream;
use std::pin::Pin;

/// Ollama client for interacting with the local LLM server
pub struct OllamaClient {
    /// HTTP client
    client: Client,
    /// Base URL for Ollama API
    base_url: String,
}

impl OllamaClient {
    /// Create a new Ollama client
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout for long responses
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            base_url: base_url.to_string(),
        }
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

    /// List available models
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

        Ok(data.models)
    }

    /// Pull a model if not already available
    pub async fn pull_model(&self, model_name: &str) -> Result<(), AIError> {
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

        // TODO: Stream the pull progress
        Ok(())
    }

    /// Generate a completion
    pub async fn generate(
        &self,
        model: &str,
        prompt: &str,
        options: Option<GenerateOptions>,
    ) -> Result<String, AIError> {
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
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AIError::OllamaConnection(format!(
                "Generation failed: {}",
                response.status()
            )));
        }

        let data: GenerateResponse = response
            .json()
            .await
            .map_err(|e| AIError::OllamaConnection(e.to_string()))?;

        Ok(data.response)
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
            let mut body = response.text().await.map_err(|e| AIError::OllamaConnection(e.to_string()))?;
            
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
    eval_count: Option<u32>,
    eval_duration: Option<u64>,
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