//! Example usage of the init system
//!
//! This file demonstrates how to use the Semantic Blueprint Pipeline
//! to initialize projects without triggering LLM limitations.

use crate::api::agent_client::AgentClient;
use crate::api::agent::AgentOptionsBuilder;
use crate::init::{InitSystem, SbpFiles};
use crate::utils::config::Config;
use anyhow::Result;

/// Example project initialization
pub async fn example_init_project() -> Result<SbpFiles> {
    // Create agent client
    let config = Config::default();
    let agent_options = AgentOptionsBuilder::new()
        .system_prompt("You are a project initialization assistant.")
        .auto_execute_tools(false)
        .build();

    let agent_client = AgentClient::new(
        "openai".to_string(),
        "https://api.openai.com/v1".to_string(),
        "your-api-key".to_string(),
        "gpt-4".to_string(),
        agent_options,
        &config,
    );

    // Create init system
    let init_system = InitSystem::new(agent_client, config);

    // Project description
    let description = "A web API for task management with user authentication, real-time updates, and data persistence";

    // Initialize project
    let blueprint = init_system.initialize_project(description).await?;

    // Generate SBP files
    let sbp_files = init_system.generate_sbp_files(&blueprint)?;

    Ok(sbp_files)
}