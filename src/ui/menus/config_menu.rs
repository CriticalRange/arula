//! Configuration menu functionality for ARULA CLI

use crate::app::App;
use crate::config::ProviderField;
use crate::output::OutputHandler;
use crate::ui::menus::common::{MenuResult, MenuAction, MenuUtils, MenuState};
use crate::ui::menus::provider_menu::ProviderMenu;
use crate::ui::menus::dialogs::Dialogs;
use anyhow::Result;
use console::style;
use crossterm::{
    event::KeyCode,
    terminal,
    ExecutableCommand,
};
use std::io::{stdout, Write};

/// Configuration menu options
#[derive(Debug, Clone)]
pub enum ConfigMenuItem {
    AIProvider,
    AIModel,
    APIUrl,
    APIKey,
    Back,
}

impl ConfigMenuItem {
    pub fn all() -> Vec<Self> {
        vec![
            ConfigMenuItem::AIProvider,
            ConfigMenuItem::AIModel,
            ConfigMenuItem::APIUrl,
            ConfigMenuItem::APIKey,
            ConfigMenuItem::Back,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            ConfigMenuItem::AIProvider => "🤖 AI Provider",
            ConfigMenuItem::AIModel => "🧠 AI Model",
            ConfigMenuItem::APIUrl => "🌐 API URL",
            ConfigMenuItem::APIKey => "🔑 API Key",
            ConfigMenuItem::Back => "← Back to Menu",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ConfigMenuItem::AIProvider => "Select AI provider (OpenAI, Anthropic, etc)",
            ConfigMenuItem::AIModel => "Choose AI model to use",
            ConfigMenuItem::APIUrl => "Set custom API endpoint URL",
            ConfigMenuItem::APIKey => "Configure API authentication key",
            ConfigMenuItem::Back => "Return to main menu",
        }
    }
}

/// Configuration menu handler
pub struct ConfigMenu {
    state: MenuState,
    items: Vec<ConfigMenuItem>,
    provider_menu: ProviderMenu,
    dialogs: Dialogs,
}

impl ConfigMenu {
    pub fn new() -> Self {
        Self {
            state: MenuState::new(),
            items: ConfigMenuItem::all(),
            provider_menu: ProviderMenu::new(),
            dialogs: Dialogs::new(),
        }
    }

    /// Display and handle the configuration menu
    pub fn show(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
        // Check terminal size
        if !MenuUtils::check_terminal_size(30, 8)? {
            output.print_system("Terminal too small for config menu")?;
            return Ok(MenuResult::Continue);
        }

        // Setup terminal
        MenuUtils::setup_terminal()?;

        let result = self.run_menu_loop(app, output);

        // Restore terminal
        MenuUtils::restore_terminal()?;

        result
    }

    /// Configuration menu event loop
    fn run_menu_loop(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
        loop {
            // Render menu
            self.render(app, output)?;

            // Handle input
            match self.handle_input(app, output)? {
                MenuAction::Continue => continue,
                MenuAction::CloseMenu => return Ok(MenuResult::BackToMain),
                MenuAction::ExitApp => return Ok(MenuResult::Exit),
            }
        }
    }

    /// Render the configuration menu with original styling
    fn render(&self, app: &App, _output: &mut OutputHandler) -> Result<()> {
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
        let frame = MenuUtils::render_box("Settings", menu_width, menu_height);
        for (i, line) in frame.iter().enumerate() {
            if i < menu_height as usize {
                stdout().execute(crossterm::cursor::MoveTo(start_col, start_row + i as u16))?;
                print!("{}", line);
            }
        }

        // Render menu items
        let start_row = 2;
        for (idx, item) in self.items.iter().enumerate() {
            if idx >= menu_height as usize - 6 {
                break;
            }

            let row = start_row + idx as u16;
            stdout().execute(crossterm::cursor::MoveTo(start_col + 2, row))?;

            let is_selected = idx == self.state.selected_index;
            let formatted = MenuUtils::format_menu_item(item.label(), is_selected);

            if is_selected {
                print!("{}", style(&formatted).cyan());
            } else {
                print!("{}", &formatted);
            }

            // Show current value for configuration items
            if is_selected {
                let desc_row = row + 1;
                stdout().execute(crossterm::cursor::MoveTo(start_col + 4, desc_row))?;

                let (value, description) = self.get_item_value_and_description(item, app);
                print!("{}", style(&format!("↳ {}", description)).dim());

                if let Some(val) = value {
                    let val_row = desc_row + 1;
                    stdout().execute(crossterm::cursor::MoveTo(start_col + 6, val_row))?;
                    print!("{}", style(&format!("Current: {}", val)).yellow());
                }
            }
        }

        // Render help text
        let help_row = menu_height - 2;
        stdout().execute(crossterm::cursor::MoveTo(start_col + 2, help_row))?;
        print!("{}", style("↑↓ Navigate  │  Enter Select  │  ESC Back").dim());

        stdout().flush()?;
        Ok(())
    }

    /// Get current value and description for menu items
    fn get_item_value_and_description(&self, item: &ConfigMenuItem, app: &App) -> (Option<String>, String) {
        match item {
            ConfigMenuItem::AIProvider => {
                (Some(app.config.active_provider.clone()), item.description().to_string())
            }
            ConfigMenuItem::AIModel => {
                (Some(app.config.get_model()), item.description().to_string())
            }
            ConfigMenuItem::APIUrl => {
                let url = app.config.get_active_provider_config()
            .and_then(|c| c.api_url.clone())
            .unwrap_or_default();
                if url.is_empty() {
                    (None, item.description().to_string())
                } else {
                    (Some(MenuUtils::truncate_text(&url, 30)), item.description().to_string())
                }
            }
            ConfigMenuItem::APIKey => {
                let has_key = !app.config.get_api_key().is_empty();
                if has_key {
                    (Some("••••••••".to_string()), item.description().to_string())
                } else {
                    (Some("Not set".to_string()), item.description().to_string())
                }
            }
            ConfigMenuItem::Back => {
                (None, item.description().to_string())
            }
        }
    }

    /// Handle keyboard input
    fn handle_input(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuAction> {
        while let Some(key_event) = MenuUtils::read_key_event()? {
            match key_event.code {
                KeyCode::Up => {
                    self.state.move_up(self.items.len());
                }
                KeyCode::Down => {
                    self.state.move_down(self.items.len());
                }
                KeyCode::Enter => {
                    return self.handle_selection(app, output);
                }
                KeyCode::Esc => {
                    return Ok(MenuAction::CloseMenu);
                }
                _ => {}
            }
        }
        Ok(MenuAction::Continue)
    }

    /// Handle selection from configuration menu
    fn handle_selection(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuAction> {
        if let Some(selected_item) = self.items.get(self.state.selected_index) {
            match selected_item {
                ConfigMenuItem::AIProvider => {
                    self.provider_menu.show(app, output)?;
                    Ok(MenuAction::Continue)
                }
                ConfigMenuItem::AIModel => {
                    self.configure_model(app, output)?;
                    Ok(MenuAction::Continue)
                }
                ConfigMenuItem::APIUrl => {
                    self.configure_api_url(app, output)?;
                    Ok(MenuAction::Continue)
                }
                ConfigMenuItem::APIKey => {
                    self.configure_api_key(app, output)?;
                    Ok(MenuAction::Continue)
                }
                ConfigMenuItem::Back => {
                    Ok(MenuAction::CloseMenu)
                }
            }
        } else {
            Ok(MenuAction::Continue)
        }
    }

    /// Configure AI model
    fn configure_model(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let current_model = app.config.get_model();
        let prompt = format!("Enter AI model (current: {}):", current_model);

        if let Some(new_model) = self.dialogs.input_dialog(&prompt, Some(&current_model), output)? {
            if !new_model.trim().is_empty() {
                app.set_model(&new_model);
                output.print_system(&format!("Model updated to: {}", new_model))?;
            }
        }
        Ok(())
    }

    /// Configure API URL
    fn configure_api_url(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let current_url = app.config.get_active_provider_config()
            .and_then(|c| c.api_url.clone())
            .unwrap_or_default();
        let prompt = if current_url.is_empty() {
            "Enter API URL:".to_string()
        } else {
            format!("Enter API URL (current: {}):", current_url)
        };

        if let Some(new_url) = self.dialogs.input_dialog(&prompt, Some(&current_url), output)? {
            if !new_url.trim().is_empty() {
                if let Some(config) = app.config.get_active_provider_config_mut() {
            config.api_url = Some(new_url.to_string());
        }
                output.print_system(&format!("API URL updated to: {}", new_url))?;
            }
        }
        Ok(())
    }

    /// Configure API key
    fn configure_api_key(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let has_key = !app.config.get_api_key().is_empty();
        let prompt = if has_key {
            "Enter new API key (leave empty to keep current):"
        } else {
            "Enter API key:"
        };

        if let Some(new_key) = self.dialogs.password_dialog(prompt, output)? {
            if !new_key.trim().is_empty() {
                app.config.set_api_key(&new_key);
                output.print_system("API key updated")?;
            } else if !has_key {
                output.print_error("API key cannot be empty")?;
            }
        }
        Ok(())
    }

    /// Reset menu state
    pub fn reset(&mut self) {
        self.state.reset();
    }
}