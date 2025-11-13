use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, queue,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal,
};
use std::io::{stdout, Write};
use crate::app::App;
use crate::output::OutputHandler;

pub struct OverlayMenu {
    selected: usize,
}

impl OverlayMenu {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn show_main_menu(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        // Clear screen before showing menu
        execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

        // Enable raw mode but DON'T use alternate screen
        terminal::enable_raw_mode()?;

        let result = self.run_main_menu(app, output);

        // Disable raw mode
        terminal::disable_raw_mode()?;

        // Clear screen after closing menu
        execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

        result
    }

    pub fn show_exit_confirmation(&mut self, output: &mut OutputHandler) -> Result<bool> {
        // Clear screen before showing confirmation
        execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

        // Enable raw mode
        terminal::enable_raw_mode()?;

        let result = self.show_confirm_dialog("Exit ARULA? (Press Ctrl+C again to confirm)")?;

        // Disable raw mode
        terminal::disable_raw_mode()?;

        // Clear screen after confirmation
        execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

        if result {
            output.print_system("Goodbye! 👋")?;
        }

        Ok(result)
    }

    fn clear_overlay(&self) -> Result<()> {
        // Move cursor down and print newlines to push overlay off screen
        execute!(stdout(), cursor::MoveToNextLine(1))?;
        println!(); // Push content down
        Ok(())
    }

    fn run_main_menu(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        let options = vec![
            "💬 Continue Chat",
            "🔧 Configuration",
            "📊 Session Info",
            "🗑️  Clear Chat",
            "❓ Help",
            "🚪 Exit",
        ];

        self.selected = 0;

        loop {
            self.draw_menu("ARULA Main Menu", &options)?;

            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Up => {
                        if self.selected > 0 {
                            self.selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if self.selected < options.len() - 1 {
                            self.selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Handle selection
                        match self.selected {
                            0 => return Ok(false), // Continue chat
                            1 => {
                                // Configuration
                                terminal::disable_raw_mode()?;
                                self.show_configuration_menu(app, output)?;
                                terminal::enable_raw_mode()?;
                            }
                            2 => {
                                // Session info
                                self.show_session_info(app)?;
                            }
                            3 => {
                                // Clear chat
                                if self.show_confirm_dialog("Clear chat history?")? {
                                    app.clear_conversation();
                                    terminal::disable_raw_mode()?;
                                    output.print_system("✅ Chat history cleared")?;
                                    return Ok(false);
                                }
                            }
                            4 => {
                                // Help
                                self.show_help()?;
                            }
                            5 => {
                                // Exit
                                if self.show_confirm_dialog("Exit ARULA?")? {
                                    terminal::disable_raw_mode()?;
                                    output.print_system("Goodbye! 👋")?;
                                    return Ok(true);
                                }
                            }
                            _ => {}
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        return Ok(false); // Close menu
                    }
                    _ => {}
                }
            }
        }
    }

    fn show_configuration_menu(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        use dialoguer::{theme::ColorfulTheme, Select, Input};

        loop {
            let config = app.get_config();

            let options = vec![
                format!("Provider: {}", config.ai.provider),
                format!("Model: {}", config.ai.model),
                format!("API URL: {}", config.ai.api_url),
                format!("API Key: {}", if config.ai.api_key.is_empty() { "Not set" } else { "********" }),
                "← Back to Main Menu".to_string(),
            ];

            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Configuration")
                .items(&options)
                .default(0)
                .interact()?;

            match selection {
                0 => {
                    // Edit provider
                    let providers = vec!["openai", "claude", "anthropic", "ollama", "custom"];
                    let current_idx = providers.iter().position(|&p| p == config.ai.provider).unwrap_or(0);

                    let provider_idx = Select::with_theme(&ColorfulTheme::default())
                        .with_prompt("Select AI Provider")
                        .items(&providers)
                        .default(current_idx)
                        .interact()?;

                    app.config.ai.provider = providers[provider_idx].to_string();
                    let _ = app.config.save();
                    let _ = app.initialize_api_client();
                    output.print_system(&format!("✅ Provider set to: {}", providers[provider_idx]))?;
                }
                1 => {
                    // Edit model
                    let model: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter model name")
                        .default(config.ai.model.clone())
                        .interact_text()?;

                    app.set_model(&model);
                    output.print_system(&format!("✅ Model set to: {}", model))?;
                }
                2 => {
                    // Edit API URL
                    let url: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter API URL")
                        .default(config.ai.api_url.clone())
                        .interact_text()?;

                    app.config.ai.api_url = url.clone();
                    let _ = app.config.save();
                    let _ = app.initialize_api_client();
                    output.print_system(&format!("✅ API URL set to: {}", url))?;
                }
                3 => {
                    // Edit API Key
                    let key: String = Input::with_theme(&ColorfulTheme::default())
                        .with_prompt("Enter API Key (or leave empty to use environment variable)")
                        .allow_empty(true)
                        .interact_text()?;

                    if !key.is_empty() {
                        app.config.ai.api_key = key;
                        let _ = app.config.save();
                        let _ = app.initialize_api_client();
                        output.print_system("✅ API Key updated")?;
                    }
                }
                4 => {
                    // Back
                    break;
                }
                _ => break,
            }
        }

        Ok(())
    }

    fn show_session_info(&mut self, app: &App) -> Result<()> {
        let (width, height) = terminal::size()?;
        let config = app.get_config();

        let lines = vec![
            "📊 Session Information".to_string(),
            "━".repeat(40),
            format!("Provider: {}", config.ai.provider),
            format!("Model: {}", config.ai.model),
            format!("API URL: {}", config.ai.api_url),
            format!("Messages in conversation: {}", app.messages.len()),
            "".to_string(),
            "Press any key to continue...".to_string(),
        ];

        let box_width = 50;
        let box_height = lines.len() as u16 + 4;
        let start_x = (width.saturating_sub(box_width)) / 2;
        let start_y = (height.saturating_sub(box_height)) / 2;

        // Don't clear - just draw over
        self.draw_box(start_x, start_y, box_width, box_height, "Session Info")?;

        for (i, line) in lines.iter().enumerate() {
            queue!(
                stdout(),
                cursor::MoveTo(start_x + 2, start_y + 2 + i as u16),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::White),
                Print(line),
                ResetColor
            )?;
        }

        stdout().flush()?;
        event::read()?;

        Ok(())
    }

    fn show_help(&mut self) -> Result<()> {
        let (width, height) = terminal::size()?;

        let lines = vec![
            "❓ ARULA Help".to_string(),
            "━".repeat(50),
            "".to_string(),
            "🔧 Commands:".to_string(),
            "  /help              - Show this help".to_string(),
            "  /menu              - Open interactive menu".to_string(),
            "  /clear             - Clear conversation history".to_string(),
            "  /config            - Show current configuration".to_string(),
            "  /model <name>      - Change AI model".to_string(),
            "  exit or quit       - Exit ARULA".to_string(),
            "".to_string(),
            "⌨️  Keyboard Shortcuts:".to_string(),
            "  Ctrl+C             - Open menu".to_string(),
            "  Ctrl+D             - Exit".to_string(),
            "  Up/Down Arrow      - Navigate command history".to_string(),
            "  Esc                - Close menu".to_string(),
            "".to_string(),
            "💡 Tips:".to_string(),
            "  - Ask ARULA to execute bash commands".to_string(),
            "  - Use natural language".to_string(),
            "  - Native terminal scrollback works!".to_string(),
            "".to_string(),
            "Press any key to continue...".to_string(),
        ];

        let box_width = 60;
        let box_height = lines.len() as u16 + 4;
        let start_x = (width.saturating_sub(box_width)) / 2;
        let start_y = (height.saturating_sub(box_height)) / 2;

        // Don't clear - just draw over
        self.draw_box(start_x, start_y, box_width, box_height, "Help")?;

        for (i, line) in lines.iter().enumerate() {
            queue!(
                stdout(),
                cursor::MoveTo(start_x + 2, start_y + 2 + i as u16),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::White),
                Print(line),
                ResetColor
            )?;
        }

        stdout().flush()?;
        event::read()?;

        Ok(())
    }

    fn show_confirm_dialog(&mut self, message: &str) -> Result<bool> {
        let (width, height) = terminal::size()?;
        let box_width = 50;
        let box_height = 7;
        let start_x = (width.saturating_sub(box_width)) / 2;
        let start_y = (height.saturating_sub(box_height)) / 2;

        let mut selected = 0; // 0 = No, 1 = Yes

        loop {
            // Don't clear - just redraw the box
            self.draw_box(start_x, start_y, box_width, box_height, "Confirm")?;

            // Draw message
            queue!(
                stdout(),
                cursor::MoveTo(start_x + 2, start_y + 2),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Yellow),
                Print(message),
                ResetColor
            )?;

            // Draw buttons
            let no_style = if selected == 0 {
                SetBackgroundColor(Color::Red)
            } else {
                SetBackgroundColor(Color::DarkGrey)
            };

            let yes_style = if selected == 1 {
                SetBackgroundColor(Color::Green)
            } else {
                SetBackgroundColor(Color::DarkGrey)
            };

            queue!(
                stdout(),
                cursor::MoveTo(start_x + 10, start_y + 4),
                no_style,
                SetForegroundColor(Color::White),
                Print(" No "),
                ResetColor
            )?;

            queue!(
                stdout(),
                cursor::MoveTo(start_x + 30, start_y + 4),
                yes_style,
                SetForegroundColor(Color::White),
                Print(" Yes "),
                ResetColor
            )?;

            stdout().flush()?;

            if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
                match code {
                    KeyCode::Left => selected = 0,
                    KeyCode::Right => selected = 1,
                    KeyCode::Enter => return Ok(selected == 1),
                    KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => return Ok(false),
                    KeyCode::Char('y') | KeyCode::Char('Y') => return Ok(true),
                    KeyCode::Char('c') if modifiers.contains(event::KeyModifiers::CONTROL) => {
                        // Ctrl+C pressed again - confirm exit
                        return Ok(true);
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_menu(&self, title: &str, options: &[&str]) -> Result<()> {
        let (width, height) = terminal::size()?;

        let box_width = 40;
        let box_height = options.len() as u16 + 4;
        let start_x = (width.saturating_sub(box_width)) / 2;
        let start_y = (height.saturating_sub(box_height)) / 2;

        // Don't clear screen - just draw over it
        self.draw_box(start_x, start_y, box_width, box_height, title)?;

        for (i, option) in options.iter().enumerate() {
            let is_selected = i == self.selected;

            if is_selected {
                queue!(
                    stdout(),
                    cursor::MoveTo(start_x + 2, start_y + 2 + i as u16),
                    SetBackgroundColor(Color::Cyan),
                    SetForegroundColor(Color::Black),
                    Print(format!(" ❯ {:<34}", option)),
                    ResetColor
                )?;
            } else {
                queue!(
                    stdout(),
                    cursor::MoveTo(start_x + 2, start_y + 2 + i as u16),
                    SetForegroundColor(Color::White),
                    Print(format!("   {:<34}", option)),
                    ResetColor
                )?;
            }
        }

        stdout().flush()?;
        Ok(())
    }

    fn draw_box(&self, x: u16, y: u16, width: u16, height: u16, title: &str) -> Result<()> {
        // Draw top border with title
        queue!(
            stdout(),
            cursor::MoveTo(x, y),
            SetBackgroundColor(Color::Black),
            SetForegroundColor(Color::Cyan),
            Print("╔"),
            Print(format!(" {} ", title)),
            Print("═".repeat((width.saturating_sub(title.len() as u16 + 4)) as usize)),
            Print("╗"),
            ResetColor
        )?;

        // Draw sides and fill interior with black background
        for i in 1..height - 1 {
            queue!(
                stdout(),
                cursor::MoveTo(x, y + i),
                SetBackgroundColor(Color::Black),
                SetForegroundColor(Color::Cyan),
                Print("║"),
                SetForegroundColor(Color::Reset),
                Print(" ".repeat((width - 2) as usize)), // Fill interior with spaces
                SetForegroundColor(Color::Cyan),
                Print("║"),
                ResetColor
            )?;
        }

        // Draw bottom border
        queue!(
            stdout(),
            cursor::MoveTo(x, y + height - 1),
            SetBackgroundColor(Color::Black),
            SetForegroundColor(Color::Cyan),
            Print("╚"),
            Print("═".repeat((width - 2) as usize)),
            Print("╝"),
            ResetColor
        )?;

        stdout().flush()?;
        Ok(())
    }
}

impl Default for OverlayMenu {
    fn default() -> Self {
        Self::new()
    }
}
