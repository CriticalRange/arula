#![allow(dead_code)]
#![allow(unreachable_code)]
#![allow(private_interfaces)]

use anyhow::Result;
use clap::Parser;

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

use arula_cli::ui::output::OutputHandler;
use arula_cli::ui::tui_app::TuiApp;
use arula_core::utils::changelog::{Changelog, ChangelogType};
use arula_core::{detect_project, is_ai_enhanced};
use arula_core::App;
use std::path::PathBuf;

/// Print changelog from remote git or local file
fn print_changelog() -> Result<()> {
    // Fetch changelog (tries remote first, falls back to local)
    let changelog = Changelog::fetch_from_remote().unwrap_or_else(|_| {
        Changelog::fetch_local()
            .unwrap_or_else(|_| Changelog::parse(&Changelog::default_changelog()))
    });

    // Detect actual build type from git
    let build_type = Changelog::detect_build_type();
    let type_label = match build_type {
        ChangelogType::Release => "📦 Release",
        ChangelogType::Custom => "🔧 Custom Build",
        ChangelogType::Development => "⚙️  Development",
    };

    // Print header
    println!(
        "{} {}",
        console::style("📋 What's New").cyan().bold(),
        console::style(format!("({})", type_label)).dim()
    );

    // Get recent changes (limit to 5)
    let changes = changelog.get_recent_changes(5);

    if changes.is_empty() {
        println!("{}", console::style("  • No recent changes").dim());
    } else {
        for change in changes {
            println!("  {}", change);
        }
    }

    Ok(())
}

/// Print project context information
fn print_project_context() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    // Check for existing manifest
    let manifest_path = cwd.join("PROJECT.manifest");
    if manifest_path.exists() {
        let enhanced = is_ai_enhanced(&manifest_path);
        let status = if enhanced { "✨ AI-Enhanced" } else { "📄 Auto-Generated" };
        println!(
            "{} {}",
            console::style("📁 Project:").cyan().bold(),
            console::style(status).yellow()
        );
    } else if let Some(project) = detect_project(&cwd) {
        // Show detected project info
        println!(
            "{} {}",
            console::style("📁 Detected Project:").cyan().bold(),
            console::style(&project.name).white().bold()
        );

        let type_str = project.project_type.as_str();
        let type_style = match project.project_type {
            arula_core::ProjectType::Rust => console::style(type_str).red(),
            arula_core::ProjectType::Node => console::style(type_str).green(),
            arula_core::ProjectType::Python => console::style(type_str).blue(),
            arula_core::ProjectType::Go => console::style(type_str).cyan(),
            arula_core::ProjectType::Unknown => console::style(type_str).dim(),
        };

        println!(
            "   {} {}",
            console::style("Type:").dim(),
            type_style
        );

        if let Some(ref framework) = project.framework {
            println!(
                "   {} {}",
                console::style("Framework:").dim(),
                console::style(framework).white()
            );
        }

        let dep_count = project.dependencies.len();
        if dep_count > 0 {
            println!(
                "   {} {} {}",
                console::style("Dependencies:").dim(),
                console::style(dep_count.to_string()).white(),
                console::style(if dep_count == 1 { "package" } else { "packages" }).dim()
            );
        }

        // Show hint to create manifest
        println!(
            "   {} {}",
            console::style("→").cyan(),
            console::style("Run /menu → Create Project Manifest to enhance AI context")
                .dim()
        );
    } else {
        // No project detected
        println!(
            "{} {}",
            console::style("📁 Project:").cyan().bold(),
            console::style("No project detected in current directory").dim()
        );
    }

    Ok(())
}

/// Print conversation starter recommendations
fn print_conversation_starters() -> Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"));

    // Generate context-aware starters
    let starters = if let Some(project) = detect_project(&cwd) {
        match project.project_type {
            arula_core::ProjectType::Rust => vec![
                "Review and improve code quality",
                "Run tests and fix any issues",
                "Add new feature with proper error handling",
            ],
            arula_core::ProjectType::Node => vec![
                "Review dependencies and update outdated packages",
                "Add tests for critical functions",
                "Improve error handling and logging",
            ],
            arula_core::ProjectType::Python => vec![
                "Review code for PEP 8 compliance",
                "Add type hints to improve code clarity",
                "Write unit tests for core functionality",
            ],
            arula_core::ProjectType::Go => vec![
                "Review code for idiomatic Go patterns",
                "Add comprehensive error handling",
                "Write benchmarks for performance",
            ],
            arula_core::ProjectType::Unknown => vec![
                "Explain the project structure",
                "Suggest improvements to code organization",
                "Add documentation for key components",
            ],
        }
    } else {
        // Default starters when no project detected
        vec![
            "Start a new conversation",
            "Ask about my capabilities",
            "Get help with a task",
        ]
    };

    println!(
        "{} {}",
        console::style("💬 Starter Recommendations").cyan().bold(),
        console::style("(Ctrl+1/2/3 to send)").dim()
    );

    for (i, starter) in starters.iter().enumerate() {
        let key_num = i + 1;
        println!(
            "   {} {} {}",
            console::style(format!("Ctrl+{}:", key_num)).cyan().bold(),
            console::style(starter).white(),
            console::style(format!("(Press Ctrl+{})", key_num)).dim()
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set debug environment variable if debug flag is enabled
    if cli.debug {
        unsafe {
            std::env::set_var("ARULA_DEBUG", "1");
        }
    }

    // Initialize global logger
    if let Err(e) = arula_core::utils::logger::init_global_logger() {
        eprintln!("⚠️ Failed to initialize logger: {}", e);
    }

    // Create app with debug flag
    let mut app = App::new()?.with_debug(cli.debug);

    // Initialize app components
    let _ = app.initialize_git_state().await;
    let _ = app.initialize_tool_registry().await;
    let _ = app.initialize_agent_client();

    // Print banner and changelog BEFORE entering TUI
    let output = OutputHandler::new();
    output.print_banner()?;
    println!();
    print_changelog()?;
    print_project_context()?;
    println!();
    print_conversation_starters()?;
    println!();

    // Run TUI
    let mut tui = TuiApp::new(app)?;
    tui.run().await?;

    Ok(())
}
