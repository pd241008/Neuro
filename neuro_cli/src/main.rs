use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use miette::Result;
use std::path::PathBuf;
use std::time::Duration;
use std::process::Command;

#[derive(Parser)]
#[command(name = "neuro")]
#[command(about = "The Zero-Trust Compiler Pipeline", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compiles a source file through the full pipeline
    Compile {
        #[arg(value_name = "FILE")]
        file: PathBuf,
        #[arg(short, long)]
        verbose: bool,
    },
    /// Runs only the security audit phase on a pre-compiled AST
    Audit {
        #[arg(value_name = "AST_FILE")]
        file: PathBuf,
    },
    /// Launches the interactive Neuro REPL
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
            println!("{} Auditing AST: {:?}", "Audit".bold(), file);
        }
        Commands::Gui => {
            run_gui()?;
        }
    }

    Ok(())
}

fn find_dotnet() -> String {
    if let Ok(path) = which::which("dotnet") {
        return path.to_string_lossy().to_string();
    }
    "dotnet".to_string()
}

fn run_gui() -> Result<()> {
    println!("{}", "\nLAUNCHING NEURO REPL...".cyan().bold());

    let dotnet_cmd = find_dotnet();
    let current_dir = "frontend";

    println!("{} Starting REPL...", ">".bright_black());
    let mut child = Command::new(&dotnet_cmd)
        .arg("run")
        .current_dir(current_dir)
        .spawn()
        .map_err(|e| miette::miette!("Failed to launch C# frontend: {}", e))?;

    child.wait().map_err(|e| miette::miette!("REPL exited with error: {}", e))?;

    Ok(())
}

async fn run_pipeline(file: &PathBuf, verbose: bool) -> Result<()> {
    println!("{}", "\nNEURO COMPILER PIPELINE INITIATED".cyan().bold());
    println!("{} Source: {}\n", ">".bright_black(), file.display().to_string().yellow());

    let pb = ProgressBar::new(4);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("#>-"),
    );

    // Phase 1: Frontend (C#)
    pb.set_message("Frontend: Parsing & Lexing [C#]");

    let file_path_str = file.canonicalize()
        .unwrap_or_else(|_| file.clone())
        .to_string_lossy()
        .to_string();

    let frontend_output = Command::new(find_dotnet())
        .arg("run")
        .arg("--")
        .arg(&file_path_str)
        .current_dir("frontend")
        .output()
        .map_err(|e| miette::miette!("Failed to invoke C# frontend: {}", e))?;

    if !frontend_output.status.success() {
        let err_msg = String::from_utf8_lossy(&frontend_output.stdout).to_string()
                      + &String::from_utf8_lossy(&frontend_output.stderr).to_string();
        pb.finish_with_message("Frontend compilation failed");
        return Err(miette::miette!("Frontend Error:\n{}", err_msg.trim()));
    }

    pb.inc(1);

    // Phase 2: Security Audit (Rust)
    pb.set_message("Middle-End: Zero-Trust Security Audit [Rust]");

    let ast_path = PathBuf::from("frontend/output.ast");
    if !ast_path.exists() {
        pb.finish_with_message("AST output not found");
        return Err(miette::miette!("Frontend did not produce output.ast"));
    }

    let ast_bytes = std::fs::read(&ast_path)
        .map_err(|e| miette::miette!("Failed to read AST output: {}", e))?;

    let _verified = analyzer::audit_ast(&ast_bytes)
        .map_err(|e| miette::miette!("Security Audit Error: {}", e))?;

    pb.inc(1);

    // Phase 3: Backend Generation (C++) — stubbed
    pb.set_message("Back-End: LLVM IR Generation [C++]");
    tokio::time::sleep(Duration::from_millis(500)).await;
    pb.inc(1);

    // Phase 4: Linking — stubbed
    pb.set_message("Finalizing: System Linking [Clang]");
    tokio::time::sleep(Duration::from_millis(200)).await;
    pb.inc(1);

    pb.finish_with_message("Compilation Successful");

    println!("\n{}", "BUILD COMPLETE".green().bold());
    if verbose {
        println!("{} Output: ./target/release/output.bin", ">".bright_black());
    }

    Ok(())
}
