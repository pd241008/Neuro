use neuro_detector::{analyze, format_report};
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (proto_path, producer_dir, consumer_dir) = match args.len() {
        4 => (
            PathBuf::from(&args[1]),
            PathBuf::from(&args[2]),
            PathBuf::from(&args[3]),
        ),
        _ => {
            // Use default paths relative to workspace root
            let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .to_path_buf();

            (
                workspace.join("shared_ast").join("ast.proto"),
                workspace.join("analyzer").join("src"),
                workspace.join("backend"),
            )
        }
    };

    if !proto_path.exists() {
        eprintln!("Error: Proto file not found: {}", proto_path.display());
        process::exit(1);
    }

    if !producer_dir.exists() {
        eprintln!("Error: Producer directory not found: {}", producer_dir.display());
        process::exit(1);
    }

    if !consumer_dir.exists() {
        eprintln!("Error: Consumer directory not found: {}", consumer_dir.display());
        process::exit(1);
    }

    let findings = analyze(&proto_path, &producer_dir, &consumer_dir);
    let report = format_report(&findings);

    print!("{}", report);

    if !findings.is_empty() {
        process::exit(1);
    }
}
