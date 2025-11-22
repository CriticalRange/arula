use crate::app::App;
use crate::colors::{ColorTheme, helpers};
use crate::config::ProviderField;
use crate::output::OutputHandler;
use anyhow::Result;
use std::io::{stdout, Write};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, KeyEventKind},
    terminal::{self, size, EnterAlternateScreen, LeaveAlternateScreen},
    cursor::{MoveTo, Show, Hide, SetCursorStyle},
    style::{Color, Print, SetForegroundColor, SetBackgroundColor, ResetColor},
    ExecutableCommand, QueueableCommand,
};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuResult {
    Continue,
    Exit,
    ClearChat,
    BackToMain,
    ConfigurationUpdated,
}

#[derive(Debug, PartialEq)]
enum MenuAction {
    Continue,     // Stay in menu
    CloseMenu,    // Exit menu, continue app
    ExitApp,      // Exit menu AND exit app
}

pub struct OverlayMenu {
    selected_index: usize,
    main_options: Vec<String>,
    config_options: Vec<String>,
    is_in_config: bool,
}

impl OverlayMenu {
    /// Truncate text to fit within max_width, adding "..." if truncated
    fn truncate_text(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else {
            format!("{}...", &text[..max_width.saturating_sub(3)])
        }
    }

    pub fn new() -> Self {
        Self {
            selected_index: 0,
            main_options: vec![
                "💬 Continue Chat".to_string(),
                "⚙️  Settings".to_string(),
                "ℹ️  Info & Help".to_string(),
                "🧹 Clear Chat".to_string(),
                "🚪 Exit ARULA".to_string(),
            ],
            config_options: vec![
                "🤖 AI Provider".to_string(),
                "🧠 AI Model".to_string(),
                "🌐 API URL".to_string(),
                "🔑 API Key".to_string(),
                "← Back to Menu".to_string(),
            ],
            is_in_config: false,
        }
    }

    pub fn show_main_menu(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        self.show_menu(app, output, false)
    }

    pub fn show_config_menu(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        self.show_menu(app, output, true)
    }

    pub fn show_exit_confirmation(&mut self, _output: &mut OutputHandler) -> Result<bool> {
        let (_original_cols, _original_rows) = size()?;

        // Enter alternate screen and hide cursor (raw mode is already handled by main app)
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(Hide)?;

        // Clear screen ONCE on entry to alternate screen for clean start
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().flush()?;

        // Show confirmation dialog directly (no animation)
        let result = self.show_confirm_dialog("Exit ARULA?")?;

        // Cleanup and restore terminal (with proper cursor restoration)
        self.cleanup_terminal()?;

        Ok(result)
    }

    fn render_exit_confirmation(&self, message: &str) -> Result<()> {
        let (cols, rows) = size()?;

        let menu_width = 40.min(cols.saturating_sub(4));
        let menu_height = 6u16;
        let start_x = if cols > menu_width { cols.saturating_sub(menu_width) / 2 } else { 0 };
        let start_y = if rows > menu_height { rows.saturating_sub(menu_height) / 2 } else { 0 };

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "Confirm")?;

        // Message with styling
        stdout().queue(MoveTo(start_x + 2, start_y + 2))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
              .queue(Print(message))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn show_menu(&mut self, app: &mut App, output: &mut OutputHandler, start_in_config: bool) -> Result<bool> {
        self.is_in_config = start_in_config;
        self.selected_index = 0;

        // Save terminal state and cursor style
        let (_original_cols, _original_rows) = size()?;

        // Enter alternate screen and hide cursor (raw mode is already handled by main app)
        stdout().execute(EnterAlternateScreen)?;
        stdout().execute(Hide)?;

        // Clear screen ONCE on entry to alternate screen for clean start
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().flush()?;

        // Main event loop (no animation)
        let result = self.run_menu_loop(app, output)?;

        // Cleanup and restore terminal
        self.cleanup_terminal()?;

        Ok(result)
    }

    fn run_menu_loop(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        let mut should_exit_app = false;
        let mut last_menu_state = self.is_in_config; // Track menu state changes
        let mut last_selected_index = self.selected_index; // Track selection changes
        let mut needs_render = true; // Track if we need to render

        // Comprehensive event clearing to prevent submenu issues
        std::thread::sleep(Duration::from_millis(50));
        for _ in 0..5 { // Multiple passes to ensure all events are cleared
            while event::poll(Duration::from_millis(0))? {
                let _ = event::read()?;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        loop {
            // Clear screen if we switched between menus to avoid artifacts
            if last_menu_state != self.is_in_config {
                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                last_menu_state = self.is_in_config;
                needs_render = true;
            }

            // Only render if state changed
            if needs_render || last_selected_index != self.selected_index {
                self.render_frame(app, output)?;
                last_selected_index = self.selected_index;
                needs_render = false;
            }

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        // Ignore any unexpected key events that might be spurious
                        match key_event.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                // If in a submenu, go back to main menu. Otherwise, exit menu.
                                if self.is_in_config {
                                    self.is_in_config = false;
                                    self.selected_index = 0;
                                    // Clear any pending events when returning to main menu to prevent immediate issues
                                    while event::poll(Duration::from_millis(0))? {
                                        let _ = event::read()?;
                                    }
                                } else {
                                    break; // Exit menu, continue app
                                }
                            }
                            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                                break; // Exit menu, continue app
                            }
                            // Only process navigation and selection keys
                            KeyCode::Up | KeyCode::Down | KeyCode::Left | KeyCode::Right |
                            KeyCode::Enter | KeyCode::Char('j') | KeyCode::Char('k') |
                            KeyCode::Char('h') | KeyCode::Char('l') | KeyCode::Tab |
                            KeyCode::Backspace | KeyCode::Delete => {
                                // Valid menu keys - process them
                                let result = self.handle_key_event(key_event, app, output, &mut needs_render)?;
                                match result {
                                    MenuAction::ExitApp => {
                                        should_exit_app = true;
                                        break;    // Exit menu AND exit app
                                    }
                                    MenuAction::CloseMenu => break,  // Exit menu, continue app
                                    MenuAction::Continue => {},      // Stay in menu
                                }
                            }
                            _ => {
                                // Ignore any other key events that might be spurious
                                continue;
                            }
                        }
                    }
                    Event::Resize(_, _) => {
                        // Mark for redraw on resize
                        needs_render = true;
                    }
                    // Ignore all other event types (mouse, focus, etc.) that might cause issues on Windows
                    _ => {
                        continue;
                    }
                }
            }
        }

        Ok(should_exit_app)
    }

    fn handle_key_event(&mut self, key_event: KeyEvent, app: &mut App, output: &mut OutputHandler, needs_render: &mut bool) -> Result<MenuAction> {
        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.move_selection(-1, app);
                Ok(MenuAction::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.move_selection(1, app);
                Ok(MenuAction::Continue)
            }
            KeyCode::Enter => {
                if self.is_in_config {
                    let exit = self.handle_config_selection(app, output)?;
                    // Force re-render after returning from submenu
                    *needs_render = true;
                    if exit {
                        Ok(MenuAction::ExitApp)
                    } else {
                        Ok(MenuAction::Continue)
                    }
                } else {
                    // Check if this is entering the Settings submenu before processing
                    let was_settings_entry = self.selected_index == 1;
                    let should_exit = self.handle_main_selection(app, output)?;

                    if should_exit {
                        Ok(MenuAction::ExitApp)
                    } else if was_settings_entry {
                        // Entering Settings submenu - continue instead of closing
                        // Force re-render when entering settings
                        *needs_render = true;
                        Ok(MenuAction::Continue)
                    } else {
                        // Other selections (like Continue Chat) - close menu normally
                        Ok(MenuAction::CloseMenu)
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h') if self.is_in_config => {
                self.is_in_config = false;
                self.selected_index = 0;
                // More aggressive event clearing when returning to main menu
                std::thread::sleep(Duration::from_millis(20));
                for _ in 0..3 {
                    while event::poll(Duration::from_millis(0))? {
                        let _ = event::read()?;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(MenuAction::Continue)
            }
            _ => Ok(MenuAction::Continue),
        }
    }

    fn handle_main_selection(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        match self.selected_index {
            0 => Ok(false), // Continue chat
            1 => { // Configuration
                self.is_in_config = true;
                self.selected_index = 0;
                // More aggressive event clearing when switching to submenu
                std::thread::sleep(Duration::from_millis(20)); // Small delay
                for _ in 0..3 { // Multiple passes to clear all pending events
                    while event::poll(Duration::from_millis(0))? {
                        let _ = event::read()?;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(false)
            }
            2 => { // Info & Help
                self.show_info_and_help(app)?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            3 => { // Clear chat
                if self.show_confirm_dialog("Clear chat history?")? {
                    app.clear_conversation();
                    output.print_system("✅ Chat history cleared")?;
                }
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            4 => { // Exit
                if self.show_confirm_dialog("Exit ARULA?")? {
                    Ok(true) // Signal to exit application
                } else {
                    Ok(false) // Continue with application
                }
            }
            _ => Ok(false),
        }
    }

    fn handle_config_selection(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        match self.selected_index {
            0 => { // Provider
                self.show_provider_selector(app, output)?;
                // Clear screen to prepare for menu re-render
                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            1 => { // Model
                self.show_model_selector(app, output)?;
                // Clear screen to prepare for menu re-render
                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            2 => { // API URL
                if app.config.is_field_editable(ProviderField::ApiUrl) {
                    if let Some(url) = self.show_text_input("Enter API URL", &app.get_config().get_api_url())? {
                        app.config.set_api_url(&url);
                        let _ = app.config.save();
                        match app.initialize_agent_client() {
                            Ok(()) => {
                                output.print_system(&format!("✅ API URL set to: {} (AI client initialized)", url))?;
                            }
                            Err(_) => {
                                output.print_system(&format!("✅ API URL set to: {} (AI client will initialize when configuration is complete)", url))?;
                            }
                        }
                    }
                    // Clear screen to prepare for menu re-render
                    stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                    // Clear any pending events that might have been generated during the dialog
                    while event::poll(Duration::from_millis(0))? {
                        let _ = event::read()?;
                    }
                }
                // If not editable, do nothing (field is already shown in gray)
                Ok(false)
            }
            3 => { // API Key
                if let Some(key) = self.show_text_input("Enter API Key (or leave empty to use environment variable)", "")? {
                    if !key.is_empty() {
                        app.config.set_api_key(&key);
                        let _ = app.config.save();
                        match app.initialize_agent_client() {
                            Ok(()) => {
                                output.print_system("✅ API Key updated (AI client initialized)")?;
                            }
                            Err(_) => {
                                output.print_system("✅ API Key updated (AI client will initialize when other settings are complete)")?;
                            }
                        }
                    }
                }
                // Clear screen to prepare for menu re-render
                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            4 | _ => { // Back
                self.is_in_config = false;
                self.selected_index = 0;
                // More aggressive event clearing when returning to main menu
                std::thread::sleep(Duration::from_millis(20));
                for _ in 0..3 {
                    while event::poll(Duration::from_millis(0))? {
                        let _ = event::read()?;
                    }
                    std::thread::sleep(Duration::from_millis(5));
                }
                Ok(false)
            }
        }
    }

    fn show_provider_selector(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let providers = vec!["openai", "anthropic", "ollama", "z.ai coding plan", "openrouter", "custom"];
        let current_config = app.get_config();
        let current_idx = providers
            .iter()
            .position(|&p| p == current_config.active_provider)
            .unwrap_or(0);

        // Clear screen once when entering submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // Comprehensive event clearing before provider selector
        std::thread::sleep(Duration::from_millis(20));
        for _ in 0..3 {
            while event::poll(Duration::from_millis(0))? {
                let _ = event::read()?;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        // Create a temporary selection for provider
        let mut selected_idx = current_idx;
        loop {
            self.render_provider_selector(&providers, selected_idx)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        // Only handle valid navigation keys
                        match key_event.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if selected_idx > 0 {
                                    selected_idx -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if selected_idx < providers.len() - 1 {
                                    selected_idx += 1;
                                }
                            }
                            KeyCode::Enter => {
                                let new_provider = providers[selected_idx].to_string();

                                // Switch to the new provider
                                let _ = app.config.switch_provider(&new_provider);

                                // Show what changed
                                output.print_system(&format!(
                                    "🔄 Model automatically set to: {}",
                                    app.config.get_model()
                                ))?;
                                output.print_system(&format!(
                                    "🌐 API URL automatically set to: {}",
                                    app.config.get_api_url()
                                ))?;

                                let _ = app.config.save();
                                match app.initialize_agent_client() {
                                    Ok(()) => {
                                        output.print_system(&format!(
                                            "✅ Provider set to: {} (AI client initialized)",
                                            providers[selected_idx]
                                        ))?;
                                    }
                                    Err(_) => {
                                        output.print_system(&format!(
                                            "✅ Provider set to: {} (AI client will initialize when configuration is complete)",
                                            providers[selected_idx]
                                        ))?;
                                    }
                                }
                                break;
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                break;
                            }
                            _ => {
                                // Ignore all other keys
                                continue;
                            }
                        }
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }

        // Clear screen once when exiting submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        Ok(())
    }

    fn render_provider_selector(&self, providers: &[&str], selected_idx: usize) -> Result<()> {
        let (cols, rows) = size()?;

        // Don't clear entire screen - causes flicker
        // We're in alternate screen mode, so just draw over existing content

        let menu_width = 50.min(cols.saturating_sub(4));
        let menu_height = providers.len() + 6; // Added space for header and footer
        let menu_height_u16 = menu_height as u16;

        // Ensure menu fits in terminal
        let menu_width = menu_width.min(cols.saturating_sub(4));
        let menu_height = if menu_height_u16 > rows.saturating_sub(4) {
            rows.saturating_sub(4) as usize
        } else {
            menu_height
        };

        let start_x = if cols > menu_width { cols.saturating_sub(menu_width) / 2 } else { 0 };
        let start_y = if rows > menu_height as u16 { rows.saturating_sub(menu_height as u16) / 2 } else { 0 };

        self.draw_modern_box(start_x, start_y, menu_width, menu_height as u16, "AI PROVIDER")?;

        // Draw title/header
        let title_y = start_y + 1;
        let title = "Select AI Provider";
        let title_x = start_x + (menu_width - title.len() as u16) / 2;
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Draw provider options
        for (i, provider) in providers.iter().enumerate() {
            let y = start_y + 3 + i as u16;
            let prefix = if i == selected_idx { "▶ " } else { "  " };
            let text = format!("{}{}", prefix, provider);

            // Use atomic padding for consistent rendering
            let padded_text = format!("{:width$}", text, width = (menu_width - 4) as usize);

            let color = if i == selected_idx {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI))
            } else {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
            };

            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(color)?
                  .queue(Print(padded_text))?
                  .queue(ResetColor)?;
        }

        // Draw footer with navigation instructions (centered, intercepting box border)
        let footer_y = start_y + menu_height as u16 - 1;
        let nav_text = "↑↓ Navigate • ↵ Select • ← Back";
        let nav_x = start_x + (menu_width - nav_text.len() as u16) / 2;

        stdout().queue(MoveTo(nav_x, footer_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn get_default_model_for_provider(&self, provider: &str) -> String {
        match provider.to_lowercase().as_str() {
            "z.ai coding plan" | "z.ai" | "zai" => "glm-4.6".to_string(),
            "openai" => "gpt-3.5-turbo".to_string(),
            "claude" | "anthropic" => "claude-3-sonnet-20240229".to_string(),
            "ollama" => "llama2".to_string(),
            "openrouter" => "openai/gpt-4o".to_string(),
            _ => "default".to_string(),
        }
    }

    /// Helper function to write debug logs to file
    fn debug_log(&self, message: &str) {
        let _ = std::fs::write("./arula_debug.log", format!("[{}] {}\n", chrono::Utc::now().format("%H:%M:%S.%3f"), message));
    }

    /// Helper function to append debug logs to file
    fn debug_log_append(&self, message: &str) {
        use std::io::Write;
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("./arula_debug.log")
            .and_then(|mut file| {
                write!(file, "[{}] {}\n", chrono::Utc::now().format("%H:%M:%S.%3f"), message)
            });
    }

    /// Get OpenRouter models with dynamic fetching and caching
    fn get_openrouter_models(&self, app: &mut App, output: &mut OutputHandler) -> (Vec<String>, bool) {
        self.debug_log("get_openrouter_models called");

        // First, try to get cached models
        match app.get_cached_openrouter_models() {
            Some(cached_models) => {
                self.debug_log_append(&format!("Cache found with {} models", cached_models.len()));
                if !cached_models.is_empty() {
                    self.debug_log_append(&format!("Cache has {} non-empty models, returning them", cached_models.len()));
                    let _ = output.print_system(&format!("✅ Using {} cached models", cached_models.len()));
                    return (cached_models, false); // (models, is_loading)
                } else {
                    self.debug_log_append("Cache is empty, will start fetching");
                }
            }
            None => {
                self.debug_log_append("No cache found, will start fetching");
            }
        }

        // Start background fetching if no cached models available
        self.debug_log_append("Starting background fetch");
        // Fetch models silently in background
        app.fetch_openrouter_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }
    /// Get OpenAI models with dynamic fetching and caching
    fn get_openai_models(&self, app: &mut App, output: &mut OutputHandler) -> (Vec<String>, bool) {
        self.debug_log("get_openai_models called");

        // First, try to get cached models
        match app.get_cached_openai_models() {
            Some(cached_models) => {
                self.debug_log_append(&format!("Cache found with {} models", cached_models.len()));
                if !cached_models.is_empty() {
                    self.debug_log_append(&format!("Cache has {} non-empty models, returning them", cached_models.len()));
                    let _ = output.print_system(&format!("✅ Using {} cached models", cached_models.len()));
                    return (cached_models, false); // (models, is_loading)
                } else {
                    self.debug_log_append("Cache is empty, will start fetching");
                }
            }
            None => {
                self.debug_log_append("No cache found, will start fetching");
            }
        }

        // Start background fetching if no cached models available
        self.debug_log_append("Starting background fetch");
        // Fetch models silently in background
        app.fetch_openai_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }

    /// Get Anthropic models with dynamic fetching and caching
    fn get_anthropic_models(&self, app: &mut App, output: &mut OutputHandler) -> (Vec<String>, bool) {
        self.debug_log("get_anthropic_models called");

        // First, try to get cached models
        match app.get_cached_anthropic_models() {
            Some(cached_models) => {
                self.debug_log_append(&format!("Cache found with {} models", cached_models.len()));
                if !cached_models.is_empty() {
                    self.debug_log_append(&format!("Cache has {} non-empty models, returning them", cached_models.len()));
                    let _ = output.print_system(&format!("✅ Using {} cached models", cached_models.len()));
                    return (cached_models, false); // (models, is_loading)
                } else {
                    self.debug_log_append("Cache is empty, will start fetching");
                }
            }
            None => {
                self.debug_log_append("No cache found, will start fetching");
            }
        }

        // Start background fetching if no cached models available
        self.debug_log_append("Starting background fetch");
        // Fetch models silently in background
        app.fetch_anthropic_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }

    /// Get Ollama models with dynamic fetching and caching
    fn get_ollama_models(&self, app: &mut App, output: &mut OutputHandler) -> (Vec<String>, bool) {
        self.debug_log("get_ollama_models called");

        // First, try to get cached models
        match app.get_cached_ollama_models() {
            Some(cached_models) => {
                self.debug_log_append(&format!("Cache found with {} models", cached_models.len()));
                if !cached_models.is_empty() {
                    self.debug_log_append(&format!("Cache has {} non-empty models, returning them", cached_models.len()));
                    let _ = output.print_system(&format!("✅ Using {} cached models", cached_models.len()));
                    return (cached_models, false); // (models, is_loading)
                } else {
                    self.debug_log_append("Cache is empty, will start fetching");
                }
            }
            None => {
                self.debug_log_append("No cache found, will start fetching");
            }
        }

        // Start background fetching if no cached models available
        self.debug_log_append("Starting background fetch");
        // Fetch models silently in background
        app.fetch_ollama_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }

    /// Get Z.AI models with dynamic fetching and caching
    fn get_zai_models(&self, app: &mut App, output: &mut OutputHandler) -> (Vec<String>, bool) {
        self.debug_log("get_zai_models called");

        // First, try to get cached models
        match app.get_cached_zai_models() {
            Some(cached_models) => {
                self.debug_log_append(&format!("Cache found with {} models", cached_models.len()));
                if !cached_models.is_empty() {
                    self.debug_log_append(&format!("Cache has {} non-empty models, returning them", cached_models.len()));
                    let _ = output.print_system(&format!("✅ Using {} cached models", cached_models.len()));
                    return (cached_models, false); // (models, is_loading)
                } else {
                    self.debug_log_append("Cache is empty, will start fetching");
                }
            }
            None => {
                self.debug_log_append("No cache found, will start fetching");
            }
        }

        // Start background fetching if no cached models available
        self.debug_log_append("Starting background fetch");
        // Fetch models silently in background
        app.fetch_zai_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }



    fn show_model_selector(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let current_config = app.get_config();
        let provider = current_config.active_provider.clone();
        let current_model = current_config.get_model();

        // Clear screen once when entering submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // For custom provider, use text input instead of selector
        if provider.to_lowercase() == "custom" {
            if let Some(model) = self.show_text_input("Enter model name", &current_model)? {
                app.set_model(&model);
                output.print_system(&format!("✅ Model set to: {}", model))?;
            }
            return Ok(());
        }

        // For predefined providers, use dynamic fetching with caching
        let (models, is_loading): (Vec<String>, bool) = match provider.to_lowercase().as_str() {
            "z.ai coding plan" | "z.ai" | "zai" => {
                // Clear cache to simulate first-run behavior
                app.cache_zai_models(Vec::new());
                let (models, loading) = self.get_zai_models(app, output);
                (models, loading)
            }
            "openai" => {
                // Clear cache to simulate first-run behavior
                app.cache_openai_models(Vec::new());
                let (models, loading) = self.get_openai_models(app, output);
                (models, loading)
            }
            "anthropic" => {
                // Clear cache to simulate first-run behavior
                app.cache_anthropic_models(Vec::new());
                let (models, loading) = self.get_anthropic_models(app, output);
                (models, loading)
            }
            "ollama" => {
                // Clear cache to simulate first-run behavior
                app.cache_ollama_models(Vec::new());
                let (models, loading) = self.get_ollama_models(app, output);
                (models, loading)
            }
            "openrouter" => {
                // For OpenRouter, fetch models dynamically with caching
                self.debug_log_append("OpenRouter provider selected, calling get_openrouter_models");

                // Force cache clear to simulate first-run behavior every time
                self.debug_log_append("Clearing cache to simulate first-run behavior");
                app.cache_openrouter_models(Vec::new());

                let (models, is_loading) = self.get_openrouter_models(app, output);
                self.debug_log_append(&format!("get_openrouter_models returned {} models, is_loading={}", models.len(), is_loading));

                // Always return tuple with loading state
                if is_loading {
                    self.debug_log_append(&format!("Starting loading state with {} models", models.len()));
                    (models, is_loading)
                } else {
                    // Models loaded very quickly, but we still want to show transition
                    self.debug_log_append(&format!("Models loaded quickly with {} models, showing loading transition", models.len()));
                    (vec!["⚡ Loading models...".to_string()], true)
                }
            }
            _ => {
                // Fallback to text input for unknown providers
                if let Some(model) = self.show_text_input("Enter model name", &current_config.get_model())? {
                    app.set_model(&model);
                    output.print_system(&format!("✅ Model set to: {}", model))?;
                }
                return Ok(());
            }
        };

        // Handle loading state consistently for all providers
        let final_models = if is_loading {
            models
        } else {
            // Models loaded quickly, but we still want to show transition
            vec!["⚡ Loading models...".to_string()]
        };

        // Handle empty models list
        if final_models.is_empty() {
            output.print_system(&format!("⚠️ No {} models available. Try selecting the provider again to fetch models.", provider))?;
            return Ok(());
        }

        let current_idx = final_models
            .iter()
            .position(|m| m == &current_model)
            .unwrap_or(0);

        // Clear any pending events in the buffer
        std::thread::sleep(Duration::from_millis(20));
        for _ in 0..3 {
            while event::poll(Duration::from_millis(0))? {
                let _ = event::read()?;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        // Create a temporary selection for model with search support
        let mut selected_idx = current_idx;
        let mut search_query = String::new();
        let mut all_models = final_models.clone();
        let mut loading_spinner = all_models.len() == 1 && (all_models[0].contains("Loading") || all_models[0].contains("⚡") || all_models[0].contains("Fetching"));
        let mut spinner_counter = 0;
        let mut needs_clear = false; // Track when to clear screen
        let mut last_selected_idx = selected_idx; // Track scrolling

        // State tracking for selective rendering - track actual render state, not calculations
        let mut last_rendered_state: Option<(Vec<String>, usize, String, bool)> = None;


        loop {
            // Always check cache until we have real models (not just "Fetching models...")
            let should_check_cache = loading_spinner ||
                (all_models.len() == 1 && (all_models[0].contains("Loading") || all_models[0].contains("⚡") || all_models[0].contains("Fetching"))) ||
                spinner_counter < 50; // Keep checking longer for real models to arrive

            if should_check_cache {
                spinner_counter += 1;

                // Re-evaluate loading spinner state in case models changed
                let was_loading = loading_spinner;
                loading_spinner = all_models.len() == 1 && (all_models[0].contains("Loading") || all_models[0].contains("⚡") || all_models[0].contains("Fetching"));
                if was_loading != loading_spinner {
                    // State changed - clear screen once to show new content
                    needs_clear = true;
                }

                // Shorter timeout after 10 seconds (100 iterations of 100ms)
                if spinner_counter > 100 {
                    all_models = vec!["⚠️ Loading taking too long - Press ESC or try a different provider".to_string()];
                    loading_spinner = false;
                    let _ = output.print_system("⚠️ Model loading timed out - try using a different provider");
                } else {
                    // Check cache every iteration for immediate response
                    let cached_models = match provider.to_lowercase().as_str() {
                        "openai" => app.get_cached_openai_models(),
                        "anthropic" => app.get_cached_anthropic_models(),
                        "ollama" => app.get_cached_ollama_models(),
                        "z.ai coding plan" | "z.ai" | "zai" => app.get_cached_zai_models(),
                        "openrouter" => app.get_cached_openrouter_models(),
                        _ => None,
                    };
                    
                    match cached_models {
                        Some(models) => {
                            if models.is_empty() {
                                // Still empty, continue loading
                            } else if models.len() == 1 && (models[0].contains("Loading") || models[0].contains("timeout") || models[0].contains("Fetching") || models[0].contains("⚡")) {
                                // Still in loading state
                            } else {
                                // Real models loaded! Update immediately and clear screen once
                                if all_models != models {
                                    all_models = models;
                                    loading_spinner = false;
                                    needs_clear = true; // Clear once when models finish loading
                                }
                            }
                        }
                        None => {
                            // Cache is None, models not loaded yet
                        }
                    }

                    // Update loading text with spinning animation
                    if loading_spinner {
                        let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
                        let spinner = spinner_chars[(spinner_counter / 2) % spinner_chars.len()];
                        all_models = vec![format!("{} Fetching models...", spinner)];
                    }
                }
            }

            // Filter models based on search query
            let filtered_models: Vec<String> = if search_query.is_empty() {
                all_models.clone()
            } else {
                all_models.iter()
                    .filter(|model| model.to_lowercase().contains(&search_query.to_lowercase()))
                    .cloned()
                    .collect()
            };

            // Update selected_idx to be within bounds of filtered models
            if filtered_models.is_empty() {
                selected_idx = 0;
            } else if selected_idx >= filtered_models.len() {
                selected_idx = filtered_models.len() - 1;
            }

            // Create current render state tuple for comparison
            let current_state = (filtered_models.clone(), selected_idx, search_query.clone(), loading_spinner);

            // Check if search query changed (requires clear and full re-render)
            let search_changed = if let Some(ref last_state) = last_rendered_state {
                last_state.2 != search_query  // Compare search query (index 2)
            } else {
                false
            };

            // Only render if the state actually changed
            let should_render = if let Some(ref last_state) = last_rendered_state {
                // Compare the actual render state, not intermediate calculations
                last_state != &current_state || needs_clear
            } else {
                // First render
                true
            };

            if should_render {
                // Clear screen once if needed (after fetch completes, major changes, or search changed)
                if needs_clear || search_changed {
                    stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                    stdout().flush()?;
                    needs_clear = false;
                }

                // Render the full UI
                self.render_model_selector_with_search(&filtered_models, selected_idx, &search_query, loading_spinner)?;

                // Update last rendered state
                last_rendered_state = Some(current_state);
                last_selected_idx = selected_idx;
            }

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        match key_event.code {
                            KeyCode::Up => {
                                if selected_idx > 0 && !filtered_models.is_empty() {
                                    selected_idx -= 1;
                                    if selected_idx != last_selected_idx {
                                        needs_clear = true; // Clear once when scrolling
                                        last_selected_idx = selected_idx;
                                    }
                                }
                            }
                            KeyCode::Down => {
                                if selected_idx + 1 < filtered_models.len() {
                                    selected_idx += 1;
                                    if selected_idx != last_selected_idx {
                                        needs_clear = true; // Clear once when scrolling
                                        last_selected_idx = selected_idx;
                                    }
                                }
                            }
                            KeyCode::PageUp => {
                                if selected_idx > 10 {
                                    selected_idx -= 10;
                                } else {
                                    selected_idx = 0;
                                }
                                if selected_idx != last_selected_idx {
                                    needs_clear = true; // Clear once when scrolling
                                    last_selected_idx = selected_idx;
                                }
                            }
                            KeyCode::PageDown => {
                                if !filtered_models.is_empty() && selected_idx + 10 < filtered_models.len() {
                                    selected_idx += 10;
                                } else if !filtered_models.is_empty() {
                                    selected_idx = filtered_models.len() - 1;
                                }
                                if selected_idx != last_selected_idx {
                                    needs_clear = true; // Clear once when scrolling
                                    last_selected_idx = selected_idx;
                                }
                            }
                            KeyCode::Home => {
                                selected_idx = 0;
                                if selected_idx != last_selected_idx {
                                    needs_clear = true; // Clear once when scrolling
                                    last_selected_idx = selected_idx;
                                }
                            }
                            KeyCode::End => {
                                if !filtered_models.is_empty() {
                                    selected_idx = filtered_models.len() - 1;
                                    if selected_idx != last_selected_idx {
                                        needs_clear = true; // Clear once when scrolling
                                        last_selected_idx = selected_idx;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                if !filtered_models.is_empty() {
                                    app.set_model(&filtered_models[selected_idx]);
                                    output.print_system(&format!(
                                        "✅ Model set to: {}",
                                        filtered_models[selected_idx]
                                    ))?;
                                }
                                break;
                            }
                            KeyCode::Esc => {
                                break;
                            }
                            KeyCode::Backspace => {
                                if !search_query.is_empty() {
                                    search_query.pop();
                                }
                            }
                            // Handle Ctrl+C BEFORE general character input
                            KeyCode::Char('c') if key_event.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                                if loading_spinner {
                                    // When loading, clear cache
                                    self.debug_log_append("Ctrl+C clear cache triggered");
                                    match provider.to_lowercase().as_str() {
                                        "openai" => { let _ = app.cache_openai_models(Vec::new()); },
                                        "anthropic" => { let _ = app.cache_anthropic_models(Vec::new()); },
                                        "ollama" => { let _ = app.cache_ollama_models(Vec::new()); },
                                        "z.ai coding plan" | "z.ai" | "zai" => { let _ = app.cache_zai_models(Vec::new()); },
                                        "openrouter" => { let _ = app.cache_openrouter_models(Vec::new()); },
                                        _ => {}
                                    }
                                    let _ = output.print_system("🗑️ Cache cleared");
                                    spinner_counter = 0;
                                } else {
                                    // When not loading, exit the menu
                                    break;
                                }
                            }
                            KeyCode::Char('r') if key_event.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                                if loading_spinner {
                                    self.debug_log_append("Ctrl+R retry triggered");
                                    let _ = output.print_system("🔄 Retrying model fetch...");
                                    app.fetch_openrouter_models();
                                    spinner_counter = 0; // Reset timeout counter
                                }
                            }
                            // General character input for search - only if not a control character
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if !loading_spinner {
                                    search_query.push(c);
                                    // Reset selection when typing
                                    selected_idx = 0;
                                }
                            }
                            _ => {
                                // Ignore other keys
                                continue;
                            }
                        }
                    }
                    _ => {
                        // Ignore other event types
                        continue;
                    }
                }
            }
        }

        // Clear screen once when exiting submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        Ok(())
    }

    fn render_model_selector(&self, models: &[String], selected_idx: usize) -> Result<()> {
        self.render_model_selector_with_search(models, selected_idx, "", false)
    }

    fn render_model_selector_with_search(&self, models: &[String], selected_idx: usize, search_query: &str, loading: bool) -> Result<()> {
        let (cols, rows) = size()?;

        // Don't clear entire screen - causes flicker
        // We're in alternate screen mode, so just draw over existing content

        let menu_width = 50.min(cols.saturating_sub(4));
        let menu_height = models.len() + 4;
        let menu_height_u16 = menu_height as u16;

        // Calculate layout that fits within terminal height
        let total_models = models.len();

        // Reserve space for title (1), search (1), borders (2), navigation (1) = 5 lines total
        let available_height = rows.saturating_sub(6) as usize; // Leave extra padding
        let max_visible_models = available_height.max(1);

        // Use single column layout with proper width
        let menu_width = std::cmp::min(cols.saturating_sub(4), 60); // Good width for model names
        let menu_height = std::cmp::min(max_visible_models, total_models) + 6; // +6 for title, search, borders, navigation
        let menu_height_u16 = menu_height as u16;

        // Ensure menu fits in terminal
        let final_menu_height = if menu_height_u16 > rows.saturating_sub(4) {
            rows.saturating_sub(4) as usize
        } else {
            menu_height
        };

        let start_x = if cols > menu_width { cols.saturating_sub(menu_width) / 2 } else { 0 };
        let start_y = if rows > final_menu_height as u16 { rows.saturating_sub(final_menu_height as u16) / 2 } else { 0 };

        // Calculate viewport - ensure selected item is visible
        let actual_visible_models = std::cmp::min(max_visible_models, final_menu_height.saturating_sub(6));
        let viewport_start = if selected_idx >= actual_visible_models {
            selected_idx - actual_visible_models + 1
        } else {
            0
        };
        let viewport_end = std::cmp::min(viewport_start + actual_visible_models, total_models);

        // Display title with search hint
        let title = if search_query.is_empty() {
            format!("Select AI Model ({} models)", total_models)
        } else {
            format!("Select AI Model ({} of {} filtered)", models.len(), total_models)
        };
        self.draw_modern_box(start_x, start_y, menu_width, final_menu_height as u16, &title)?;

        // Show search input
        let search_y = start_y + 1;
        let search_text = if loading {
            "🔄 Fetching models...".to_string()
        } else if search_query.is_empty() {
            "🔍 Type to search models".to_string()
        } else {
            format!("🔍 {}", search_query)
        };

        // Print search text (pad with spaces to clear previous content)
        let padded_search = format!("{:width$}", search_text, width = (menu_width - 4) as usize);
        stdout().queue(MoveTo(start_x + 2, search_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(&padded_search))?
              .queue(ResetColor)?;

        // Display models in viewport
        let max_text_width = menu_width.saturating_sub(6) as usize; // Leave space for prefix and padding

        if models.is_empty() {
            // Show a nice "no models found" message
            let y = start_y + 3;
            let no_results_msg = if search_query.is_empty() {
                "🔍 No models available"
            } else {
                "🔍 No models found with that name"
            };
            let padded_msg = format!("{:^width$}", no_results_msg, width = (menu_width - 4) as usize);
            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(SetForegroundColor(crossterm::style::Color::DarkGrey))?
                  .queue(Print(&padded_msg))?
                  .queue(ResetColor)?;
        } else {
            // Safe subtraction with saturating_sub to prevent overflow
            let items_to_show = viewport_end.saturating_sub(viewport_start);

            for (idx, model) in models.iter().enumerate().skip(viewport_start).take(items_to_show) {
                let y = start_y + 3 + (idx - viewport_start) as u16;

                // Truncate long model names to fit
                let display_text = if model.len() > max_text_width {
                    format!("{}...", &model[..max_text_width.saturating_sub(3)])
                } else {
                    model.clone()
                };

                let prefix = if idx == selected_idx { "▶ " } else { "  " };
                let text = format!("{}{}", prefix, display_text);

                // Pad with spaces to clear any previous content
                let padded_text = format!("{:width$}", text, width = (menu_width - 4) as usize);

                let color = if idx == selected_idx {
                    SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI))
                } else {
                    SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
                };

                // Print the padded model text
                stdout().queue(MoveTo(start_x + 2, y))?
                      .queue(color)?
                      .queue(Print(&padded_text))?
                      .queue(ResetColor)?;
            }
        }

        // Show navigation hint (centered)
        let nav_y = start_y + final_menu_height as u16 - 1;
        let nav_text = if models.is_empty() {
            "No results - Press ESC to go back".to_string()
        } else if viewport_start == 0 && viewport_end == total_models {
            // All models visible - show enter to select and ESC to go back
            "↑↓ Navigate • ↵ Select • ← Back".to_string()
        } else {
            // Showing a subset - show position with enter and back options
            format!("↑↓ Navigate ({}-{} of {}) • ↵ Select • ← Back",
                    viewport_start + 1, viewport_end, total_models)
        };

        // Print navigation text (centered)
        let nav_x = start_x + (menu_width - nav_text.len() as u16) / 2;
        stdout().queue(MoveTo(nav_x, nav_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(&nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    // Selective rendering helper: Only render the search bar
    fn render_search_bar(&self, search_query: &str, loading: bool, cols: u16, rows: u16) -> Result<()> {
        let menu_width = std::cmp::min(cols.saturating_sub(4), 60);
        let available_height = rows.saturating_sub(6) as usize;
        let max_visible_models = available_height.max(1);
        let menu_height = max_visible_models + 6;
        let final_menu_height = if menu_height as u16 > rows.saturating_sub(4) {
            rows.saturating_sub(4) as usize
        } else {
            menu_height
        };
        let start_x = if cols > menu_width { cols.saturating_sub(menu_width) / 2 } else { 0 };
        let start_y = if rows > final_menu_height as u16 { rows.saturating_sub(final_menu_height as u16) / 2 } else { 0 };
        let search_y = start_y + 1;

        let search_text = if loading {
            "🔄 Fetching models...".to_string()
        } else if search_query.is_empty() {
            "🔍 Type to search models".to_string()
        } else {
            format!("🔍 {}", search_query)
        };

        // Clear the search line
        stdout().queue(MoveTo(start_x + 2, search_y))?;
        for _ in 0..(menu_width - 4) {
            stdout().queue(Print(" "))?;
        }

        // Print the search text
        stdout().queue(MoveTo(start_x + 2, search_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(&search_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    // Selective rendering helper: Only render the changed selection lines
    fn render_model_selection_change(&self, models: &[String], old_idx: usize, new_idx: usize, viewport_start: usize, cols: u16, rows: u16) -> Result<()> {
        let menu_width = std::cmp::min(cols.saturating_sub(4), 60);
        let max_text_width = menu_width.saturating_sub(6) as usize;
        let available_height = rows.saturating_sub(6) as usize;
        let max_visible_models = available_height.max(1);
        let menu_height = std::cmp::min(max_visible_models, models.len()) + 6;
        let final_menu_height = if menu_height as u16 > rows.saturating_sub(4) {
            rows.saturating_sub(4) as usize
        } else {
            menu_height
        };
        let start_x = if cols > menu_width { cols.saturating_sub(menu_width) / 2 } else { 0 };
        let start_y = if rows > final_menu_height as u16 { rows.saturating_sub(final_menu_height as u16) / 2 } else { 0 };

        // Render old selection (now unselected)
        if old_idx >= viewport_start && old_idx < models.len() {
            let relative_idx = old_idx - viewport_start;
            let y = start_y + 3 + relative_idx as u16;

            if let Some(model) = models.get(old_idx) {
                // Clear the line
                stdout().queue(MoveTo(start_x + 2, y))?;
                for _ in 0..(menu_width - 4) {
                    stdout().queue(Print(" "))?;
                }

                // Print unselected item
                let display_text = if model.len() > max_text_width {
                    format!("{}...", &model[..max_text_width.saturating_sub(3)])
                } else {
                    model.clone()
                };
                let text = format!("  {}", display_text);

                stdout().queue(MoveTo(start_x + 2, y))?
                      .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                      .queue(Print(&text))?
                      .queue(ResetColor)?;
            }
        }

        // Render new selection (now selected)
        if new_idx >= viewport_start && new_idx < models.len() {
            let relative_idx = new_idx - viewport_start;
            let y = start_y + 3 + relative_idx as u16;

            if let Some(model) = models.get(new_idx) {
                // Clear the line
                stdout().queue(MoveTo(start_x + 2, y))?;
                for _ in 0..(menu_width - 4) {
                    stdout().queue(Print(" "))?;
                }

                // Print selected item
                let display_text = if model.len() > max_text_width {
                    format!("{}...", &model[..max_text_width.saturating_sub(3)])
                } else {
                    model.clone()
                };
                let text = format!("▶ {}", display_text);

                stdout().queue(MoveTo(start_x + 2, y))?
                      .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI)))?
                      .queue(Print(&text))?
                      .queue(ResetColor)?;
            }
        }

        stdout().flush()?;
        Ok(())
    }

    fn show_text_input(&mut self, prompt: &str, default: &str) -> Result<Option<String>> {
        let mut input = default.to_string();
        let mut cursor_pos = input.len();

        // Clear screen once when entering submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // Clear any pending events in the buffer
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        loop {
            self.render_text_input(prompt, &input, cursor_pos)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        // Only handle valid input keys
                        match key_event.code {
                            KeyCode::Enter => {
                                return Ok(Some(input));
                            }
                            KeyCode::Esc => {
                                return Ok(None);
                            }
                            KeyCode::Char(c) => {
                                input.insert(cursor_pos, c);
                                cursor_pos += 1;
                            }
                            KeyCode::Backspace => {
                                if cursor_pos > 0 {
                                    input.remove(cursor_pos - 1);
                                    cursor_pos -= 1;
                                }
                            }
                            KeyCode::Delete => {
                                if cursor_pos < input.len() {
                                    input.remove(cursor_pos);
                                }
                            }
                            KeyCode::Left => {
                                if cursor_pos > 0 {
                                    cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if cursor_pos < input.len() {
                                    cursor_pos += 1;
                                }
                            }
                            _ => {
                                // Ignore all other keys
                                continue;
                            }
                        }
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }
    }

    fn render_text_input(&self, prompt: &str, input: &str, cursor_pos: usize) -> Result<()> {
        let (cols, rows) = size()?;

        // Don't clear entire screen - causes flicker
        // We're in alternate screen mode, so just draw over existing content

        let menu_width = 60.min(cols.saturating_sub(4));
        let menu_height = 8u16; // Increased for footer
        let start_x = cols.saturating_sub(menu_width) / 2;
        let start_y = rows.saturating_sub(menu_height) / 2;

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "INPUT")?;

        // Draw title/header
        let title_y = start_y + 1;
        let title_x = start_x + (menu_width - prompt.len() as u16) / 2;
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(prompt)))?;

        // Draw input field
        let input_y = start_y + 3;
        let input_text = if input.is_empty() {
            "← Type here..."
        } else {
            input
        };

        // Draw input text with appropriate colors
        if input.is_empty() {
            stdout().queue(MoveTo(start_x + 2, input_y))?
                  .queue(SetForegroundColor(crossterm::style::Color::DarkGrey))?
                  .queue(Print(input_text))?
                  .queue(ResetColor)?;
        } else {
            stdout().queue(MoveTo(start_x + 2, input_y))?
                  .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                  .queue(Print(input_text))?
                  .queue(ResetColor)?;
        }

        // Draw cursor with primary color
        let display_cursor_pos = if input.is_empty() { 0 } else { cursor_pos };
        stdout().queue(MoveTo(start_x + 2 + display_cursor_pos as u16, input_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI)))?
              .queue(Print("█"))?
              .queue(ResetColor)?;

        // Draw footer with navigation instructions (centered, intercepting box border)
        let footer_y = start_y + menu_height - 1;
        let nav_text = "↵ Submit • Esc Cancel";
        let nav_x = start_x + (menu_width - nav_text.len() as u16) / 2;

        stdout().queue(MoveTo(nav_x, footer_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn show_info_and_help(&mut self, app: &App) -> Result<()> {
        // Clear screen once when entering submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // Clear any pending events in the buffer
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        let mut scroll_offset = 0;

        loop {
            self.render_help(scroll_offset)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        match key_event.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if scroll_offset > 0 {
                                    scroll_offset -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                // Get help content and calculate max scroll
                                let help_lines = self.get_help_content(app);
                                let menu_height = 22u16;
                                let content_height = (menu_height - 5) as usize; // Space for content display
                                let max_scroll = help_lines.len().saturating_sub(content_height);

                                if scroll_offset < max_scroll {
                                    scroll_offset += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                scroll_offset = scroll_offset.saturating_sub(5);
                            }
                            KeyCode::PageDown => {
                                let help_lines = self.get_help_content(app);
                                let menu_height = 22u16;
                                let content_height = (menu_height - 5) as usize;
                                let max_scroll = help_lines.len().saturating_sub(content_height);

                                scroll_offset = (scroll_offset + 5).min(max_scroll);
                            }
                            KeyCode::Home => {
                                scroll_offset = 0;
                            }
                            KeyCode::End => {
                                let help_lines = self.get_help_content(app);
                                let menu_height = 22u16;
                                let content_height = (menu_height - 5) as usize;
                                scroll_offset = help_lines.len().saturating_sub(content_height);
                            }
                            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                                break;
                            }
                            _ => {
                                // Ignore all other keys
                                continue;
                            }
                        }
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }

        // Clear screen once when exiting submenu to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        Ok(())
    }

    fn get_help_content(&self, app: &App) -> Vec<String> {
        let config = app.get_config();
        let messages = app.get_message_history();

        vec![
            "📊 Session Information".to_string(),
            format!("  Active Provider: {}", config.active_provider),
            format!("  Current Model: {}", config.get_model()),
            format!("  API URL: {}", config.get_api_url()),
            format!("  Messages in history: {}", messages.len()),
            "".to_string(),
            "🔧 Commands:".to_string(),
            "  /help     - Show this help".to_string(),
            "  /menu     - Open interactive menu".to_string(),
            "  /clear    - Clear conversation history".to_string(),
            "  /config   - Show current configuration".to_string(),
            "  /model <name> - Change AI model".to_string(),
            "  exit or quit - Exit ARULA".to_string(),
            "".to_string(),
            "⌨️  Keyboard Shortcuts:".to_string(),
            "  Ctrl+C    - Open menu".to_string(),
            "  m         - Open menu".to_string(),
            "  Ctrl+D    - Exit".to_string(),
            "  Up/Down   - Navigate command history".to_string(),
            "".to_string(),
            "💡 Tips:".to_string(),
            "  • End line with \\ to continue on next line".to_string(),
            "  • Ask ARULA to execute bash commands".to_string(),
            "  • Use natural language".to_string(),
            "  • Native terminal scrollback works!".to_string(),
            "".to_string(),
            "🛠️  Available Tools:".to_string(),
            "  • execute_bash - Run shell commands".to_string(),
            "  • read_file - Read file contents".to_string(),
            "  • write_file - Create or overwrite files".to_string(),
            "  • edit_file - Edit existing files".to_string(),
            "  • list_directory - Browse directories".to_string(),
            "  • search_files - Fast parallel search".to_string(),
            "  • visioneer - Desktop automation".to_string(),
        ]
    }

    fn render_help(&self, scroll_offset: usize) -> Result<()> {
        let (cols, rows) = size()?;

        // Don't clear entire screen - causes flicker
        // We're in alternate screen mode, so just draw over existing content

        let menu_width = 70.min(cols - 4);
        let menu_height = 22u16; // Increased for header and footer
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "HELP")?;

        // Draw title/header
        let title_y = start_y + 1;
        let title = "ARULA Info & Help";
        let title_x = start_x + (menu_width - title.len() as u16) / 2;
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Get all help content
        let help_lines = vec![
            "🔧 Commands:",
            "  /help     - Show this help",
            "  /menu     - Open interactive menu",
            "  /clear    - Clear conversation history",
            "  /config   - Show current configuration",
            "  /model <name> - Change AI model",
            "  exit or quit - Exit ARULA",
            "",
            "⌨️  Keyboard Shortcuts:",
            "  Ctrl+C    - Open menu",
            "  m         - Open menu",
            "  Ctrl+D    - Exit",
            "  Up/Down   - Navigate command history",
            "",
            "💡 Tips:",
            "  • End line with \\ to continue on next line",
            "  • Ask ARULA to execute bash commands",
            "  • Use natural language",
            "  • Native terminal scrollback works!",
            "",
            "🛠️  Available Tools:",
            "  • execute_bash - Run shell commands",
            "  • read_file - Read file contents",
            "  • write_file - Create or overwrite files",
            "  • edit_file - Edit existing files",
            "  • list_directory - Browse directories",
            "  • search_files - Fast parallel search",
            "  • visioneer - Desktop automation",
        ];

        // Calculate visible area
        let content_height = (menu_height - 5) as usize; // Reserve space for title, border, and footer
        let visible_lines: Vec<&str> = help_lines
            .iter()
            .skip(scroll_offset)
            .take(content_height)
            .copied()
            .collect();

        // Draw visible lines
        for (i, line) in visible_lines.iter().enumerate() {
            let y = start_y + 3 + i as u16;

            // Use different colors for different sections
            let color = if line.starts_with("🔧") || line.starts_with("⌨️") || line.starts_with("💡") || line.starts_with("🛠️") || line.starts_with("📊") {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI))
            } else if line.starts_with("  •") {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
            } else {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
            };

            // Clear the line first to remove any previous content
            stdout().queue(MoveTo(start_x + 2, y))?;
            for _ in 0..(menu_width - 4) {
                stdout().queue(Print(" "))?;
            }

            // Draw the text
            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(color)?
                  .queue(Print(*line))?
                  .queue(ResetColor)?;
        }

        // Clear any remaining lines if content is shorter than viewport
        for i in visible_lines.len()..content_height {
            let y = start_y + 3 + i as u16;
            stdout().queue(MoveTo(start_x + 2, y))?;
            for _ in 0..(menu_width - 4) {
                stdout().queue(Print(" "))?;
            }
        }

        // Draw footer with dynamic scroll indicator (centered, intercepting box border)
        let footer_y = start_y + menu_height - 1;
        let max_scroll = help_lines.len().saturating_sub(content_height);

        // Determine scroll indicator text for footer
        let scroll_part = if max_scroll == 0 {
            "".to_string()
        } else if scroll_offset == 0 {
            "⬇ More".to_string()
        } else if scroll_offset >= max_scroll {
            "⬆ Top".to_string()
        } else {
            format!("↑↓ {}/{}", scroll_offset + 1, max_scroll + 1)
        };

        // Build navigation text with scroll indicator
        let nav_text = if scroll_part.is_empty() {
            "↵ Continue • Esc Back".to_string()
        } else {
            format!("{} • ↵ Continue • Esc Back", scroll_part)
        };

        let nav_x = start_x + (menu_width - nav_text.len() as u16) / 2;

        stdout().queue(MoveTo(nav_x, footer_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn show_confirm_dialog(&mut self, message: &str) -> Result<bool> {
        let mut selected = false; // false for No, true for Yes

        // Clear screen once when entering dialog to avoid artifacts
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // Clear any pending events in the buffer
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        loop {
            self.render_confirm_dialog(message, selected)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        match key_event.code {
                            KeyCode::Enter => {
                                return Ok(selected);
                            }
                            KeyCode::Esc => {
                                return Ok(false);
                            }
                            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                                // Ctrl+C should exit the app (same as selecting "Yes" on exit confirmation)
                                return Ok(true);
                            }
                            KeyCode::Left | KeyCode::Right | KeyCode::Tab |
                            KeyCode::Char('h') | KeyCode::Char('l') => {
                                selected = !selected;
                            }
                            _ => {
                                // Ignore all other keys
                                continue;
                            }
                        }
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }
    }

    fn render_confirm_dialog(&self, message: &str, selected: bool) -> Result<()> {
        let (cols, rows) = size()?;

        // Don't clear entire screen - causes flicker
        // We're in alternate screen mode, so just draw over existing content

        let menu_width = 50.min(cols - 4);
        let menu_height = 9u16; // Consistent height
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        // Draw modern box for confirmation
        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "CONFIRM")?;

        // Draw title
        let title_y = start_y + 1;
        let title = message;
        let title_x = start_x + (menu_width - title.len() as u16) / 2;
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Modern styled options
        let no_text = "NO";
        let yes_text = "YES";

        let options_y = start_y + 3;
        let no_x = start_x + menu_width / 2 - 10;
        let yes_x = start_x + menu_width / 2 + 2;

        // Draw NO option
        if !selected {
            // Selected (NO is the default)
            stdout().queue(MoveTo(no_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::Red))?
                  .queue(SetForegroundColor(crossterm::style::Color::White))?
                  .queue(Print(format!(" {} ", no_text)))?
                  .queue(ResetColor)?;
        } else {
            // Unselected
            stdout().queue(MoveTo(no_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::DarkGrey))?
                  .queue(SetForegroundColor(crossterm::style::Color::White))?
                  .queue(Print(format!(" {} ", no_text)))?
                  .queue(ResetColor)?;
        }

        // Draw YES option
        if selected {
            // Selected
            stdout().queue(MoveTo(yes_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::Green))?
                  .queue(SetForegroundColor(crossterm::style::Color::White))?
                  .queue(Print(format!(" {} ", yes_text)))?
                  .queue(ResetColor)?;
        } else {
            // Unselected
            stdout().queue(MoveTo(yes_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::DarkGrey))?
                  .queue(SetForegroundColor(crossterm::style::Color::White))?
                  .queue(Print(format!(" {} ", yes_text)))?
                  .queue(ResetColor)?;
        }

        // Draw footer with navigation instructions (centered, intercepting box border)
        let footer_y = start_y + menu_height - 1;
        let nav_text = "←→ Navigate • ↵ Select • Esc Cancel";
        let nav_x = start_x + (menu_width - nav_text.len() as u16) / 2;

        stdout().queue(MoveTo(nav_x, footer_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn render_frame(&self, app: &App, _output: &OutputHandler) -> Result<()> {
        let (_cols, _rows) = size()?;

        // Clear the screen when switching between menus to avoid artifacts
        // This is a one-time clear, not on every frame
        // The menu loop should call this only when transitioning

        if self.is_in_config {
            self.render_config_menu(app)?;
        } else {
            self.render_main_menu()?;
        }

        stdout().flush()?;
        Ok(())
    }

    fn render_main_menu(&self) -> Result<()> {
        let (cols, rows) = size()?;

        let menu_width = 50.min(cols - 4);
        let menu_height = 12; // Fixed height for better layout
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        // Draw modern box with gradient effect
        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "ARULA")?;

        // Draw title with modern styling
        let title_y = start_y + 1;
        let title = "● MENU";
        let title_len = title.len() as u16;
        let title_x = if menu_width > title_len + 2 {
            start_x + menu_width / 2 - title_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Draw menu items with modern styling
        let items_start_y = start_y + 3;
        for (i, option) in self.main_options.iter().enumerate() {
            let y = items_start_y + i as u16;

            if i == self.selected_index {
                // Selected item with modern highlight
                self.draw_selected_item(start_x + 2, y, menu_width - 4, option)?;
            } else {
                // Unselected item - clear the line first to remove any previous selection background
                stdout().queue(MoveTo(start_x + 2, y))?;
                for _ in 0..(menu_width - 4) {
                    stdout().queue(Print(" "))?;
                }
                // Then draw the text
                stdout().queue(MoveTo(start_x + 4, y))?
                      .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                      .queue(Print(option))?
                      .queue(ResetColor)?;
            }
        }

        // Draw modern help text (intercepting box border)
        let help_y = start_y + menu_height - 1;
        let help_text = "↑↓ Navigate • Enter Select • ESC Exit";
        let help_len = help_text.len() as u16;
        let help_x = if menu_width > help_len + 2 {
            start_x + menu_width / 2 - help_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(help_x, help_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(help_text))?
              .queue(ResetColor)?;

        Ok(())
    }

    fn render_config_menu(&self, app: &App) -> Result<()> {
        let (cols, rows) = size()?;

        let config = app.get_config();
        let mut display_options = self.config_options.clone();

        let menu_width = 60.min(cols - 4);

        // Calculate max width for menu items (menu_width - 6 for padding and marker)
        let max_item_width = menu_width.saturating_sub(6) as usize;

        // Update display values with modern styling and overflow protection
        display_options[0] = format!("○ Provider: {}", Self::truncate_text(&config.active_provider, max_item_width.saturating_sub(13)));
        display_options[1] = format!("○ Model: {}", Self::truncate_text(&config.get_model(), max_item_width.saturating_sub(11)));
        display_options[2] = format!("○ API URL: {}", Self::truncate_text(&config.get_api_url(), max_item_width.saturating_sub(13)));
        display_options[3] = format!(
            "○ API Key: {}",
            if config.get_api_key().is_empty() {
                "Not set"
            } else {
                "••••••••"
            }
        );
        let menu_height = 12; // Fixed height for consistency
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        // Draw modern box
        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "SETTINGS")?;

        // Draw title with modern styling
        let title_y = start_y + 1;
        let title = "⚙️ SETTINGS";
        let title_len = title.len() as u16;
        let title_x = if menu_width > title_len + 2 {
            start_x + menu_width / 2 - title_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Draw config items with modern styling
        let items_start_y = start_y + 3;
        for (i, option) in display_options.iter().enumerate() {
            let y = items_start_y + i as u16;

            // Check if this item is editable (API URL is index 2)
            let is_editable = if i == 2 {
                app.config.is_field_editable(ProviderField::ApiUrl)
            } else {
                true
            };

            if i == self.selected_index {
                // Selected item with modern highlight
                self.draw_selected_item(start_x + 2, y, menu_width - 4, option)?;
            } else {
                // Unselected item - clear the line first to remove any previous selection background
                stdout().queue(MoveTo(start_x + 2, y))?;
                for _ in 0..(menu_width - 4) {
                    stdout().queue(Print(" "))?;
                }
                // Then draw the text with gray color if not editable
                let color = if is_editable {
                    crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)
                } else {
                    crossterm::style::Color::DarkGrey
                };
                stdout().queue(MoveTo(start_x + 4, y))?
                      .queue(SetForegroundColor(color))?
                      .queue(Print(option))?
                      .queue(ResetColor)?;
            }
        }

        // Draw modern help text (intercepting box border)
        let help_y = start_y + menu_height - 1;
        let help_text = "↑↓ Edit • Enter Select • ESC Exit";
        let help_len = help_text.len() as u16;
        let help_x = if menu_width > help_len + 2 {
            start_x + menu_width / 2 - help_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(help_x, help_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(help_text))?
              .queue(ResetColor)?;

        Ok(())
    }

    fn draw_modern_box(&self, x: u16, y: u16, width: u16, height: u16, _title: &str) -> Result<()> {
        // Modern box with rounded corners using our color theme
        let top_left = "╭";
        let top_right = "╮";
        let bottom_left = "╰";
        let bottom_right = "╯";
        let horizontal = "─";
        let vertical = "│";

        // Validate dimensions to prevent overflow
        if width < 2 || height < 2 {
            return Ok(());
        }

        // Don't clear the area - causes flicker!
        // The alternate screen is already clean on entry
        // Just draw the box borders directly

        // Draw borders using our AI highlight color (steel blue)
        stdout().queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?;

        // Draw vertical borders
        for i in 0..height {
            stdout().queue(MoveTo(x, y + i))?.queue(Print(vertical))?;
            stdout().queue(MoveTo(x + width.saturating_sub(1), y + i))?.queue(Print(vertical))?;
        }

        // Top border
        stdout().queue(MoveTo(x, y))?.queue(Print(top_left))?;
        for _i in 1..width.saturating_sub(1) {
            stdout().queue(Print(horizontal))?;
        }
        stdout().queue(Print(top_right))?;

        // Bottom border
        stdout().queue(MoveTo(x, y + height.saturating_sub(1)))?.queue(Print(bottom_left))?;
        for _i in 1..width.saturating_sub(1) {
            stdout().queue(Print(horizontal))?;
        }
        stdout().queue(Print(bottom_right))?;

        stdout().queue(ResetColor)?;
        Ok(())
    }

    fn draw_selected_item(&self, x: u16, y: u16, width: u16, text: &str) -> Result<()> {
        // Validate dimensions
        if width < 3 {
            return Ok(());
        }

        // Draw selection background using our background color
        stdout().queue(MoveTo(x, y))?;

        // Background fill with bounds checking using our theme colors
        for _i in 0..width {
            stdout().queue(SetBackgroundColor(crossterm::style::Color::AnsiValue(crate::colors::BACKGROUND_ANSI)))?;
            stdout().queue(Print(" "))?;
        }

        // Reset background for text
        stdout().queue(ResetColor)?;

        // Draw text with proper spacing and our primary color
        let display_text = format!("▶ {}", text);
        let safe_text = if display_text.len() > width.saturating_sub(4) as usize {
            // Truncate if too long
            let safe_len = width.saturating_sub(7) as usize;
            format!("▶ {}...", &text[..safe_len.min(text.len())])
        } else {
            display_text
        };

        stdout().queue(MoveTo(x + 2, y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI)))?
              .queue(SetBackgroundColor(crossterm::style::Color::AnsiValue(crate::colors::BACKGROUND_ANSI)))?
              .queue(Print(safe_text))?
              .queue(ResetColor)?;

        Ok(())
    }

    fn draw_box(&self, x: u16, y: u16, width: u16, height: u16, title: &str) -> Result<()> {
        // Keep the old method for compatibility
        let top_left = "╔";
        let top_right = "╗";
        let bottom_left = "╚";
        let bottom_right = "╝";
        let horizontal = "═";
        let vertical = "║";

        // Set purple color for borders
        stdout().queue(SetForegroundColor(Color::DarkMagenta))?;

        // Top border
        stdout().queue(MoveTo(x, y))?.queue(Print(top_left))?;
        for _i in 1..width-1 {
            stdout().queue(Print(horizontal))?;
        }
        stdout().queue(Print(top_right))?;

        // Title in top border
        if !title.is_empty() && title.len() < width as usize - 4 {
            let title_start = x + 2;
            stdout().queue(MoveTo(title_start, y))?;
            stdout().queue(SetBackgroundColor(Color::DarkMagenta))?
                  .queue(SetForegroundColor(Color::Yellow))?
                  .queue(Print(format!(" {} ", title)))?
                  .queue(ResetColor)?;
        }

        // Vertical borders
        for _i in 1..height-1 {
            stdout().queue(MoveTo(x, y + _i))?.queue(Print(vertical))?;
            stdout().queue(MoveTo(x + width - 1, y + _i))?.queue(Print(vertical))?;
        }

        // Bottom border
        stdout().queue(MoveTo(x, y + height - 1))?.queue(Print(bottom_left))?;
        for _i in 1..width-1 {
            stdout().queue(Print(horizontal))?;
        }
        stdout().queue(Print(bottom_right))?;

        stdout().queue(ResetColor)?;
        Ok(())
    }

    fn move_selection(&mut self, direction: isize, app: &App) {
        let options = if self.is_in_config {
            &self.config_options
        } else {
            &self.main_options
        };

        let mut new_index = self.selected_index as isize + direction;
        new_index = new_index.clamp(0, (options.len() - 1) as isize);

        // If in config menu, skip API URL (index 2) if it's not editable
        if self.is_in_config && new_index == 2 && !app.config.is_field_editable(ProviderField::ApiUrl) {
            // Skip the non-editable API URL by continuing in the same direction
            new_index += direction;
            new_index = new_index.clamp(0, (options.len() - 1) as isize);

            // Edge case: if we're at the boundary and trying to skip, stay at boundary
            // but make sure we don't land on index 2
            if new_index == 2 {
                // We wrapped around, so go to the opposite boundary
                if direction > 0 {
                    new_index = 3; // Skip to API Key
                } else {
                    new_index = 1; // Skip to Model
                }
            }
        }

        self.selected_index = new_index as usize;
    }

    fn cleanup_terminal(&self) -> Result<()> {
        let mut stdout = stdout();

        // Leave alternate screen FIRST to return to main terminal
        stdout.execute(LeaveAlternateScreen)?;

        // Reset terminal colors and attributes
        stdout.execute(crossterm::style::ResetColor)?;

        // Restore cursor visibility and style to match main app
        stdout.execute(Show)?;
        stdout.execute(SetCursorStyle::BlinkingBlock)?;

        // Move cursor to beginning of line for clean shell prompt
        stdout.execute(crossterm::cursor::MoveToColumn(0))?;

        // Ensure all commands are sent to terminal
        stdout.flush()?;

        Ok(())
    }
}

// Simple color formatting functions
fn format_colored(text: &str, color_code: &str) -> String {
    format!("\x1b[{}m{}\x1b[0m", color_code, text)
}

fn format_colored_bold(text: &str, color_code: &str) -> String {
    format!("\x1b[1;{}m{}\x1b[0m", color_code, text)
}

trait ColoredText {
    fn red(&self) -> String;
    fn green(&self) -> String;
    fn yellow(&self) -> String;
    fn blue(&self) -> String;
    fn purple(&self) -> String;
    fn dim(&self) -> String;
    fn bold(&self) -> String;
    fn yellow_bold(&self) -> String;
}

impl ColoredText for str {
    fn red(&self) -> String {
        format_colored(self, "31")
    }

    fn green(&self) -> String {
        format_colored(self, "32")
    }

    fn yellow(&self) -> String {
        format_colored(self, "33")
    }

    fn blue(&self) -> String {
        format_colored(self, "34")
    }

    fn purple(&self) -> String {
        format_colored(self, "35")
    }

    fn dim(&self) -> String {
        format_colored(self, "2")
    }

    fn bold(&self) -> String {
        format_colored_bold(self, "1")
    }

    fn yellow_bold(&self) -> String {
        format_colored_bold(self, "33")
    }
}

impl ColoredText for String {
    fn red(&self) -> String {
        self.as_str().red()
    }

    fn green(&self) -> String {
        self.as_str().green()
    }

    fn yellow(&self) -> String {
        self.as_str().yellow()
    }

    fn blue(&self) -> String {
        self.as_str().blue()
    }

    fn purple(&self) -> String {
        self.as_str().purple()
    }

    fn dim(&self) -> String {
        self.as_str().dim()
    }

    fn bold(&self) -> String {
        self.as_str().bold()
    }

    fn yellow_bold(&self) -> String {
        self.as_str().yellow_bold()
    }
}

impl Default for OverlayMenu {
    fn default() -> Self {
        Self::new()
    }
}