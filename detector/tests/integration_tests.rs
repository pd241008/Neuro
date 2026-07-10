use neuro_detector::{analyze, format_report, Finding};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Helper to create a temporary project structure
fn create_test_project(
    proto_content: &str,
    rust_content: &str,
    cpp_content: &str,
) -> TempDir {
    let tmp = TempDir::new().unwrap();

    // Create proto file
    let proto_path = tmp.path().join("test.proto");
    fs::write(&proto_path, proto_content).unwrap();

    // Create Rust producer directory
    let producer_dir = tmp.path().join("producer");
    fs::create_dir_all(&producer_dir).unwrap();
    fs::write(producer_dir.join("lib.rs"), rust_content).unwrap();

    // Create C++ consumer directory
    let consumer_dir = tmp.path().join("consumer");
    fs::create_dir_all(&consumer_dir).unwrap();
    fs::write(consumer_dir.join("main.cpp"), cpp_content).unwrap();

    tmp
}

#[test]
fn test_case_1_verified_field_is_checked() {
    // A proto with a semantic bool that IS read by the consumer
    // Should produce ZERO findings
    let proto = r#"
syntax = "proto3";
message VerifiedData {
    bytes payload = 1;
    bool signature_checked = 2;
}
"#;

    let rust = r#"
fn produce() -> VerifiedData {
    VerifiedData {
        payload: vec![],
        signature_checked: true,
    }
}
"#;

    let cpp = r#"
void consume(const VerifiedData& data) {
    if (!data.signature_checked()) {
        throw std::runtime_error("Signature not checked");
    }
    process(data.payload());
}
"#;

    let tmp = create_test_project(proto, rust, cpp);
    let findings = analyze(
        &tmp.path().join("test.proto"),
        &tmp.path().join("producer"),
        &tmp.path().join("consumer"),
    );

    assert!(
        findings.is_empty(),
        "Expected zero findings when consumer reads the field, got: {:?}",
        findings
    );
}

#[test]
fn test_case_2_non_semantic_bool_not_flagged() {
    // A proto with a non-semantic bool (is_async) that's unread
    // Should NOT fire - the suffix heuristic should filter it out
    let proto = r#"
syntax = "proto3";
message TaskConfig {
    string name = 1;
    bool is_async = 2;
}
"#;

    let rust = r#"
fn create_task() -> TaskConfig {
    TaskConfig {
        name: "test".to_string(),
        is_async: false,
    }
}
"#;

    let cpp = r#"
void run_task(const TaskConfig& config) {
    execute(config.name());
}
"#;

    let tmp = create_test_project(proto, rust, cpp);
    let findings = analyze(
        &tmp.path().join("test.proto"),
        &tmp.path().join("producer"),
        &tmp.path().join("consumer"),
    );

    assert!(
        findings.is_empty(),
        "Expected zero findings for non-semantic bool, got: {:?}",
        findings
    );
}

#[test]
fn test_case_3_neuro_like_unchecked_semantic_bools() {
    // Reproduces the Neuro pattern: semantic bools written but never read
    // Should flag BOTH fields
    let proto = r#"
syntax = "proto3";
message VerifiedProgram {
    Program program = 1;
    bool borrow_check_passed = 2;
    bool type_check_passed = 3;
}
message Program {
    string name = 1;
}
"#;

    let rust = r#"
fn audit() -> VerifiedProgram {
    VerifiedProgram {
        program: Some(Program { name: "test".into() }),
        borrow_check_passed: true,
        type_check_passed: true,
    }
}
"#;

    let cpp = r#"
void emit(const VerifiedProgram& verified) {
    if (!verified.has_program()) {
        return;
    }
    const Program& prog = verified.program();
    emit_program(prog);
}
"#;

    let tmp = create_test_project(proto, rust, cpp);
    let findings = analyze(
        &tmp.path().join("test.proto"),
        &tmp.path().join("producer"),
        &tmp.path().join("consumer"),
    );

    assert_eq!(findings.len(), 2, "Expected 2 findings, got: {:?}", findings);

    let field_names: Vec<&str> = findings
        .iter()
        .map(|f| f.field.field_name.as_str())
        .collect();

    assert!(
        field_names.contains(&"borrow_check_passed"),
        "Should flag borrow_check_passed, got: {:?}",
        field_names
    );

    assert!(
        field_names.contains(&"type_check_passed"),
        "Should flag type_check_passed, got: {:?}",
        field_names
    );

    // Verify each finding has write-sites but no read-sites
    for finding in &findings {
        assert!(
            !finding.write_sites.is_empty(),
            "Finding {} should have write-sites",
            finding.field.field_name
        );
        assert!(
            finding.read_sites.is_empty(),
            "Finding {} should have no read-sites",
            finding.field.field_name
        );
    }
}

#[test]
fn test_report_formatting() {
    let proto = r#"
syntax = "proto3";
message VerifiedResult {
    bool is_valid = 1;
}
"#;

    let rust = r#"
fn validate() -> VerifiedResult {
    VerifiedResult { is_valid: true }
}
"#;

    let cpp = r#"
void use_result(const VerifiedResult& res) {
    // Does NOT check is_valid
    process();
}
"#;

    let tmp = create_test_project(proto, rust, cpp);
    let findings = analyze(
        &tmp.path().join("test.proto"),
        &tmp.path().join("producer"),
        &tmp.path().join("consumer"),
    );

    let report = format_report(&findings);
    assert!(report.contains("VerifiedResult.is_valid"), "Report should contain field name");
    assert!(report.contains("Write-sites"), "Report should mention write-sites");
    assert!(report.contains("Read-sites"), "Report should mention read-sites");
    assert!(report.contains("NONE"), "Report should note missing read-sites");
}
