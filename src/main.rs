use anyhow::Result;
use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;

#[derive(Parser)]
#[command(name = "arula")]
#[command(about = "ARULA CLI - Autonomous AI Interface with chat", long_about = None)]
struct Cli {
    /// Run in verbose mode
    #[arg(short, long)]
    verbose: bool,

    /// API endpoint to connect to
    #[arg(long, default_value = "http://localhost:8080")]
    endpoint: String,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
}

mod app;
mod chat;
mod config;
mod output;
mod api;
mod tool_call;
mod overlay_menu;

use app::App;
use output::OutputHandler;
use overlay_menu::OverlayMenu;

#[tokio::main]
async fn main() -> Result<()> {
    // Install color-eyre for better error reporting
    let _ = color_eyre::install();

    let cli = Cli::parse();

    if cli.verbose {
        println!("🚀 Starting ARULA CLI with endpoint: {}", cli.endpoint);
    }

    // Create output handler and app
    let mut output = OutputHandler::new();
    let mut app = App::new()?;

    // Initialize AI client if configuration is valid
    match app.initialize_api_client() {
        Ok(()) => {
            if cli.verbose {
                println!("✅ AI client initialized successfully");
            }
        }
        Err(e) => {
            if cli.verbose {
                println!("⚠️  AI client initialization failed: {}", e);
                println!("💡 You can configure AI settings in the application menu");
            }
        }
    }

    // Print banner
    output.print_banner()?;
    println!();

    // Create rustyline editor
    let mut rl = DefaultEditor::new()?;

    // Create overlay menu
    let mut menu = OverlayMenu::new();

    // Load history if exists
    let history_path = dirs::home_dir()
        .map(|p| p.join(".arula_history"))
        .unwrap_or_else(|| std::path::PathBuf::from(".arula_history"));

    let _ = rl.load_history(&history_path);

    // Main input loop
    loop {
        // Check for AI responses (non-blocking)
        if let Some(response) = app.check_ai_response_nonblocking() {
            match response {
                app::AiResponse::StreamStart => {
                    output.start_ai_message()?;
                }
                app::AiResponse::StreamChunk(chunk) => {
                    output.print_streaming_chunk(&chunk)?;
                }
                app::AiResponse::StreamEnd => {
                    output.end_line()?;
                    // Execute bash commands if any
                    if let Some(commands) = app.get_pending_bash_commands() {
                        for cmd in commands {
                            output.print_system(&format!("Executing: {}", cmd))?;
                            match app.execute_bash_command(&cmd).await {
                                Ok(result) => {
                                    output.print_tool_result(&result)?;
                                }
                                Err(e) => {
                                    output.print_error(&format!("Command failed: {}", e))?;
                                }
                            }
                        }
                    }
                }
                app::AiResponse::Success { response, usage: _ } => {
                    output.print_ai_message(&response)?;
                    // Execute bash commands if any
                    if let Some(commands) = app.get_pending_bash_commands() {
                        for cmd in commands {
                            output.print_system(&format!("Executing: {}", cmd))?;
                            match app.execute_bash_command(&cmd).await {
                                Ok(result) => {
                                    output.print_tool_result(&result)?;
                                }
                                Err(e) => {
                                    output.print_error(&format!("Command failed: {}", e))?;
                                }
                            }
                        }
                    }
                }
                app::AiResponse::Error(error_msg) => {
                    output.print_error(&error_msg)?;
                }
            }
        }

        // Read user input with rustyline
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                let input = line.trim();

                // Check for empty input
                if input.is_empty() {
                    continue;
                }

                // Add to history
                let _ = rl.add_history_entry(input);

                // Check for special shortcuts
                if input == "m" || input == "menu" {
                    // Quick menu shortcut
                    if menu.show_main_menu(&mut app, &mut output)? {
                        break;
                    }
                    continue;
                }

                // Check for exit commands
                if input == "exit" || input == "quit" {
                    output.print_system("Goodbye! 👋")?;
                    break;
                }

                // Print user message
                output.print_user_message(input)?;

                // Handle command
                if input.starts_with('/') {
                    // Handle CLI commands
                    handle_cli_command(input, &mut app, &mut output, &mut menu).await?;
                } else {
                    // Send to AI
                    app.send_to_ai(input).await?;
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C - Show exit confirmation
                if menu.show_exit_confirmation(&mut output)? {
                    // Exit confirmed
                    break;
                }
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D
                output.print_system("Goodbye! 👋")?;
                break;
            }
            Err(err) => {
                output.print_error(&format!("Error: {:?}", err))?;
                break;
            }
        }
    }

    // Save history
    let _ = rl.save_history(&history_path);

    Ok(())
}

async fn handle_cli_command(
    input: &str,
    app: &mut App,
    output: &mut OutputHandler,
    menu: &mut OverlayMenu,
) -> Result<()> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    let command = parts[0];

    match command {
        "/help" => {
            output.print_system("╔══════════════════════════════════════╗")?;
            output.print_system("║          ARULA HELP MENU             ║")?;
            output.print_system("╚══════════════════════════════════════╝")?;
            output.print_system("")?;
            output.print_system("📋 Commands:")?;
            output.print_system("  /help              - Show this help")?;
            output.print_system("  /menu              - Open interactive menu")?;
            output.print_system("  /clear             - Clear conversation history")?;
            output.print_system("  /config            - Show current configuration")?;
            output.print_system("  /model <name>      - Change AI model")?;
            output.print_system("  exit or quit       - Exit ARULA")?;
            output.print_system("")?;
            output.print_system("⌨️  Quick Shortcuts:")?;
            output.print_system("  m         - Open menu (type 'm' and press Enter)")?;
            output.print_system("  menu      - Open menu")?;
            output.print_system("  Ctrl+C    - Exit confirmation")?;
            output.print_system("  Ctrl+D    - Exit immediately")?;
            output.print_system("")?;
            output.print_system("💡 TIP: Just type 'm' to open the menu anytime!")?;
        }
        "/menu" => {
            // Show menu
            if menu.show_main_menu(app, output)? {
                // Exit requested
                std::process::exit(0);
            }
        }
        "/clear" => {
            app.clear_conversation();
            output.print_system("Conversation cleared")?;
        }
        "/config" => {
            let config = app.get_config();
            output.print_system(&format!("Provider: {}", config.ai.provider))?;
            output.print_system(&format!("Model: {}", config.ai.model))?;
            output.print_system(&format!("API Key: {}", if config.ai.api_key.is_empty() { "Not set" } else { "Set" }))?;
        }
        "/model" => {
            if parts.len() < 2 {
                output.print_error("Usage: /model <name>")?;
            } else {
                let model = parts[1];
                app.set_model(model);
                output.print_system(&format!("Model changed to: {}", model))?;
            }
        }
        _ => {
            output.print_error(&format!("Unknown command: {}", command))?;
            output.print_system("Type /help for available commands")?;
        }
    }

    Ok(())
}
