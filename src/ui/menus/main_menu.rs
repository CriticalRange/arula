//! Main menu functionality for ARULA CLI

use crate::app::App;
use crate::output::OutputHandler;
use crate::ui::menus::common::{MenuResult, MenuAction, MenuUtils, MenuState};
use anyhow::Result;
use console::style;
use crossterm::{
    event::KeyCode,
    terminal,
    ExecutableCommand,
};
use std::io::{stdout, Write};
use std::time::Duration;

/// Main menu options
#[derive(Debug, Clone)]
pub enum MainMenuItem {
    ContinueChat,
    Settings,
    InfoHelp,
    ClearChat,
}

impl MainMenuItem {
    pub fn all() -> Vec<Self> {
        vec![
            MainMenuItem::ContinueChat,
            MainMenuItem::Settings,
            MainMenuItem::InfoHelp,
            MainMenuItem::ClearChat,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            MainMenuItem::ContinueChat => "⦿ Continue Chat",
            MainMenuItem::Settings => "⚙ Settings",
            MainMenuItem::InfoHelp => "ℹ Info & Help",
            MainMenuItem::ClearChat => "Ⓒ Clear Chat",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            MainMenuItem::ContinueChat => "Return to conversation",
            MainMenuItem::Settings => "Configure AI provider and settings",
            MainMenuItem::InfoHelp => "View help and session information",
            MainMenuItem::ClearChat => "Clear conversation history",
        }
    }
}

/// Main menu handler
pub struct MainMenu {
    state: MenuState,
    items: Vec<MainMenuItem>,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            state: MenuState::new(),
            items: MainMenuItem::all(),
        }
    }

    /// Display and handle the main menu
    pub fn show(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
        // Check terminal size
        if !MenuUtils::check_terminal_size(30, 8)? {
            output.print_system("Terminal too small for menu")?;
            return Ok(MenuResult::Continue);
        }

        // Setup terminal
        MenuUtils::setup_terminal()?;

        let result = self.run_menu_loop(app, output);

        // Restore terminal
        MenuUtils::restore_terminal()?;

        result
    }

    /// Main menu event loop
    fn run_menu_loop(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
        loop {
            // Render menu
            self.render(output)?;

            // Handle input with integrated selection logic
            match self.handle_input(app, output)? {
                MenuResult::Continue => return Ok(MenuResult::Continue),
                MenuResult::Settings => return Ok(MenuResult::Settings),
                MenuResult::ClearChat => return Ok(MenuResult::ClearChat),
                MenuResult::Exit => return Ok(MenuResult::Exit),
                _ => return Ok(MenuResult::Continue),
            }
        }
    }

    /// Render the main menu with original styling
    fn render(&self, _output: &mut OutputHandler) -> Result<()> {
        let (cols, rows) = crossterm::terminal::size()?;
        let menu_width = 40.min(cols);
        let menu_height = 10;

        // Clear entire screen before each render
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;

        // Center the menu
        let start_col = (cols - menu_width) / 2;
        let start_row = (rows - menu_height) / 2;

        // Render menu frame
        let frame = MenuUtils::render_box("ARULA Menu", menu_width, menu_height);
        for (i, line) in frame.iter().enumerate() {
            if i < menu_height as usize {
                stdout().execute(crossterm::cursor::MoveTo(start_col, start_row + i as u16))?;
                print!("{}", line);
            }
        }

        // Render menu items
        let start_row = 2;
        for (idx, item) in self.items.iter().enumerate() {
            if idx >= menu_height as usize - 4 {
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
        }

        // Render help text
        let help_row = menu_height - 2;
        stdout().execute(crossterm::cursor::MoveTo(start_col + 2, help_row))?;
        print!("{}", style("↑↓ Navigate  │  Enter Select  │  ESC Cancel").dim());

        stdout().flush()?;
        Ok(())
    }

    /// Handle keyboard input with selection logic
    fn handle_input(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
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
                    return Ok(MenuResult::Continue);
                }
                _ => {}
            }
        }
        Ok(MenuResult::Continue)
    }

    /// Handle selection from main menu
    pub fn handle_selection(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<MenuResult> {
        if let Some(selected_item) = self.items.get(self.state.selected_index) {
            match selected_item {
                MainMenuItem::ContinueChat => {
                    Ok(MenuResult::Continue)
                }
                MainMenuItem::Settings => {
                    Ok(MenuResult::BackToMain)
                }
                MainMenuItem::InfoHelp => {
                    self.show_info_and_help(app, output)?;
                    Ok(MenuResult::Continue)
                }
                MainMenuItem::ClearChat => {
                    Ok(MenuResult::ClearChat)
                }
            }
        } else {
            Ok(MenuResult::Continue)
        }
    }

    /// Show information and help dialog
    fn show_info_and_help(&self, app: &App, output: &mut OutputHandler) -> Result<()> {
        use crossterm::terminal::Clear;

        // Clear screen for info display
        stdout().execute(Clear(crossterm::terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;

        // Display information
        let provider_line = format!("Provider: {}", app.config.active_provider);
        let model_line = format!("Model: {}", app.config.get_model());
        let info_lines = vec![
            "ARULA CLI - Autonomous AI Assistant",
            "",
            "Version: 0.1.0",
            provider_line.as_str(),
            model_line.as_str(),
            "",
            "Commands:",
            "  menu, m        - Show this menu",
            "  clear, c       - Clear conversation",
            "  help, h        - Show help",
            "  exit, quit     - Exit application",
            "",
            "Press any key to continue...",
        ];

        let mut row = 2;
        for line in info_lines {
            stdout().execute(crossterm::cursor::MoveTo(2, row))?;

            if line.starts_with("ARULA") {
                print!("{}", style(line).green());
            } else if line.starts_with("  ") {
                print!("{}", style(line).cyan());
            } else {
                println!("{}", line);
            }

            row += 1;
        }

        stdout().flush()?;

        // Wait for any key
        while MenuUtils::read_key_event()?.is_none() {
            std::thread::sleep(Duration::from_millis(50));
        }

        Ok(())
    }

    /// Reset menu state
    pub fn reset(&mut self) {
        self.state.reset();
    }

    /// Get current selected index
    pub fn selected_index(&self) -> usize {
        self.state.selected_index
    }
}