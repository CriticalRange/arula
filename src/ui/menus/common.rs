//! Common utilities and types for the ARULA menu system

use crate::app::App;
use crate::output::OutputHandler;
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    terminal::{self, size},
    cursor::{Hide, Show},
    style::{Color, SetForegroundColor, SetBackgroundColor, ResetColor},
    ExecutableCommand, QueueableCommand,
};
use std::io::{stdout, Write};
use std::time::Duration;

/// Common result types for menu operations
#[derive(Debug, Clone, PartialEq)]
pub enum MenuResult {
    Continue,
    Settings,
    Exit,
    ClearChat,
    BackToMain,
    ConfigurationUpdated,
}

/// Internal menu action for flow control
#[derive(Debug, PartialEq)]
pub enum MenuAction {
    Continue,     // Stay in menu
    CloseMenu,    // Exit menu, continue app
    ExitApp,      // Exit menu AND exit app
}

/// Common menu utilities
pub struct MenuUtils;

impl MenuUtils {
    /// Truncate text to fit within max_width, adding "..." if truncated
    pub fn truncate_text(text: &str, max_width: usize) -> String {
        if text.len() <= max_width {
            text.to_string()
        } else {
            format!("{}...", &text[..max_width.saturating_sub(3)])
        }
    }

    /// Check if terminal has enough space for menu
    pub fn check_terminal_size(min_cols: u16, min_rows: u16) -> Result<bool> {
        let (cols, rows) = size()?;
        Ok(cols >= min_cols && rows >= min_rows)
    }

    /// Setup terminal for menu display
    pub fn setup_terminal() -> Result<()> {
        terminal::enable_raw_mode()?;
        stdout().execute(Hide)?;
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;
        stdout().flush()?;
        Ok(())
    }

    /// Restore terminal state after menu
    pub fn restore_terminal() -> Result<()> {
        terminal::disable_raw_mode()?;
        stdout().execute(Show)?;
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;
        stdout().flush()?;
        Ok(())
    }

    /// Wait for key event with timeout
    pub fn wait_for_key(timeout_ms: u64) -> Result<Option<KeyEvent>> {
        if event::poll(Duration::from_millis(timeout_ms))? {
            if let Event::Key(key_event) = event::read()? {
                return Ok(Some(key_event));
            }
        }
        Ok(None)
    }

    /// Read key event with press/release filtering
    pub fn read_key_event() -> Result<Option<KeyEvent>> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => Ok(Some(key)),
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    /// Render a box frame around content
    pub fn render_box(title: &str, width: u16, height: u16) -> Vec<String> {
        let mut output = Vec::new();
        let title_with_padding = format!(" {} ", title);
        let title_start = (width as usize / 2).saturating_sub(title_with_padding.len() / 2);
        let title_end = title_start + title_with_padding.len();

        // Top border with title
        let mut top_border = "┌".to_string();
        for i in 1..(width - 1) {
            let i_usize = i as usize;
            if i_usize >= title_start && i_usize < title_end && title_end <= width as usize {
                let title_char_index = i_usize - title_start;
                if title_char_index < title_with_padding.len() {
                    top_border.push(title_with_padding.chars().nth(title_char_index).unwrap_or('─'));
                } else {
                    top_border.push('─');
                }
            } else {
                top_border.push('─');
            }
        }
        top_border.push('┐');
        output.push(top_border);

        // Side borders with empty content
        for _ in 1..(height - 1) {
            output.push(format!("│{}│", " ".repeat(width as usize - 2)));
        }

        // Bottom border
        output.push(format!("└{}┘", "─".repeat(width as usize - 2)));

        output
    }

    /// Format menu item with selection indicator
    pub fn format_menu_item(item: &str, selected: bool) -> String {
        if selected {
            format!("▶ {}", item)
        } else {
            format!("  {}", item)
        }
    }
}

/// Common menu state management
pub struct MenuState {
    pub selected_index: usize,
    pub is_in_submenu: bool,
}

impl MenuState {
    pub fn new() -> Self {
        Self {
            selected_index: 0,
            is_in_submenu: false,
        }
    }

    pub fn move_up(&mut self, max_index: usize) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else {
            self.selected_index = max_index.saturating_sub(1);
        }
    }

    pub fn move_down(&mut self, max_index: usize) {
        if self.selected_index < max_index.saturating_sub(1) {
            self.selected_index += 1;
        } else {
            self.selected_index = 0;
        }
    }

    pub fn reset(&mut self) {
        self.selected_index = 0;
        self.is_in_submenu = false;
    }
}