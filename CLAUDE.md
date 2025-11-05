# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview
ARULA CLI is an autonomous AI command-line interface built with Rust and Ratatui. It features a chat-style TUI for interacting with AI, executing shell commands, and managing Git operations autonomously.

## Common Development Commands

```bash
# Build and run
cargo build                    # Debug build
cargo run                      # Run in debug mode
cargo build --release          # Optimized release build
cargo run -- --verbose         # Run with verbose logging
cargo run -- --endpoint URL    # Run with custom API endpoint

# Code quality
cargo check                    # Fast compilation check
cargo clippy                   # Linting
cargo fmt                      # Format code
cargo test                     # Run tests

# Termux-specific (for Android development)
export TERM=xterm-256color     # Set terminal type before running
pkg install rust git clang     # Install required packages
```

## Architecture Overview

### Core Event Loop (main.rs)
The application uses an async event loop with:
- **Crossterm** for terminal raw mode and event handling
- **Ratatui** for rendering the TUI
- **Tokio** async runtime for non-blocking operations
- Event polling every 50ms for responsive input
- Separate handling for keyboard, mouse, and focus events

**Key Flow**: `main()` → `run_app()` → event loop → `app.handle_key_event()` or `app.handle_command()`

### Application State (app.rs)
`App` struct is the central state manager:
- **Messages**: Chat history stored in `Vec<ChatMessage>`
- **Input handling**: Cursor position tracking, input buffer
- **Async commands**: `pending_command` field for async execution
- **Git operations**: `git_ops` field wraps git2 library
- **API client**: Optional HTTP client for remote AI communication

**Command Routing**:
1. Commands starting with `/` are built-in (e.g., `/git`, `/exec`)
2. All other input is forwarded to AI via `handle_ai_command()`
3. Built-in commands are handled synchronously in `handle_builtin_command()`

### Module Organization

- **main.rs**: Entry point, terminal setup, event loop
- **app.rs**: Application state, command routing, business logic (932 lines - largest module)
- **layout.rs**: TUI rendering with Ratatui widgets
- **ui_components.rs**: Custom UI components (Theme, Gauge)
- **chat.rs**: Message types and data structures
- **api.rs**: HTTP client for AI API integration (reqwest)
- **git_ops.rs**: Git wrapper using git2 library
- **cli_commands.rs**: Shell command execution using duct
- **art.rs**: ASCII art generation
- **config.rs**: Configuration structures (YAML-based)

### Key Design Patterns

1. **Async Command Execution**: Commands set `app.pending_command`, which is processed in the main loop to avoid blocking the UI
2. **Error Handling**: Uses `anyhow::Result` throughout for flexible error propagation
3. **Message-based UI**: All output goes through `app.add_message()` which limits history to 50 messages
4. **Repository Pattern**: `GitOperations` maintains optional `Repository` instance, requiring `open_repository()` before operations

### Development Principles

- **Single Responsibility**: Each module has one clear purpose
- **Consistent Naming**: Use simple, descriptive names (e.g., `render_chat()` not `render_conversation_interface()`)
- **Predictable Patterns**: Same approach for similar functionality
- **Consistent Error Handling**: Use `anyhow::Result` throughout

## Important Implementation Details

### Adding New Commands
To add a new built-in command:
1. Add pattern matching in `app.rs::handle_builtin_command()`
2. Commands starting with `/` are reserved for built-in features
3. All other input is forwarded to AI via `handle_ai_command()`
4. Use `app.add_message()` to display output to user

### Git Operations
The `GitOperations` struct requires opening a repository before use:
```rust
// Always check if repo is open first
if let Err(_) = self.git_ops.open_repository(".") {
    // Handle error - not a git repo
    return;
}
// Then perform operations
self.git_ops.create_branch("feature")?;
```

### Async Command Pattern
For long-running operations, use the pending command pattern:
```rust
// In handle_key_event (sync context):
self.pending_command = Some(command);

// In main loop (async context):
if let Some(command) = app.pending_command.take() {
    app.handle_command(command).await;
}
```

### Terminal Compatibility (Termux/Android)
- Use `export TERM=xterm-256color` before running
- Install required packages: `pkg install rust git clang`
- If builds fail, check linker: `export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=aarch64-linux-android-clang`
- Binary must run in actual terminal (checks `stdout().is_terminal()`)

## Key Dependencies
- **ratatui** (0.28): TUI framework with all-widgets feature
- **crossterm** (0.28): Cross-platform terminal manipulation
- **tokio** (1.48): Async runtime with full features
- **git2** (0.20): Git operations with vendored-libgit2 and vendored-openssl
- **reqwest** (0.12): HTTP client with rustls-tls (no native OpenSSL)
- **clap** (4.5): CLI argument parsing
- **duct** (1.1): Shell command execution
- **indicatif** (0.18): Progress bars