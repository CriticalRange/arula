//! Reedline-integrated menu system for ARULA
//!
//! Provides an ESC-triggered menu built into reedline's rendering system.
//! This replaces the crossterm-based overlay menu with a native reedline menu.

use crossterm::style::Stylize;

/// Menu items for ARULA
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Continue,
    Settings,
    Info,
    ClearChat,
    Exit,
}

impl MenuItem {
    pub fn all() -> Vec<Self> {
        vec![
            MenuItem::Continue,
            MenuItem::Settings,
            MenuItem::Info,
            MenuItem::ClearChat,
            MenuItem::Exit,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            MenuItem::Continue => "💬 Continue Chat",
            MenuItem::Settings => "⚙️  Settings",
            MenuItem::Info => "ℹ️  Info & Help",
            MenuItem::ClearChat => "🧹 Clear Chat",
            MenuItem::Exit => "🚪 Exit ARULA",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            MenuItem::Continue => "Return to conversation",
            MenuItem::Settings => "Configure AI provider and settings",
            MenuItem::Info => "View help and session information",
            MenuItem::ClearChat => "Clear conversation history",
            MenuItem::Exit => "Exit the application",
        }
    }
}

/// Settings sub-menu items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    Provider,
    Model,
    ApiUrl,
    ApiKey,
    Back,
}

impl SettingsItem {
    pub fn all() -> Vec<Self> {
        vec![
            SettingsItem::Provider,
            SettingsItem::Model,
            SettingsItem::ApiUrl,
            SettingsItem::ApiKey,
            SettingsItem::Back,
        ]
    }

    pub fn label(&self) -> &str {
        match self {
            SettingsItem::Provider => "🤖 AI Provider",
            SettingsItem::Model => "🧠 AI Model",
            SettingsItem::ApiUrl => "🌐 API URL",
            SettingsItem::ApiKey => "🔑 API Key",
            SettingsItem::Back => "← Back to Menu",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            SettingsItem::Provider => "Select AI provider (OpenAI, Anthropic, etc)",
            SettingsItem::Model => "Choose AI model to use",
            SettingsItem::ApiUrl => "Set custom API endpoint URL",
            SettingsItem::ApiKey => "Configure API authentication key",
            SettingsItem::Back => "Return to main menu",
        }
    }
}

/// Custom ARULA menu for reedline
pub struct ArulaMenu {
    /// Current menu items to display
    items: Vec<String>,
    /// Descriptions for each item
    descriptions: Vec<String>,
    /// Currently selected index
    selected: usize,
    /// Menu title
    title: String,
    /// Whether we're in settings submenu
    in_settings: bool,
}

impl ArulaMenu {
    pub fn new() -> Self {
        let items: Vec<String> = MenuItem::all().iter().map(|m| m.label().to_string()).collect();
        let descriptions: Vec<String> = MenuItem::all()
            .iter()
            .map(|m| m.description().to_string())
            .collect();

        Self {
            items,
            descriptions,
            selected: 0,
            title: "ARULA Menu".to_string(),
            in_settings: false,
        }
    }

    pub fn switch_to_settings(&mut self) {
        self.items = SettingsItem::all()
            .iter()
            .map(|s| s.label().to_string())
            .collect();
        self.descriptions = SettingsItem::all()
            .iter()
            .map(|s| s.description().to_string())
            .collect();
        self.selected = 0;
        self.title = "Settings".to_string();
        self.in_settings = true;
    }

    pub fn switch_to_main(&mut self) {
        self.items = MenuItem::all().iter().map(|m| m.label().to_string()).collect();
        self.descriptions = MenuItem::all()
            .iter()
            .map(|m| m.description().to_string())
            .collect();
        self.selected = 0;
        self.title = "ARULA Menu".to_string();
        self.in_settings = false;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.items.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected < self.items.len() - 1 {
            self.selected += 1;
        } else {
            self.selected = 0;
        }
    }

    pub fn get_selected_main_item(&self) -> Option<MenuItem> {
        if self.in_settings {
            return None;
        }
        MenuItem::all().get(self.selected).copied()
    }

    pub fn get_selected_settings_item(&self) -> Option<SettingsItem> {
        if !self.in_settings {
            return None;
        }
        SettingsItem::all().get(self.selected).copied()
    }

    /// Render the menu as styled text
    pub fn render(&self, width: u16) -> Vec<String> {
        let mut output = Vec::new();

        // Title
        let title_line = format!("╭─ {} ─╮", self.title);
        output.push(title_line);

        // Menu items
        for (idx, item) in self.items.iter().enumerate() {
            let is_selected = idx == self.selected;
            let prefix = if is_selected { "▶ " } else { "  " };

            let item_line = if is_selected {
                format!("{}{}", prefix, item).cyan().bold().to_string()
            } else {
                format!("{}{}", prefix, item).to_string()
            };

            output.push(format!("│ {:width$} │", item_line, width = width as usize - 4));

            // Description for selected item
            if is_selected {
                let desc = &self.descriptions[idx];
                let desc_line = format!("  {}", desc).dark_grey().to_string();
                output.push(format!("│ {:width$} │", desc_line, width = width as usize - 4));
            }
        }

        // Footer
        output.push("╰─────────────────────────────╯".to_string());
        output.push("".to_string());
        output.push("  ↑↓ Navigate  │  Enter Select  │  ESC Cancel".dark_grey().to_string());

        output
    }
}

/// Menu state machine for handling ESC key behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuState {
    Hidden,
    Main,
    Settings,
}

pub struct MenuStateMachine {
    state: MenuState,
    esc_count: usize,
    last_esc_time: std::time::Instant,
}

impl MenuStateMachine {
    pub fn new() -> Self {
        Self {
            state: MenuState::Hidden,
            esc_count: 0,
            last_esc_time: std::time::Instant::now(),
        }
    }

    pub fn handle_esc(&mut self) -> MenuState {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.last_esc_time);

        // Reset counter if too much time passed (>1 second)
        if elapsed.as_secs() > 1 {
            self.esc_count = 0;
        }

        self.last_esc_time = now;
        self.esc_count += 1;

        match self.state {
            MenuState::Hidden => {
                if self.esc_count >= 2 {
                    // Second ESC - show menu
                    self.state = MenuState::Main;
                    self.esc_count = 0;
                }
                // First ESC - just cleared input
            }
            MenuState::Main | MenuState::Settings => {
                // ESC in menu - hide menu
                self.state = MenuState::Hidden;
                self.esc_count = 0;
            }
        }

        self.state
    }

    pub fn show_main_menu(&mut self) {
        self.state = MenuState::Main;
        self.esc_count = 0;
    }

    pub fn show_settings_menu(&mut self) {
        self.state = MenuState::Settings;
    }

    pub fn hide_menu(&mut self) {
        self.state = MenuState::Hidden;
        self.esc_count = 0;
    }

    pub fn is_visible(&self) -> bool {
        self.state != MenuState::Hidden
    }

    pub fn current_state(&self) -> MenuState {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_menu_items() {
        let items = MenuItem::all();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].label(), "💬 Continue Chat");
    }

    #[test]
    fn test_settings_items() {
        let items = SettingsItem::all();
        assert_eq!(items.len(), 5);
        assert_eq!(items[0].label(), "🤖 AI Provider");
    }

    #[test]
    fn test_menu_navigation() {
        let mut menu = ArulaMenu::new();
        assert_eq!(menu.selected, 0);

        menu.move_down();
        assert_eq!(menu.selected, 1);

        menu.move_up();
        assert_eq!(menu.selected, 0);

        menu.move_up(); // Wraps to end
        assert_eq!(menu.selected, 4);
    }

    #[test]
    fn test_menu_state_machine() {
        let mut sm = MenuStateMachine::new();
        assert_eq!(sm.current_state(), MenuState::Hidden);

        // First ESC - stay hidden
        sm.handle_esc();
        assert_eq!(sm.current_state(), MenuState::Hidden);

        // Second ESC - show menu
        sm.handle_esc();
        assert_eq!(sm.current_state(), MenuState::Main);

        // ESC in menu - hide
        sm.handle_esc();
        assert_eq!(sm.current_state(), MenuState::Hidden);
    }

    #[test]
    fn test_menu_switching() {
        let mut menu = ArulaMenu::new();
        assert!(!menu.in_settings);

        menu.switch_to_settings();
        assert!(menu.in_settings);
        assert_eq!(menu.title, "Settings");

        menu.switch_to_main();
        assert!(!menu.in_settings);
        assert_eq!(menu.title, "ARULA Menu");
    }
}
