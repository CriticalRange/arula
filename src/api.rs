use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc;
use futures::StreamExt;
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestAssistantMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client as OpenAIClient,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: Option<u32>,
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub message: ChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponse {
    pub choices: Vec<OpenAIChoice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub response: String,
    pub success: bool,
    pub error: Option<String>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone)]
pub enum StreamingResponse {
    Start,
    Chunk(String),
    End(ApiResponse),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum AIProvider {
    OpenAI,
    Claude,
    Ollama,
    ZAiCoding,
    Custom,
}

#[derive(Debug, Clone)]
pub struct ApiClient {
    client: Client,
    openai_client: Option<OpenAIClient<OpenAIConfig>>,
    pub provider: AIProvider,
    endpoint: String,
    api_key: String,
    model: String,
}

impl ApiClient {
    pub fn new(provider: String, endpoint: String, api_key: String, model: String) -> Self {
        let provider_type = match provider.to_lowercase().as_str() {
            "openai" => AIProvider::OpenAI,
            "claude" | "anthropic" => AIProvider::Claude,
            "ollama" => AIProvider::Ollama,
            "z.ai coding plan" | "z.ai" | "zai" => AIProvider::ZAiCoding,
            _ => AIProvider::Custom,
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("arula-cli/1.0")
            .build()
            .expect("Failed to create HTTP client");

        // Initialize OpenAI client for streaming support
        let openai_client = if matches!(provider_type, AIProvider::OpenAI) && !api_key.is_empty() {
            let mut config = OpenAIConfig::new().with_api_key(&api_key);
            if !endpoint.is_empty() && endpoint != "https://api.openai.com/v1" {
                config = config.with_api_base(&endpoint);
            }
            Some(OpenAIClient::with_config(config))
        } else {
            None
        };

        Self { client, openai_client, provider: provider_type, endpoint, api_key, model }
    }

    pub async fn send_message(&self, message: &str, conversation_history: Option<Vec<ChatMessage>>) -> Result<ApiResponse> {
        let mut messages = Vec::new();

        // Add system message
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: "You are ARULA, an Autonomous AI Interface assistant. You help users with coding, shell commands, and general software development tasks. Be concise, helpful, and provide practical solutions.".to_string(),
        });

        // Add conversation history if provided
        if let Some(history) = conversation_history {
            for msg in history {
                if msg.role != "system" {
                    messages.push(msg);
                }
            }
        }

        // Add current user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        match self.provider {
            AIProvider::OpenAI => self.send_openai_request(messages).await,
            AIProvider::Claude => self.send_claude_request(messages).await,
            AIProvider::Ollama => self.send_ollama_request(messages).await,
            AIProvider::ZAiCoding => self.send_zai_request(messages).await,
            AIProvider::Custom => self.send_custom_request(messages).await,
        }
    }

    pub async fn send_message_stream(&self, message: &str, conversation_history: Option<Vec<ChatMessage>>) -> Result<mpsc::UnboundedReceiver<StreamingResponse>> {
        let mut messages = Vec::new();

        // Add system message
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: "You are ARULA, an Autonomous AI Interface assistant. You help users with coding, shell commands, and general software development tasks. Be concise, helpful, and provide practical solutions.".to_string(),
        });

        // Add conversation history if provided
        if let Some(history) = conversation_history {
            for msg in history {
                if msg.role != "system" {
                    messages.push(msg);
                }
            }
        }

        // Add current user message
        messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let (tx, rx) = mpsc::unbounded_channel();

        match self.provider {
            AIProvider::OpenAI => {
                if let Some(openai_client) = self.openai_client.clone() {
                    let model = self.model.clone();
                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_openai_stream(&openai_client, messages, &model, tx.clone()).await {
                            let _ = tx.send(StreamingResponse::Error(format!("OpenAI streaming error: {}", e)));
                        }
                    });
                } else {
                    tokio::spawn(async move {
                        let _ = tx.send(StreamingResponse::Error("OpenAI client not initialized. Please configure your API key.".to_string()));
                    });
                }
            }
            _ => {
                // Fallback to non-streaming for other providers
                let client = self.clone();
                let last_message = messages.last().map(|m| m.content.clone()).unwrap_or_default();
                tokio::spawn(async move {
                    match client.send_message(&last_message, Some(messages)).await {
                        Ok(response) => {
                            let _ = tx.send(StreamingResponse::Start);
                            let _ = tx.send(StreamingResponse::Chunk(response.response.clone()));
                            let _ = tx.send(StreamingResponse::End(response));
                        }
                        Err(e) => {
                            let _ = tx.send(StreamingResponse::Error(format!("Request failed: {}", e)));
                        }
                    }
                });
            }
        }

        Ok(rx)
    }

    async fn send_openai_request(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let mut request_builder = self.client
            .post(format!("{}/chat/completions", self.endpoint))
            .json(&request);

        // Add authorization header if API key is provided
        if !self.api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            let openai_response: OpenAIResponse = response.json().await?;

            if let Some(choice) = openai_response.choices.first() {
                Ok(ApiResponse {
                    response: choice.message.content.clone(),
                    success: true,
                    error: None,
                    usage: openai_response.usage,
                })
            } else {
                Ok(ApiResponse {
                    response: "No response received".to_string(),
                    success: false,
                    error: Some("No choices in response".to_string()),
                    usage: None,
                })
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("OpenAI API request failed: {}", error_text))
        }
    }

    async fn send_claude_request(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        let claude_messages: Vec<Value> = messages.into_iter().map(|msg| {
            json!({
                "role": msg.role,
                "content": msg.content
            })
        }).collect();

        let request = json!({
            "model": self.model,
            "messages": claude_messages,
            "max_tokens": 2048,
            "temperature": 0.7
        });

        let mut request_builder = self.client
            .post(format!("{}/v1/messages", self.endpoint))
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .json(&request);

        // Add authorization header if API key is provided
        if !self.api_key.is_empty() {
            request_builder = request_builder.header("x-api-key", &self.api_key);
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            let claude_response: Value = response.json().await?;

            if let Some(content) = claude_response["content"].as_array() {
                if let Some(text_block) = content.first() {
                    if let Some(text) = text_block["text"].as_str() {
                        return Ok(ApiResponse {
                            response: text.to_string(),
                            success: true,
                            error: None,
                            usage: None, // Claude has different usage format
                        });
                    }
                }
            }

            Ok(ApiResponse {
                response: "Invalid Claude response format".to_string(),
                success: false,
                error: Some("Could not parse Claude response".to_string()),
                usage: None,
            })
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Claude API request failed: {}", error_text))
        }
    }

    async fn send_ollama_request(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        // Convert messages to Ollama format
        let prompt = messages.iter()
            .map(|msg| format!("{}: {}", msg.role.to_uppercase(), msg.content))
            .collect::<Vec<_>>()
            .join("\n");

        let request = json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.7,
                "num_predict": 2048
            }
        });

        let response = self.client
            .post(format!("{}/api/generate", self.endpoint))
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let ollama_response: Value = response.json().await?;

            if let Some(response_text) = ollama_response["response"].as_str() {
                Ok(ApiResponse {
                    response: response_text.to_string(),
                    success: true,
                    error: None,
                    usage: None,
                })
            } else {
                Ok(ApiResponse {
                    response: "Invalid Ollama response format".to_string(),
                    success: false,
                    error: Some("Could not parse Ollama response".to_string()),
                    usage: None,
                })
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Ollama API request failed: {}", error_text))
        }
    }

    async fn send_zai_request(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        // Z.AI uses OpenAI-compatible format with specific endpoint
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let mut request_builder = self.client
            .post(format!("{}/chat/completions", self.endpoint))  // Z.AI uses this exact path
            .json(&request);

        // Add authorization header if API key is provided
        if !self.api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            let openai_response: OpenAIResponse = response.json().await?;

            if let Some(choice) = openai_response.choices.first() {
                Ok(ApiResponse {
                    response: choice.message.content.clone(),
                    success: true,
                    error: None,
                    usage: openai_response.usage,
                })
            } else {
                Ok(ApiResponse {
                    response: "No response received".to_string(),
                    success: false,
                    error: Some("No choices in response".to_string()),
                    usage: None,
                })
            }
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Z.AI API request failed: {}", error_text))
        }
    }

    async fn send_custom_request(&self, messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        // For custom providers, use a generic format similar to OpenAI
        let request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: Some(2048),
            stream: Some(false),
        };

        let mut request_builder = self.client
            .post(format!("{}/api/chat", self.endpoint))
            .json(&request);

        // Add authorization header if API key is provided
        if !self.api_key.is_empty() {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request_builder.send().await?;

        if response.status().is_success() {
            let api_response: ApiResponse = response.json().await?;
            Ok(api_response)
        } else {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            Err(anyhow::anyhow!("Custom API request failed: {}", error_text))
        }
    }


    // Handle OpenAI streaming with real API
    async fn handle_openai_stream(
        openai_client: &OpenAIClient<OpenAIConfig>,
        messages: Vec<ChatMessage>,
        model: &str,
        tx: mpsc::UnboundedSender<StreamingResponse>,
    ) -> Result<()> {
        // Send start signal
        let _ = tx.send(StreamingResponse::Start);

        // Convert our ChatMessage format to OpenAI format
        let mut openai_messages = Vec::new();
        for msg in messages {
            let message: ChatCompletionRequestMessage = match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content)
                    .build()?
                    .into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content)
                    .build()?
                    .into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content)
                    .build()?
                    .into(),
            };
            openai_messages.push(message);
        }

        // Create streaming request
        let request = CreateChatCompletionRequestArgs::default()
            .model(model)
            .messages(openai_messages)
            .temperature(0.7)
            .max_tokens(2048_u16)
            .build()?;

        // Get the stream
        let mut stream = openai_client.chat().create_stream(request).await?;

        let mut full_response = String::new();

        // Process stream chunks
        while let Some(result) = stream.next().await {
            match result {
                Ok(response) => {
                    for choice in response.choices {
                        if let Some(content) = choice.delta.content {
                            full_response.push_str(&content);
                            let _ = tx.send(StreamingResponse::Chunk(content));
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(StreamingResponse::Error(format!("Stream error: {}", e)));
                    return Err(anyhow::anyhow!("Stream error: {}", e));
                }
            }
        }

        // Send final response
        let final_response = ApiResponse {
            response: full_response,
            success: true,
            error: None,
            usage: None,
        };
        let _ = tx.send(StreamingResponse::End(final_response));

        Ok(())
    }

    // Fallback for non-streaming providers
    async fn fallback_non_streaming(messages: Vec<ChatMessage>) -> Result<ApiResponse> {
        // This is a simple fallback - in a real implementation, you'd want to reuse existing non-streaming logic
        let _system_content = messages.iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "You are ARULA, an AI assistant.".to_string());

        let user_content = messages.iter()
            .find(|m| m.role == "user")
            .map(|m| m.content.clone())
            .unwrap_or_else(|| "Hello".to_string());

        // For now, return a simple response
        Ok(ApiResponse {
            response: format!("Fallback response to: {}", user_content),
            success: true,
            error: None,
            usage: None,
        })
    }

    #[allow(dead_code)]
    pub async fn test_connection(&self) -> Result<bool> {
        let test_message = "Hello! This is a connection test. Please respond briefly.";
        match self.send_message(test_message, None).await {
            Ok(response) => Ok(response.success),
            Err(_) => Ok(false),
        }
    }
}