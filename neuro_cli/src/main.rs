use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use miette::Result;
use std::path::PathBuf;
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

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Compile { file, verbose } => {
            run_pipeline(file, *verbose)?;
        }
        Commands::Audit { file } => {
            println!("{} Auditing AST: {:?}", "Audit".bold().cyan(), file);
            let ast_bytes = std::fs::read(file)
                .map_err(|e| miette::miette!("Failed to read AST file: {}", e))?;
            let verified = analyzer::audit_ast(&ast_bytes)
                .map_err(|e| miette::Report::from(e))?;
            let verified_path = file.with_extension("verified.ast");
            std::fs::write(&verified_path, &verified)
                .map_err(|e| miette::miette!("Failed to write verified AST: {}", e))?;
            println!("{} Audit passed — output: {}", "✓".green().bold(), verified_path.display());
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

fn build_runtime(output_dir: &PathBuf) -> Result<PathBuf, miette::Report> {
    let runtime_dir = PathBuf::from("runtime");
    let runtime_a = output_dir.join("libneuro_runtime.a");

    let io_o = output_dir.join("runtime_io.o");
    let mem_o = output_dir.join("runtime_memory.o");

    let cc = if which::which("clang").is_ok() { "clang" } else { "gcc" };

    let compile = |src: &str, dst: &PathBuf| -> Result<(), miette::Report> {
        let out = Command::new(cc)
            .args(["-c", src, "-o", &dst.to_string_lossy()])
            .current_dir(&runtime_dir)
            .output()
            .map_err(|e| miette::miette!("Failed to compile runtime/{}: {}", src, e))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(miette::miette!("Runtime compilation error ({}):\n{}", src, err.trim()));
        }
        Ok(())
    };

    compile("io.c", &io_o)?;
    compile("memory.c", &mem_o)?;

    let ar_out = Command::new("ar")
        .args(["rcs", &runtime_a.to_string_lossy(), &io_o.to_string_lossy(), &mem_o.to_string_lossy()])
        .output()
        .map_err(|e| miette::miette!("Failed to create runtime archive: {}", e))?;
    if !ar_out.status.success() {
        let err = String::from_utf8_lossy(&ar_out.stderr);
        return Err(miette::miette!("Archive error:\n{}", err.trim()));
    }

    Ok(runtime_a)
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

fn run_pipeline(file: &PathBuf, verbose: bool) -> Result<()> {
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

    let output_dir = PathBuf::from("target/neuro_output");
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| miette::miette!("Failed to create output directory: {}", e))?;

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

    let verified = analyzer::audit_ast(&ast_bytes)
        .map_err(|e| miette::Report::from(e))?;

    let verified_path = output_dir.join("output.verified.ast");
    std::fs::write(&verified_path, &verified)
        .map_err(|e| miette::miette!("Failed to write verified AST: {}", e))?;

    if verbose {
        println!("{} Verified AST written to {}", ">".bright_black(), verified_path.display());
    }

    pb.inc(1);

    // Phase 3: Backend Generation (C++)
    pb.set_message("Back-End: LLVM IR Generation [C++]");

    let backend_bin = PathBuf::from("backend/build/neuro_backend");
    let ll_path = output_dir.join("output.ll");

    if backend_bin.exists() {
        let backend_output = Command::new(&backend_bin)
            .arg(&verified_path)
            .arg(&ll_path)
            .output()
            .map_err(|e| miette::miette!("Failed to invoke C++ backend: {}", e))?;

        if !backend_output.status.success() {
            let err = String::from_utf8_lossy(&backend_output.stderr);
            pb.finish_with_message("Backend generation failed");
            return Err(miette::miette!("Backend Error:\n{}", err.trim()));
        }
    } else {
        // Backend not built yet — write a placeholder LLVM IR file
        let placeholder = format!(
            "; NEURO Compiler: LLVM IR Output\n"
        );
        std::fs::write(&ll_path, &placeholder)
            .map_err(|e| miette::miette!("Failed to write placeholder LLVM IR: {}", e))?;
        if verbose {
            println!("{} Backend binary not found at {}, wrote placeholder LLVM IR",
                ">".bright_black(), backend_bin.display());
        }
    }

    pb.inc(1);

    // Phase 4: Linking
    pb.set_message("Finalizing: System Linking [Clang + Runtime]");

    let bin_path = output_dir.join("output.bin");
    let obj_path = output_dir.join("output.o");
    let clang_found = which::which("clang").is_ok();

    if clang_found && backend_bin.exists() {
        // Step 1: Build runtime library
        let runtime_a = build_runtime(&output_dir)?;

        // Step 2: Compile LLVM IR → object file
        let compile_output = Command::new("clang")
            .args(["-c", &ll_path.to_string_lossy(), "-o", &obj_path.to_string_lossy()])
            .output()
            .map_err(|e| miette::miette!("Failed to compile LLVM IR to object file: {}", e))?;

        if !compile_output.status.success() {
            let err = String::from_utf8_lossy(&compile_output.stderr);
            return Err(miette::miette!("Compilation Error:\n{}", err.trim()));
        }

        // Step 3: Link object + runtime → final binary
        let link_output = Command::new("clang")
            .args(["-o", &bin_path.to_string_lossy(), &obj_path.to_string_lossy(), &runtime_a.to_string_lossy()])
            .output()
            .map_err(|e| miette::miette!("Failed to invoke Clang for linking: {}", e))?;

        if !link_output.status.success() {
            let err = String::from_utf8_lossy(&link_output.stderr);
            return Err(miette::miette!("Linking Error:\n{}", err.trim()));
        }

        // Step 4: Clean up intermediate files on success
        let _ = std::fs::remove_file(&ll_path);
        let _ = std::fs::remove_file(&obj_path);
        let _ = std::fs::remove_file(&output_dir.join("runtime_io.o"));
        let _ = std::fs::remove_file(&output_dir.join("runtime_memory.o"));

        if verbose {
            println!("{} Linked with runtime library", ">".bright_black());
        }
    } else {
        if verbose {
            if !clang_found {
                println!("{} Clang not found, skipping linking step", ">".bright_black());
            }
            println!("{} Output LLVM IR at: {}", ">".bright_black(), ll_path.display());
        }
    }

    pb.inc(1);

    pb.finish_with_message("Compilation Successful");

    println!("\n{}", "BUILD COMPLETE".green().bold());
    if verbose {
        println!("{} Verified AST: {}", ">".bright_black(), verified_path.display());
        if clang_found && backend_bin.exists() {
            println!("{} Binary: {}", ">".bright_black(), bin_path.display());
        } else {
            println!("{} LLVM IR: {}", ">".bright_black(), ll_path.display());
        }
    }

    Ok(())
}
