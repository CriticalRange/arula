//! Provider selection menu for ARULA CLI

use crate::app::App;
use crate::output::OutputHandler;
use crate::ui::menus::common::{MenuUtils, MenuState};
use anyhow::Result;
use console::style;
use crossterm::{
    event::KeyCode,
    terminal,
    ExecutableCommand,
};
use std::io::{stdout, Write};

/// Provider menu handler
pub struct ProviderMenu {
    state: MenuState,
    providers: Vec<String>,
}

impl ProviderMenu {
    pub fn new() -> Self {
        Self {
            state: MenuState::new(),
            providers: vec![
                "openai".to_string(),
                "anthropic".to_string(),
                "ollama".to_string(),
                "z.ai coding plan".to_string(),
                "openrouter".to_string(),
                "custom".to_string(),
            ],
        }
    }

    /// Display and handle the provider selection menu
    pub fn show(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        // Check terminal size
        if !MenuUtils::check_terminal_size(30, 8)? {
            output.print_system("Terminal too small for provider menu")?;
            return Ok(());
        }

        // Get current provider index
        let current_provider = app.get_config().active_provider.clone();
        let current_idx = self.providers
            .iter()
            .position(|p| p == &current_provider)
            .unwrap_or(0);
        self.state.selected_index = current_idx;

        // Setup terminal
        MenuUtils::setup_terminal()?;

        let result = self.run_provider_loop(app, output);

        // Restore terminal
        MenuUtils::restore_terminal()?;

        // Clear any pending events
        self.clear_pending_events()?;

        result
    }

    /// Provider selection event loop
    fn run_provider_loop(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        loop {
            // Render provider menu
            self.render(output)?;

            // Handle input
            if self.handle_input(app)? {
                break; // Selection made
            }
        }
        Ok(())
    }

    /// Render the provider selection menu with original styling
    fn render(&self, _output: &mut OutputHandler) -> Result<()> {
        let (cols, rows) = crossterm::terminal::size()?;
        let menu_width = 45.min(cols);
        let menu_height = 12;

        // Clear entire screen before each render
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;

        // Center the menu
        let start_col = (cols - menu_width) / 2;
        let start_row = (rows - menu_height) / 2;

        // Render menu frame
        let frame = MenuUtils::render_box("AI Provider", menu_width, menu_height);
        for (i, line) in frame.iter().enumerate() {
            if i < menu_height as usize {
                stdout().execute(crossterm::cursor::MoveTo(start_col, start_row + i as u16))?;
                print!("{}", line);
            }
        }

        // Render provider options
        let start_row = 2;
        for (idx, provider) in self.providers.iter().enumerate() {
            if idx >= menu_height as usize - 6 {
                break;
            }

            let row = start_row + idx as u16;
            stdout().execute(crossterm::cursor::MoveTo(start_col + 2, row))?;

            let is_selected = idx == self.state.selected_index;
            let formatted = MenuUtils::format_menu_item(provider, is_selected);

            if is_selected {
                print!("{}", style(&formatted).cyan());
            } else {
                print!("{}", &formatted);
            }
        }

        // Render help text
        let help_row = menu_height - 2;
        stdout().execute(crossterm::cursor::MoveTo(start_col + 2, help_row))?;
        print!("{}", style("↑↓ Navigate  │  Enter Select  │  ESC Cancel").dim());

        stdout().flush()?;
        Ok(())
    }

    /// Handle keyboard input for provider selection
    fn handle_input(&mut self, app: &mut App) -> Result<bool> {
        while let Some(key_event) = MenuUtils::read_key_event()? {
            match key_event.code {
                KeyCode::Up => {
                    self.state.move_up(self.providers.len());
                }
                KeyCode::Down => {
                    self.state.move_down(self.providers.len());
                }
                KeyCode::Enter => {
                    if let Some(provider) = self.providers.get(self.state.selected_index).cloned() {
                        self.select_provider(app, &provider)?;
                        return Ok(true);
                    }
                }
                KeyCode::Esc => {
                    return Ok(true); // Cancel selection
                }
                _ => {}
            }
        }
        Ok(false)
    }

    /// Select and configure the provider
    fn select_provider(&mut self, app: &mut App, provider: &str) -> Result<()> {
        app.config.active_provider = provider.to_string();

        // Set default values based on provider
        match provider {
            "openai" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = Some("https://api.openai.com/v1".to_string());
                }
                app.config.set_model("gpt-3.5-turbo");
            }
            "anthropic" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = Some("https://api.anthropic.com".to_string());
                }
                app.config.set_model("claude-3-sonnet-20240229");
            }
            "ollama" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = Some("http://localhost:11434".to_string());
                }
                app.config.set_model("llama2");
            }
            "z.ai coding plan" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = Some("https://z.ai/api".to_string());
                }
                app.config.set_model("coding-plan");
            }
            "openrouter" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = Some("https://openrouter.ai/api/v1".to_string());
                }
                app.config.set_model("anthropic/claude-3-sonnet");
            }
            "custom" => {
                if let Some(config) = app.config.get_active_provider_config_mut() {
                    config.api_url = None;
                }
                app.config.set_model("");
            }
            _ => {}
        }

        Ok(())
    }

    /// Clear pending keyboard events
    fn clear_pending_events(&self) -> Result<()> {
        std::thread::sleep(std::time::Duration::from_millis(20));
        for _ in 0..3 {
            while crossterm::event::poll(std::time::Duration::from_millis(0))? {
                let _ = crossterm::event::read()?;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        Ok(())
    }

    /// Reset menu state
    pub fn reset(&mut self) {
        self.state.reset();
    }

    /// Get available providers
    pub fn get_providers(&self) -> &[String] {
        &self.providers
    }
}