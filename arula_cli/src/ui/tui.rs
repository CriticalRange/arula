//! Terminal mode management and inline rendering
//! Based on codex-rs TUI implementation with keyboard enhancement support

use anyhow::Result;
use crossterm::cursor::Show;
use crossterm::event::{
    DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
    EnableFocusChange, EnableMouseCapture, KeyEventKind,
};
use crossterm::event::KeyboardEnhancementFlags;
use crossterm::event::PopKeyboardEnhancementFlags;
use crossterm::event::PushKeyboardEnhancementFlags;
use crossterm::{execute, terminal::disable_raw_mode, terminal::enable_raw_mode};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::io::{self, Stdout};
use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub type CustomTerminal = crate::ui::custom_terminal::CustomTerminal<CrosstermBackend<Stdout>>;

/// Terminal focused state tracking
pub struct TerminalFocus {
    pub is_focused: Arc<AtomicBool>,
}

impl TerminalFocus {
    pub fn new() -> Self {
        Self {
            is_focused: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused.load(Ordering::Relaxed)
    }

    pub fn set_unfocused(&self) {
        self.is_focused.store(false, Ordering::Relaxed);
    }

    pub fn set_focused(&self) {
        self.is_focused.store(true, Ordering::Relaxed);
    }
}

/// Terminal modes state
pub struct TerminalModes {
    pub terminal: Option<CustomTerminal>,
    pub focus: TerminalFocus,
    enhanced_keys_supported: bool,
}

impl TerminalModes {
    /// Initialize terminal modes
    pub fn new(_height: u16) -> Result<Self> {
        set_modes()?;
        set_panic_hook();

        let backend = CrosstermBackend::new(io::stdout());
        let terminal = CustomTerminal::with_options(backend)?;
        // Try to detect keyboard enhancement support, default to false
        let enhanced_keys_supported = Self::detect_keyboard_enhancement();

        Ok(Self {
            terminal: Some(terminal),
            focus: TerminalFocus::new(),
            enhanced_keys_supported,
        })
    }

    /// Detect if keyboard enhancement is supported
    fn detect_keyboard_enhancement() -> bool {
        // Check the TERM environment variable as a hint
        std::env::var("TERM")
            .map(|term| {
                matches!(
                    term.as_str(),
                    "xterm-256color" | "xterm-new" | "screen" | "tmux" | "alacritty" | "kitty"
                )
            })
            .unwrap_or(false)
    }

    /// Get the terminal
    pub fn terminal(&mut self) -> &mut CustomTerminal {
        self.terminal.as_mut().unwrap()
    }

    /// Check if enhanced keys are supported
    pub fn enhanced_keys_supported(&self) -> bool {
        self.enhanced_keys_supported
    }

    /// Get terminal focus tracker
    pub fn focus(&self) -> &TerminalFocus {
        &self.focus
    }
}

impl Drop for TerminalModes {
    fn drop(&mut self) {
        let _ = restore();
    }
}

/// Set terminal modes (raw mode, bracketed paste, keyboard enhancement)
pub fn set_modes() -> Result<()> {
    execute!(io::stdout(), EnableBracketedPaste)?;
    enable_raw_mode()?;

    // Enable keyboard enhancement flags for better key disambiguation
    // This allows distinguishing Enter from Ctrl+M, Tab from Ctrl+I, etc.
    let _ = execute!(
        io::stdout(),
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    );

    // Enable focus change events
    let _ = execute!(io::stdout(), EnableFocusChange);

    // Enable mouse mode for scroll events
    let _ = execute!(io::stdout(), EnableMouseCapture);

    Ok(())
}

/// Restore terminal to original state
pub fn restore() -> Result<()> {
    let _ = execute!(io::stdout(), PopKeyboardEnhancementFlags);
    let _ = execute!(io::stdout(), DisableMouseCapture);
    execute!(io::stdout(), DisableBracketedPaste)?;
    let _ = execute!(io::stdout(), DisableFocusChange);
    disable_raw_mode()?;
    let _ = execute!(io::stdout(), Show);
    Ok(())
}

/// Set panic hook to restore terminal on panic
fn set_panic_hook() {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        hook(panic_info);
    }));
}

/// Check if a key event should be filtered (only Press events)
pub fn should_accept_key_event(kind: KeyEventKind) -> bool {
    kind == KeyEventKind::Press
}

/// A renderer that draws TUI widgets inline at the bottom of the terminal
/// using Ratatui's native inline viewport.
pub struct InlineRenderer {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl InlineRenderer {
    /// Create a new inline renderer with a fixed height viewport
    pub fn new(height: u16) -> Result<Self> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);

        let viewport = Viewport::Inline(height);
        let terminal = Terminal::with_options(backend, TerminalOptions { viewport })?;

        Ok(Self { terminal })
    }

    /// Resize the inline viewport
    pub fn resize(&mut self, height: u16) -> Result<()> {
        let stdout = io::stdout();
        let backend = CrosstermBackend::new(stdout);
        let viewport = Viewport::Inline(height);
        self.terminal = Terminal::with_options(backend, TerminalOptions { viewport })?;
        Ok(())
    }

    /// Clear the inline viewport (remove it from view)
    pub fn clear(&mut self) -> Result<()> {
        self.terminal.clear()?;
        Ok(())
    }
}
