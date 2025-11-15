use std::io::{self, Write};
use console::style;
use crate::api::Usage;

pub struct OutputHandler {
    debug: bool,
}

impl OutputHandler {
    pub fn new() -> Self {
        Self { debug: false }
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    pub fn is_debug(&self) -> bool {
        self.debug
    }

    pub fn print_user_message(&mut self, content: &str) -> io::Result<()> {
        println!("{} {}", style("You:").cyan().bold(), content);
        Ok(())
    }

    pub fn print_ai_message(&mut self, content: &str) -> io::Result<()> {
        println!("{} {}", style("ARULA:").green().bold(), content);
        Ok(())
    }

    pub fn print_error(&mut self, content: &str) -> io::Result<()> {
        println!("{} {}", style("Error:").red().bold(), content);
        Ok(())
    }

    pub fn print_system(&mut self, content: &str) -> io::Result<()> {
        println!("{}", style(content).yellow().dim());
        Ok(())
    }

    pub fn print_tool_call(&mut self, name: &str, args: &str) -> io::Result<()> {
        if self.debug {
            println!("{} {}", style("🔧 Tool Call:").magenta().bold(), style(name).magenta());
            if !args.is_empty() {
                println!("   {}", style(format!("Args: {}", args)).dim());
            }
        }
        Ok(())
    }

    pub fn print_tool_result(&mut self, result: &str) -> io::Result<()> {
        let max_lines = if self.debug { 50 } else { 10 };
        let truncated_result = self.truncate_output(result, max_lines);
        if self.debug {
            println!("   {}", style(format!("Result: {}", truncated_result)).blue());
        } else {
            println!("   {}", style(truncated_result).blue());
        }
        Ok(())
    }

    fn truncate_output(&self, output: &str, max_lines: usize) -> String {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() <= max_lines {
            output.to_string()
        } else {
            let truncated_lines: Vec<String> = lines
                .iter()
                .take(max_lines)
                .map(|line| line.to_string())
                .collect();

            format!("{}\n... ({} more lines)", truncated_lines.join("\n"), lines.len() - max_lines)
        }
    }

    pub fn print_streaming_chunk(&mut self, chunk: &str) -> io::Result<()> {
        print!("{}", chunk);
        std::io::stdout().flush()?;
        Ok(())
    }

    pub fn start_ai_message(&mut self) -> io::Result<()> {
        print!("{} ", style("ARULA:").green().bold());
        std::io::stdout().flush()?;
        Ok(())
    }

    pub fn end_line(&mut self) -> io::Result<()> {
        println!();
        Ok(())
    }

    pub fn print_banner(&mut self) -> io::Result<()> {
        println!("{}", style("╔═══════════════════════════════════════╗").cyan().bold());
        println!("{}", style("║      ARULA - Autonomous AI CLI        ║").cyan().bold());
        println!("{}", style("╚═══════════════════════════════════════╝").cyan().bold());
        Ok(())
    }

    /// Print context usage information at the end of AI responses
    pub fn print_context_usage(&mut self, usage: Option<&Usage>) -> io::Result<()> {
        if self.debug {
            eprintln!("DEBUG: print_context_usage called with usage: {:?}", usage);
        }

        println!();
        println!("{}", style("┌─ Context Usage ───────────────────────").dim());

        if let Some(usage_info) = usage {
            // Standard context limits (adjust based on model)
            let max_context_tokens: u32 = 128000; // Typical for modern models
            let remaining_tokens = max_context_tokens.saturating_sub(usage_info.total_tokens);
            let usage_percentage = (usage_info.total_tokens as f64 / max_context_tokens as f64) * 100.0;

            // Choose color based on usage level for tokens used
            let used_color = if usage_percentage > 90.0 {
                style(format!("{}", usage_info.total_tokens)).red().bold()
            } else if usage_percentage > 75.0 {
                style(format!("{}", usage_info.total_tokens)).yellow().bold()
            } else {
                style(format!("{}", usage_info.total_tokens)).green()
            };

            // Choose color based on usage level for remaining tokens
            let remaining_color = if usage_percentage > 90.0 {
                style(format!("{}", remaining_tokens)).red().bold()
            } else if usage_percentage > 75.0 {
                style(format!("{}", remaining_tokens)).yellow().bold()
            } else {
                style(format!("{}", remaining_tokens)).green()
            };

            println!("│ {} tokens used ({:.1}%)", used_color, usage_percentage);
            println!("│ {} tokens remaining", remaining_color);

            // Add visual indicator
            let used_bars = (usage_percentage / 100.0 * 20.0) as usize;
            let remaining_bars = 20 - used_bars;
            let bar = "█".repeat(used_bars) + &"░".repeat(remaining_bars);

            let bar_color = if usage_percentage > 90.0 {
                style(&bar).red().bold()
            } else if usage_percentage > 75.0 {
                style(&bar).yellow().bold()
            } else {
                style(&bar).green()
            };

            println!("│ [{}]", bar_color);

            if usage_percentage > 90.0 {
                println!("│ {}", style("⚠️  Warning: Approaching context limit!").red().bold());
            } else if usage_percentage > 75.0 {
                println!("│ {}", style("ℹ️  Note: Context usage is getting high").yellow());
            }
        } else {
            // No usage data available from API
            println!("│ {}", style("Usage data not available from API").dim());
            println!("│ {} tokens estimated available", style("128,000").dim());
            println!("│ [{}]", style("░░░░░░░░░░░░░░░░░░░░").dim());
            println!("│ {}", style("💡 Note: Some providers don't return usage stats").dim());
        }

        println!("{}", style("└───────────────────────────────────").dim());
        Ok(())
    }
}

impl Default for OutputHandler {
    fn default() -> Self {
        Self::new()
    }
}
