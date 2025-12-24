//! Custom terminal implementation with better viewport management
//! Simplified for compatibility with ratatui 0.29 and crossterm 0.28

use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::style::{Print, SetAttribute};
use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::style::Modifier;
use ratatui::widgets::Widget;
use std::io::Write;

/// Custom Frame for better viewport control
pub struct Frame<'a> {
    /// Where should the cursor be after drawing this frame?
    pub cursor_position: Option<Position>,
    /// The area of the viewport
    pub viewport_area: Rect,
    /// The buffer that is used to draw the current frame
    pub buffer: &'a mut Buffer,
}

impl Frame<'_> {
    /// The area of the current frame
    pub const fn area(&self) -> Rect {
        self.viewport_area
    }

    /// Render a widget to the current buffer
    pub fn render_widget<W: Widget>(&mut self, widget: W, area: Rect) {
        widget.render(area, self.buffer);
    }

    /// Set cursor position after drawing frame
    pub fn set_cursor_position<P: Into<Position>>(&mut self, position: P) {
        self.cursor_position = Some(position.into());
    }

    /// Gets the buffer that this Frame draws into as a mutable reference
    pub fn buffer_mut(&mut self) -> &mut Buffer {
        self.buffer
    }
}

/// Custom Terminal with better viewport management
pub struct CustomTerminal<B>
where
    B: Backend + Write,
{
    /// The backend used to interface with the terminal
    backend: B,
    /// Holds the results of the current and previous draw calls
    buffers: [Buffer; 2],
    /// Index of the current buffer in the previous array
    current: usize,
    /// Whether the cursor is currently hidden
    pub hidden_cursor: bool,
    /// Area of the viewport
    pub viewport_area: Rect,
    /// Last known size of the terminal
    pub last_known_screen_size: Size,
    /// Last known position of the cursor
    pub last_known_cursor_pos: Position,
}

impl<B> CustomTerminal<B>
where
    B: Backend + Write,
{
    /// Creates a new CustomTerminal with the given backend
    pub fn with_options(mut backend: B) -> std::io::Result<Self> {
        let screen_size = backend.size()?;
        let cursor_pos = backend.get_cursor_position()?;
        Ok(Self {
            backend,
            buffers: [Buffer::empty(Rect::ZERO), Buffer::empty(Rect::ZERO)],
            current: 0,
            hidden_cursor: false,
            viewport_area: Rect::new(0, cursor_pos.y, 0, 0),
            last_known_screen_size: screen_size,
            last_known_cursor_pos: cursor_pos,
        })
    }

    /// Get a Frame object for rendering
    pub fn get_frame(&mut self) -> Frame<'_> {
        Frame {
            cursor_position: None,
            viewport_area: self.viewport_area,
            buffer: self.current_buffer_mut(),
        }
    }

    /// Gets the current buffer as a reference
    fn current_buffer(&self) -> &Buffer {
        &self.buffers[self.current]
    }

    /// Gets the current buffer as a mutable reference
    fn current_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[self.current]
    }

    /// Gets the previous buffer as a reference
    fn previous_buffer(&self) -> &Buffer {
        &self.buffers[1 - self.current]
    }

    /// Gets the previous buffer as a mutable reference
    fn previous_buffer_mut(&mut self) -> &mut Buffer {
        &mut self.buffers[1 - self.current]
    }

    /// Gets the backend
    pub const fn backend(&self) -> &B {
        &self.backend
    }

    /// Gets the backend as a mutable reference
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Flush changes to the terminal
    pub fn flush(&mut self) -> std::io::Result<()> {
        // Clone the needed buffer data to avoid borrow issues
        let prev_buffer = self.previous_buffer().clone();
        let curr_buffer = self.current_buffer().clone();
        let updates = diff_buffers(&prev_buffer, &curr_buffer);
        draw(&mut self.backend, updates.into_iter())?;
        Backend::flush(&mut self.backend)?;
        Ok(())
    }

    /// Updates the Terminal so that internal buffers match the requested area
    pub fn resize(&mut self, screen_size: Size) -> std::io::Result<()> {
        self.last_known_screen_size = screen_size;
        Ok(())
    }

    /// Sets the viewport area
    pub fn set_viewport_area(&mut self, area: Rect) {
        self.current_buffer_mut().resize(area);
        self.previous_buffer_mut().resize(area);
        self.viewport_area = area;
    }

    /// Queries the backend for size and resizes if necessary
    pub fn autoresize(&mut self) -> std::io::Result<()> {
        let screen_size = self.size()?;
        if screen_size != self.last_known_screen_size {
            self.resize(screen_size)?;
        }
        Ok(())
    }

    /// Draws a single frame to the terminal
    pub fn draw<F>(&mut self, render_callback: F) -> std::io::Result<()>
    where
        F: FnOnce(&mut Frame),
    {
        self.autoresize()?;

        let mut frame = self.get_frame();
        render_callback(&mut frame);

        let cursor_position = frame.cursor_position;

        self.flush()?;

        match cursor_position {
            None => self.hide_cursor()?,
            Some(position) => {
                self.show_cursor()?;
                self.set_cursor_position(position)?;
            }
        }

        self.swap_buffers();
        Ok(())
    }

    /// Hides the cursor
    pub fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.backend.hide_cursor()?;
        self.hidden_cursor = true;
        Ok(())
    }

    /// Shows the cursor
    pub fn show_cursor(&mut self) -> std::io::Result<()> {
        self.backend.show_cursor()?;
        self.hidden_cursor = false;
        Ok(())
    }

    /// Gets the current cursor position
    pub fn get_cursor_position(&mut self) -> std::io::Result<Position> {
        self.backend.get_cursor_position()
    }

    /// Sets the cursor position
    pub fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> std::io::Result<()> {
        let position = position.into();
        self.backend.set_cursor_position(position)?;
        self.last_known_cursor_pos = position;
        Ok(())
    }

    /// Clear the terminal and force a full redraw
    pub fn clear(&mut self) -> std::io::Result<()> {
        if self.viewport_area.is_empty() {
            return Ok(());
        }
        self.backend
            .set_cursor_position(self.viewport_area.as_position())?;
        self.backend.clear_region(ratatui::backend::ClearType::All)?;
        self.previous_buffer_mut().reset();
        Ok(())
    }

    /// Clears the inactive buffer and swaps it with the current buffer
    pub fn swap_buffers(&mut self) {
        self.previous_buffer_mut().reset();
        self.current = 1 - self.current;
    }

    /// Queries the real size of the backend
    pub fn size(&self) -> std::io::Result<Size> {
        self.backend.size()
    }
}

/// Diff two buffers to find what changed
fn diff_buffers(previous: &Buffer, current: &Buffer) -> Vec<DrawCommand> {
    let width = current.area.width;
    let height = current.area.height;

    let mut commands = Vec::new();

    for y in 0..height {
        for x in 0..width {
            let prev_cell = &previous[(x, y)];
            let curr_cell = &current[(x, y)];

            if prev_cell != curr_cell {
                commands.push(DrawCommand::Cell {
                    x,
                    y,
                    fg: curr_cell.fg,
                    bg: curr_cell.bg,
                    modifier: curr_cell.modifier,
                    symbol: curr_cell.symbol().to_string(),
                });
            }
        }
    }

    commands
}

/// Draw commands for terminal updates
#[derive(Debug)]
enum DrawCommand {
    Cell {
        x: u16,
        y: u16,
        fg: ratatui::style::Color,
        bg: ratatui::style::Color,
        modifier: Modifier,
        symbol: String,
    },
}

/// Convert ratatui color to crossterm color
fn to_crossterm_color(c: ratatui::style::Color) -> crossterm::style::Color {
    use ratatui::style::Color as R;
    match c {
        R::Reset => crossterm::style::Color::Reset,
        R::Black => crossterm::style::Color::Black,
        R::Red => crossterm::style::Color::Red,
        R::Green => crossterm::style::Color::Green,
        R::Yellow => crossterm::style::Color::Yellow,
        R::Blue => crossterm::style::Color::Blue,
        R::Magenta => crossterm::style::Color::Magenta,
        R::Cyan => crossterm::style::Color::Cyan,
        R::Gray => crossterm::style::Color::Grey,
        R::DarkGray => crossterm::style::Color::DarkGrey,
        R::LightRed => crossterm::style::Color::Red,
        R::LightGreen => crossterm::style::Color::Green,
        R::LightYellow => crossterm::style::Color::Yellow,
        R::LightBlue => crossterm::style::Color::Blue,
        R::LightMagenta => crossterm::style::Color::Magenta,
        R::LightCyan => crossterm::style::Color::Cyan,
        R::White => crossterm::style::Color::White,
        R::Indexed(v) => crossterm::style::Color::AnsiValue(v),
        R::Rgb(r, g, b) => crossterm::style::Color::Rgb { r, g, b },
    }
}

/// Draw commands to the terminal
fn draw<B>(backend: &mut B, commands: impl Iterator<Item = DrawCommand>) -> std::io::Result<()>
where
    B: Write,
{
    let mut last_fg = ratatui::style::Color::Reset;
    let mut last_bg = ratatui::style::Color::Reset;
    let mut last_modifier = Modifier::empty();

    for cmd in commands {
        let DrawCommand::Cell {
            x,
            y,
            fg,
            bg,
            modifier,
            symbol,
        } = cmd;

        // Move cursor
        queue!(backend, MoveTo(x, y))?;

        // Set colors if changed
        if fg != last_fg || bg != last_bg {
            queue!(
                backend,
                crossterm::style::SetColors(crossterm::style::Colors::new(
                    to_crossterm_color(fg),
                    to_crossterm_color(bg),
                ))
            )?;
            last_fg = fg;
            last_bg = bg;
        }

        // Set modifiers
        if modifier != last_modifier {
            let removed = last_modifier - modifier;
            let added = modifier - last_modifier;

            if removed.contains(Modifier::REVERSED) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::NoReverse))?;
            }
            if removed.contains(Modifier::BOLD) {
                queue!(
                    backend,
                    SetAttribute(crossterm::style::Attribute::NormalIntensity)
                )?;
                if added.contains(Modifier::DIM) {
                    queue!(backend, SetAttribute(crossterm::style::Attribute::Dim))?;
                }
            }
            if removed.contains(Modifier::ITALIC) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::NoItalic))?;
            }
            if removed.contains(Modifier::UNDERLINED) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::NoUnderline))?;
            }
            if removed.contains(Modifier::DIM) {
                queue!(
                    backend,
                    SetAttribute(crossterm::style::Attribute::NormalIntensity)
                )?;
            }

            if added.contains(Modifier::REVERSED) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::Reverse))?;
            }
            if added.contains(Modifier::BOLD) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::Bold))?;
            }
            if added.contains(Modifier::ITALIC) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::Italic))?;
            }
            if added.contains(Modifier::UNDERLINED) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::Underlined))?;
            }
            if added.contains(Modifier::DIM) {
                queue!(backend, SetAttribute(crossterm::style::Attribute::Dim))?;
            }

            last_modifier = modifier;
        }

        queue!(backend, Print(symbol))?;
    }

    // Reset styles
    queue!(
        backend,
        crossterm::style::SetForegroundColor(crossterm::style::Color::Reset),
        crossterm::style::SetBackgroundColor(crossterm::style::Color::Reset),
        SetAttribute(crossterm::style::Attribute::Reset),
    )?;

    Ok(())
}
