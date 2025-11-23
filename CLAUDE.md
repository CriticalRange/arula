# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

ARULA CLI - Autonomous AI command-line interface with native terminal scrollback.

## Development Commands

```bash
cargo build && cargo run         # Build and run
cargo build --release           # Optimized release build
cargo test                       # Run tests
cargo test -- <test_name>       # Run specific test
cargo clippy && cargo fmt        # Code quality
cargo check                      # Quick compile check
cargo run -- --help             # Show CLI options
cargo run -- --debug            # Run in debug mode
```

## Architecture

**Core Flow**: `main()` → modern input event loop → `app.send_to_ai()` / `app.check_ai_response_nonblocking()`

**Key Modules**:
- `app.rs`: Application state and AI message handling (~260 lines)
- `main.rs`: Event loop, command handling, AI response processing
- `reedline_input.rs`: Modern reedline input handler (CURRENT)
- `input_handler.rs`: Legacy custom input handler (DEPRECATED)
- `api.rs`: Traditional AI client with streaming support
- `agent.rs`: Modern AI agent framework with type-safe tool calling
- `agent_client.rs`: Client for agent-based AI interactions
- `tools.rs`: Modern tool implementations (BashTool, etc.)
- `output.rs`: Colored terminal output to stdout
- `overlay_menu.rs`: Crossterm-based overlay menu system
- `reedline_menu.rs`: Reedline-based menu implementation
- `tool_call.rs`: Legacy bash command extraction from AI responses
- `config.rs`: YAML-based configuration management
- `chat.rs`: Chat message types and data structures
- `visioneer.rs`: Desktop automation and screen analysis (Windows only)
- `changelog.rs`: Changelog display functionality

**Dual AI Architecture**:
- **Legacy API**: Traditional streaming via `api.rs` for backward compatibility
- **Modern Agent**: Type-safe tool calling via `agent.rs` and `tools.rs`
- **AI Streaming**: Uses `tokio::sync::mpsc::unbounded_channel()` for non-blocking responses
- **Terminal Design**: No alternate screen - all output flows to native scrollback buffer

**Multi-Provider AI Support**: Supports OpenAI, Anthropic, Ollama, Z.AI, OpenRouter, and custom providers via `config.rs`

**CLI Interface**: Uses `clap` for command-line argument parsing with options:
- `--verbose`: Verbose mode output
- `--endpoint <url>`: API endpoint (default: http://localhost:8080)
- `--debug`: Debug mode for development

## Design Principles

**Core Principles Followed:**

1. **Single Responsibility Principle (SRP)**
   - Each module has one clear purpose
   - `output.rs` handles display, `app.rs` handles logic, `overlay_menu.rs` handles menus
   - Successfully reduced `app.rs` from 2058 lines to ~260 lines

2. **Don't Repeat Yourself (DRY)**
   - Extract common patterns into reusable functions
   - `OutputHandler` centralizes all terminal output formatting

3. **KISS Principle**
   - Keep code simple and straightforward
   - Replaced complex ratatui TUI with simple event-driven input loop
   - Direct stdout printing instead of render buffers
   - Modern inquire styling without blocking event handling

4. **Command-Query-Separation (CQS)**
   - Commands perform actions: `send_to_ai()`, `execute_bash_command()`
   - Queries return data: `get_config()`, `check_ai_response_nonblocking()`

5. **Encapsulation**
   - `OutputHandler` encapsulates colored output
   - `OverlayMenu` encapsulates menu state and rendering
   - `ApiClient` encapsulates API communication

## Implementation Patterns

**AI Streaming**:
```rust
let (tx, rx) = mpsc::unbounded_channel();
self.ai_response_rx = Some(rx);
tokio::spawn(async move {
    match api_client.send_message_stream(&msg, Some(message_history)).await {
        Ok(mut stream_rx) => {
            let _ = tx.send(AiResponse::StreamStart);
            while let Some(response) = stream_rx.recv().await {
                match response {
                    StreamingResponse::Chunk(chunk) => {
                        let _ = tx.send(AiResponse::StreamChunk(chunk));
                    }
                    StreamingResponse::End(_) => {
                        let _ = tx.send(AiResponse::StreamEnd);
                        break;
                    }
                }
            }
        }
    }
});
```

**Non-blocking Response Check**:
```rust
// In main loop
if let Some(response) = app.check_ai_response_nonblocking() {
    match response {
        AiResponse::StreamStart => output.start_ai_message()?,
        AiResponse::StreamChunk(chunk) => output.print_streaming_chunk(&chunk)?,
        AiResponse::StreamEnd => output.end_line()?,
    }
}
```

**Overlay Menu Pattern**:
```rust
// Clear screen, enable raw mode, show menu, restore
execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
terminal::enable_raw_mode()?;
let result = self.run_main_menu(app, output);
terminal::disable_raw_mode()?;
execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
```

**Adding Menu Options**:
1. Add option to `options` vector in `run_main_menu()`
2. Add match arm in `KeyCode::Enter` handler
3. Implement the option's logic (may call other methods)
4. Use `show_confirm_dialog()` for confirmations

**Tool Development Pattern**:
```rust
// Define tool parameters
#[derive(Debug, Deserialize)]
pub struct MyToolParams {
    pub input: String,
}

// Implement the tool
pub struct MyTool;
impl MyTool {
    pub async fn execute(params: MyToolParams) -> ToolResult {
        // Tool implementation
        ToolResult::success(json!({"result": "success"}))
    }
}
```

## Configuration

Configuration is handled through YAML files in the user's config directory:
- Multi-provider support with automatic migration from legacy single-provider format
- Loaded via `Config::load_or_default()` in `app.rs`
- Supports API endpoints, model settings, and user preferences
- Provider-specific settings persistence
- Uses serde for serialization/deserialization
- Interactive configuration menu accessible via overlay system

## Terminal Notes

- Termux: `export TERM=xterm-256color`
- Native scrollback enabled - no alternate screen
- Menu shortcuts: `m`, `menu`, or `/menu`
- Ctrl+C shows exit confirmation (double press to exit)
- All output uses console for consistent styling
- CursorGuard ensures proper cursor cleanup on exit

## Key Libraries

- **reedline**: Modern readline replacement with syntax highlighting and multi-line support
  - `ExternalPrinter` for concurrent output while typing
  - `Validator` trait for input validation
  - `Highlighter` trait for syntax highlighting
- **nu-ansi-term**: Cross-platform color handling for terminal output
- **crossterm**: Terminal manipulation (raw mode, cursor, styling)
- **console**: Colored output with rich styling options
- **tokio**: Async runtime for AI streaming
- **reqwest**: HTTP client with rustls-tls (no OpenSSL dependency)
- **clap**: Command-line argument parsing
- **memmap2**: Memory-mapped file operations for tools
- **walkdir + ignore**: File system traversal with gitignore support
- **duct**: Command execution with proper I/O handling
- **async-trait**: Async trait support for tool interfaces
- **indicatif**: Progress bars and spinners for loading animations
- **fastrand**: Simple and fast random number generation
- **syntect**: Syntax highlighting for code blocks (supports many languages)
- **termimad**: Markdown rendering for terminal output
- **serde_yaml**: YAML configuration parsing
- **base64**: Base64 encoding for vision inputs
- **windows**: Windows APIs for Visioneer desktop automation (Windows only)
- **uiautomation**: UI automation framework (Windows only)
- **screenshots**: Screen capture functionality (Windows only)

## Recent Improvements & Fixes

### Empty Input Prevention System (Latest)
**Problem**: Users could submit empty input to AI by pressing Enter without typing anything.

**Solution**: Comprehensive multi-layer empty input prevention system:

1. **Reedline Validator Level** (`src/ui/reedline_input.rs:235-237`)
   ```rust
   if line.trim().is_empty() {
       ValidationResult::Incomplete  // Prevents submission at source
   }
   ```

2. **Multiple Reedline Safety Checks** (`src/ui/reedline_input.rs:637-639, 665-667`)
   ```rust
   if buffer.is_empty() || buffer.trim().is_empty() || buffer.chars().all(|c| c.is_whitespace()) {
       self.editor.run_edit_commands(&[EditCommand::Clear]);  // Clear line to prevent buildup
       continue;
   }
   ```

3. **Main Loop Final Safety Net** (`src/main.rs:382-387`)
   ```rust
   if input.trim().is_empty() {
       continue 'main_loop;  // Ultimate protection
   }
   ```

4. **User Escape Mechanism** (`src/ui/reedline_input.rs:370-374`)
   - `Ctrl+L` keybinding clears input line to escape incomplete state
   - Users can recover from accidental incomplete states

**Benefits**:
- ✅ Zero empty input reaches AI
- ✅ Clean user experience with no unwanted multiline behavior
- ✅ Preserves legitimate multiline (trailing backslash)
- ✅ Multiple safety nets ensure robustness
- ✅ Debug visibility for troubleshooting

### Terminal Scroll Positioning (Documented)
**Current Issue**: When AI responses stream through reedline's `ExternalPrinter`, terminal may not scroll optimally to keep input visible at bottom.

**Status**: Documented for future comprehensive solution
- Multiple terminal escape sequence approaches attempted
- Issue tracked in `src/app.rs:499-501` with TODO comment
- Clean implementation ready for future improvements

## TODO: Future Enhancements

### UI/UX Improvements
- [x] Progress indicators with spinners (using `indicatif` and console built-in)
- [x] Formatted code blocks with syntax highlighting (using `syntect`)
- [x] Syntax highlighting for AI responses (using `syntect`)
- [x] Better markdown rendering (using `termimad`)
- [x] Multi-line input support (Shift+Enter for new line, Enter to send)
- [x] Enhanced input prompt with status indicators (⚡🔧▶ states)
- [x] Token count display with color-coded warnings
- [x] Real-time changelog display in startup banner
- [x] Empty input prevention system with comprehensive validation
- [x] Multi-layer input validation with escape mechanisms
- [ ] Message history browser
- [ ] Terminal scroll positioning optimization for better AI response visibility

### Features
- [x] Multi-provider AI support (OpenAI, Anthropic, Ollama, Z.AI, OpenRouter, custom)
- [x] Interactive configuration menu with provider switching
- [x] Desktop automation (Visioneer) for Windows systems
- [x] Function/tool calling support with 5 built-in tools
- [x] Image input support (base64 encoding for vision models)
- [ ] Save/load conversation sessions
- [ ] Export conversations to markdown
- [ ] Custom system prompts
