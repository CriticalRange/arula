# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
ARULA CLI is an autonomous AI command-line interface built with Rust and Ratatui. It features a modern chat-style TUI for interacting with multiple AI providers (OpenAI, Claude, Ollama, custom endpoints), executing shell commands, and managing Git operations autonomously.

## Common Development Commands

```bash
# Build and run
cargo build                    # Debug build
cargo run                      # Run in debug mode
cargo build --release          # Optimized release build
cargo run -- --verbose         # Run with verbose logging
cargo run -- --endpoint URL    # Run with custom API endpoint
cargo run -- --debug           # Enable debug mode

# Code quality
cargo check                    # Fast compilation check
cargo clippy                   # Linting
cargo fmt                      # Format code
cargo test                     # Run tests

# Testing commands
cargo test                     # Run all tests
cargo test --lib               # Run unit tests only
cargo test --test integration  # Run integration tests only
cargo test api::tests          # Run tests in specific module
cargo test -- --nocapture      # Don't capture stdout
cargo test -- --test-threads=1 # Run tests sequentially

# Termux-specific (for Android development)
export TERM=xterm-256color     # Set terminal type before running
pkg install rust git clang     # Install required packages
```

## Architecture Overview

### Core Event Loop (main.rs - 284 lines)
The application uses an async event loop with:
- **Crossterm** for terminal raw mode and event handling
- **Ratatui** (0.29) for rendering the TUI with modern features
- **Tokio** async runtime with full features for non-blocking operations
- Event polling every 50ms for responsive input
- Mouse capture support for scroll wheel interactions
- Separate handling for keyboard (KeyEventKind::Press only), mouse, and terminal resize events

**Key Flow**: `main()` → `run_app()` → event loop → `app.handle_key_event()` / `app.handle_menu_navigation()` / `app.check_ai_response()`

**Critical Pattern**: The loop continuously calls `app.check_ai_response()` to poll for streaming AI responses via `mpsc::UnboundedReceiver<AiResponse>`, enabling non-blocking UI updates during AI processing.

### Application State (app.rs - 2058 lines, largest module)
`App` struct is the central state manager with complex state transitions:
- **State Management**: `AppState` enum with `Chat`, `Menu(MenuType)`, and `Exiting` states
- **Messages**: Chat history in `Vec<ChatMessage>`, saved to `ConversationManager`
- **Input handling**: Uses `tui-textarea::TextArea` for multi-line input with placeholder text
- **Menu system**: Hierarchical menu with `MenuType` (Main, Commands, Configuration, etc.) and `menu_selected` index
- **AI streaming**: `ai_response_rx` for async message streaming, `is_ai_thinking` for animation state
- **Git operations**: `git_ops: GitOperations` for repository management
- **API client**: `api_client: Option<ApiClient>` supports multiple AI providers
- **Configuration**: In-menu editing with `EditableField` enum for live config changes
- **Conversation persistence**: `conversation_manager` saves chats to `~/.arula/conversations/`

**State Transitions**:
1. Chat mode: All input forwarded to `handle_ai_command()` for AI processing
2. Menu mode: ESC opens main menu, arrow keys navigate, Enter selects, Backspace goes back
3. Exit confirmation: Ctrl+C shows confirmation dialog before exiting

### Module Organization (by size and responsibility)

- **app.rs** (2058 lines): Application state, command routing, menu system, AI interaction logic
- **layout.rs** (631 lines): TUI rendering with `ScrollView`, responsive menu layouts, vertical/horizontal terminal detection
- **api.rs** (512 lines): Multi-provider AI client (OpenAI, Claude, Ollama, custom), streaming support via `async-openai`
- **ui_components.rs** (332 lines): Custom widgets (Theme, Gauge), color schemes (Cyberpunk, Matrix, Ocean, Sunset)
- **main.rs** (284 lines): Entry point, terminal setup, event loop, Ctrl+C graceful exit handling
- **git_ops.rs** (253 lines): Git wrapper using git2 with vendored-libgit2 and vendored-openssl
- **conversation.rs** (179 lines): Conversation persistence to `~/.arula/` directory with ARULA.md memory file
- **config.rs** (130 lines): YAML-based configuration (AI, Git, Logging, Art, Workspace settings)
- **art.rs** (92 lines): ASCII art generation (ARULA logo, Rust crab, fractals)
- **progress.rs** (67 lines): Progress indicators with indicatif spinner styles
- **chat.rs** (42 lines): Message types (User, AI, System, Error, Code)
- **cli_commands.rs** (29 lines): Shell command execution using duct

### Key Design Patterns

1. **Async AI Streaming**: AI responses stream via `tokio::sync::mpsc::unbounded_channel()`. The sender goes to `ApiClient::send_message_streaming()`, receiver stored in `app.ai_response_rx`. The main loop calls `app.check_ai_response()` to poll for `AiResponse` enum variants (StreamStart, StreamChunk, StreamEnd, Error).

2. **Channel-based Async**: Instead of `app.pending_command`, the app uses message passing channels for AI responses to avoid blocking the UI during streaming.

3. **Error Handling**: Uses `anyhow::Result` throughout for flexible error propagation with context.

4. **Message Persistence**: All messages go through `app.add_message()` which both updates `app.messages` and saves to `conversation_manager` for persistent storage in `~/.arula/conversations/`.

5. **Repository Pattern**: `GitOperations` maintains `Option<Repository>`, requiring `open_repository()` before operations. Check if repo is open before performing git operations.

6. **Responsive Layout**: `layout.rs::is_vertical_terminal()` detects terminal orientation (height > width * 1.2 or width < 50) and adapts menu layouts accordingly to prevent buffer overflow issues.

7. **State-driven UI**: UI rendering changes based on `AppState` (Chat/Menu/Exiting) and menu navigation uses index-based selection with `menu_selected`.

## Important Implementation Details

### AI Streaming Architecture
The app uses channel-based streaming for non-blocking AI responses:

```rust
// In app.rs::handle_ai_command():
let (tx, rx) = mpsc::unbounded_channel();
self.ai_response_rx = Some(rx);
self.is_ai_thinking = true;

// Spawn async task that sends responses through channel
tokio::spawn(async move {
    api_client.send_message_streaming(prompt, message_history, tx).await;
});

// In main loop (main.rs):
app.check_ai_response(); // Polls rx for new chunks

// In app.rs::check_ai_response():
if let Some(rx) = &mut self.ai_response_rx {
    while let Ok(response) = rx.try_recv() {
        match response {
            AiResponse::StreamChunk(chunk) => {
                // Append to current_streaming_message
            }
            AiResponse::StreamEnd => {
                // Finalize message, clear rx
            }
        }
    }
}
```

### Adding Menu Options
To add new menu functionality:
1. Add variant to `MenuOption` enum in `app.rs`
2. Add to appropriate menu in `App::menu_options()`
3. Add display text in `App::option_display()` and `App::option_info()`
4. Handle selection in `App::handle_menu_navigation()` Enter key match
5. For detail menus, add `MenuType` variant and content in `App::menu_content()`

### Git Operations
The `GitOperations` struct requires opening a repository before use:
```rust
// Always check if repo is open first
if let Err(_) = self.git_ops.open_repository(".") {
    self.add_message(MessageType::Error, "Not a git repository");
    return;
}
// Then perform operations
self.git_ops.create_branch("feature")?;
```

### Configuration Editing
In-app configuration editing uses `EditableField` enum:
```rust
// To make a field editable:
self.editing_field = Some(EditableField::ApiKey(current_value.clone()));

// Handle key events in handle_menu_navigation():
if let Some(field) = &mut self.editing_field {
    match field {
        EditableField::ApiKey(ref mut text) => {
            // Handle character input, backspace, etc.
        }
    }
}
```

### Terminal Compatibility
- **Termux/Android**:
  - Use `export TERM=xterm-256color` before running
  - Install required packages: `pkg install rust git clang`
  - If builds fail, check linker: `export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android-clang`
- **Terminal Requirements**: Binary requires actual terminal (checks `stdout().is_terminal()`)
- **Narrow terminals** (width < 50): Layout automatically switches to vertical mode to prevent buffer overflow
- **Mouse support**: Mouse capture enabled for scroll wheel (PageUp/PageDown also work)

## Testing Guidelines

Currently the project has minimal test coverage. When adding tests, follow these patterns:

### Testing Structure
```
arula-cli/
├── src/
│   ├── main.rs             # Entry point (no tests yet)
│   ├── app.rs              # Application logic (add unit tests in #[cfg(test)] mod tests)
│   ├── api.rs              # API client (add unit tests for message formatting)
│   ├── config.rs           # Configuration (add tests for load/save)
│   └── git_ops.rs          # Git operations (add tests with temp repos)
└── tests/                  # Integration tests (to be created)
    ├── cli_tests.rs        # CLI application tests with assert_cmd
    └── api_integration.rs  # API tests with mockito
```

### Testing Async Code with Tokio
Use `#[tokio::test]` for async functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_streaming() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        // Test streaming responses
        // ...
    }
}
```

### Mocking API Responses
For API tests, use mockito or wiremock (add to dev-dependencies):

```rust
#[tokio::test]
async fn test_openai_client() {
    let _m = mockito::mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"choices":[{"message":{"role":"assistant","content":"Hello"}}]}"#)
        .create();

    // Test client against mock
}
```

## Key Dependencies

### Core Runtime & UI
- **ratatui** (0.29): Modern TUI framework with all-widgets feature
- **crossterm** (0.28): Cross-platform terminal manipulation
- **tokio** (1.48): Async runtime with full features
- **tui-textarea** (0.7): Multi-line text input widget
- **tui-scrollview** (0.5.3): Scrollable view widget for chat history
- **tui-markdown** (0.3.6): Markdown rendering in TUI

### AI & Networking
- **async-openai** (0.24): OpenAI API client with streaming support
- **reqwest** (0.12): HTTP client with rustls-tls (no native OpenSSL), JSON, and streaming features
- **serde** & **serde_json**: JSON serialization for API requests

### Git & CLI
- **git2** (0.20): Git operations with vendored-libgit2 and vendored-openssl (avoids system OpenSSL)
- **duct** (1.1): Shell command execution
- **clap** (4.5): CLI argument parsing with derive macros

### UI Helpers
- **indicatif** (0.18): Progress bars and spinners
- **color-eyre** (0.6): Pretty error reporting
- **unicode-width** (0.1): Text width calculation for TUI
- **dirs** (5.0): Home directory detection for config storage

### Configuration & Serialization
- **serde_yaml** (0.9): YAML config file support
- **chrono** (0.4): Date/time handling with serde support

### Testing Dependencies (to be added)
Currently no dev-dependencies. Suggested additions:
- **mockito** or **wiremock**: HTTP mocking for API tests
- **assert_cmd**: CLI application testing
- **predicates**: Output assertions
- **tempfile**: Temporary file/directory testing for git operations