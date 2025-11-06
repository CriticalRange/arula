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

# Testing commands
cargo test                     # Run all tests
cargo test --lib               # Run unit tests only
cargo test --test integration  # Run integration tests only
cargo test unit_tests         # Run specific test module
cargo test api::tests          # Run tests in specific module
cargo test -- --ignored        # Run ignored tests
cargo test -- --show-output    # Show test output
cargo test -- --test-threads=1 # Run tests sequentially
cargo test -- --nocapture      # Don't capture stdout

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

## Testing Guidelines

### Testing Structure
```
project/
├── src/
│   ├── lib.rs              # Main library code
│   ├── api.rs              # API client - unit tests in module
│   ├── app.rs              # Application logic - unit tests in module
│   ├── config.rs           # Configuration - unit tests in module
│   └── git_ops.rs          # Git operations - unit tests in module
├── tests/
│   ├── integration_test.rs # Integration tests (black-box)
│   ├── api_integration.rs  # API integration tests
│   └── cli_tests.rs        # CLI application tests
└── benches/
    └── performance.rs      # Performance benchmarks
```

### Unit Tests
Unit tests test individual modules in isolation and can access private functions:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_client_creation() {
        let client = ApiClient::new(
            "openai".to_string(),
            "https://api.openai.com".to_string(),
            "test_key".to_string(),
            "gpt-3.5-turbo".to_string(),
        );
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_async_ai_request() {
        // Test async functionality
        let result = some_async_function().await;
        assert!(result.is_ok());
    }

    #[test]
    #[should_panic(expected = "API key required")]
    fn test_missing_api_key() {
        // Test that function panics with specific message
        create_client_without_key();
    }
}
```

### Integration Tests
Integration tests are external to your library and test public APIs:

```rust
// tests/api_integration.rs
use arula_cli::{App, Config, ApiClient};

#[test]
fn test_full_ai_workflow() {
    // Test complete AI workflow from configuration to response
    let config = Config::load();
    let mut app = App::new().unwrap();

    // Test initialization
    assert!(app.initialize_api_client().is_ok());

    // Test message sending (mocked)
    // ... integration test logic
}

#[tokio::test]
async fn test_real_api_connection() {
    // Only run if API keys are available
    if std::env::var("OPENAI_API_KEY").is_ok() {
        let client = ApiClient::new(/* config */);
        let result = client.test_connection().await;
        assert!(result.is_ok());
    }
}
```

### Testing Async Code
Use `#[tokio::test]` for async functions:

```rust
#[tokio::test]
async fn test_async_functionality() {
    let result = async_operation().await;
    assert_eq!(result, expected_value);
}
```

### Testing CLI Applications
Use `assert_cmd` for CLI testing:

```bash
# Add to Cargo.toml:
[dev-dependencies]
assert_cmd = "2.0"
predicates = "3.0"
```

```rust
use assert_cmd::Command;

#[test]
fn test_cli_help() {
    Command::cargo_bin("arula-cli")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicates::str::contains("ARULA CLI"));
}
```

### Testing Environment Variables
Control test environment with conditional compilation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_with_env_vars() {
        // Set up test environment
        env::set_var("OPENAI_API_KEY", "test_key");

        // Run test
        let result = function_that_uses_env_var();
        assert!(result.is_ok());

        // Clean up
        env::remove_var("OPENAI_API_KEY");
    }
}
```

### Mocking HTTP Requests
Use mock servers for API testing:

```bash
# Add to Cargo.toml:
[dev-dependencies]
mockito = "1.0"
wiremock = "0.5"
```

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url};

    #[tokio::test]
    async fn test_api_with_mock() {
        let _mock = mock("POST", "/chat/completions")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"choices":[{"message":{"content":"Hello!"}}]}"#)
            .create();

        let client = ApiClient::new("openai", &server_url(), "test_key", "gpt-3.5-turbo");
        let result = client.send_message("Hello", None).await;
        assert!(result.is_ok());
    }
}
```

### When to Check Each Documentation

#### **Rust Official Documentation**
- **When**: Learning Rust syntax, ownership, borrowing, or standard library features
- **How**: `https://doc.rust-lang.org/book/` or `https://doc.rust-lang.org/reference/`
- **Use case**: Understanding `#[test]`, `assert!`, async patterns, error handling

#### **Rust by Example**
- **When**: Need practical examples of Rust patterns and idioms
- **How**: `https://doc.rust-lang.org/rust-by-example/`
- **Use case**: Quick reference for testing syntax, common patterns, best practices

#### **Tokio Documentation**
- **When**: Working with async code, networking, or concurrent operations
- **How**: `https://tokio.rs/tokio/topics/testing`
- **Use case**: Testing async functions, mocking async operations, handling tokio runtime

#### **Crates.io Documentation**
- **When**: Using external crates (ratatui, reqwest, git2, etc.)
- **How**: `https://docs.rs/<crate-name>/<version>/`
- **Use case**: Understanding crate APIs, testing utilities, configuration options

#### **Context7 (Current Method)**
- **When**: Need comprehensive, searchable documentation with code examples
- **How**: Use `mcp__context7` tools in Claude Code
- **Use case**: Quick lookup of specific APIs, patterns, or implementations

### Testing Best Practices for ARULA CLI

1. **Test Configuration Loading**: Verify configuration loads correctly from files and environment variables
2. **Test API Client**: Mock HTTP responses for reliable testing without external dependencies
3. **Test Git Operations**: Use temporary git repositories for testing git functionality
4. **Test CLI Interface**: Use `assert_cmd` to test command-line arguments and exit codes
5. **Test Error Handling**: Verify proper error messages and graceful failures
6. **Test Async Operations**: Use `#[tokio::test]` for all async functionality
7. **Test Terminal UI**: Consider snapshot testing for TUI components if needed

### Running Tests in Different Scenarios

```bash
# Run all tests including doctests
cargo test

# Run only unit tests (in src/ files)
cargo test --lib

# Run only integration tests (in tests/ directory)
cargo test --test integration

# Run specific test module
cargo test api::tests

# Run tests matching a pattern
cargo test config

# Run tests with verbose output
cargo test -- --nocapture

# Run tests sequentially (useful for tests with shared state)
cargo test -- --test-threads=1

# Run ignored tests (for expensive or slow tests)
cargo test -- --ignored

# Run specific test by exact name
cargo test test_api_client_creation

# Run tests for a specific package in workspace
cargo test -p arula-cli
```

## Key Dependencies
- **ratatui** (0.28): TUI framework with all-widgets feature
- **crossterm** (0.28): Cross-platform terminal manipulation
- **tokio** (1.48): Async runtime with full features
- **git2** (0.20): Git operations with vendored-libgit2 and vendored-openssl
- **reqwest** (0.12): HTTP client with rustls-tls (no native OpenSSL)
- **clap** (4.5): CLI argument parsing
- **duct** (1.1): Shell command execution
- **indicatif** (0.18): Progress bars

### Testing Dependencies (dev-dependencies)
- **mockito**: HTTP mocking for API tests
- **wiremock**: Mock HTTP servers
- **assert_cmd**: CLI application testing
- **predicates**: Output assertions for CLI tests
- **tokio-test**: Async testing utilities
- **tempfile**: Temporary file testing