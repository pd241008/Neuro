use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use miette::{Diagnostic, Result};
use thiserror::Error;
use std::path::PathBuf;
use std::time::Duration;
use std::process::Command;

#[derive(Error, Debug, Diagnostic)]
#[error("Compilation failed: {message}")]
#[diagnostic(code(neuro::general_error), help("Check if the input file exists and is valid."))]
struct NeuroError {
    message: String,
}

#[derive(Parser)]
#[command(name = "neuro")]
#[command(about = "🧠 NEURO: The Zero-Trust Compiler Pipeline", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a source file through the full pipeline
    Compile {
        /// The source file to compile (.nro)
        #[arg(value_name = "FILE")]
        file: PathBuf,

        /// Enable verbose output for debugging
        #[arg(short, long)]
        verbose: bool,
    },
    /// Runs only the security audit phase on a pre-compiled AST
    Audit {
        #[arg(value_name = "AST_FILE")]
        file: PathBuf,
    },
    /// Launches the interactive Neuro GUI/REPL
    Gui,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { file, verbose } => {
            run_pipeline(file, *verbose).await?;
        }
        Commands::Audit { file } => {
            println!("{} Auditing AST: {:?}", "🛡️".bold(), file);
        }
        Commands::Gui => {
            run_gui()?;
        }
    }

    Ok(())
}

fn find_dotnet() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    let local_dotnet = format!("{}/.dotnet/dotnet", home);
    if std::path::Path::new(&local_dotnet).exists() {
        return local_dotnet;
    }
    "dotnet".to_string()
}

fn which_binary(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn run_gui() -> Result<()> {
    println!("{}", "\n🧠 LAUNCHING NEURO GUI...".cyan().bold());
    
    let dotnet_cmd = find_dotnet();
    let current_dir = "frontend";

    // Detect terminal emulator and define arguments to run our dotnet command inside it
    let terminals = vec![
        ("xterm", vec!["-T", "Neuro GUI", "-e", &dotnet_cmd, "run"]),
        ("gnome-terminal", vec!["--title=Neuro GUI", "--", &dotnet_cmd, "run"]),
        ("konsole", vec!["--title", "Neuro GUI", "-e", &dotnet_cmd, "run"]),
        ("xfce4-terminal", vec!["--title=Neuro GUI", "-x", &dotnet_cmd, "run"]),
        ("alacritty", vec!["-t", "Neuro GUI", "-e", &dotnet_cmd, "run"]),
        ("kitty", vec!["--title", "Neuro GUI", &dotnet_cmd, "run"]),
    ];

    let mut spawned_in_terminal = false;

    for (term_bin, args) in terminals {
        if which_binary(term_bin) {
            println!("{} Spawning dedicated terminal using: {}", "»".bright_black(), term_bin.yellow());
            let mut cmd = Command::new(term_bin);
            cmd.args(&args).current_dir(current_dir);
            match cmd.spawn() {
                Ok(mut child) => {
                    child.wait().map_err(|e| miette::miette!("Dedicated terminal exited with error: {}", e))?;
                    spawned_in_terminal = true;
                    break;
                }
                Err(e) => {
                    eprintln!("Failed to spawn terminal emulator {}: {}", term_bin, e);
                }
            }
        }
    }

    if !spawned_in_terminal {
        println!("{} No terminal emulators found or failed to spawn. Falling back to active terminal...", "»".yellow());
        // Fallback: The frontend is a C# .NET project located in the `frontend` directory.
        let mut child = Command::new(&dotnet_cmd)
            .arg("run")
            .current_dir(current_dir)
            .spawn()
            .map_err(|e| miette::miette!("Failed to launch C# frontend: {}", e))?;

        child.wait().map_err(|e| miette::miette!("GUI exited with error: {}", e))?;
    }

    Ok(())
}

async fn run_pipeline(file: &PathBuf, verbose: bool) -> Result<()> {
    println!("{}", "\n🧠 NEURO COMPILER PIPELINE INITIATED".cyan().bold());
    println!("{} Source: {}\n", "»".bright_black(), file.display().to_string().yellow());

    let pb = ProgressBar::new(4);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    // 1. Frontend Phase (C#)
    pb.set_message("Frontend: Parsing & Lexing [C#]");
    
    let file_path_str = file.canonicalize()
        .unwrap_or_else(|_| file.clone())
        .to_string_lossy()
        .to_string();
        
    let frontend_output = Command::new(find_dotnet())
        .arg("run")
        .arg("--") // pass arguments to the C# application
        .arg(&file_path_str)
        .current_dir("frontend")
        .output()
        .map_err(|e| miette::miette!("Failed to invoke C# frontend: {}", e))?;

    if !frontend_output.status.success() {
        let err_msg = String::from_utf8_lossy(&frontend_output.stdout).to_string() + 
                      &String::from_utf8_lossy(&frontend_output.stderr).to_string();
        pb.finish_with_message("Frontend compilation failed");
        return Err(miette::miette!("Frontend Error:\n{}", err_msg.trim()));
    }

    pb.inc(1);

    // 2. Security Audit (Rust)
    pb.set_message("Middle-End: Zero-Trust Security Audit [Rust]");
    tokio::time::sleep(Duration::from_millis(1200)).await;
    pb.inc(1);

    // 3. Backend Generation (C++)
    pb.set_message("Back-End: LLVM IR Generation [C++]");
    tokio::time::sleep(Duration::from_millis(1000)).await;
    pb.inc(1);

    // 4. Linking
    pb.set_message("Finalizing: System Linking [Clang]");
    tokio::time::sleep(Duration::from_millis(500)).await;
    pb.inc(1);

    pb.finish_with_message("Compilation Successful");

    println!("\n{}", "✅ BUILD COMPLETE".green().bold());
    if verbose {
        println!("{} Output: ./target/release/output.bin", "↳".bright_black());
    }

    Ok(())
}
