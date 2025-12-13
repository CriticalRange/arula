//! Semantic fragment definitions
//!
//! Minimal data structures that AI can reliably generate
//! without triggering LLM limitations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Domain fragment - 4 fields max
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DomainFragment {
    pub primary: String,
    pub secondary_concerns: Vec<String>,
    pub scale_category: String,
    pub data_sensitivity: String,
}

/// Flow fragment - flat list of actions
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowFragment {
    pub actions: Vec<String>,
}

/// Constraint fragment - key-value pairs only
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConstraintFragment {
    pub constraints: HashMap<String, String>,
}

/// Example fragment - input/output pairs
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExampleFragment {
    pub scenarios: Vec<(String, String)>,
}

/// Complete project blueprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBlueprint {
    pub domain: DomainFragment,
    pub flow: FlowFragment,
    pub constraints: ConstraintFragment,
    pub examples: ExampleFragment,
}

/// Generated SBP files
#[derive(Debug, Clone)]
pub struct SbpFiles {
    pub domain_sbp: String,
    pub flow_sbp: String,
    pub constraints_sbp: String,
    pub examples_sbp: String,
}

impl DomainFragment {
    pub fn validate(&self) -> Result<(), String> {
        if self.primary.is_empty() {
            return Err("Primary domain required".to_string());
        }
        // Validation now handles truncation automatically in parsing
        Ok(())
    }
}

impl FlowFragment {
    pub fn validate(&self) -> Result<(), String> {
        if self.actions.is_empty() {
            return Err("At least 1 action required".to_string());
        }
        // Allow any number of actions but limit to 10 in SBP generation
        for action in &self.actions {
            if action.split_whitespace().count() > 3 {
                return Err(format!("Action too long: {}", action));
            }
        }
        Ok(())
    }
}

impl ConstraintFragment {
    pub fn validate(&self) -> Result<(), String> {
        const VALID_CATEGORIES: &[&str] = &[
            "platform", "language", "framework",
            "compliance", "performance", "security"
        ];

        for (category, value) in &self.constraints {
            if !VALID_CATEGORIES.contains(&category.as_str()) {
                return Err(format!("Invalid category: {}", category));
            }
            if value.len() > 15 {
                return Err(format!("Value too long for {}: {}", category, value));
            }
        }
        Ok(())
    }
}

impl ExampleFragment {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenarios.is_empty() {
            return Err("At least 1 example required".to_string());
        }
        // Allow any number of examples but limit to 3 in SBP generation
        for (input, output) in &self.scenarios {
            if input.len() > 40 || output.len() > 40 {
                return Err("Example too long (max 40 chars)".to_string());
            }
        }
        Ok(())
    }
}

impl ProjectBlueprint {
    pub fn validate(&self) -> Result<(), String> {
        self.domain.validate()?;
        self.flow.validate()?;
        self.constraints.validate()?;
        self.examples.validate()?;
        Ok(())
    }
}