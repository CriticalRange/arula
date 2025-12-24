//! Z.AI endpoint selection menu for ARULA CLI
//! Allows selecting between Coding Plan and Anthropic Compatible endpoints

use crate::app::App;
use crate::ui::menus::common::draw_modern_box;
use crate::ui::output::OutputHandler;
use arula_core::utils::config::ZaiEndpoint;
use anyhow::Result;
use console::style;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    style::{Print, ResetColor, SetForegroundColor},
    terminal, ExecutableCommand, QueueableCommand,
};
use std::io::{stdout, Write};
use std::time::Duration;

/// Z.AI endpoint selection menu
pub struct ZaiEndpointSelector;

impl ZaiEndpointSelector {
    pub fn new() -> Self {
        Self
    }

    /// Show the Z.AI endpoint selector menu
    pub fn show(&self, app: &mut App, output: &mut OutputHandler) -> Result<()> {
        // Clear screen when entering submenu
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;

        // Get available endpoints and current selection
        let endpoints = ZaiEndpoint::all();
        let current_url = app
            .config
            .get_active_provider_config()
            .and_then(|c| c.api_url.clone())
            .unwrap_or_default();

        // Find current endpoint (default to first one)
        let current_endpoint = ZaiEndpoint::by_url(&current_url)
            .unwrap_or_else(|| ZaiEndpoint::default_endpoint());

        let mut selected_idx = endpoints
            .iter()
            .position(|e| e.name == current_endpoint.name)
            .unwrap_or(0);

        // Clear any pending events
        std::thread::sleep(Duration::from_millis(20));
        for _ in 0..3 {
            while event::poll(Duration::from_millis(0))? {
                let _ = event::read()?;
            }
            std::thread::sleep(Duration::from_millis(5));
        }

        loop {
            self.render(&endpoints, selected_idx)?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key_event) => {
                        if key_event.kind != KeyEventKind::Press {
                            continue;
                        }

                        match key_event.code {
                            KeyCode::Up => {
                                if selected_idx > 0 {
                                    selected_idx -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if selected_idx + 1 < endpoints.len() {
                                    selected_idx += 1;
                                }
                            }
                            KeyCode::Enter => {
                                let selected = &endpoints[selected_idx];
                                // Update the API URL for Z.AI provider
                                if let Some(config) = app.config.get_active_provider_config_mut() {
                                    config.api_url = Some(selected.url.clone());
                                }

                                // Save config
                                if let Err(e) = app.config.save() {
                                    output.print_error(&format!("Failed to save configuration: {}", e))?;
                                } else {
                                    output.print_system(&format!(
                                        "✅ Z.AI endpoint set to: {} ({})",
                                        selected.name, selected.description
                                    ))?;

                                    // Reinitialize agent client with new endpoint
                                    let _ = app.initialize_agent_client();
                                }

                                // Clear screen and exit
                                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                                stdout().flush()?;
                                break;
                            }
                            KeyCode::Esc => {
                                // Clear screen and exit without saving
                                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                                stdout().flush()?;
                                break;
                            }
                            KeyCode::Char('c') if crossterm::event::poll(Duration::from_millis(0)).is_ok() => {
                                // Ctrl+C - exit
                                stdout().execute(terminal::Clear(terminal::ClearType::All))?;
                                stdout().flush()?;
                                break;
                            }
                            _ => {}
                        }
                    }
                    Event::Resize(_, _) => {
                        // Re-render on resize
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    /// Render the endpoint selector menu
    fn render(&self, endpoints: &[ZaiEndpoint], selected_idx: usize) -> Result<()> {
        let (cols, rows) = crossterm::terminal::size()?;

        let menu_width = 60.min(cols.saturating_sub(4));
        let menu_height = endpoints.len() as u16 + 6; // +6 for title, borders, padding, help

        let start_x = (cols - menu_width) / 2;
        let start_y = if rows > menu_height + 2 {
            (rows - menu_height) / 2
        } else {
            1
        };

        // Clear screen first
        stdout().execute(terminal::Clear(terminal::ClearType::All))?;
        stdout().execute(crossterm::cursor::MoveTo(0, 0))?;

        // Draw modern box
        draw_modern_box(start_x, start_y, menu_width, menu_height)?;

        // Draw title
        let title_y = start_y + 1;
        let title = "⚙ Z.AI ENDPOINT";
        let title_len = title.len() as u16;
        let title_x = if menu_width > title_len + 2 {
            start_x + menu_width / 2 - title_len / 2
        } else {
            start_x + 1
        };
        stdout()
            .queue(crossterm::cursor::MoveTo(title_x, title_y))?
            .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(
                crate::utils::colors::MISC_ANSI,
            )))?
            .queue(Print(style(title).bold()))?
            .queue(ResetColor)?;

        // Draw endpoints
        let items_start_y = start_y + 3;
        let max_text_width = menu_width.saturating_sub(8) as usize;

        for (idx, endpoint) in endpoints.iter().enumerate() {
            let y = items_start_y + idx as u16;

            let display_name = if endpoint.name.len() > max_text_width {
                format!("{}...", &endpoint.name[..max_text_width.saturating_sub(3)])
            } else {
                endpoint.name.clone()
            };

            let text = format!("▶ {} ({})", display_name, endpoint.description);
            let color = if idx == selected_idx {
                SetForegroundColor(crossterm::style::Color::AnsiValue(
                    crate::utils::colors::PRIMARY_ANSI,
                ))
            } else {
                SetForegroundColor(crossterm::style::Color::AnsiValue(
                    crate::utils::colors::MISC_ANSI,
                ))
            };

            // Clear the line first
            stdout()
                .queue(crossterm::cursor::MoveTo(start_x + 2, y))?;
            for _ in 0..(menu_width.saturating_sub(4)) {
                stdout().queue(Print(" "))?;
            }

            // Then draw the text
            stdout()
                .queue(crossterm::cursor::MoveTo(start_x + 4, y))?
                .queue(color)?
                .queue(Print(&text))?
                .queue(ResetColor)?;
        }

        // Draw help text
        let help_y = start_y + menu_height - 1;
        let help_text = "↑↓ Navigate • Enter Select • ESC Back";
        let help_x = start_x + 2;
        stdout()
            .queue(crossterm::cursor::MoveTo(help_x, help_y))?
            .queue(SetForegroundColor(crossterm::style::Color::AnsiValue(
                crate::utils::colors::AI_HIGHLIGHT_ANSI,
            )))?
            .queue(Print(help_text))?
            .queue(ResetColor)?;

        stdout().flush()?;
        Ok(())
    }
}

impl Default for ZaiEndpointSelector {
    fn default() -> Self {
        Self::new()
    }
}
