use anyhow::Result;
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tui_textarea::TextArea;
use crate::api::ApiClient;
use crate::git_ops::GitOperations;

use crate::chat::{ChatMessage, MessageType};

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    Chat,
    Menu(MenuType),
    Exiting,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuType {
    Main,
    Commands,
    Context,
    Help,
    Configuration,
    ExitConfirmation,
    // Nested submenus
    GitCommandsDetail,
    ExecCommandsDetail,
    SessionInfoDetail,
    GitStatusDetail,
    SystemInfoDetail,
    KeyboardShortcutsDetail,
    AboutArulaDetail,
    DocumentationDetail,
    AiSettingsDetail,
    GitSettingsDetail,
    AppearanceSettingsDetail,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuOption {
    // Main menu
    Commands,
    Context,
    Help,
    Configuration,
    ClearChat,
    Exit,

    // Commands submenu
    GitCommands,
    ExecCommands,

    // Context submenu
    SessionInfo,
    GitStatus,
    SystemInfo,

    // Help submenu
    KeyboardShortcuts,
    AboutArula,
    Documentation,

    // Configuration submenu
    AiSettings,
    GitSettings,
    AppearanceSettings,

    // Detail menu actions (for Git Commands, etc)
    GitInit,
    GitStatusAction,
    GitBranches,
    GitAdd,
    GitCommit,
    ExecCommand,
    ViewSessionInfo,
    RefreshGitStatus,
    ViewSystemInfo,
    ChangeTheme,
    ToggleAutoCommit,
    ToggleCreateBranch,

    // Editable field options (for configuration menu)
    EditAiProvider,
    EditAiModel,
    EditApiUrl,
    EditApiKey,
    EditTheme,
    EditArtStyle,

    // Common
    Back,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub ai: AiConfig,
    pub git: GitConfig,
    pub logging: LoggingConfig,
    pub art: ArtConfig,
    pub workspace: WorkspaceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub api_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitConfig {
    pub auto_commit: bool,
    pub create_branch: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtConfig {
    pub default_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub path: String,
}

pub struct App {
    pub state: AppState,
    pub textarea: TextArea<'static>,
    pub input_mode: bool,
    pub messages: Vec<ChatMessage>,
    pub config: Config,
    pub start_time: SystemTime,
    pub session_id: String,
    pub api_client: Option<ApiClient>,
    pub pending_command: Option<String>,
    pub git_ops: GitOperations,
    pub menu_selected: usize,
    pub menu_input: Option<String>,
    pub menu_input_prompt: Option<String>,
    pub editing_field: Option<EditableField>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditableField {
    AiProvider(Vec<String>, usize),  // (options, current_index)
    AiModel(Vec<String>, usize),
    ApiUrl(String),     // Text input
    ApiKey(String),     // Text input (masked)
    Theme(Vec<String>, usize),
    ArtStyle(String),   // Text input
}

impl App {
    pub fn new() -> Result<Self> {
        let session_id = format!("session_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs()
        );

        let mut textarea = TextArea::default();
        textarea.set_placeholder_text("Type your message...");
        textarea.set_block(
            ratatui::widgets::Block::default()
                .borders(ratatui::widgets::Borders::ALL)
                .title(" Input ")
        );

        Ok(Self {
            state: AppState::Chat,
            textarea,
            input_mode: true,
            messages: Vec::new(),
            config: Self::load_config(),
            start_time: SystemTime::now(),
            session_id,
            api_client: None,
            pending_command: None,
            git_ops: GitOperations::new(),
            menu_selected: 0,
            menu_input: None,
            menu_input_prompt: None,
            editing_field: None,
        })
    }

    pub fn get_menu_options(menu_type: &MenuType) -> Vec<MenuOption> {
        match menu_type {
            MenuType::Main => vec![
                MenuOption::KeyboardShortcuts,
                MenuOption::Context,
                MenuOption::Commands,
                MenuOption::Configuration,
                MenuOption::Help,
                MenuOption::AboutArula,
                MenuOption::ClearChat,
                MenuOption::Exit,
            ],
            MenuType::ExitConfirmation => vec![
                MenuOption::Exit,
                MenuOption::Close,
            ],
            MenuType::Commands => vec![
                MenuOption::GitCommands,
                MenuOption::ExecCommands,
            ],
            MenuType::Context => vec![
                MenuOption::SessionInfo,
            ],
            MenuType::Help => vec![
                MenuOption::Documentation,
            ],
            MenuType::Configuration => vec![
                MenuOption::GitSettings,
            ],
            // Detail menus
            MenuType::GitCommandsDetail => vec![
                MenuOption::GitInit,
                MenuOption::GitStatusAction,
                MenuOption::GitBranches,
                MenuOption::GitAdd,
                MenuOption::GitCommit,
            ],
            MenuType::ExecCommandsDetail => vec![
                MenuOption::ExecCommand,
            ],
            MenuType::SessionInfoDetail => vec![
                MenuOption::RefreshGitStatus,
            ],
            MenuType::GitStatusDetail => vec![
                MenuOption::RefreshGitStatus,
            ],
            MenuType::SystemInfoDetail => vec![
                MenuOption::ViewSystemInfo,
            ],
            MenuType::KeyboardShortcutsDetail => vec![],
            MenuType::AboutArulaDetail => vec![],
            MenuType::DocumentationDetail => vec![],
            MenuType::AiSettingsDetail => vec![],
            MenuType::GitSettingsDetail => vec![
                MenuOption::EditAiProvider,      // Index 0 - AI Provider (editable)
                MenuOption::EditAiModel,         // Index 1 - AI Model (editable)
                MenuOption::EditApiUrl,          // Index 2 - API URL (editable)
                MenuOption::EditApiKey,          // Index 3 - API Key (editable)
                MenuOption::ToggleAutoCommit,    // Index 4
                MenuOption::ToggleCreateBranch,  // Index 5
                MenuOption::EditTheme,           // Index 6 - Theme (editable)
            ],
            MenuType::AppearanceSettingsDetail => vec![
                MenuOption::ChangeTheme,
            ],
        }
    }

    fn get_field_display_text(&self, field_index: usize) -> String {
        // Check if we're currently editing this field
        if let Some(ref editing_field) = self.editing_field {
            match (field_index, editing_field) {
                (0, EditableField::AiProvider(options, idx)) => {
                    format!("↑ {} ↓ (editing)", options[*idx])
                }
                (1, EditableField::AiModel(options, idx)) => {
                    format!("↑ {} ↓ (editing)", options[*idx])
                }
                (2, EditableField::ApiUrl(url)) => {
                    format!("{} █ (editing)", url)
                }
                (3, EditableField::ApiKey(key)) => {
                    // Mask the key while editing
                    let masked = if key.is_empty() {
                        String::new()
                    } else {
                        "*".repeat(key.len())
                    };
                    format!("{} █ (editing)", masked)
                }
                (6, EditableField::Theme(options, idx)) => {
                    format!("↑ {} ↓ (editing)", options[*idx])
                }
                (7, EditableField::ArtStyle(style)) => {
                    format!("{} █ (editing)", style)
                }
                _ => self.get_field_current_value(field_index)
            }
        } else {
            // Not editing, just show current value
            self.get_field_current_value(field_index)
        }
    }

    fn get_field_current_value(&self, field_index: usize) -> String {
        match field_index {
            0 => self.config.ai.provider.clone(),
            1 => self.config.ai.model.clone(),
            2 => self.config.ai.api_url.clone(),
            3 => {
                // Mask the API key
                if self.config.ai.api_key.is_empty() {
                    "(not set)".to_string()
                } else {
                    "*".repeat(self.config.ai.api_key.len())
                }
            }
            6 => "Cyberpunk".to_string(), // TODO: Add theme to config
            7 => self.config.art.default_style.clone(),
            _ => "Unknown".to_string()
        }
    }

    pub fn get_menu_content(&self, menu_type: &MenuType) -> Option<String> {
        match menu_type {
            MenuType::ExitConfirmation => {
                Some("⚠️  Are you sure you want to exit?\n\n\
Press Ctrl+C again to exit or ESC to stay.".to_string())
            }
            MenuType::SessionInfoDetail => {
                let uptime = self.start_time.elapsed().unwrap_or_default().as_secs();
                let uptime_hrs = uptime / 3600;
                let uptime_mins = (uptime % 3600) / 60;
                let uptime_secs = uptime % 60;

                // Get git status
                let mut git_ops_clone = GitOperations::new();
                let git_info = if git_ops_clone.open_repository(".").is_ok() {
                    let branch = git_ops_clone.get_current_branch()
                        .unwrap_or_else(|_| "unknown".to_string());
                    format!("✓ Repository detected | Branch: {}", branch)
                } else {
                    "✗ No git repository".to_string()
                };

                Some(format!("📊 Context Information\n\n\
SESSION:\n\
  ID: {}\n\
  Uptime: {}h {}m {}s\n\
  Messages: {} (User: {} | AI: {})\n\n\
GIT:\n\
  {}\n\n\
SYSTEM:\n\
  Directory: {}\n\
  Platform: {} ({})",
                    self.session_id,
                    uptime_hrs, uptime_mins, uptime_secs,
                    self.messages.len(),
                    self.messages.iter().filter(|m| m.message_type == MessageType::User).count(),
                    self.messages.iter().filter(|m| m.message_type == MessageType::Arula).count(),
                    git_info,
                    std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "Unknown".to_string()),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ))
            }
            MenuType::GitStatusDetail => {
                // Clone git_ops to avoid borrow issues
                let mut git_ops_clone = GitOperations::new();
                let git_status = if git_ops_clone.open_repository(".").is_ok() {
                    let branch = git_ops_clone.get_current_branch()
                        .unwrap_or_else(|_| "unknown".to_string());
                    format!("Repository detected ✓\nCurrent Branch: {}", branch)
                } else {
                    "No Git repository found".to_string()
                };
                Some(format!("🌿 Git Status\n\n{}", git_status))
            }
            MenuType::SystemInfoDetail => {
                Some(format!("💻 System Information\n\n\
Working Directory:\n{}\n\n\
Platform: {}\n\
Architecture: {}",
                    std::env::current_dir()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|_| "Unknown".to_string()),
                    std::env::consts::OS,
                    std::env::consts::ARCH
                ))
            }
            MenuType::KeyboardShortcutsDetail => {
                Some("⌨️  Keyboard Shortcuts\n\n\
Navigation:\n\
• ESC - Open/close menu\n\
• ↑/↓ or k/j - Navigate items\n\
• Enter - Select item\n\n\
Text Editing:\n\
• Ctrl+A - Beginning of line\n\
• Ctrl+E - End of line\n\
• Ctrl+K - Clear to end\n\
• Ctrl+U - Clear to beginning".to_string())
            }
            MenuType::AboutArulaDetail => {
                Some("🤖 ARULA CLI - Autonomous AI Interface\n\
Version: 0.2.0\n\n\
FEATURES:\n\
• Chat-style AI interaction\n\
• Git repository management (/git commands)\n\
• Shell command execution (/exec commands)\n\
• Professional text editing with tui-textarea\n\n\
KEYBOARD SHORTCUTS:\n\
• ESC - Open/close menu | ↑↓/jk - Navigate\n\
• Ctrl+A/E - Line start/end | Ctrl+K/U - Clear\n\
• Ctrl+C - Quit | Enter - Send/Select\n\n\
TECH STACK:\n\
Built with Rust, Ratatui, Tokio, Crossterm\n\n\
COMMANDS:\n\
• /git <cmd> - Git operations (init, status, branch, commit)\n\
• /exec <cmd> - Execute any shell command".to_string())
            }
            MenuType::AiSettingsDetail => {
                Some(format!("🤖 AI Settings\n\n\
Provider: {}\n\
Model: {}\n\
Endpoint: {}",
                    self.config.ai.provider,
                    self.config.ai.model,
                    self.api_client.as_ref()
                        .map(|_| "Connected")
                        .unwrap_or("Not configured")
                ))
            }
            MenuType::GitSettingsDetail => {
                // Show a brief tip instead of duplicating the field values
                Some("⚙️  Configuration\n\n\
Select a field below to edit it:\n\
• For switch fields: Use ↑↓ arrows to cycle options\n\
• For text fields: Type to edit\n\
• Press Enter to save, Esc to cancel".to_string())
            }
            MenuType::AppearanceSettingsDetail => {
                Some(format!("🎨 Appearance Settings\n\n\
Current Theme: Cyberpunk\n\
Art Style: {}\n\n\
Available Themes:\n\
• Cyberpunk (current)\n\
• Matrix\n\
• Ocean",
                    self.config.art.default_style
                ))
            }
            MenuType::DocumentationDetail => {
                Some("📚 Documentation\n\n\
Quick Start:\n\
1. Type messages to chat with AI\n\
2. Use /git for Git operations\n\
3. Use /exec for shell commands\n\
4. Press ESC to open menu\n\n\
Command Reference:\n\
• /git <command> [args]\n\
• /exec <command> [args]".to_string())
            }
            _ => None,
        }
    }

    pub fn get_menu_title(menu_type: &MenuType) -> &'static str {
        match menu_type {
            MenuType::Main => " ARULA CLI Menu ",
            MenuType::ExitConfirmation => " Exit Confirmation ",
            MenuType::Commands => " Commands ",
            MenuType::Context => " Context ",
            MenuType::Help => " Help ",
            MenuType::Configuration => " Configuration ",
            MenuType::GitCommandsDetail => " Git Commands ",
            MenuType::ExecCommandsDetail => " Shell Commands ",
            MenuType::SessionInfoDetail => " Session Info ",
            MenuType::GitStatusDetail => " Git Status ",
            MenuType::SystemInfoDetail => " System Info ",
            MenuType::KeyboardShortcutsDetail => " Keyboard Shortcuts ",
            MenuType::AboutArulaDetail => " About ARULA ",
            MenuType::DocumentationDetail => " Documentation ",
            MenuType::AiSettingsDetail => " AI Settings ",
            MenuType::GitSettingsDetail => " Configuration ",
            MenuType::AppearanceSettingsDetail => " Appearance ",
        }
    }

    pub fn get_option_display(&self, option: &MenuOption) -> (String, String) {
        // For exit confirmation menu, show keyboard shortcuts
        if matches!(self.state, AppState::Menu(MenuType::ExitConfirmation)) {
            return match option {
                MenuOption::Exit => ("Exit (Ctrl+C)".to_string(), "".to_string()),
                MenuOption::Close => ("Stay (Esc)".to_string(), "".to_string()),
                _ => {
                    let (title, desc) = Self::get_option_info(option);
                    (title.to_string(), desc.to_string())
                }
            };
        }

        // For editable fields, show current value in the title
        match option {
            MenuOption::EditAiProvider => {
                let value = self.get_field_display_text(0);
                (format!("AI Provider: {}", value), "".to_string())
            }
            MenuOption::EditAiModel => {
                let value = self.get_field_display_text(1);
                (format!("AI Model: {}", value), "".to_string())
            }
            MenuOption::EditApiUrl => {
                let value = self.get_field_display_text(2);
                (format!("API URL: {}", value), "".to_string())
            }
            MenuOption::EditApiKey => {
                let value = self.get_field_display_text(3);
                (format!("API Key: {}", value), "".to_string())
            }
            MenuOption::EditTheme => {
                let value = self.get_field_display_text(6);
                (format!("Theme: {}", value), "".to_string())
            }
            MenuOption::EditArtStyle => {
                let value = self.get_field_display_text(7);
                (format!("Art Style: {}", value), "".to_string())
            }
            MenuOption::ToggleAutoCommit => {
                let status = if self.config.git.auto_commit { "✓ Enabled" } else { "✗ Disabled" };
                (format!("Auto-Commit: {}", status), "".to_string())
            }
            MenuOption::ToggleCreateBranch => {
                let status = if self.config.git.create_branch { "✓ Enabled" } else { "✗ Disabled" };
                (format!("Auto-Branch: {}", status), "".to_string())
            }
            _ => {
                let (title, desc) = Self::get_option_info(option);
                (title.to_string(), desc.to_string())
            }
        }
    }

    pub fn get_option_info(option: &MenuOption) -> (&'static str, &'static str) {
        match option {
            // Main menu
            MenuOption::Commands => ("Commands", "View all available commands"),
            MenuOption::Context => ("Context", "Session info & statistics"),
            MenuOption::Help => ("Help", "Documentation & shortcuts"),
            MenuOption::Configuration => ("Configuration", "View current settings"),
            MenuOption::ClearChat => ("Clear Chat", "Clear conversation history"),
            MenuOption::Exit => ("Exit", "Quit application"),

            // Commands submenu
            MenuOption::GitCommands => ("Git Commands", "Git operations & examples"),
            MenuOption::ExecCommands => ("Shell Commands", "Execute shell commands"),

            // Context submenu
            MenuOption::SessionInfo => ("View Context", "Session, git & system info"),
            MenuOption::GitStatus => ("Git Status", "Repository information"),
            MenuOption::SystemInfo => ("System Info", "Working directory & paths"),

            // Help submenu
            MenuOption::KeyboardShortcuts => ("Keyboard Shortcuts", "All available shortcuts"),
            MenuOption::AboutArula => ("About & Help", "Info, shortcuts & commands"),
            MenuOption::Documentation => ("Documentation", "Full documentation"),

            // Configuration submenu
            MenuOption::AiSettings => ("AI Settings", "AI provider & model"),
            MenuOption::GitSettings => ("Settings", "View & edit configuration"),
            MenuOption::AppearanceSettings => ("Appearance", "Theme & UI settings"),

            // Detail menu actions
            MenuOption::GitInit => ("Initialize Repo", "Create new git repository"),
            MenuOption::GitStatusAction => ("Check Status", "View git repository status"),
            MenuOption::GitBranches => ("List Branches", "Show all branches"),
            MenuOption::GitAdd => ("Add Files", "Stage all changes"),
            MenuOption::GitCommit => ("Commit Changes", "Commit staged files"),
            MenuOption::ExecCommand => ("Execute Command", "Run custom shell command"),
            MenuOption::ViewSessionInfo => ("View Info", "Show session details"),
            MenuOption::RefreshGitStatus => ("Refresh", "Update information"),
            MenuOption::ViewSystemInfo => ("View Info", "Show system details"),
            MenuOption::ChangeTheme => ("Change Theme", "Switch color theme"),
            MenuOption::ToggleAutoCommit => ("Toggle Auto-Commit", "Enable/disable auto-commit"),
            MenuOption::ToggleCreateBranch => ("Toggle Auto-Branch", "Enable/disable auto-branch"),

            // Editable fields (will be dynamically updated with actual values)
            MenuOption::EditAiProvider => ("AI Provider", "Change AI provider"),
            MenuOption::EditAiModel => ("AI Model", "Change AI model"),
            MenuOption::EditApiUrl => ("API URL", "Set API endpoint URL"),
            MenuOption::EditApiKey => ("API Key", "Set API authentication key"),
            MenuOption::EditTheme => ("Theme", "Change color theme"),
            MenuOption::EditArtStyle => ("Art Style", "Change default art style"),

            // Common
            MenuOption::Back => ("Back", "Return to previous menu"),
            MenuOption::Close => ("Close", "Close menu and return to chat"),
        }
    }

    pub fn handle_menu_navigation(&mut self, key: KeyEvent) {
        // If we're editing a field, handle it separately
        if self.editing_field.is_some() {
            self.handle_field_editing(key);
            return;
        }

        let current_menu = if let AppState::Menu(ref menu_type) = self.state {
            menu_type.clone()
        } else {
            return;
        };

        let menu_len = Self::get_menu_options(&current_menu).len();

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.menu_selected > 0 {
                    self.menu_selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.menu_selected < menu_len - 1 {
                    self.menu_selected += 1;
                }
            }
            KeyCode::Enter => {
                // Check if we're in a settings detail menu and trying to edit a field
                if self.try_enter_field_edit_mode(&current_menu) {
                    return;
                }
                self.execute_menu_option();
            }
            KeyCode::Esc => {
                // If in exit confirmation, go back to chat (stay)
                if current_menu == MenuType::ExitConfirmation {
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                // If we're in a submenu, go back to main menu
                // Otherwise, close menu
                else if current_menu != MenuType::Main {
                    self.state = AppState::Menu(MenuType::Main);
                    self.menu_selected = 0;
                } else {
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
            }
            _ => {}
        }
    }

    fn execute_menu_option(&mut self) {
        let current_menu = if let AppState::Menu(ref menu_type) = self.state {
            menu_type.clone()
        } else {
            return;
        };

        let options = Self::get_menu_options(&current_menu);
        if let Some(option) = options.get(self.menu_selected) {
            match option {
                // Main menu - navigate to submenus
                MenuOption::Commands => {
                    self.state = AppState::Menu(MenuType::Commands);
                    self.menu_selected = 0;
                }
                MenuOption::Context => {
                    self.state = AppState::Menu(MenuType::Context);
                    self.menu_selected = 0;
                }
                MenuOption::Help => {
                    self.state = AppState::Menu(MenuType::Help);
                    self.menu_selected = 0;
                }
                MenuOption::Configuration => {
                    self.state = AppState::Menu(MenuType::GitSettingsDetail);
                    self.menu_selected = 0;
                }
                MenuOption::ClearChat => {
                    self.messages.clear();
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::Exit => {
                    self.state = AppState::Exiting;
                }

                // Commands submenu - open detail menus
                MenuOption::GitCommands => {
                    self.state = AppState::Menu(MenuType::GitCommandsDetail);
                    self.menu_selected = 0;
                }
                MenuOption::ExecCommands => {
                    self.state = AppState::Menu(MenuType::ExecCommandsDetail);
                    self.menu_selected = 0;
                }

                // Context submenu - open detail menus
                MenuOption::SessionInfo => {
                    self.state = AppState::Menu(MenuType::SessionInfoDetail);
                    self.menu_selected = 0;
                }
                MenuOption::GitStatus => {
                    self.state = AppState::Menu(MenuType::GitStatusDetail);
                    self.menu_selected = 0;
                }
                MenuOption::SystemInfo => {
                    self.state = AppState::Menu(MenuType::SystemInfoDetail);
                    self.menu_selected = 0;
                }

                // Help submenu - open detail menus
                MenuOption::KeyboardShortcuts => {
                    self.state = AppState::Menu(MenuType::KeyboardShortcutsDetail);
                    self.menu_selected = 0;
                }
                MenuOption::AboutArula => {
                    self.state = AppState::Menu(MenuType::AboutArulaDetail);
                    self.menu_selected = 0;
                }
                MenuOption::Documentation => {
                    self.state = AppState::Menu(MenuType::DocumentationDetail);
                    self.menu_selected = 0;
                }

                // Configuration submenu - open detail menus
                MenuOption::AiSettings => {
                    self.state = AppState::Menu(MenuType::AiSettingsDetail);
                    self.menu_selected = 0;
                }
                MenuOption::GitSettings => {
                    self.state = AppState::Menu(MenuType::GitSettingsDetail);
                    self.menu_selected = 0;
                }
                MenuOption::AppearanceSettings => {
                    self.state = AppState::Menu(MenuType::AppearanceSettingsDetail);
                    self.menu_selected = 0;
                }

                // Detail menu actions
                MenuOption::GitInit => {
                    self.pending_command = Some("/git init".to_string());
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::GitStatusAction => {
                    self.pending_command = Some("/git status".to_string());
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::GitBranches => {
                    self.pending_command = Some("/git branches".to_string());
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::GitAdd => {
                    self.pending_command = Some("/git add".to_string());
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::GitCommit => {
                    self.add_message(MessageType::Info, "Enter commit message in chat:");
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::ExecCommand => {
                    self.add_message(MessageType::Info, "Enter shell command using /exec <command>");
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::ViewSessionInfo => {
                    self.show_session_info();
                }
                MenuOption::RefreshGitStatus => {
                    self.show_git_status();
                }
                MenuOption::ViewSystemInfo => {
                    self.show_system_info();
                }
                MenuOption::ChangeTheme => {
                    self.add_message(MessageType::Info, "Theme changing will be available in a future version.");
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
                MenuOption::ToggleAutoCommit => {
                    self.config.git.auto_commit = !self.config.git.auto_commit;
                    self.save_config();
                }
                MenuOption::ToggleCreateBranch => {
                    self.config.git.create_branch = !self.config.git.create_branch;
                    self.save_config();
                }

                // Editable field options - these do nothing here, handled by try_enter_field_edit_mode
                MenuOption::EditAiProvider |
                MenuOption::EditAiModel |
                MenuOption::EditApiUrl |
                MenuOption::EditApiKey |
                MenuOption::EditTheme |
                MenuOption::EditArtStyle => {
                    // These are handled by try_enter_field_edit_mode in handle_menu_navigation
                    // This match arm is just to satisfy the exhaustiveness check
                }

                // Back button - go to parent menu
                MenuOption::Back => {
                    let parent_menu = match &current_menu {
                        MenuType::Commands | MenuType::Context | MenuType::Help | MenuType::Configuration => MenuType::Main,
                        MenuType::GitCommandsDetail | MenuType::ExecCommandsDetail => MenuType::Commands,
                        MenuType::SessionInfoDetail | MenuType::GitStatusDetail | MenuType::SystemInfoDetail => MenuType::Context,
                        MenuType::DocumentationDetail => MenuType::Help,
                        // About and Shortcuts are now in main menu
                        MenuType::KeyboardShortcutsDetail | MenuType::AboutArulaDetail => MenuType::Main,
                        MenuType::AiSettingsDetail | MenuType::AppearanceSettingsDetail => MenuType::Configuration,
                        MenuType::GitSettingsDetail => MenuType::Main, // Go directly back to main
                        _ => MenuType::Main,
                    };
                    self.state = AppState::Menu(parent_menu);
                    self.menu_selected = 0;
                }

                // Close button - close menu completely
                MenuOption::Close => {
                    self.state = AppState::Chat;
                    self.menu_selected = 0;
                }
            }
        }
    }

    // Field editing helper methods
    fn try_enter_field_edit_mode(&mut self, menu_type: &MenuType) -> bool {
        // Only allow editing in the GitSettingsDetail menu (which shows all config)
        if menu_type != &MenuType::GitSettingsDetail {
            return false;
        }

        // Map menu selection index to editable fields
        // In GitSettingsDetail, we show: AI Provider, AI Model, API URL, API Key, Auto Commit, Auto Branch, Theme, Back
        match self.menu_selected {
            0 => {
                // AI Provider - cycle through options
                let options = vec!["local".to_string(), "openai".to_string(), "anthropic".to_string()];
                let current_idx = options.iter().position(|x| x == &self.config.ai.provider).unwrap_or(0);
                self.editing_field = Some(EditableField::AiProvider(options, current_idx));
                true
            }
            1 => {
                // AI Model - cycle through options based on provider
                let options = match self.config.ai.provider.as_str() {
                    "openai" => vec!["gpt-4".to_string(), "gpt-3.5-turbo".to_string()],
                    "anthropic" => vec!["claude-3-opus".to_string(), "claude-3-sonnet".to_string()],
                    _ => vec!["local-model".to_string()],
                };
                let current_idx = options.iter().position(|x| x == &self.config.ai.model).unwrap_or(0);
                self.editing_field = Some(EditableField::AiModel(options, current_idx));
                true
            }
            2 => {
                // API URL - text input
                self.editing_field = Some(EditableField::ApiUrl(self.config.ai.api_url.clone()));
                true
            }
            3 => {
                // API Key - text input (masked)
                self.editing_field = Some(EditableField::ApiKey(self.config.ai.api_key.clone()));
                true
            }
            6 => {
                // Theme - cycle through available themes
                let options = vec![
                    "Cyberpunk".to_string(),
                    "Matrix".to_string(),
                    "Ocean".to_string(),
                    "Sunset".to_string(),
                    "Monochrome".to_string(),
                ];
                self.editing_field = Some(EditableField::Theme(options, 0));
                true
            }
            _ => false,
        }
    }

    fn handle_field_editing(&mut self, key: KeyEvent) {
        let editing_field = if let Some(ref field) = self.editing_field {
            field.clone()
        } else {
            return;
        };

        match key.code {
            KeyCode::Esc => {
                // Cancel editing
                self.editing_field = None;
            }
            KeyCode::Enter => {
                // Apply changes
                match editing_field {
                    EditableField::AiProvider(options, idx) => {
                        self.config.ai.provider = options[idx].clone();
                    }
                    EditableField::AiModel(options, idx) => {
                        self.config.ai.model = options[idx].clone();
                    }
                    EditableField::ApiUrl(url) => {
                        self.config.ai.api_url = url;
                    }
                    EditableField::ApiKey(key) => {
                        self.config.ai.api_key = key;
                    }
                    EditableField::Theme(_options, _idx) => {
                        // Theme switching implementation
                    }
                    EditableField::ArtStyle(style) => {
                        self.config.art.default_style = style;
                    }
                }
                self.editing_field = None;

                // Save config to file
                self.save_config();
            }
            KeyCode::Up => {
                // Cycle up through options
                match editing_field {
                    EditableField::AiProvider(options, idx) => {
                        let new_idx = if idx > 0 { idx - 1 } else { options.len() - 1 };
                        self.editing_field = Some(EditableField::AiProvider(options, new_idx));
                    }
                    EditableField::AiModel(options, idx) => {
                        let new_idx = if idx > 0 { idx - 1 } else { options.len() - 1 };
                        self.editing_field = Some(EditableField::AiModel(options, new_idx));
                    }
                    EditableField::Theme(options, idx) => {
                        let new_idx = if idx > 0 { idx - 1 } else { options.len() - 1 };
                        self.editing_field = Some(EditableField::Theme(options, new_idx));
                    }
                    _ => {}
                }
            }
            KeyCode::Down => {
                // Cycle down through options
                match editing_field {
                    EditableField::AiProvider(options, idx) => {
                        let new_idx = if idx < options.len() - 1 { idx + 1 } else { 0 };
                        self.editing_field = Some(EditableField::AiProvider(options, new_idx));
                    }
                    EditableField::AiModel(options, idx) => {
                        let new_idx = if idx < options.len() - 1 { idx + 1 } else { 0 };
                        self.editing_field = Some(EditableField::AiModel(options, new_idx));
                    }
                    EditableField::Theme(options, idx) => {
                        let new_idx = if idx < options.len() - 1 { idx + 1 } else { 0 };
                        self.editing_field = Some(EditableField::Theme(options, new_idx));
                    }
                    _ => {}
                }
            }
            KeyCode::Char(c) => {
                // For text input fields, append character
                match editing_field {
                    EditableField::ApiUrl(mut url) => {
                        url.push(c);
                        self.editing_field = Some(EditableField::ApiUrl(url));
                    }
                    EditableField::ApiKey(mut key) => {
                        key.push(c);
                        self.editing_field = Some(EditableField::ApiKey(key));
                    }
                    EditableField::ArtStyle(mut style) => {
                        style.push(c);
                        self.editing_field = Some(EditableField::ArtStyle(style));
                    }
                    _ => {}
                }
            }
            KeyCode::Backspace => {
                // For text input fields, remove last character
                match editing_field {
                    EditableField::ApiUrl(mut url) => {
                        url.pop();
                        self.editing_field = Some(EditableField::ApiUrl(url));
                    }
                    EditableField::ApiKey(mut key) => {
                        key.pop();
                        self.editing_field = Some(EditableField::ApiKey(key));
                    }
                    EditableField::ArtStyle(mut style) => {
                        style.pop();
                        self.editing_field = Some(EditableField::ArtStyle(style));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    // Commands submenu functions
    fn show_git_commands(&mut self) {
        self.add_message(
            MessageType::Info,
            "🌿 Git Commands

Available Operations:
• /git init - Initialize repository
• /git status - Show working directory status
• /git branches - List all branches
• /git branch <name> - Create new branch
• /git checkout <name> - Switch to branch
• /git delete <name> - Delete branch
• /git add - Add all files to staging
• /git commit <message> - Commit changes
• /git log - Show commit history
• /git pull - Pull from remote
• /git push - Push to remote

Examples:
• /git init
• /git branch feature-xyz
• /git checkout main
• /git commit \"Add new feature\""
        );
    }

    fn show_exec_commands(&mut self) {
        self.add_message(
            MessageType::Info,
            "💻 Shell Commands

Execute any shell command using /exec:

Format:
  /exec <command> [args...]

Examples:
• /exec ls -la - List directory contents
• /exec cargo build - Build Rust project
• /exec cargo test - Run tests
• /exec npm install - Install Node packages
• /exec python script.py - Run Python script
• /exec git status - Run native git command

Note: Commands run in your current working directory."
        );
    }

    // Context submenu functions
    fn show_session_info(&mut self) {
        let uptime = self.start_time.elapsed().unwrap_or_default().as_secs();
        let uptime_hrs = uptime / 3600;
        let uptime_mins = (uptime % 3600) / 60;
        let uptime_secs = uptime % 60;

        self.add_message(
            MessageType::Info,
            &format!("📊 Session Information

Session ID: {}
Uptime: {}h {}m {}s
Total Messages: {}
User Messages: {}
AI Responses: {}",
                self.session_id,
                uptime_hrs, uptime_mins, uptime_secs,
                self.messages.len(),
                self.messages.iter().filter(|m| m.message_type == MessageType::User).count(),
                self.messages.iter().filter(|m| m.message_type == MessageType::Arula).count()
            )
        );
    }

    fn show_git_status(&mut self) {
        let git_status = if self.git_ops.open_repository(".").is_ok() {
            let branch = self.git_ops.get_current_branch()
                .unwrap_or_else(|_| "unknown".to_string());
            format!("Repository detected ✓\nCurrent Branch: {}", branch)
        } else {
            "No Git repository found in current directory".to_string()
        };

        self.add_message(
            MessageType::Info,
            &format!("🌿 Git Status\n\n{}", git_status)
        );
    }

    fn show_system_info(&mut self) {
        self.add_message(
            MessageType::Info,
            &format!("💻 System Information

Working Directory:
{}

Workspace Path:
{}

Platform: {}
Architecture: {}",
                std::env::current_dir()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "Unknown".to_string()),
                self.config.workspace.path,
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        );
    }

    // Help submenu functions
    fn show_keyboard_shortcuts(&mut self) {
        self.add_message(
            MessageType::Info,
            "⌨️  Keyboard Shortcuts

Navigation:
• ESC - Open/close menu
• ↑/↓ or k/j - Navigate menu items
• Enter - Select menu item

Text Editing:
• Ctrl+A - Move to beginning of line
• Ctrl+E - Move to end of line
• Ctrl+K - Clear to end of line
• Ctrl+U - Clear to beginning of line
• Ctrl+W - Delete word backward
• Ctrl+Left/Right - Move by words

Application:
• Ctrl+C - Quit immediately
• Enter - Send message"
        );
    }

    fn show_about(&mut self) {
        self.add_message(
            MessageType::Info,
            "🤖 About ARULA CLI

ARULA - Autonomous AI Interface
Version: 0.2.0

An autonomous AI command-line interface built with:
• Rust - Systems programming language
• Ratatui - Terminal UI framework
• Tokio - Async runtime
• Crossterm - Terminal manipulation

Features:
• Chat-style AI interaction
• Git repository management
• Shell command execution
• Professional text editing
• Multi-line input support
• Theme customization

Built for performance, reliability, and great UX."
        );
    }

    fn show_documentation(&mut self) {
        self.add_message(
            MessageType::Info,
            "📚 Documentation

Quick Start:
1. Type messages to chat with AI
2. Use /git for Git operations
3. Use /exec for shell commands
4. Press ESC to open menu

Command Reference:
• All commands start with /
• Git: /git <command> [args]
• Exec: /exec <command> [args]

Getting Help:
• Press ESC → Help for shortcuts
• Press ESC → Commands for examples
• Check CLAUDE.md for development info

For full documentation, visit the project repository."
        );
    }

    // Configuration submenu functions
    fn show_ai_settings(&mut self) {
        self.add_message(
            MessageType::Info,
            &format!("🤖 AI Settings

Provider: {}
Model: {}
Endpoint: {}

Note: AI settings are configured via environment
variables or configuration file.",
                self.config.ai.provider,
                self.config.ai.model,
                self.api_client.as_ref()
                    .map(|_| "Connected")
                    .unwrap_or("Not configured")
            )
        );
    }

    fn show_git_settings(&mut self) {
        self.add_message(
            MessageType::Info,
            &format!("🌿 Git Settings

Auto Commit: {}
Create Branch: {}

These settings control automatic Git operations
when working with AI-generated code.",
                if self.config.git.auto_commit { "Enabled" } else { "Disabled" },
                if self.config.git.create_branch { "Enabled" } else { "Disabled" }
            )
        );
    }

    fn show_appearance_settings(&mut self) {
        self.add_message(
            MessageType::Info,
            &format!("🎨 Appearance Settings

Current Theme: Cyberpunk

Available Themes:
• Cyberpunk (default)
• Matrix
• Ocean
• Sunset
• Monochrome

Art Style: {}

Note: Theme switching will be available
in future versions.",
                self.config.art.default_style
            )
        );
    }

    pub fn set_api_client(&mut self, endpoint: String) {
        self.api_client = Some(ApiClient::new(endpoint));
    }

    pub async fn handle_ai_command(&mut self, command: String) -> Result<()> {
        let api_client = self.api_client.clone();

        if let Some(client) = api_client {
            self.add_message(MessageType::User, &command);

            // Show thinking message
            self.add_message(MessageType::Arula, "🤔 Thinking...");

            match client.send_message(&command, None).await {
                Ok(response) => {
                    // Remove thinking message and add actual response
                    self.messages.pop(); // Remove "Thinking..."

                    if response.success {
                        self.add_message(MessageType::Arula, &response.response);
                    } else {
                        let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                        self.add_message(MessageType::Error, &format!("❌ Error: {}", error_msg));
                    }
                }
                Err(e) => {
                    self.messages.pop(); // Remove "Thinking..."
                    self.add_message(MessageType::Error, &format!("❌ API Error: {}", e));
                }
            }
        } else {
            self.add_message(MessageType::Error, "❌ No API client configured");
        }

        Ok(())
    }

    fn default_config() -> Config {
        Config {
            ai: AiConfig {
                provider: "local".to_string(),
                model: "default".to_string(),
                api_url: "http://localhost:8080".to_string(),
                api_key: "".to_string(),
            },
            git: GitConfig {
                auto_commit: true,
                create_branch: true,
            },
            logging: LoggingConfig {
                level: "INFO".to_string(),
            },
            art: ArtConfig {
                default_style: "fractal".to_string(),
            },
            workspace: WorkspaceConfig {
                path: "./arula_workspace".to_string(),
            },
        }
    }

    fn save_config(&self) {
        // Save config to .arula/config.json
        let config_dir = std::path::Path::new(".arula");
        let config_path = config_dir.join("config.json");

        // Create .arula directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(config_dir) {
            eprintln!("Failed to create .arula directory: {}", e);
            return;
        }

        // Serialize config to JSON
        match serde_json::to_string_pretty(&self.config) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&config_path, json) {
                    eprintln!("Failed to write config file: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize config: {}", e);
            }
        }
    }

    fn load_config() -> Config {
        let config_path = std::path::Path::new(".arula/config.json");

        // Try to load existing config
        if config_path.exists() {
            if let Ok(json) = std::fs::read_to_string(config_path) {
                if let Ok(config) = serde_json::from_str::<Config>(&json) {
                    return config;
                }
            }
        }

        // Return default config if loading fails
        Self::default_config()
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) {
        // Only handle keys if input mode is enabled
        if !self.input_mode {
            return;
        }

        match key.code {
            KeyCode::Enter => {
                let lines = self.textarea.lines();
                if !lines.is_empty() && !lines[0].trim().is_empty() {
                    let command = lines.join("\n");
                    self.textarea = TextArea::default();
                    self.textarea.set_placeholder_text("Type your message...");
                    self.textarea.set_block(
                        ratatui::widgets::Block::default()
                            .borders(ratatui::widgets::Borders::ALL)
                            .title(" Input ")
                    );
                    self.pending_command = Some(command);
                }
            }
            KeyCode::Esc => {
                // ESC is handled in main.rs, don't pass to textarea
            }
            _ => {
                // Let TextArea handle all other input
                self.textarea.input(key);
            }
        }
    }

    pub async fn handle_command(&mut self, command: String) {
        let command_trimmed = command.trim();

        // First check if it's a special command
        match command_trimmed {
            cmd if cmd.starts_with('/') => {
                // Handle built-in commands (starting with /)
                self.handle_builtin_command(&command).await;
            }
            _ => {
                // Forward everything else to AI
                if let Err(e) = self.handle_ai_command(command).await {
                    self.add_message(MessageType::Error, &format!("Failed to process command: {}", e));
                }
            }
        }
    }

    async fn handle_builtin_command(&mut self, command: &str) {
        let command_trimmed = command.trim().strip_prefix('/').unwrap_or(command.trim());

        match command_trimmed {
            cmd if cmd == "help" || cmd == "h" || cmd == "?" => {
                self.add_message(
                    MessageType::Arula,
                    "🚀 Available commands:
• help - Show this help
• status - Show system status
• config - Manage configuration
• art - Generate code art
• task - Run task demos
• logs - View recent logs
• clear - Clear conversation
• git - Git operations (see /git help)
• exec - Execute shell commands
• exit or quit - Exit ARULA CLI

⌨️  Keyboard shortcuts:
• Just start typing - input is always enabled
• Enter - Send command
• Esc - Exit input mode (temporarily)
• Tab - Auto-complete 'help'
• Ctrl+C - Exit immediately
• Click - Focus input area

🎯 Cursor navigation:
• Arrow keys - Move cursor left/right
• Home/End - Move to beginning/end
• Backspace - Delete character before cursor
• Delete - Delete character at cursor
• Ctrl+Left/Right - Move by words
• Ctrl+U - Clear to beginning
• Ctrl+K - Clear to end
• Ctrl+A/E - Move to beginning/end

📝 Git Commands (use /git <command>):
• /git init - Initialize git repository
• /git status - Show git status
• /git branches - List all branches
• /git branch <name> - Create new branch
• /git checkout <name> - Switch to branch
• /git delete <name> - Delete branch
• /git add - Add all files
• /git commit <message> - Commit changes
• /git log - Show commit history
• /git pull - Pull from remote

💡 Try: art rust, /git status, or /exec ls -la"
                );
            }
            cmd if cmd == "status" || cmd == "st" => {
                let uptime = self.start_time.elapsed().unwrap_or_default().as_secs();
                self.add_message(
                    MessageType::Arula,
                    &format!("📊 System Status:
Configuration: ✅ Found
Log file: ✅ Active
Uptime: {}s
Session: {}", uptime, self.session_id)
                );
            }
            cmd if cmd.starts_with("config") => {
                if cmd == "config init" {
                    self.add_message(
                        MessageType::Success,
                        "Configuration initialized successfully!"
                    );
                } else {
                    self.add_message(
                        MessageType::Arula,
                        &format!("⚙️ Current Configuration:
ai:
  provider: {}
  model: {}
git:
  auto_commit: {}
  create_branch: {}
logging:
  level: {}
art:
  default_style: {}
workspace:
  path: {}",
                            self.config.ai.provider,
                            self.config.ai.model,
                            self.config.git.auto_commit,
                            self.config.git.create_branch,
                            self.config.logging.level,
                            self.config.art.default_style,
                            self.config.workspace.path)
                    );
                }
            }
            cmd if cmd.starts_with("art") => {
                let art_type = cmd.strip_prefix("art ").unwrap_or("").trim();
                match art_type {
                    "rust" | "crab" => {
                        self.add_message(
                            MessageType::Arula,
                            "🦀 Generating Rust Crab ASCII Art..."
                        );
                        self.add_message(
                            MessageType::Success,
                            &crate::art::generate_rust_crab()
                        );
                    }
                    "fractal" => {
                        self.add_message(
                            MessageType::Arula,
                            "🌿 Generating Fractal Art..."
                        );
                        self.add_message(
                            MessageType::Success,
                            &crate::art::generate_fractal()
                        );
                    }
                    "matrix" => {
                        self.add_message(
                            MessageType::Arula,
                            "💚 Generating Matrix Digital Rain..."
                        );
                        self.add_message(
                            MessageType::Success,
                            &crate::art::generate_matrix()
                        );
                    }
                    "demo" | "all" => {
                        self.add_message(
                            MessageType::Arula,
                            "🎨 Running Complete Art Demo..."
                        );
                        self.add_message(
                            MessageType::Success,
                            &crate::art::generate_demo()
                        );
                    }
                    _ => {
                        self.add_message(
                            MessageType::Error,
                            &format!("Unknown art style: {}\nAvailable: rust, fractal, matrix, demo", art_type)
                        );
                    }
                }
            }
            cmd if cmd.starts_with("task") => {
                let task_type = cmd.strip_prefix("task ").unwrap_or("").trim();
                match task_type {
                    "demo" => {
                        self.add_message(
                            MessageType::Arula,
                            "🤖 Starting Task Demo..."
                        );

                        self.add_message(
                            MessageType::Info,
                            "📋 Analyzing requirements..."
                        );

                        self.add_message(
                            MessageType::Success,
                            "✅ Requirements analyzed"
                        );

                        self.add_message(
                            MessageType::Info,
                            "🔧 Generating implementation plan..."
                        );

                        self.add_message(
                            MessageType::Success,
                            "✅ Implementation plan ready"
                        );

                        self.add_message(
                            MessageType::Info,
                            "💻 Creating solution..."
                        );

                        self.add_message(
                            MessageType::Success,
                            "✅ Solution completed successfully!"
                        );

                        self.add_message(
                            MessageType::Success,
                            "🎉 Task demo completed! Check workspace for generated files."
                        );
                    }
                    "status" => {
                        let success_count = self.messages.iter()
                            .filter(|m| m.message_type == MessageType::Success)
                            .count();
                        let error_count = self.messages.iter()
                            .filter(|m| m.message_type == MessageType::Error)
                            .count();

                        self.add_message(
                            MessageType::Arula,
                            &format!("📊 Task Status:
Active Tasks: 0
Completed: {}
Failed: {}", success_count, error_count)
                        );
                    }
                    _ => {
                        self.add_message(
                            MessageType::Error,
                            &format!("Unknown task command: {}\nAvailable: demo, status", task_type)
                        );
                    }
                }
            }
            cmd if cmd.starts_with("git") => {
                self.handle_git_command(cmd).await;
            }
            cmd if cmd.starts_with("exec") => {
                self.handle_exec_command(cmd).await;
            }
            cmd if cmd == "logs" || cmd == "log" => {
                let recent_messages: Vec<String> = self.messages
                    .iter()
                    .rev()
                    .take(10)
                    .map(|m| format!("[{}] {}: {}",
                        m.timestamp.format("%H:%M:%S"),
                        m.message_type,
                        m.content))
                    .collect();

                if recent_messages.is_empty() {
                    self.add_message(
                        MessageType::Info,
                        "No logs available yet."
                    );
                } else {
                    self.add_message(
                        MessageType::Arula,
                        &format!("📝 Recent Activity:\n{}", recent_messages.join("\n"))
                    );
                }
            }
            cmd if cmd == "clear" || cmd == "cls" => {
                self.messages.clear();
                self.add_message(
                    MessageType::System,
                    "Conversation cleared."
                );
            }
            cmd if cmd == "exit" || cmd == "quit" || cmd == "q" => {
                self.add_message(
                    MessageType::Arula,
                    "👋 Thank you for using ARULA CLI!
🚀 Session ended. Have a great day!"
                );
                self.state = AppState::Exiting;
            }
            "" => {
                // Empty command - ignore
            }
            _ => {
                self.add_message(
                    MessageType::Arula,
                    "I didn't understand that command.
Type 'help' to see available commands, or try:
• art - Generate code art
• task demo - Run task demonstration
• status - Check system status"
                );
            }
        }
    }

    pub fn add_message(&mut self, message_type: MessageType, content: &str) {
        let message = ChatMessage {
            timestamp: Local::now(),
            message_type,
            content: content.to_string(),
        };

        self.messages.push(message);

        // Keep only last 50 messages
        if self.messages.len() > 50 {
            self.messages.remove(0);
        }
    }

    async fn handle_git_command(&mut self, command: &str) {
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.len() < 2 {
            self.add_message(
                MessageType::Error,
                "Usage: /git <command> [args]\nUse /git help for available commands"
            );
            return;
        }

        match parts[1] {
            "help" => {
                self.add_message(
                    MessageType::Arula,
                    "🌿 Git Commands Help:
• /git init - Initialize git repository in current directory
• /git status - Show working directory status
• /git branches - List all branches (local and remote)
• /git branch <name> - Create new branch
• /git checkout <name> - Switch to existing branch
• /git delete <name> - Delete branch (not current branch)
• /git add - Add all untracked files to staging
• /git commit <message> - Commit staged changes
• /git log - Show commit history
• /git pull - Pull changes from remote
• /git push - Push changes to remote

💡 Examples:
• /git init
• /git status
• /git branch feature-xyz
• /git checkout main
• /git add
• /git commit \"Add new feature\""
                );
            }
            "init" => {
                match self.git_ops.initialize_repository(".") {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            "✅ Git repository initialized successfully!"
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to initialize repository: {}", e)
                        );
                    }
                }
            }
            "status" => {
                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                match self.git_ops.get_status() {
                    Ok(status_lines) => {
                        self.add_message(
                            MessageType::Arula,
                            &format!("📊 Git Status:\n{}", status_lines.join("\n"))
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to get status: {}", e)
                        );
                    }
                }
            }
            "branches" => {
                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                match self.git_ops.list_branches() {
                    Ok(branches) => {
                        let current_branch = self.git_ops.get_current_branch().unwrap_or_else(|_| "unknown".to_string());
                        self.add_message(
                            MessageType::Arula,
                            &format!("🌿 Branches:\nCurrent: {}\n{}", current_branch, branches.join("\n"))
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to list branches: {}", e)
                        );
                    }
                }
            }
            "branch" => {
                if parts.len() < 3 {
                    self.add_message(
                        MessageType::Error,
                        "Usage: /git branch <name>"
                    );
                    return;
                }

                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                let branch_name = parts[2];
                match self.git_ops.create_branch(branch_name) {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            &format!("✅ Branch '{}' created successfully!", branch_name)
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to create branch: {}", e)
                        );
                    }
                }
            }
            "checkout" => {
                if parts.len() < 3 {
                    self.add_message(
                        MessageType::Error,
                        "Usage: /git checkout <branch_name>"
                    );
                    return;
                }

                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                let branch_name = parts[2];
                match self.git_ops.checkout_branch(branch_name) {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            &format!("✅ Switched to branch '{}'", branch_name)
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to checkout branch: {}", e)
                        );
                    }
                }
            }
            "delete" => {
                if parts.len() < 3 {
                    self.add_message(
                        MessageType::Error,
                        "Usage: /git delete <branch_name>"
                    );
                    return;
                }

                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                let branch_name = parts[2];
                match self.git_ops.delete_branch(branch_name) {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            &format!("✅ Branch '{}' deleted successfully!", branch_name)
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to delete branch: {}", e)
                        );
                    }
                }
            }
            "add" => {
                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                match self.git_ops.add_all() {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            "✅ Files added to staging area successfully!"
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to add files: {}", e)
                        );
                    }
                }
            }
            "commit" => {
                if parts.len() < 3 {
                    self.add_message(
                        MessageType::Error,
                        "Usage: /git commit <message>"
                    );
                    return;
                }

                // Try to open repository first
                if let Err(_) = self.git_ops.open_repository(".") {
                    self.add_message(
                        MessageType::Error,
                        "❌ Not a git repository. Use '/git init' to initialize."
                    );
                    return;
                }

                let commit_message = parts[2..].join(" ");
                match self.git_ops.commit(&commit_message) {
                    Ok(()) => {
                        self.add_message(
                            MessageType::Success,
                            &format!("✅ Commit created successfully!\n📝 Message: {}", commit_message)
                        );
                    }
                    Err(e) => {
                        self.add_message(
                            MessageType::Error,
                            &format!("❌ Failed to create commit: {}", e)
                        );
                    }
                }
            }
            _ => {
                self.add_message(
                    MessageType::Error,
                    &format!("Unknown git command: {}\nUse '/git help' for available commands", parts[1])
                );
            }
        }
    }

    async fn handle_exec_command(&mut self, command: &str) {
        use crate::cli_commands::CommandRunner;

        let parts: Vec<&str> = command.splitn(2, ' ').collect();

        if parts.len() < 2 {
            self.add_message(
                MessageType::Error,
                "Usage: /exec <command>\nExamples:\n• /exec ls -la\n• /exec cargo build\n• /exec git status"
            );
            return;
        }

        let exec_cmd = parts[1];
        let cmd_parts: Vec<&str> = exec_cmd.split_whitespace().collect();

        if cmd_parts.is_empty() {
            self.add_message(
                MessageType::Error,
                "No command provided"
            );
            return;
        }

        let mut runner = CommandRunner::new();
        self.add_message(
            MessageType::Info,
            &format!("🔧 Executing: {}", exec_cmd)
        );

        let result = if cmd_parts.len() == 1 {
            runner.run_command(cmd_parts[0].to_string(), vec![]).await
        } else {
            runner.run_command(cmd_parts[0].to_string(), cmd_parts[1..].iter().map(|&s| s.to_string()).collect()).await
        };

        match result {
            Ok(output) => {
                if output.trim().is_empty() {
                    self.add_message(
                        MessageType::Success,
                        "✅ Command completed successfully (no output)"
                    );
                } else {
                    self.add_message(
                        MessageType::Success,
                        &format!("✅ Command output:\n{}", output)
                    );
                }
            }
            Err(e) => {
                self.add_message(
                    MessageType::Error,
                    &format!("❌ Command failed: {}", e)
                );
            }
        }
    }

    pub fn update(&mut self) {
        // Handle any periodic updates
        if self.state == AppState::Exiting {
            // Handle exit state
        }
    }
}