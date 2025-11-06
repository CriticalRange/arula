use anyhow::Result;
use duct::cmd;
use crate::progress::ProgressHelper;

pub struct CommandRunner {
    progress: ProgressHelper,
}

impl CommandRunner {
    pub fn new() -> Self {
        Self {
            progress: ProgressHelper::new(),
        }
    }

    pub async fn run_command(&mut self, command: String, args: Vec<String>) -> Result<String> {
        let message = format!("Running: {} {}", command, args.join(" "));
        self.progress.with_progress(&message, move || {
            let expression = cmd(command, args);
            expression.read().map_err(|e| anyhow::anyhow!("Command failed: {}", e))
        }).await
    }

    }

impl Drop for CommandRunner {
    fn drop(&mut self) {
        self.progress.finish();
    }
}