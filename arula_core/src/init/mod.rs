//! Project initialization system using Semantic Blueprint Pipeline
//!
//! This module implements the `/init` command that uses AI to generate
//! project blueprints without triggering LLM limitations.

use crate::api::agent_client::AgentClient;
use crate::utils::config::Config;
use anyhow::Result;

pub mod example;
pub mod fragments;
pub mod pipeline;
pub mod sbp_assembler;

pub use example::*;
pub use fragments::*;
pub use pipeline::*;
pub use sbp_assembler::*;

/// Main init system orchestrator
#[derive(Clone)]
pub struct InitSystem {
    agent_client: AgentClient,
    config: Config,
}

impl InitSystem {
    pub fn new(agent_client: AgentClient, config: Config) -> Self {
        Self { agent_client, config }
    }

    /// Initialize a new project using the Semantic Blueprint Pipeline
    pub async fn initialize_project(&self, description: &str) -> Result<ProjectBlueprint> {
        let pipeline = InitPipeline::new(self.agent_client.clone());

        // Execute pipeline steps
        let domain_fragment = pipeline.extract_domain(description).await?;
        let flow_fragment = pipeline.decompose_flow(description).await?;
        let constraint_fragment = pipeline.capture_constraints(description).await?;
        let example_fragment = pipeline.extract_examples(description).await?;

        // Assemble blueprint
        let blueprint = ProjectBlueprint {
            domain: domain_fragment,
            flow: flow_fragment,
            constraints: constraint_fragment,
            examples: example_fragment,
        };

        Ok(blueprint)
    }

    /// Generate SBP files from blueprint
    pub fn generate_sbp_files(&self, blueprint: &ProjectBlueprint) -> Result<SbpFiles> {
        let assembler = SbpAssembler::new();
        assembler.assemble(blueprint)
    }
}