use clap::Parser;
use std::path::PathBuf;
use tbsg_detect::analyze;

/// Trust Boundary Semantic Gap Detector
///
/// Scans an IDL schema and producer/consumer source code to find semantic
/// fields written by the producer but never read by the consumer.
#[derive(Parser)]
#[command(name = "tbsg-detect")]
#[command(about = "Detect trust boundary semantic gaps in IPC pipelines")]
struct Cli {
    /// Path to the IDL schema file (proto3)
    schema: PathBuf,

    /// Path to the producer source directory
    producer_dir: PathBuf,

    /// Path to the consumer source directory
    consumer_dir: PathBuf,

    /// Print findings as JSON
    #[arg(long)]
    json: bool,
}

fn main() {
    let cli = Cli::parse();

    let findings = analyze(&cli.schema, &cli.producer_dir, &cli.consumer_dir);

    if cli.json {
        // Minimal JSON output
        println!("[");
        for (i, finding) in findings.iter().enumerate() {
            let max_confidence = finding.write_sites.iter()
                .map(|w| format!("{}", w.confidence))
                .max()
                .unwrap_or_else(|| "LOW".to_string());

            let write_sites: Vec<_> = finding.write_sites.iter().map(|w| {
                format!(
                    r#"{{"file":"{}","line":{},"confidence":"{}","content":"{}"}}"#,
                    w.file_path.display(),
                    w.line_number,
                    w.confidence,
                    w.line_content.replace('\\', "\\\\").replace('"', "\\\"")
                )
            }).collect();

            println!(
                r#"  {{"message":"{}","field":"{}","type":"{}","proto_line":{},"confidence":"{}","writes":[{}],"reads":[]}}"#,
                finding.field.message_name,
                finding.field.field_name,
                finding.field.field_type,
                finding.field.line_number,
                max_confidence,
                write_sites.join(", ")
            );

            if i < findings.len() - 1 {
                println!(",");
            } else {
                println!();
            }
        }
        println!("]");
    } else {
        let report = tbsg_detect::format_report(&findings);
        print!("{}", report);
    }

    if !findings.is_empty() {
        std::process::exit(1);
    }
}
