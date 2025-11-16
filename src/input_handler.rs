use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};
use std::collections::VecDeque;

/// Custom input handler that manages input line independently
pub struct InputHandler {
    buffer: String,
    cursor_pos: usize,
    history: VecDeque<String>,
    history_index: Option<usize>,
    temp_buffer: Option<String>, // Temporary storage when navigating history
    prompt: String,
    max_history: usize,
}

impl InputHandler {
    pub fn new(prompt: &str) -> Self {
        Self {
            buffer: String::new(),
            cursor_pos: 0,
            history: VecDeque::new(),
            history_index: None,
            temp_buffer: None,
            prompt: prompt.to_string(),
            max_history: 1000,
        }
    }

    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.to_string();
    }

    /// Add entry to history
    pub fn add_to_history(&mut self, entry: String) {
        if entry.trim().is_empty() {
            return;
        }

        // Don't add duplicates of the last entry
        if self.history.back() == Some(&entry) {
            return;
        }

        self.history.push_back(entry);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
    }

    /// Load history from lines
    pub fn load_history(&mut self, lines: Vec<String>) {
        for line in lines {
            if !line.trim().is_empty() {
                self.history.push_back(line);
            }
        }
        if self.history.len() > self.max_history {
            self.history.drain(0..self.history.len() - self.max_history);
        }
    }

    /// Get history entries
    pub fn get_history(&self) -> Vec<String> {
        self.history.iter().cloned().collect()
    }

    /// Draw the input prompt and buffer at current cursor position
    pub fn draw(&self) -> io::Result<()> {
        // Clear current line
        execute!(
            io::stdout(),
            cursor::MoveToColumn(0),
            terminal::Clear(ClearType::CurrentLine)
        )?;

        // Print prompt and buffer without extra spacing
        print!("{}{}", self.prompt, self.buffer);

        // Position cursor correctly
        let cursor_col = (self.prompt.chars().count() + self.cursor_pos) as u16;
        execute!(io::stdout(), cursor::MoveToColumn(cursor_col))?;

        io::stdout().flush()?;
        Ok(())
    }

    /// Handle a key event, returns Some(input) if user submitted
    pub fn handle_key(&mut self, key: KeyEvent) -> io::Result<Option<String>> {
        match key.code {
            KeyCode::Enter => {
                // Submit input
                let input = self.buffer.clone();
                self.buffer.clear();
                self.cursor_pos = 0;
                self.history_index = None;
                self.temp_buffer = None;

                // Move to new line
                println!();

                Ok(Some(input))
            }
            KeyCode::Char(c) => {
                // Insert character at cursor position
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    // Handle Ctrl+C, Ctrl+D etc
                    match c {
                        'c' | 'C' => {
                            // Ctrl+C - return special signal
                            self.buffer.clear();
                            self.cursor_pos = 0;
                            println!();
                            return Ok(Some("__CTRL_C__".to_string()));
                        }
                        'd' | 'D' => {
                            // Ctrl+D - EOF
                            if self.buffer.is_empty() {
                                println!();
                                return Ok(Some("__CTRL_D__".to_string()));
                            }
                        }
                        'u' | 'U' => {
                            // Ctrl+U - clear line
                            self.buffer.clear();
                            self.cursor_pos = 0;
                        }
                        'a' | 'A' => {
                            // Ctrl+A - move to start
                            self.cursor_pos = 0;
                        }
                        'e' | 'E' => {
                            // Ctrl+E - move to end
                            self.cursor_pos = self.buffer.len();
                        }
                        'w' | 'W' => {
                            // Ctrl+W - delete word backwards
                            if self.cursor_pos > 0 {
                                let before_cursor = &self.buffer[..self.cursor_pos];
                                let trimmed = before_cursor.trim_end();
                                let last_space = trimmed.rfind(' ').map(|i| i + 1).unwrap_or(0);
                                self.buffer.drain(last_space..self.cursor_pos);
                                self.cursor_pos = last_space;
                            }
                        }
                        _ => {}
                    }
                } else {
                    self.buffer.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.history_index = None;
                }
                self.draw()?;
                Ok(None)
            }
            KeyCode::Backspace => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.buffer.remove(self.cursor_pos);
                    self.history_index = None;
                }
                self.draw()?;
                Ok(None)
            }
            KeyCode::Delete => {
                if self.cursor_pos < self.buffer.len() {
                    self.buffer.remove(self.cursor_pos);
                    self.history_index = None;
                }
                self.draw()?;
                Ok(None)
            }
            KeyCode::Left => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                }
                self.draw()?;
                Ok(None)
            }
            KeyCode::Right => {
                if self.cursor_pos < self.buffer.len() {
                    self.cursor_pos += 1;
                }
                self.draw()?;
                Ok(None)
            }
            KeyCode::Home => {
                self.cursor_pos = 0;
                self.draw()?;
                Ok(None)
            }
            KeyCode::End => {
                self.cursor_pos = self.buffer.len();
                self.draw()?;
                Ok(None)
            }
            KeyCode::Up => {
                // Navigate history backwards
                if self.history.is_empty() {
                    return Ok(None);
                }

                if self.history_index.is_none() {
                    // Save current buffer
                    self.temp_buffer = Some(self.buffer.clone());
                    self.history_index = Some(self.history.len() - 1);
                } else if let Some(idx) = self.history_index {
                    if idx > 0 {
                        self.history_index = Some(idx - 1);
                    }
                }

                if let Some(idx) = self.history_index {
                    self.buffer = self.history[idx].clone();
                    self.cursor_pos = self.buffer.len();
                }

                self.draw()?;
                Ok(None)
            }
            KeyCode::Down => {
                // Navigate history forwards
                if let Some(idx) = self.history_index {
                    if idx < self.history.len() - 1 {
                        self.history_index = Some(idx + 1);
                        self.buffer = self.history[idx + 1].clone();
                    } else {
                        // Restore temp buffer
                        self.history_index = None;
                        self.buffer = self.temp_buffer.take().unwrap_or_default();
                    }
                    self.cursor_pos = self.buffer.len();
                    self.draw()?;
                }
                Ok(None)
            }
            KeyCode::Tab => {
                // Could implement tab completion here
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    /// Clear the current input
    pub fn clear(&mut self) -> io::Result<()> {
        self.buffer.clear();
        self.cursor_pos = 0;
        self.draw()
    }
}
