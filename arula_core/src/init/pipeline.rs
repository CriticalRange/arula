//! AI Pipeline for extracting semantic fragments
//!
//! This module implements the 4-step pipeline that extracts minimal
//! semantic fragments from AI without triggering limitations.

use crate::api::agent_client::AgentClient;
use crate::init::fragments::*;
use anyhow::Result;

/// Failure mitigation rules prepended to every AI call
const FAILURE_RULES: &str = "
RULES:
- Never use markdown formatting
- Output exactly 1-15 lines per response
- Never add explanations or commentary
- Follow specified output format precisely
- Never introduce undefined fields
- Use lowercase unless specifying proper names
- One concept per line when listing items
- Never refuse to answer with minimal fragments
";

/// Pipeline executor for AI fragment extraction
pub struct InitPipeline {
    agent_client: AgentClient,
}

impl InitPipeline {
    pub fn new(agent_client: AgentClient) -> Self {
        Self { agent_client }
    }

    /// Step 1: Extract domain fragment
    pub async fn extract_domain(&self, description: &str) -> Result<DomainFragment> {
        let instruction = format!(
            "{}Given the project description, identify the primary domain and up to 3 secondary concerns.

Description: {}

Respond with format:
domain: <primary_domain>
concerns: <max 3 items, comma-separated>
scale: <small|medium|large>
sensitivity: <low|medium|high>",
            FAILURE_RULES,
            description
        );

        let response = self.query_ai(&instruction).await?;
        self.parse_domain_fragment(&response)
    }

    /// Step 2: Extract flow fragment
    pub async fn decompose_flow(&self, description: &str) -> Result<FlowFragment> {
        let instruction = format!(
            "{}List the essential operations this project requires.

Description: {}

List operations (verb or verb-noun pairs).
One per line. Max 3 words per line.
Aim for 5-10 but any number is acceptable.",
            FAILURE_RULES,
            description
        );

        let response = self.query_ai(&instruction).await?;
        self.parse_flow_fragment(&response)
    }

    /// Step 3: Extract constraint fragment
    pub async fn capture_constraints(&self, description: &str) -> Result<ConstraintFragment> {
        let instruction = format!(
            "{}Return constraints for this project.

Description: {}

Categories: platform, language, framework, compliance, performance, security
Format: category: value
One per line. Max 15 chars per line.",
            FAILURE_RULES,
            description
        );

        let response = self.query_ai(&instruction).await?;
        self.parse_constraint_fragment(&response)
    }

    /// Step 4: Extract example fragment
    pub async fn extract_examples(&self, description: &str) -> Result<ExampleFragment> {
        let instruction = format!(
            "{}Show concrete usage examples for this project.

Description: {}

Format:
INPUT: <example>
OUTPUT: <example>

Show 2-3 examples. Keep under 40 chars each line.",
            FAILURE_RULES,
            description
        );

        let response = self.query_ai(&instruction).await?;
        self.parse_example_fragment(&response)
    }

    /// Execute AI query with retry logic
    async fn query_ai(&self, instruction: &str) -> Result<String> {
        // Use retry logic for robustness
        let mut attempts = 0;
        let max_attempts = 3;

        while attempts < max_attempts {
            match self.agent_client.query(instruction, None).await {
                Ok(mut blocks) => {
                    // Extract text from response blocks
                    let mut content = String::new();
                    use futures::StreamExt;
                    while let Some(block) = blocks.next().await {
                        if let crate::api::agent::ContentBlock::Text { text } = block {
                            content.push_str(&text);
                        }
                    }

                    if self.validate_response(&content) {
                        return Ok(content.trim().to_string());
                    }
                }
                Err(e) if attempts == max_attempts - 1 => return Err(e),
                Err(_) => attempts += 1,
            }
        }

        Err(anyhow::anyhow!("Failed to get valid AI response after {} attempts", max_attempts))
    }

    /// Validate response follows constraints
    fn validate_response(&self, response: &str) -> bool {
        !response.is_empty()
            && response.lines().count() <= 15
            && !response.contains("```")
            && !response.contains("**")
            && !response.contains("##")
    }

    /// Parse domain fragment from AI response
    fn parse_domain_fragment(&self, response: &str) -> Result<DomainFragment> {
        let mut fragment = DomainFragment::default();

        for line in response.lines() {
            if line.starts_with("domain:") {
                fragment.primary = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("concerns:") {
                fragment.secondary_concerns = line.split(':')
                    .nth(1)
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .take(3)  // Limit to first 3 concerns
                    .collect();
            } else if line.starts_with("scale:") {
                fragment.scale_category = line.split(':').nth(1).unwrap_or("").trim().to_string();
            } else if line.starts_with("sensitivity:") {
                fragment.data_sensitivity = line.split(':').nth(1).unwrap_or("").trim().to_string();
            }
        }

        Ok(fragment)
    }

    /// Parse flow fragment from AI response
    fn parse_flow_fragment(&self, response: &str) -> Result<FlowFragment> {
        let mut fragment = FlowFragment::default();
        fragment.actions = response
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(fragment)
    }

    /// Parse constraint fragment from AI response
    fn parse_constraint_fragment(&self, response: &str) -> Result<ConstraintFragment> {
        let mut fragment = ConstraintFragment::default();
        let mut map = std::collections::HashMap::new();

        for line in response.lines() {
            if let Some((key, value)) = line.split_once(':') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        fragment.constraints = map;
        Ok(fragment)
    }

    /// Parse example fragment from AI response
    fn parse_example_fragment(&self, response: &str) -> Result<ExampleFragment> {
        let mut fragment = ExampleFragment::default();
        let lines: Vec<&str> = response.lines().collect();

        for i in (0..lines.len()).step_by(2) {
            if i + 1 < lines.len() {
                let input = lines[i];
                let output = lines[i + 1];

                if let Some(input_val) = input.strip_prefix("INPUT:").map(|s| s.trim()) {
                    if let Some(output_val) = output.strip_prefix("OUTPUT:").map(|s| s.trim()) {
                        fragment.scenarios.push((input_val.to_string(), output_val.to_string()));
                    }
                }
            }
        }

        Ok(fragment)
    }
}