use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Semantic suffixes that indicate safety/security relevance
const SEMANTIC_SUFFIXES: &[&str] = &[
    "_passed",
    "_verified",
    "_checked",
    "_valid",
    "_ok",
    "_safe",
    "_secure",
    "_auth",
    "_trusted",
    "_complete",
];

/// Represents a boolean field from the proto schema
#[derive(Debug, Clone)]
pub struct ProtoBoolField {
    pub message_name: String,
    pub field_name: String,
    pub line_number: usize,
}

/// Represents a write-site in producer code
#[derive(Debug, Clone)]
pub struct WriteSite {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Represents a read-site in consumer code
#[derive(Debug, Clone)]
pub struct ReadSite {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Finding: a field written but never read
#[derive(Debug)]
pub struct Finding {
    pub field: ProtoBoolField,
    pub write_sites: Vec<WriteSite>,
    pub read_sites: Vec<ReadSite>,
}

/// Parse proto file to extract boolean fields with semantic suffixes
pub fn parse_proto_semantic_bools(proto_path: &Path) -> Vec<ProtoBoolField> {
    let content = fs::read_to_string(proto_path).expect("Failed to read proto file");
    let mut fields = Vec::new();
    let mut current_message = String::new();

    // Regex to match message declarations
    let message_re = Regex::new(r"^\s*message\s+(\w+)\s*\{").unwrap();
    // Regex to match bool fields with semantic suffixes
    let bool_re = Regex::new(r"^\s*bool\s+(\w+)\s*=\s*\d+").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = message_re.captures(line) {
            current_message = caps[1].to_string();
        }

        if let Some(caps) = bool_re.captures(line) {
            let field_name = caps[1].to_string();
            if is_semantic_bool(&field_name) {
                fields.push(ProtoBoolField {
                    message_name: current_message.clone(),
                    field_name,
                    line_number: line_num + 1,
                });
            }
        }
    }

    fields
}

/// Check if a bool field name has a semantic suffix
fn is_semantic_bool(field_name: &str) -> bool {
    SEMANTIC_SUFFIXES
        .iter()
        .any(|suffix| field_name.ends_with(suffix))
}

/// Search Rust producer code for write-sites of semantic bool fields
pub fn find_write_sites(producer_dir: &Path, fields: &[ProtoBoolField]) -> HashMap<String, Vec<WriteSite>> {
    let mut results: HashMap<String, Vec<WriteSite>> = HashMap::new();

    for entry in WalkDir::new(producer_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            for field in fields {
                // Look for field assignment patterns:
                // - field_name: true/false
                // - field_name = true/false
                // - { field_name: true/false, ... }
                if line.contains(&field.field_name)
                    && (line.contains("true") || line.contains("false"))
                    && (line.contains(":") || line.contains("="))
                {
                    results
                        .entry(field.field_name.clone())
                        .or_insert_with(Vec::new)
                        .push(WriteSite {
                            file_path: entry.path().to_path_buf(),
                            line_number: line_num + 1,
                            line_content: line.trim().to_string(),
                        });
                }
            }
        }
    }

    results
}

/// Search C++ consumer code for read-sites of semantic bool fields
pub fn find_read_sites(consumer_dir: &Path, fields: &[ProtoBoolField]) -> HashMap<String, Vec<ReadSite>> {
    let mut results: HashMap<String, Vec<ReadSite>> = HashMap::new();

    for entry in WalkDir::new(consumer_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let ext = e.path().extension().map_or(false, |ext| ext == "cpp" || ext == "h" || ext == "hpp");
            ext
        })
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            for field in fields {
                // Look for field access patterns in C++:
                // - verified.field_name()
                // - verified.field_name() &&
                // - if (verified.field_name())
                let accessor = format!(".{}()", field.field_name);
                if line.contains(&accessor) {
                    results
                        .entry(field.field_name.clone())
                        .or_insert_with(Vec::new)
                        .push(ReadSite {
                            file_path: entry.path().to_path_buf(),
                            line_number: line_num + 1,
                            line_content: line.trim().to_string(),
                        });
                }
            }
        }
    }

    results
}

/// Analyze the pipeline and return findings
pub fn analyze(
    proto_path: &Path,
    producer_dir: &Path,
    consumer_dir: &Path,
) -> Vec<Finding> {
    let fields = parse_proto_semantic_bools(proto_path);
    let write_sites = find_write_sites(producer_dir, &fields);
    let read_sites = find_read_sites(consumer_dir, &fields);

    let mut findings = Vec::new();

    for field in &fields {
        let writes = write_sites.get(&field.field_name).cloned().unwrap_or_default();
        let reads = read_sites.get(&field.field_name).cloned().unwrap_or_default();

        // A finding exists if the field is written but never read
        if !writes.is_empty() && reads.is_empty() {
            findings.push(Finding {
                field: field.clone(),
                write_sites: writes,
                read_sites: reads,
            });
        }
    }

    findings
}

/// Format findings as plain text report
pub fn format_report(findings: &[Finding]) -> String {
    let mut report = String::new();

    report.push_str("=== Unauthenticated Provenance Boundary Detector ===\n\n");

    if findings.is_empty() {
        report.push_str("No findings. All semantic bool fields are read by consumers.\n");
        return report;
    }

    report.push_str(&format!("Found {} field(s) written but never read:\n\n", findings.len()));

    for (i, finding) in findings.iter().enumerate() {
        report.push_str(&format!(
            "[{}] {}.{} (proto line {})\n",
            i + 1,
            finding.field.message_name,
            finding.field.field_name,
            finding.field.line_number
        ));

        report.push_str("  Write-sites (producer):\n");
        for site in &finding.write_sites {
            report.push_str(&format!(
                "    {}:{} - {}\n",
                site.file_path.display(),
                site.line_number,
                site.line_content
            ));
        }

        report.push_str("  Read-sites (consumer): NONE\n");
        report.push_str("\n");
    }

    report.push_str("---\n");
    report.push_str("Severity: HIGH (design-level)\n");
    report.push_str("Remediation: Consumer must verify semantic bool fields before proceeding.\n");

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_is_semantic_bool() {
        assert!(is_semantic_bool("borrow_check_passed"));
        assert!(is_semantic_bool("type_check_passed"));
        assert!(is_semantic_bool("is_verified"));
        assert!(is_semantic_bool("signature_checked"));
        assert!(!is_semantic_bool("is_async"));
        assert!(!is_semantic_bool("enabled"));
        assert!(!is_semantic_bool("is_mutable"));
    }

    #[test]
    fn test_parse_proto_semantic_bools() {
        let tmp = TempDir::new().unwrap();
        let proto_path = tmp.path().join("test.proto");

        let proto_content = r#"
syntax = "proto3";
message VerifiedProgram {
    Program program = 1;
    bool borrow_check_passed = 2;
    bool type_check_passed = 3;
}
message Function {
    string name = 1;
    bool is_mutable = 2;
}
"#;
        fs::write(&proto_path, proto_content).unwrap();

        let fields = parse_proto_semantic_bools(&proto_path);
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|f| f.field_name == "borrow_check_passed"));
        assert!(fields.iter().any(|f| f.field_name == "type_check_passed"));
        assert!(!fields.iter().any(|f| f.field_name == "is_mutable"));
    }
}
