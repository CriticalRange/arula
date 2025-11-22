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
                "📊 Session Info".to_string(),
                "🧹 Clear Chat".to_string(),
                "💡 Help & Tips".to_string(),
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

        // Show confirmation dialog directly (no animation)
        let result = self.show_confirm_dialog("Exit ARULA?")?;

        // Cleanup and restore terminal (with proper cursor restoration)
        self.cleanup_terminal()?;

        Ok(result)
    }

    fn render_exit_confirmation(&self, message: &str) -> Result<()> {
        let (cols, rows) = size()?;

        stdout().queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;

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

        // Main event loop (no animation)
        let result = self.run_menu_loop(app, output)?;

        // Cleanup and restore terminal
        self.cleanup_terminal()?;

        Ok(result)
    }

    fn run_menu_loop(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<bool> {
        let mut should_exit_app = false;

        // Comprehensive event clearing to prevent submenu issues
        std::thread::sleep(Duration::from_millis(50));
        for _ in 0..5 { // Multiple passes to ensure all events are cleared
            while event::poll(Duration::from_millis(0))? {
                let _ = event::read()?;
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        loop {
            self.render_frame(app, output)?;

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
                                let result = self.handle_key_event(key_event, app, output)?;
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
                        // Redraw on resize
                        self.render_frame(app, output)?;
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

    fn handle_key_event(&mut self, key_event: KeyEvent, app: &mut App, output: &mut OutputHandler) -> Result<MenuAction> {
        match key_event.code {
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                self.move_selection(-1);
                Ok(MenuAction::Continue)
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => {
                self.move_selection(1);
                Ok(MenuAction::Continue)
            }
            KeyCode::Enter => {
                if self.is_in_config {
                    if self.handle_config_selection(app, output)? {
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
            2 => { // Session info
                self.show_session_info(app)?;
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
            4 => { // Help
                self.show_help()?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            5 => { // Exit
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
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            1 => { // Model
                self.show_model_selector(app, output)?;
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            2 => { // API URL
                if app.config.ai.is_field_editable(ProviderField::ApiUrl) {
                    if let Some(url) = self.show_text_input("Enter API URL", &app.get_config().ai.api_url)? {
                        app.config.ai.api_url = url.clone();
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
                } else {
                    output.print_system(&format!(
                        "🚫 API URL is not editable for {}. Current: {}",
                        app.config.ai.provider,
                        app.config.ai.api_url
                    ))?;
                }
                // Clear any pending events that might have been generated during the dialog
                while event::poll(Duration::from_millis(0))? {
                    let _ = event::read()?;
                }
                Ok(false)
            }
            3 => { // API Key
                if let Some(key) = self.show_text_input("Enter API Key (or leave empty to use environment variable)", "")? {
                    if !key.is_empty() {
                        app.config.ai.api_key = key;
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
            .position(|&p| p == current_config.ai.provider)
            .unwrap_or(0);

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
                                let old_api_key = app.config.ai.api_key.clone();

                                // Update provider and apply provider defaults
                                app.config.ai.provider = new_provider.clone();
                                app.config.ai.apply_provider_defaults(true); // Preserve existing API key

                                // Show what changed
                                if old_api_key != app.config.ai.api_key && !old_api_key.is_empty() {
                                    output.print_system("🔑 API key preserved from previous provider")?;
                                }

                                output.print_system(&format!(
                                    "🔄 Model automatically set to: {}",
                                    app.config.ai.model
                                ))?;
                                output.print_system(&format!(
                                    "🌐 API URL automatically set to: {}",
                                    app.config.ai.api_url
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

        Ok(())
    }

    fn render_provider_selector(&self, providers: &[&str], selected_idx: usize) -> Result<()> {
        let (cols, rows) = size()?;

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 40.min(cols.saturating_sub(4));
        let menu_height = providers.len() + 4;
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

        self.draw_modern_box(start_x, start_y, menu_width, menu_height as u16, "Select AI Provider")?;

        for (i, provider) in providers.iter().enumerate() {
            let y = start_y + 2 + i as u16;
            let prefix = if i == selected_idx { "▶ " } else { "  " };
            let text = format!("{}{}", prefix, provider);
            let color = if i == selected_idx {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI))
            } else {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
            };

            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(color)?
                  .queue(Print(text))?
                  .queue(ResetColor)?;
        }

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
        let _ = output.print_system("🔄 Fetching models...");
        app.fetch_openrouter_models();

        // Return loading state - keep menu open while fetching
        self.debug_log_append("Returning loading state with 1 model");
        (vec!["Fetching models...".to_string()], true) // (models, is_loading)
    }

    fn show_model_selector(&mut self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        let current_config = app.get_config();
        let provider = current_config.ai.provider.clone();
        let current_model = current_config.ai.model.clone();

        // For custom provider, use text input instead of selector
        if provider.to_lowercase() == "custom" {
            if let Some(model) = self.show_text_input("Enter model name", &current_model)? {
                app.set_model(&model);
                output.print_system(&format!("✅ Model set to: {}", model))?;
            }
            return Ok(());
        }

        // For predefined providers, use selector
        let models: Vec<String> = match provider.to_lowercase().as_str() {
            "z.ai coding plan" | "z.ai" | "zai" => {
                vec!["glm-4.6".to_string(), "glm-4.5".to_string(), "glm-4.5-air".to_string()]
            }
            "openai" => {
                vec!["gpt-4".to_string(), "gpt-4-turbo".to_string(), "gpt-3.5-turbo".to_string()]
            }
            "anthropic" => {
                vec!["claude-3-opus-20240229".to_string(), "claude-3-sonnet-20240229".to_string(), "claude-3-haiku-20240307".to_string()]
            }
            "ollama" => {
                vec!["llama2".to_string(), "codellama".to_string(), "mistral".to_string(), "vicuna".to_string()]
            }
            "openrouter" => {
                // For OpenRouter, fetch models dynamically with caching
                self.debug_log_append("OpenRouter provider selected, calling get_openrouter_models");

                // Force cache clear to simulate first-run behavior every time
                self.debug_log_append("Clearing cache to simulate first-run behavior");
                app.cache_openrouter_models(Vec::new());

                let (models, is_loading) = self.get_openrouter_models(app, output);
                self.debug_log_append(&format!("get_openrouter_models returned {} models, is_loading={}", models.len(), is_loading));

                // Always pass to loading loop, even if models might load quickly
                if is_loading {
                    self.debug_log_append(&format!("Starting loading state with {} models", models.len()));
                    models
                } else {
                    // Models loaded very quickly, but we still want to show transition
                    self.debug_log_append(&format!("Models loaded quickly with {} models, showing loading transition", models.len()));
                    vec!["⚡ Loading models...".to_string()]
                }
            }
            _ => {
                // Fallback to text input for unknown providers
                if let Some(model) = self.show_text_input("Enter model name", &current_config.ai.model)? {
                    app.set_model(&model);
                    output.print_system(&format!("✅ Model set to: {}", model))?;
                }
                return Ok(());
            }
        };

        // Handle empty models list
        if models.is_empty() {
            output.print_system("⚠️ No OpenRouter models available. Try selecting OpenRouter provider again to fetch models.")?;
            return Ok(());
        }

        let current_idx = models
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
        let mut all_models = models.clone();
        let mut loading_spinner = all_models.len() == 1 && (all_models[0].contains("Loading") || all_models[0].contains("⚡") || all_models[0].contains("Fetching"));
        let mut spinner_counter = 0;

  
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
                    // State changed, but we don't need to log it
                }

                // Shorter timeout after 10 seconds (100 iterations of 100ms)
                if spinner_counter > 100 {
                    all_models = vec!["⚠️ Loading taking too long - Press ESC or try a different provider".to_string()];
                    loading_spinner = false;
                    let _ = output.print_system("⚠️ Model loading timed out - try using a different provider");
                } else {
                    // Check cache every iteration for immediate response
                    match app.get_cached_openrouter_models() {
                        Some(cached_models) => {
                            if cached_models.is_empty() {
                                // Still empty, continue loading
                            } else if cached_models.len() == 1 && (cached_models[0].contains("Loading") || cached_models[0].contains("timeout") || cached_models[0].contains("Fetching") || cached_models[0].contains("⚡")) {
                                // Still in loading state
                            } else {
                                // Real models loaded! Update immediately
                                all_models = cached_models;
                                loading_spinner = false;
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

            // Render with search state and loading indicator
            self.render_model_selector_with_search(&filtered_models, selected_idx, &search_query, loading_spinner)?;

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
                                }
                            }
                            KeyCode::Down => {
                                if selected_idx + 1 < filtered_models.len() {
                                    selected_idx += 1;
                                }
                            }
                            KeyCode::PageUp => {
                                if selected_idx > 10 {
                                    selected_idx -= 10;
                                } else {
                                    selected_idx = 0;
                                }
                            }
                            KeyCode::PageDown => {
                                if !filtered_models.is_empty() && selected_idx + 10 < filtered_models.len() {
                                    selected_idx += 10;
                                } else if !filtered_models.is_empty() {
                                    selected_idx = filtered_models.len() - 1;
                                }
                            }
                            KeyCode::Home => {
                                selected_idx = 0;
                            }
                            KeyCode::End => {
                                if !filtered_models.is_empty() {
                                    selected_idx = filtered_models.len() - 1;
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
                            KeyCode::Char(c) if c.is_ascii() && !c.is_control() => {
                                if !loading_spinner {
                                    search_query.push(c);
                                    // Reset selection when typing
                                    selected_idx = 0;
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
                            KeyCode::Char('c') if key_event.modifiers == crossterm::event::KeyModifiers::CONTROL => {
                                if loading_spinner {
                                    self.debug_log_append("Ctrl+C clear cache triggered");
                                    let _ = app.cache_openrouter_models(Vec::new());
                                    let _ = output.print_system("🗑️ Cache cleared");
                                    spinner_counter = 0;
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

        Ok(())
    }

    fn render_model_selector(&self, models: &[String], selected_idx: usize) -> Result<()> {
        self.render_model_selector_with_search(models, selected_idx, "", false)
    }

    fn render_model_selector_with_search(&self, models: &[String], selected_idx: usize, search_query: &str, loading: bool) -> Result<()> {
        let (cols, rows) = size()?;

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 50.min(cols.saturating_sub(4));
        let menu_height = models.len() + 4;
        let menu_height_u16 = menu_height as u16;

        // Calculate layout that fits within terminal height
        let total_models = models.len();

        // Reserve space for title (1), search (1), borders (2), navigation (1) = 5 lines total
        let available_height = rows.saturating_sub(6) as usize; // Leave extra padding
        let max_visible_models = available_height.max(1);

        // Use single column layout with proper width
        let menu_width = std::cmp::min(cols.saturating_sub(8), 60); // Good width for model names
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
            format!("🔍 Search: {}", search_query)
        };

        stdout().queue(MoveTo(start_x + 2, search_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(search_text))?
              .queue(ResetColor)?;

        // Display models in viewport
        let max_text_width = menu_width.saturating_sub(6) as usize; // Leave space for prefix and padding

        for (idx, model) in models.iter().enumerate().skip(viewport_start).take(viewport_end - viewport_start) {
            let y = start_y + 3 + (idx - viewport_start) as u16;

            // Truncate long model names to fit
            let display_text = if model.len() > max_text_width {
                format!("{}...", &model[..max_text_width.saturating_sub(3)])
            } else {
                model.clone()
            };

            let prefix = if idx == selected_idx { "▶ " } else { "  " };
            let text = format!("{}{}", prefix, display_text);

            let color = if idx == selected_idx {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::PRIMARY_ANSI))
            } else {
                SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
            };

            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(color)?
                  .queue(Print(text))?
                  .queue(ResetColor)?;
        }

        // Show navigation hint
        let nav_y = start_y + final_menu_height as u16 - 1;
        let nav_text = if models.is_empty() {
            "No models found".to_string()
        } else if loading {
            "↑↓ Navigate • Ctrl+R: retry • Ctrl+C: clear cache • ESC: cancel".to_string()
        } else if viewport_start == 0 && viewport_end == total_models {
            // All models visible
            "↑↓ Navigate".to_string()
        } else {
            // Showing a subset - show position
            format!("↑↓ Navigate ({}-{} of {})",
                    viewport_start + 1, viewport_end, total_models)
        };
        stdout().queue(MoveTo(start_x + 2, nav_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(nav_text))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn show_text_input(&mut self, prompt: &str, default: &str) -> Result<Option<String>> {
        let mut input = default.to_string();
        let mut cursor_pos = input.len();

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

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 60.min(cols.saturating_sub(4));
        let menu_height = 6u16;
        let start_x = cols.saturating_sub(menu_width) / 2;
        let start_y = rows.saturating_sub(menu_height) / 2;

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, prompt)?;

        // Draw input field
        let input_y = start_y + 2;
        let input_text = if input.is_empty() {
            "← Type here..."
        } else {
            input
        };

        // Draw input text with appropriate colors
        if input.is_empty() {
            stdout().queue(MoveTo(start_x + 2, input_y))?
                  .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
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

        stdout().flush()?;
        Ok(())
    }

    fn show_session_info(&mut self, app: &App) -> Result<()> {
        // Clear any pending events in the buffer
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        loop {
            self.render_session_info(app)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
                            break;
                        }
                        // Ignore all other keys
                        continue;
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }
        Ok(())
    }

    fn render_session_info(&self, app: &App) -> Result<()> {
        let (cols, rows) = size()?;

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 50.min(cols - 4);
        let menu_height = 10u16;
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "📊 Session Information")?;

        let config = app.get_config();

        // Calculate max width for text (menu_width - 4 for padding)
        let max_text_width = menu_width.saturating_sub(4) as usize;

        let info_lines = vec![
            format!("🤖 Provider: {}", Self::truncate_text(&config.ai.provider, max_text_width.saturating_sub(12))),
            format!("🧠 Model: {}", Self::truncate_text(&config.ai.model, max_text_width.saturating_sub(10))),
            format!("🌐 API URL: {}", Self::truncate_text(&config.ai.api_url, max_text_width.saturating_sub(12))),
            format!("💬 Messages: {}", app.messages.len()),
        ];

        for (i, line) in info_lines.iter().enumerate() {
            let y = start_y + 2 + i as u16;
            stdout().queue(MoveTo(start_x + 2, y))?
                  .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                  .queue(Print(line))?
                  .queue(ResetColor)?;
        }

        // Instructions with better styling
        let instruction_y = start_y + menu_height - 2;
        stdout().queue(MoveTo(start_x + 2, instruction_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print("Press Enter to continue..."))?
              .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }

    fn show_help(&mut self) -> Result<()> {
        // Clear any pending events in the buffer
        while event::poll(Duration::from_millis(0))? {
            let _ = event::read()?;
        }

        loop {
            self.render_help()?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        // Only handle key press events to avoid double-processing on Windows
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        if matches!(key_event.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q')) {
                            break;
                        }
                        // Ignore all other keys
                        continue;
                    }
                    _ => {
                        // Ignore all other event types
                        continue;
                    }
                }
            }
        }
        Ok(())
    }

    fn render_help(&self) -> Result<()> {
        let (cols, rows) = size()?;

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 70.min(cols - 4);
        let menu_height = 20u16;
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "❓ ARULA Help")?;

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
        ];

        for (i, line) in help_lines.iter().enumerate() {
            if i < help_lines.len() {
                let y = start_y + 2 + i as u16;
                // Use different colors for different sections
                let color = if line.starts_with("🔧") || line.starts_with("⌨️") || line.starts_with("💡") {
                    SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI))
                } else if line.starts_with("  •") {
                    SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
                } else {
                    SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI))
                };

                stdout().queue(MoveTo(start_x + 2, y))?
                      .queue(color)?
                      .queue(Print(*line))?
                      .queue(ResetColor)?;
            }
        }

        stdout().flush()?;
        Ok(())
    }

    fn show_confirm_dialog(&mut self, message: &str) -> Result<bool> {
        let mut selected = false; // false for No, true for Yes

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

        stdout().queue(terminal::Clear(terminal::ClearType::All))?;

        let menu_width = 50.min(cols - 8);
        let menu_height = 8u16;
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        // Draw modern box for confirmation
        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "CONFIRM")?;

        // Draw title
        let title_y = start_y + 1;
        let title = "?";
        stdout().queue(MoveTo(start_x + menu_width / 2 - 1, title_y))?
              .queue(Print(ColorTheme::primary().bold().apply_to(title)))?;

        // Message
        stdout().queue(MoveTo(start_x + 2, start_y + 3))?
              .queue(Print(helpers::tool_result().apply_to(message)))?;

        // Modern styled options
        let no_text = "NO";
        let yes_text = "YES";

        let options_y = start_y + 5;
        let no_x = start_x + menu_width / 2 - 10;
        let yes_x = start_x + menu_width / 2 + 2;

        // Draw NO option
        if !selected {
            // Unselected
            stdout().queue(MoveTo(no_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::DarkGrey))?
                  .queue(SetForegroundColor(crossterm::style::Color::White))?
                  .queue(Print(format!(" {} ", no_text)))?
                  .queue(ResetColor)?;
        } else {
            // Selected
            stdout().queue(MoveTo(no_x, options_y))?
                  .queue(SetBackgroundColor(crossterm::style::Color::Red))?
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

        stdout().flush()?;
        Ok(())
    }

    fn render_frame(&self, app: &App, _output: &OutputHandler) -> Result<()> {
        let (_cols, _rows) = size()?;

        stdout().queue(crossterm::terminal::Clear(crossterm::terminal::ClearType::All))?;

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

        let menu_width = 50.min(cols - 8);
        let menu_height = 12; // Fixed height for better layout
        let start_x = (cols - menu_width) / 2;
        let start_y = (rows - menu_height) / 2;

        // Draw modern box with gradient effect
        self.draw_modern_box(start_x, start_y, menu_width, menu_height, "ARULA")?;

        // Draw title with modern styling
        let title_y = start_y + 2;
        let title = "● MENU";
        let title_len = title.len() as u16;
        let title_x = if menu_width > title_len + 2 {
            start_x + menu_width / 2 - title_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(title))?
              .queue(ResetColor)?;

        // Draw menu items with modern styling
        let items_start_y = start_y + 4;
        for (i, option) in self.main_options.iter().enumerate() {
            let y = items_start_y + i as u16;

            if i == self.selected_index {
                // Selected item with modern highlight
                self.draw_selected_item(start_x + 2, y, menu_width - 4, option)?;
            } else {
                // Unselected item with subtle styling
                stdout().queue(MoveTo(start_x + 4, y))?
                      .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                      .queue(Print(option))?
                      .queue(ResetColor)?;
            }
        }

        // Draw modern help text
        let help_y = start_y + menu_height - 2;
        let help_text = "↑↓ Navigate • Enter Select • ESC Exit";
        let help_len = help_text.len() as u16;
        let help_x = if menu_width > help_len + 2 {
            start_x + menu_width / 2 - help_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(help_x, help_y))?
              .queue(Print(ColorTheme::dim().apply_to(help_text)))?;

        Ok(())
    }

    fn render_config_menu(&self, app: &App) -> Result<()> {
        let (cols, rows) = size()?;

        let config = app.get_config();
        let mut display_options = self.config_options.clone();

        let menu_width = 60.min(cols - 8);

        // Calculate max width for menu items (menu_width - 6 for padding and marker)
        let max_item_width = menu_width.saturating_sub(6) as usize;

        // Update display values with modern styling and overflow protection
        display_options[0] = format!("○ Provider: {}", Self::truncate_text(&config.ai.provider, max_item_width.saturating_sub(13)));
        display_options[1] = format!("○ Model: {}", Self::truncate_text(&config.ai.model, max_item_width.saturating_sub(11)));
        display_options[2] = format!("○ API URL: {}", Self::truncate_text(&config.ai.api_url, max_item_width.saturating_sub(13)));
        display_options[3] = format!(
            "○ API Key: {}",
            if config.ai.api_key.is_empty() {
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
        let title_y = start_y + 2;
        let title = "⚙️ SETTINGS";
        let title_len = title.len() as u16;
        let title_x = if menu_width > title_len + 2 {
            start_x + menu_width / 2 - title_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(title_x, title_y))?
              .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::AI_HIGHLIGHT_ANSI)))?
              .queue(Print(title))?
              .queue(ResetColor)?;

        // Draw config items with modern styling
        let items_start_y = start_y + 4;
        for (i, option) in display_options.iter().enumerate() {
            let y = items_start_y + i as u16;

            if i == self.selected_index {
                // Selected item with modern highlight
                self.draw_selected_item(start_x + 2, y, menu_width - 4, option)?;
            } else {
                // Unselected item with subtle styling
                stdout().queue(MoveTo(start_x + 4, y))?
                      .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(crate::colors::MISC_ANSI)))?
                      .queue(Print(option))?
                      .queue(ResetColor)?;
            }
        }

        // Draw modern help text
        let help_y = start_y + menu_height - 2;
        let help_text = "↑↓ Edit • Enter Select • ESC Exit";
        let help_len = help_text.len() as u16;
        let help_x = if menu_width > help_len + 2 {
            start_x + menu_width / 2 - help_len / 2
        } else {
            start_x + 1
        };
        stdout().queue(MoveTo(help_x, help_y))?
              .queue(Print(ColorTheme::dim().apply_to(help_text)))?;

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

        // Clear the area first with bounds checking
        for row in y..std::cmp::min(y + height, u16::MAX) {
            stdout().queue(MoveTo(x, row))?;
            for _col in x..std::cmp::min(x + width, u16::MAX) {
                stdout().queue(Print(" "))?;
            }
        }

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

    fn move_selection(&mut self, direction: isize) {
        let options = if self.is_in_config {
            &self.config_options
        } else {
            &self.main_options
        };

        let new_index = self.selected_index as isize + direction;
        self.selected_index = new_index.clamp(0, (options.len() - 1) as isize) as usize;
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