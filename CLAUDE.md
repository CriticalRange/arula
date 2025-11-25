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
- `app.rs`: Application state and AI message handling (~1870 lines)
- `main.rs`: Event loop, command handling, AI response processing, menu integration
- `reedline_input.rs`: Modern reedline input handler with ExternalPrinter support
- `api/agent.rs`: Modern AI agent framework with type-safe tool calling
- `api/agent_client.rs`: Client for agent-based AI interactions
- `api/api.rs`: Traditional AI client with streaming support (legacy compatibility)
- `tools/tools.rs`: Modern tool implementations (BashTool, etc.)
- `ui/output.rs`: Colored terminal output to stdout
- `ui/menus/`: Complete menu system (main, config, conversation, dialogs)
- `ui/custom_spinner.rs`: Custom orbital spinner animations
- `utils/config.rs`: Multi-provider YAML/JSON configuration management
- `utils/chat.rs`: Chat message types and data structures
- `utils/conversation.rs`: Conversation persistence and loading
- `utils/changelog.rs`: Real-time changelog display functionality

**Dual AI Architecture**:
- **Legacy API**: Traditional streaming via `api.rs` for backward compatibility
- **Modern Agent**: Type-safe tool calling via `agent.rs` and `tools.rs`
- **AI Streaming**: Uses `tokio::sync::mpsc::unbounded_channel()` for non-blocking responses
- **Full-Duplex Terminal**: `ExternalPrinter` enables AI output while user types with reedline
- **Native Scrollback**: No alternate screen - all output flows to native terminal buffer

**Multi-Provider AI Support**: Supports OpenAI, Anthropic, Ollama, Z.AI, OpenRouter, and custom providers via `config.rs`

**CLI Interface**: Uses `clap` for command-line argument parsing with options:
- `--verbose`: Verbose mode output
- `--endpoint <url>`: API endpoint (default: http://localhost:8080)
- `--debug`: Debug mode for development

## Design Principles

**Core Principles Followed**:

1. **Single Responsibility Principle (SRP)**
   - Each module has one clear purpose
   - `output.rs` handles display, `app.rs` handles logic, menus handle interactions
   - Successfully organized from monolithic structure to modular components

2. **Don't Repeat Yourself (DRY)**
   - Extract common patterns into reusable functions
   - `OutputHandler` centralizes all terminal output formatting
   - Menu system uses shared components in `menus/common.rs`

3. **KISS Principle**
   - Keep code simple and straightforward
   - Direct stdout printing instead of complex render buffers
   - Modern reedline input with built-in history and completion

4. **Command-Query-Separation (CQS)**
   - Commands perform actions: `send_to_ai()`, `execute_bash_command()`
   - Queries return data: `get_config()`, `check_ai_response_nonblocking()`

5. **Encapsulation**
   - `OutputHandler` encapsulates colored output
   - Menu encapsulation with state and rendering
   - `ApiClient`/`AgentClient` encapsulate API communication

## Implementation Patterns

**Full-Duplex AI Streaming**:
```rust
// Create ExternalPrinter for concurrent output while typing
let external_printer = reedline_input.get_printer_sender();
app.set_external_printer(external_printer.clone());

// AI responses stream directly to ExternalPrinter while read_line() is active
// Achieves true full-duplex mode - user can type while AI streams responses
```

**Non-blocking Response Check**:
```rust
// In main loop
if let Some(response) = app.check_ai_response_nonblocking() {
    match response {
        AiResponse::AgentStreamStart => output.start_ai_message()?,
        AiResponse::AgentStreamText(chunk) => output.print_streaming_chunk(&chunk)?,
        AiResponse::AgentToolCall { id, name, arguments } => {
            // Handle tool calls automatically via agent framework
        }
        AiResponse::AgentStreamEnd => output.end_line()?,
    }
}
```

**Menu Integration Pattern**:
```rust
// Clear screen, enable raw mode, show menu, restore
execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
terminal::enable_raw_mode()?;
let result = menu.show(app, output)?;
terminal::disable_raw_mode()?;
execute!(stdout(), terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;
```

**Modern Tool Development Pattern**:
```rust
// Define tool parameters with serde
#[derive(Debug, Deserialize)]
pub struct MyToolParams {
    pub input: String,
}

// Implement the tool using async_trait
#[async_trait]
impl Tool for MyTool {
    type Params = MyToolParams;
    type Result = MyResult;

    fn name(&self) -> &str { "my_tool" }

    fn description(&self) -> &str { "Tool description" }

    async fn execute(&self, params: Self::Params) -> Result<Self::Result, String> {
        // Tool implementation with automatic error handling
    }
}
```

**Reedline Input with Multi-line Support**:
```rust
// Validator prevents empty input submission
impl Validator for ArulaValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        if line.trim().is_empty() {
            ValidationResult::Incomplete  // Prevents empty submission
        } else {
            ValidationResult::Complete
        }
    }
}

// Multi-line input with backslash continuation
// Shift+Enter for new line, Enter to send
// Ctrl+Space for completion menu
// ESC/Double-ESC for menu access
```

## Configuration

Multi-provider configuration system with JSON storage and automatic YAML migration:

**Configuration Structure**:
- `active_provider`: Currently selected AI provider
- `providers`: HashMap of provider configurations (OpenAI, Anthropic, Ollama, Z.AI, OpenRouter, custom)
- Automatic API key detection from environment variables
- Interactive configuration menu accessible via `/config` or menu system
- Model caching and background fetching for all providers

**Provider Support**:
- **OpenAI**: GPT models with official API
- **Anthropic**: Claude models with official API
- **Ollama**: Local models with configurable endpoints
- **Z.AI**: GLM models with coding-optimized endpoints
- **OpenRouter**: Aggregated access to multiple models
- **Custom**: User-defined endpoints and models

## Terminal & UI Features

**Reedline Integration**:
- Modern Emacs-style keybindings with full undo/redo
- Graphical columnar completion menu (Ctrl+Space)
- Inline history-based hints
- Context-aware syntax highlighting
- Dynamic prompt with AI status indicators (⚡🔧▶)
- Embedded orbital spinner animations
- Transient prompts (old prompts collapse)
- ExternalPrinter for concurrent AI output
- Clipboard integration (Ctrl+V/K/U/W)
- Multi-line input with prettier continuation (╎)
- Persistent history with immediate save
- Bracketed paste support

**Menu System**:
- **Main Menu**: Conversations, configuration, system status
- **Config Menu**: Provider switching, model selection, API keys
- **Conversation Menu**: Load, save, delete conversations
- **Dialog System**: Confirmations, input prompts, selections
- Crossterm-based with keyboard navigation
- Escape key handling and cancel support

**Output Formatting**:
- Colored terminal output using `console` and `nu-ansi-term`
- Markdown rendering with `termimad`
- Syntax highlighting for code blocks with `syntect`
- Progress indicators and spinners with `indicatif`
- Tool call formatting with icons and descriptions
- Error handling with colored error messages

## Conversation System

**Auto-Save Conversations**:
- Automatic conversation persistence to JSON files
- Real-time saving from background tokio tasks
- Shared conversation state between main thread and AI tasks
- Message history with tool calls and results
- Title generation from first user message
- Duration tracking and metadata

**Conversation Loading**:
- Full conversation restoration with tool calls
- Message type preservation (User, Assistant, Tool, System)
- Provider and model metadata
- Conversation browser and management

## Key Libraries & Dependencies

**Core Dependencies**:
- **reedline 0.43**: Modern readline replacement with ExternalPrinter for concurrent output
- **tokio 1.48**: Async runtime with full features for streaming and concurrent tasks
- **reqwest 0.12**: HTTP client with rustls-tls (no OpenSSL dependency)
- **serde + serde_json**: Serialization for configuration and conversations
- **anyhow + color-eyre**: Comprehensive error handling
- **clap 4.5**: Command-line argument parsing

**UI & Terminal**:
- **crossterm 0.28**: Cross-platform terminal manipulation
- **console 0.15**: Colored output with rich styling
- **nu-ansi-term 0.50**: Cross-platform color handling
- **termimad 0.20**: Markdown rendering for terminal
- **syntect 5.0**: Syntax highlighting for code blocks
- **indicatif 0.17**: Progress bars and spinners

**Tools & File Operations**:
- **memmap2 0.9**: Memory-mapped file operations
- **walkdir + ignore**: File system traversal with gitignore support
- **duct 0.13**: Command execution with proper I/O handling
- **async-trait 0.1**: Async trait support for tool interfaces

**Platform-Specific**:
- **windows 0.58**: Windows APIs for Visioneer desktop automation
- **uiautomation 0.13**: UI automation framework (Windows only)
- **screenshots 0.7**: Screen capture functionality (Windows only)

## Testing & Benchmarks

**Test Organization**:
- Unit tests for core components in `src/` modules
- Integration tests for tool execution
- Mockall for mocking external dependencies
- Wiremock for HTTP API testing
- Criterion benchmarks for performance-critical code

**Benchmark Categories**:
- `config_benchmarks`: Configuration loading and saving performance
- `chat_benchmarks`: Message processing and conversation management
- `tools_benchmarks`: Tool execution performance

## Recent Improvements & Features

### Full-Duplex ExternalPrinter Integration (Latest)
**Achievement**: True concurrent AI output while user types using reedline's ExternalPrinter

**Implementation**:
- AI responses stream directly to ExternalPrinter while `read_line()` is active
- User can type next message while AI is still responding
- Automatic markdown rendering and tool call formatting
- No more blocking - fully responsive terminal experience

### Reedline Modern Input System
**Features**:
- Multi-line input with Shift+Enter continuation
- Graphical completion menu with Ctrl+Space
- Dynamic prompt with AI status indicators and spinner
- History search and navigation
- Empty input prevention with comprehensive validation
- Escape mechanism (Ctrl+L) for incomplete states

### Comprehensive Menu System
**Structure**:
- Modular menu system with common components
- Interactive configuration with provider switching
- Conversation management with save/load
- Real-time model fetching and caching
- Keyboard navigation and accessibility

### Multi-Provider AI Architecture
**Providers**:
- OpenAI GPT models with official API integration
- Anthropic Claude models with streaming support
- Ollama local models with configurable endpoints
- Z.AI GLM models optimized for coding
- OpenRouter aggregated model access
- Custom provider support for any OpenAI-compatible endpoint

### Advanced Configuration Management
**Features**:
- JSON-based configuration with automatic YAML migration
- Environment variable integration for API keys
- Provider-specific defaults and model detection
- Background model fetching with caching
- Interactive configuration interface

### Tool Framework
**Implementation**:
- Type-safe tool calling with async execution
- Automatic parameter validation and error handling
- Extensible tool system with schema definitions
- Built-in tools: bash execution, file operations, search
- Windows-specific Visioneer desktop automation

## Development Patterns

**Error Handling**:
- Comprehensive error handling with `anyhow` and `color-eyre`
- Graceful degradation for network failures
- User-friendly error messages with context
- Debug logging with `ARULA_DEBUG` environment variable

**Async Architecture**:
- Non-blocking AI response handling
- Background model fetching
- Concurrent tool execution
- Shared state management with Arc<Mutex<>>

**Code Organization**:
- Modular structure with clear separation of concerns
- Feature-based organization in `src/` subdirectories
- Reusable components and utilities
- Comprehensive documentation and examples

## Performance Considerations

**Optimizations**:
- Connection pooling for HTTP requests
- Model caching to avoid repeated API calls
- Memory-mapped file operations for large files
- Async tool execution to prevent blocking
- Efficient terminal rendering with minimal redraws

**Memory Management**:
- Careful lifetime management for long-running operations
- Shared state with Arc<Mutex<>> for thread safety
- Cleanup of resources and cancellation tokens
- Proper channel handling for streaming responses

## Debugging & Troubleshooting

**Debug Mode**:
- Run with `--debug` flag or set `ARULA_DEBUG=1`
- Detailed logging of AI interactions and tool calls
- Request/response logging to `.arula/debug.log`
- Terminal state debugging and cursor management

**Common Issues**:
- Empty input prevention with multiple safety nets
- Terminal scroll positioning for AI responses
- Model caching and background updates
- Configuration migration and validation