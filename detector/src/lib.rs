use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Semantic suffixes that indicate safety/security relevance.
/// Fields whose names end with any of these are flagged for analysis.
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
    "_sealed",
    "_signed",
    "_attested",
    "_certified",
    "_status",
];

/// Source file extensions to search, grouped by language role.
const PRODUCER_EXTENSIONS: &[&str] = &["rs", "py", "java", "go", "ts", "js", "c", "cpp", "cc"];
const CONSUMER_EXTENSIONS: &[&str] = &["rs", "py", "java", "go", "ts", "js", "c", "cpp", "cc", "h", "hpp"];

/// Represents a field from the IDL schema (bool or status-like)
#[derive(Debug, Clone)]
pub struct IdlField {
    pub message_name: String,
    pub field_name: String,
    pub field_type: String,
    pub line_number: usize,
}

/// Represents a write-site in producer code
#[derive(Debug, Clone)]
pub struct WriteSite {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub confidence: Confidence,
}

/// Represents a read-site in consumer code
#[derive(Debug, Clone)]
pub struct ReadSite {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub line_content: String,
}

/// Confidence that a write-site represents an attestation (not just data)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    High,   // hardcoded true/false — attestation is decorative by construction
    Medium, // assigned from a variable or conditional — may carry information
    Low,    // passed through from function call — uncertain
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::High => write!(f, "HIGH"),
            Confidence::Medium => write!(f, "MEDIUM"),
            Confidence::Low => write!(f, "LOW"),
        }
    }
}

/// A finding: a field written but never read by the consumer
#[derive(Debug)]
pub struct Finding {
    pub field: IdlField,
    pub write_sites: Vec<WriteSite>,
    pub read_sites: Vec<ReadSite>,
}

// ── IDL Schema Parsing ─────────────────────────────────────────────────

/// Parse a protobuf schema file to extract semantic boolean fields.
///
/// Currently supports proto3 syntax. Cap'n Proto and FlatBuffers would need
/// different parsers:
/// - Cap'n Proto: `struct VerifiedProgram { borrowCheckPassed @1 :Bool; }`
/// - FlatBuffers: `table VerifiedProgram { borrow_check_passed:bool; }`
/// The regex patterns for those would replace the proto3 patterns below.
pub fn parse_proto(schema_path: &Path) -> Vec<IdlField> {
    let content = fs::read_to_string(schema_path).expect("Failed to read schema file");
    let mut fields = Vec::new();
    let mut current_message = String::new();

    let message_re = Regex::new(r"^\s*message\s+(\w+)\s*\{").unwrap();
    // Match bool fields
    let bool_re = Regex::new(r"^\s*bool\s+(\w+)\s*=\s*\d+").unwrap();
    // Match enum fields (status-like)
    let enum_re = Regex::new(r"^\s*(\w+)\s+(\w+)\s*=\s*\d+").unwrap();

    // Collect enum type names so we can recognize status-like fields
    let mut enum_types: Vec<String> = Vec::new();
    let enum_def_re = Regex::new(r"^\s*enum\s+(\w+)\s*\{").unwrap();
    let mut in_enum = false;
    for line in content.lines() {
        if let Some(caps) = enum_def_re.captures(line) {
            enum_types.push(caps[1].to_string());
            in_enum = true;
        } else if in_enum && line.trim() == "}" {
            in_enum = false;
        }
    }

    for (line_num, line) in content.lines().enumerate() {
        if let Some(caps) = message_re.captures(line) {
            current_message = caps[1].to_string();
        }

        // Bool fields with semantic suffixes
        if let Some(caps) = bool_re.captures(line) {
            let field_name = caps[1].to_string();
            if has_semantic_suffix(&field_name) {
                fields.push(IdlField {
                    message_name: current_message.clone(),
                    field_name,
                    field_type: "bool".to_string(),
                    line_number: line_num + 1,
                });
            }
        }

        // Enum fields with semantic suffixes (e.g., `Status status = 4;`)
        if let Some(caps) = enum_re.captures(line) {
            let type_name = caps[1].to_string();
            let field_name = caps[2].to_string();
            if enum_types.contains(&type_name) && has_semantic_suffix(&field_name) {
                fields.push(IdlField {
                    message_name: current_message.clone(),
                    field_name,
                    field_type: type_name,
                    line_number: line_num + 1,
                });
            }
        }
    }

    fields
}

/// Check if a field name has a semantic suffix
fn has_semantic_suffix(field_name: &str) -> bool {
    SEMANTIC_SUFFIXES
        .iter()
        .any(|suffix| field_name.ends_with(suffix))
}

// ── Producer Write-Site Detection (Language-Agnostic) ──────────────────

/// Generate all name variants for a field to match across languages.
/// Returns (snake_case, camelCase, PascalCase) variants.
fn field_name_variants(field_name: &str) -> (String, String, String) {
    (
        field_name.to_string(),
        to_camel_case(field_name),
        to_pascal_case(field_name),
    )
}

/// Check if a line mentions any variant of the field name.
fn line_contains_field(line: &str, field_name: &str) -> bool {
    let (snake, camel, pascal) = field_name_variants(field_name);
    line.contains(&snake) || line.contains(&camel) || line.contains(&pascal)
}

/// Search producer source code for write-sites of semantic fields.
/// Walks all source files matching PRODUCER_EXTENSIONS.
///
/// Language-specific write patterns detected:
/// - Rust: `field_name: true`, `field_name = true`
/// - C++: `set_field_name(true)`, `.set_field_name(true)`
/// - Python: `field_name = True`, `field_name=True`
/// - Java: `.setFieldName(true)`, `.setField_name(true)`
/// - Go: `FieldName: true` (struct literal)
pub fn find_write_sites(producer_dir: &Path, fields: &[IdlField]) -> HashMap<String, Vec<WriteSite>> {
    let mut results: HashMap<String, Vec<WriteSite>> = HashMap::new();

    for entry in WalkDir::new(producer_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| {
                PRODUCER_EXTENSIONS.contains(&ext.to_str().unwrap_or(""))
            })
        })
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("*") {
                continue;
            }

            for field in fields {
                if !line_contains_field(line, &field.field_name) {
                    continue;
                }

                let confidence = classify_write_confidence(line, &field.field_name);

                results
                    .entry(field.field_name.clone())
                    .or_insert_with(Vec::new)
                    .push(WriteSite {
                        file_path: entry.path().to_path_buf(),
                        line_number: line_num + 1,
                        line_content: trimmed.to_string(),
                        confidence,
                    });
            }
        }
    }

    results
}

/// Classify the confidence that a write-site is a decorative attestation.
fn classify_write_confidence(line: &str, field_name: &str) -> Confidence {
    let trimmed = line.trim();
    let (snake, camel, pascal) = field_name_variants(field_name);

    // HIGH: hardcoded true/false/1/0 in struct literal or assignment
    // Patterns: `field_name: true`, `set_field_name(true)`, `setFieldName(True)`
    for variant in &[&snake, &camel, &pascal] {
        let escaped = regex::escape(variant);
        let high_re = Regex::new(&format!(
            r"{escaped}[\s:=]*[{{(]?\s*(true|True|TRUE|false|False|FALSE|1|0)\s*[)}}]?",
        )).unwrap();
        if high_re.is_match(trimmed) {
            return Confidence::High;
        }
    }

    // MEDIUM: assigned from a variable or function (setter pattern, assignment)
    // Patterns: `field_name = some_var()`, `setFieldName(computed_value)`
    let has_assignment = trimmed.contains("=") || trimmed.contains("set_");
    let has_variant = trimmed.contains(&snake) || trimmed.contains(&camel) || trimmed.contains(&pascal);
    if has_variant && has_assignment {
        return Confidence::Medium;
    }

    // LOW: line mentions the field but no clear write pattern
    Confidence::Low
}

// ── Consumer Read-Site Detection (Language-Agnostic) ───────────────────

/// Search consumer source code for read-sites of semantic fields.
/// Walks all source files matching CONSUMER_EXTENSIONS.
///
/// Language-specific read patterns detected:
/// - C++ protobuf: `.field_name()`
/// - C++ struct: `.field_name`
/// - Rust: `.field_name` or `field_name` (struct field)
/// - Python: `.field_name`
/// - Java protobuf: `.getFieldName()` or `.getField_name()`
/// - Java: `.fieldName`
/// - Go: `.FieldName`
pub fn find_read_sites(consumer_dir: &Path, fields: &[IdlField]) -> HashMap<String, Vec<ReadSite>> {
    let mut results: HashMap<String, Vec<ReadSite>> = HashMap::new();

    for entry in WalkDir::new(consumer_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().extension().map_or(false, |ext| {
                CONSUMER_EXTENSIONS.contains(&ext.to_str().unwrap_or(""))
            })
        })
    {
        let content = fs::read_to_string(entry.path()).unwrap_or_default();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            // Skip comments
            if trimmed.starts_with("//") || trimmed.starts_with("#") || trimmed.starts_with("*") {
                continue;
            }

            for field in fields {
                let patterns = generate_read_patterns(&field.field_name);
                for pattern in &patterns {
                    if line.contains(pattern) {
                        results
                            .entry(field.field_name.clone())
                            .or_insert_with(Vec::new)
                            .push(ReadSite {
                                file_path: entry.path().to_path_buf(),
                                line_number: line_num + 1,
                                line_content: trimmed.to_string(),
                            });
                        break; // one match per line per field is enough
                    }
                }
            }
        }
    }

    results
}

/// Generate language-specific read patterns for a field name.
fn generate_read_patterns(field_name: &str) -> Vec<String> {
    let mut patterns = Vec::new();

    // C++ protobuf accessor: .field_name()
    patterns.push(format!(".{}()", field_name));

    // C++/Rust/Python/Java struct field access: .field_name
    // (but not followed by '(' to avoid double-matching with the protobuf accessor)
    // We just use the simpler form and accept the overlap
    patterns.push(format!(".{}", field_name));

    // Java protobuf getter: .getFieldName() (PascalCase)
    let pascal = to_pascal_case(field_name);
    patterns.push(format!(".get{}()", pascal));

    // Java protobuf getter: .getField_name() (snake_case preserved)
    patterns.push(format!(".get_{}()", field_name));

    // Go exported field: .FieldName (PascalCase)
    let pascal = to_pascal_case(field_name);
    patterns.push(format!(".{}", pascal));

    patterns
}

/// Convert snake_case to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Convert snake_case to PascalCase
fn to_pascal_case(s: &str) -> String {
    let camel = to_camel_case(s);
    let mut chars = camel.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            upper + chars.as_str()
        }
    }
}

// ── Analysis ───────────────────────────────────────────────────────────

/// Analyze the pipeline and return findings.
///
/// `schema_path`: path to the IDL schema (protobuf, Cap'n Proto, FlatBuffers)
/// `producer_dir`: path to the producer source directory
/// `consumer_dir`: path to the consumer source directory
pub fn analyze(
    schema_path: &Path,
    producer_dir: &Path,
    consumer_dir: &Path,
) -> Vec<Finding> {
    let fields = parse_proto(schema_path);
    if fields.is_empty() {
        return Vec::new();
    }

    let write_sites = find_write_sites(producer_dir, &fields);
    let read_sites = find_read_sites(consumer_dir, &fields);

    let mut findings = Vec::new();

    for field in &fields {
        let writes = write_sites.get(&field.field_name).cloned().unwrap_or_default();
        let reads = read_sites.get(&field.field_name).cloned().unwrap_or_default();

        // A finding exists if the field is written but never read by the consumer.
        // If reads exist, the field is consumed — no finding.
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

// ── Reporting ──────────────────────────────────────────────────────────

/// Format findings as a structured plain-text report.
pub fn format_report(findings: &[Finding]) -> String {
    let mut report = String::new();

    report.push_str("=== Trust Boundary Semantic Gap Detector ===\n\n");

    if findings.is_empty() {
        report.push_str("No findings. All semantic fields are read by consumers.\n");
        return report;
    }

    report.push_str(&format!(
        "Found {} field(s) with potential TBSG (written by producer, not read by consumer):\n\n",
        findings.len()
    ));

    for (i, finding) in findings.iter().enumerate() {
        // Determine max confidence across write-sites
        let max_confidence = finding.write_sites.iter()
            .map(|w| &w.confidence)
            .max()
            .unwrap_or(&Confidence::Low);

        report.push_str(&format!(
            "[{}] {}.{} ({}, proto line {}, confidence: {})\n",
            i + 1,
            finding.field.message_name,
            finding.field.field_name,
            finding.field.field_type,
            finding.field.line_number,
            max_confidence
        ));

        report.push_str("  Producer write-sites:\n");
        for site in &finding.write_sites {
            report.push_str(&format!(
                "    {}:{} [{}] {}\n",
                site.file_path.display(),
                site.line_number,
                site.confidence,
                site.line_content
            ));
        }

        report.push_str("  Consumer read-sites: NONE FOUND\n");
        report.push_str("\n");
    }

    report.push_str("---\n");
    report.push_str("Severity: HIGH (design-level) if all write-sites are HIGH confidence\n");
    report.push_str("         MEDIUM if any write-sites are MEDIUM/LOW confidence\n");
    report.push_str("Remediation: Consumer must verify semantic fields before trusting them.\n");

    report
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_semantic_suffix_detection() {
        assert!(has_semantic_suffix("borrow_check_passed"));
        assert!(has_semantic_suffix("type_check_passed"));
        assert!(has_semantic_suffix("is_verified"));
        assert!(has_semantic_suffix("signature_checked"));
        assert!(has_semantic_suffix("data_sealed"));
        assert!(!has_semantic_suffix("is_async"));
        assert!(!has_semantic_suffix("enabled"));
        assert!(!has_semantic_suffix("is_mutable"));
        assert!(!has_semantic_suffix("name"));
    }

    #[test]
    fn test_parse_proto_bools() {
        let tmp = TempDir::new().unwrap();
        let proto_path = tmp.path().join("test.proto");
        let content = r#"
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
        fs::write(&proto_path, content).unwrap();
        let fields = parse_proto(&proto_path);
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|f| f.field_name == "borrow_check_passed"));
        assert!(fields.iter().any(|f| f.field_name == "type_check_passed"));
        assert!(!fields.iter().any(|f| f.field_name == "is_mutable"));
    }

    #[test]
    fn test_parse_proto_enum_fields() {
        let tmp = TempDir::new().unwrap();
        let proto_path = tmp.path().join("test.proto");
        let content = r#"
syntax = "proto3";
enum VerifyStatus {
    UNKNOWN = 0;
    PASSED = 1;
    FAILED = 2;
}
message Result {
    string data = 1;
    VerifyStatus verify_status = 2;
}
"#;
        fs::write(&proto_path, content).unwrap();
        let fields = parse_proto(&proto_path);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].field_name, "verify_status");
        assert_eq!(fields[0].field_type, "VerifyStatus");
    }

    #[test]
    fn test_camel_case_conversion() {
        assert_eq!(to_camel_case("borrow_check_passed"), "borrowCheckPassed");
        assert_eq!(to_pascal_case("borrow_check_passed"), "BorrowCheckPassed");
        assert_eq!(to_camel_case("type_check_passed"), "typeCheckPassed");
    }

    #[test]
    fn test_write_site_confidence() {
        assert_eq!(
            classify_write_confidence("borrow_check_passed: true", "borrow_check_passed"),
            Confidence::High
        );
        assert_eq!(
            classify_write_confidence("borrow_check_passed = true", "borrow_check_passed"),
            Confidence::High
        );
        assert_eq!(
            classify_write_confidence("set_borrow_check_passed(true)", "borrow_check_passed"),
            Confidence::High
        );
        assert_eq!(
            classify_write_confidence("borrow_check_passed = result.ok()", "borrow_check_passed"),
            Confidence::Medium
        );
    }

    #[test]
    fn test_read_pattern_generation() {
        let patterns = generate_read_patterns("borrow_check_passed");
        assert!(patterns.contains(&".borrow_check_passed()".to_string()));
        assert!(patterns.contains(&".borrow_check_passed".to_string()));
        assert!(patterns.contains(&".getBorrowCheckPassed()".to_string()));
    }

    #[test]
    fn test_comment_skipping() {
        let tmp = TempDir::new().unwrap();
        let producer_dir = tmp.path().join("producer");
        fs::create_dir_all(&producer_dir).unwrap();
        // Write a comment that contains the field name + true — should be skipped
        fs::write(
            producer_dir.join("lib.rs"),
            "// borrow_check_passed: true is a comment\nlet x = 1;\n",
        ).unwrap();

        let fields = vec![IdlField {
            message_name: "Test".into(),
            field_name: "borrow_check_passed".into(),
            field_type: "bool".into(),
            line_number: 1,
        }];

        let writes = find_write_sites(&producer_dir, &fields);
        assert!(
            writes.get("borrow_check_passed").map_or(true, |v| v.is_empty()),
            "Comments should not be flagged as write-sites"
        );
    }
}
