use anyhow::Result;
use tokio::sync::mpsc;
use crate::api::ApiClient;
use crate::config::Config;
use crate::chat::{ChatMessage, MessageType};
use crate::tool_call::extract_bash_commands;

#[derive(Debug, Clone)]
pub enum AiResponse {
    Success { response: String, usage: Option<crate::api::Usage> },
    Error(String),
    StreamStart,
    StreamChunk(String),
    StreamEnd,
}

pub struct App {
    pub config: Config,
    pub api_client: Option<ApiClient>,
    pub messages: Vec<ChatMessage>,
    pub ai_response_rx: Option<mpsc::UnboundedReceiver<AiResponse>>,
    pub current_streaming_message: Option<String>,
    pub pending_bash_commands: Option<Vec<String>>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load_or_default()?;

        Ok(Self {
            config,
            api_client: None,
            messages: Vec::new(),
            ai_response_rx: None,
            current_streaming_message: None,
            pending_bash_commands: None,
        })
    }

    pub fn initialize_api_client(&mut self) -> Result<()> {
        self.api_client = Some(ApiClient::new(
            self.config.ai.provider.clone(),
            self.config.ai.api_url.clone(),
            self.config.ai.api_key.clone(),
            self.config.ai.model.clone(),
        ));
        Ok(())
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn set_model(&mut self, model: &str) {
        self.config.ai.model = model.to_string();
        let _ = self.config.save();
        // Reinitialize API client with new model
        let _ = self.initialize_api_client();
    }

    pub fn clear_conversation(&mut self) {
        self.messages.clear();
    }

    pub async fn send_to_ai(&mut self, message: &str) -> Result<()> {
        // Add user message to history
        self.messages.push(ChatMessage::new(MessageType::User, message.to_string()));

        // Get API client
        let api_client = match &self.api_client {
            Some(client) => client.clone(),
            None => {
                return Err(anyhow::anyhow!("API client not initialized"));
            }
        };

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();
        self.ai_response_rx = Some(rx);

        // Convert message history to API format
        let message_history: Vec<crate::api::ChatMessage> = self.messages
            .iter()
            .map(|m| {
                let role = match m.message_type {
                    MessageType::User => "user".to_string(),
                    MessageType::Arula => "assistant".to_string(),
                    _ => "system".to_string(),
                };
                crate::api::ChatMessage {
                    role,
                    content: m.content.clone(),
                }
            })
            .collect();

        let msg = message.to_string();

        // Send message in background
        tokio::spawn(async move {
            match api_client.send_message_stream(&msg, Some(message_history)).await {
                Ok(mut stream_rx) => {
                    let _ = tx.send(AiResponse::StreamStart);

                    while let Some(response) = stream_rx.recv().await {
                        match response {
                            crate::api::StreamingResponse::Start => {}
                            crate::api::StreamingResponse::Chunk(chunk) => {
                                let _ = tx.send(AiResponse::StreamChunk(chunk));
                            }
                            crate::api::StreamingResponse::End(_) => {
                                let _ = tx.send(AiResponse::StreamEnd);
                                break;
                            }
                            crate::api::StreamingResponse::Error(err) => {
                                let _ = tx.send(AiResponse::Error(err));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(AiResponse::Error(format!("Failed to send message: {}", e)));
                }
            }
        });

        Ok(())
    }

    pub fn check_ai_response_nonblocking(&mut self) -> Option<AiResponse> {
        if let Some(rx) = &mut self.ai_response_rx {
            match rx.try_recv() {
                Ok(response) => {
                    match &response {
                        AiResponse::StreamStart => {
                            self.current_streaming_message = Some(String::new());
                        }
                        AiResponse::StreamChunk(chunk) => {
                            if let Some(msg) = &mut self.current_streaming_message {
                                msg.push_str(chunk);
                            }
                        }
                        AiResponse::StreamEnd => {
                            if let Some(full_message) = self.current_streaming_message.take() {
                                // Extract bash commands before adding to messages
                                let bash_commands = extract_bash_commands(&full_message);
                                if !bash_commands.is_empty() {
                                    self.pending_bash_commands = Some(bash_commands);
                                }

                                // Remove code blocks from message
                                let cleaned = Self::remove_code_blocks(&full_message);
                                let final_message = if cleaned.is_empty() {
                                    "Executing commands...".to_string()
                                } else {
                                    cleaned
                                };

                                self.messages.push(ChatMessage::new(
                                    MessageType::Arula,
                                    final_message,
                                ));
                            }
                            self.ai_response_rx = None;
                        }
                        AiResponse::Success { response, .. } => {
                            // Extract bash commands
                            let bash_commands = extract_bash_commands(response);
                            if !bash_commands.is_empty() {
                                self.pending_bash_commands = Some(bash_commands);
                            }

                            // Remove code blocks from message
                            let cleaned = Self::remove_code_blocks(response);
                            let final_message = if cleaned.is_empty() {
                                "Executing commands...".to_string()
                            } else {
                                cleaned
                            };

                            self.messages.push(ChatMessage::new(
                                MessageType::Arula,
                                final_message,
                            ));
                            self.ai_response_rx = None;
                        }
                        AiResponse::Error(_) => {
                            self.ai_response_rx = None;
                        }
                    }
                    Some(response)
                }
                Err(mpsc::error::TryRecvError::Empty) => None,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.ai_response_rx = None;
                    Some(AiResponse::Error("AI request failed unexpectedly".to_string()))
                }
            }
        } else {
            None
        }
    }

    pub fn get_pending_bash_commands(&mut self) -> Option<Vec<String>> {
        self.pending_bash_commands.take()
    }

    pub async fn execute_bash_command(&self, command: &str) -> Result<String> {
        use std::process::Command;

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/C", command])
                .output()?
        } else {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(if stdout.is_empty() {
                "Command executed successfully".to_string()
            } else {
                stdout
            })
        } else {
            Err(anyhow::anyhow!("{}", if stderr.is_empty() {
                "Command failed".to_string()
            } else {
                stderr
            }))
        }
    }

    fn remove_code_blocks(text: &str) -> String {
        let mut result = String::new();
        let mut in_code_block = false;

        for line in text.lines() {
            if line.trim().starts_with("```") {
                in_code_block = !in_code_block;
            } else if !in_code_block {
                result.push_str(line);
                result.push('\n');
            }
        }

        result.trim().to_string()
    }
}
