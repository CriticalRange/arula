//! SBP (Semantic Blueprint) file assembler
//!
//! Converts semantic fragments into deterministic SBP files.

use crate::init::fragments::*;
use anyhow::Result;

/// Assembles SBP files from fragments
pub struct SbpAssembler;

impl SbpAssembler {
    pub fn new() -> Self {
        Self
    }

    /// Convert blueprint to SBP files
    pub fn assemble(&self, blueprint: &ProjectBlueprint) -> Result<SbpFiles> {
        blueprint.validate().map_err(|e| anyhow::anyhow!("Invalid blueprint: {}", e))?;

        let domain_sbp = self.assemble_domain_sbp(&blueprint.domain);
        let flow_sbp = self.assemble_flow_sbp(&blueprint.flow);
        let constraints_sbp = self.assemble_constraints_sbp(&blueprint.constraints);
        let examples_sbp = self.assemble_examples_sbp(&blueprint.examples);

        Ok(SbpFiles {
            domain_sbp,
            flow_sbp,
            constraints_sbp,
            examples_sbp,
        })
    }

    /// Assemble DOMAIN.sbp from domain fragment
    fn assemble_domain_sbp(&self, fragment: &DomainFragment) -> String {
        let mut sbp = String::new();

        sbp.push_str("DOMAIN {\n");
        sbp.push_str(&format!("  primary: {}\n", fragment.primary));

        if !fragment.secondary_concerns.is_empty() {
            sbp.push_str("  concerns: [\n");
            for concern in &fragment.secondary_concerns {
                sbp.push_str(&format!("    {}\n", concern));
            }
            sbp.push_str("  ]\n");
        }

        if !fragment.scale_category.is_empty() {
            sbp.push_str(&format!("  scale: {}\n", fragment.scale_category));
        }

        if !fragment.data_sensitivity.is_empty() {
            sbp.push_str(&format!("  sensitivity: {}\n", fragment.data_sensitivity));
        }

        sbp.push_str("}\n");

        sbp
    }

    /// Assemble FLOW.sbp from flow fragment
    fn assemble_flow_sbp(&self, fragment: &FlowFragment) -> String {
        let mut sbp = String::new();

        sbp.push_str("FLOW {\n");
        sbp.push_str("  actions: [\n");

        // Limit to first 10 actions
        for (i, action) in fragment.actions.iter().take(10).enumerate() {
            sbp.push_str(&format!("    {}: {}\n", i + 1, action));
        }

        sbp.push_str("  ]\n");
        sbp.push_str("}\n");

        sbp
    }

    /// Assemble CONSTRAINTS.sbp from constraint fragment
    fn assemble_constraints_sbp(&self, fragment: &ConstraintFragment) -> String {
        let mut sbp = String::new();

        sbp.push_str("CONSTRAINTS {\n");

        for (category, value) in &fragment.constraints {
            sbp.push_str(&format!("  {}: {}\n", category, value));
        }

        sbp.push_str("}\n");

        sbp
    }

    /// Assemble EXAMPLES.sbp from example fragment
    fn assemble_examples_sbp(&self, fragment: &ExampleFragment) -> String {
        let mut sbp = String::new();

        sbp.push_str("EXAMPLES {\n");
        sbp.push_str("  scenarios: [\n");

        // Limit to first 3 examples
        for (i, (input, output)) in fragment.scenarios.iter().take(3).enumerate() {
            sbp.push_str(&format!(
                "    {} {{\n      input: \"{}\"\n      output: \"{}\"\n    }}\n",
                i + 1,
                input.replace('"', "\\\""),
                output.replace('"', "\\\"")
            ));
        }

        sbp.push_str("  ]\n");
        sbp.push_str("}\n");

        sbp
    }
}