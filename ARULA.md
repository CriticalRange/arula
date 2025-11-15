# ARULA CLI - Project Instructions

## Project Overview
This is the ARULA CLI (Autonomous AI command-line interface) project built with Rust.

## Key Architecture
- **Core Flow**: `main()` → rustyline readline loop → `app.send_to_ai()` / `app.check_ai_response_nonblocking()`
- **Agent System**: Uses modern agent architecture with tool calling
- **API Integration**: Supports OpenAI, Claude, Ollama, and Z.AI providers
- **Tool Suite**: 5 built-in tools for file operations and command execution

## Development Commands
```bash
cargo build && cargo run         # Build and run
cargo clippy && cargo fmt        # Code quality
cargo test                       # Run tests
cargo run -- --debug            # Run with debug output
```

## Important Files
- `src/main.rs` - Entry point with rustyline input loop
- `src/app.rs` - Application state and AI message handling
- `src/agent_client.rs` - Modern agent client with tool calling
- `src/tools.rs` - Tool implementations (list_directory, read_file, write_file, edit_file, execute_bash)
- `src/api.rs` - HTTP client for AI providers with connection pooling
- `src/output.rs` - Colored terminal output formatting
- `src/overlay_menu.rs` - Interactive menu system using dialoguer

## Key Features
- Native terminal scrollback (no alternate screen)
- Modern tool calling with automatic execution
- Context-aware AI with project-specific instructions
- Robust error handling and automatic backups
- Debug mode for troubleshooting

## Design Principles
- Single Responsibility Principle (SRP)
- Don't Repeat Yourself (DRY)
- KISS Principle - Keep code simple
- Command-Query-Separation (CQS)
- Proper encapsulation

## Testing Notes
- All 5 tools are fully tested and working
- Network connectivity is stable across all providers
- Tool calling workflow is end-to-end functional
- UI/UX improvements with modern dialog components
