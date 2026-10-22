use std::fs;
use tempfile::TempDir;
use std::path::Path;
use tbsg_detect::{
    analyze, parse_proto, find_write_sites, find_read_sites, classify_verdict, classify_read_kind,
    IdlField, ReadSite, ReadKind, VerificationVerdict,
};

fn create_proto_file(tmp: &Path, content: &str) -> std::path::PathBuf {
    let path = tmp.join("schema.proto");
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn test_bool_field_without_suffix_ignored() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message Test {
    bool is_async = 1;
    string name = 2;
}
"#);
    let fields = parse_proto(&schema);
    assert!(fields.is_empty());
}

#[test]
fn test_bool_field_with_suffix_detected() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message Test {
    bool borrow_check_passed = 1;
    string name = 2;
}
"#);
    let fields = parse_proto(&schema);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].field_name, "borrow_check_passed");
}

#[test]
fn test_write_site_rust_hardcoded() {
    let tmp = TempDir::new().unwrap();
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();
    fs::write(
        producer.join("lib.rs"),
        "pub fn build() -> VerifiedProgram {\n    VerifiedProgram {\n        borrow_check_passed: true,\n        data: vec![],\n    }\n}",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 3,
    }];

    let writes = find_write_sites(&producer, &fields);
    let sites = writes.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].file_path.file_name().unwrap(), "lib.rs");
    assert_eq!(sites[0].line_number, 3);
}

#[test]
fn test_write_site_cpp_setter() {
    let tmp = TempDir::new().unwrap();
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();
    fs::write(
        producer.join("builder.cpp"),
        "void build() {\n    result.set_borrow_check_passed(true);\n}",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 3,
    }];

    let writes = find_write_sites(&producer, &fields);
    let sites = writes.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 2);
}

#[test]
fn test_write_site_python_assignment() {
    let tmp = TempDir::new().unwrap();
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();
    fs::write(
        producer.join("builder.py"),
        "def build():\n    result = VerifiedProgram()\n    result.borrow_check_passed = True\n    return result",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 1,
    }];

    let writes = find_write_sites(&producer, &fields);
    let sites = writes.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 3);
}

#[test]
fn test_write_site_java_setter() {
    let tmp = TempDir::new().unwrap();
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();
    fs::write(
        producer.join("Builder.java"),
        "void build() {\n    result.setBorrowCheckPassed(true);\n}",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 1,
    }];

    let writes = find_write_sites(&producer, &fields);
    let sites = writes.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 2);
}

#[test]
fn test_read_site_cpp_accessor() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("verifier.cpp"),
        "if (result.borrow_check_passed()) {\n    // ok\n}",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 3,
    }];

    let reads = find_read_sites(&consumer, &fields);
    let sites = reads.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 1);
}

#[test]
fn test_read_site_java_getter() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("Verifier.java"),
        "if (result.getBorrowCheckPassed()) {\n    // ok\n}",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 1,
    }];

    let reads = find_read_sites(&consumer, &fields);
    let sites = reads.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 1);
}

#[test]
fn test_read_site_python_attribute() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("verifier.py"),
        "if result.borrow_check_passed:\n    # ok\n    pass",
    ).unwrap();

    let fields = vec![IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 1,
    }];

    let reads = find_read_sites(&consumer, &fields);
    let sites = reads.get("borrow_check_passed").unwrap();
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].line_number, 1);
}

#[test]
fn test_findings_producer_writes_consumer_doesnt_read() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
    bytes data = 2;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    // Producer writes the field
    fs::write(
        producer.join("builder.rs"),
        "fn build() -> VerifiedProgram {\n    VerifiedProgram {\n        borrow_check_passed: true,\n        data: vec![],\n    }\n}",
    ).unwrap();

    // Consumer only reads data, not borrow_check_passed
    fs::write(
        consumer.join("verifier.rs"),
        "fn verify(prog: &VerifiedProgram) {\n    println!(\"{}\", prog.data.len());\n}",
    ).unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].field.field_name, "borrow_check_passed");
    assert!(findings[0].write_sites.len() > 0);
    assert!(findings[0].read_sites.is_empty());
}

#[test]
fn test_no_findings_when_field_is_read() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    fs::write(
        producer.join("builder.rs"),
        "VerifiedProgram { borrow_check_passed: true }",
    ).unwrap();

    // Consumer reads it via multiple languages
    fs::write(
        consumer.join("check.cpp"),
        "result.borrow_check_passed()",
    ).unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert!(findings.is_empty());
}

#[test]
fn test_no_findings_when_no_writes() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    // Producer doesn't write it (maybe defaults)
    fs::write(producer.join("lib.rs"), "let x = 1;").unwrap();
    // Consumer doesn't read it
    fs::write(consumer.join("lib.rs"), "let x = 1;").unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert!(findings.is_empty());
}

#[test]
fn test_no_findings_when_not_bool() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    string name = 1;
    bytes data = 2;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    fs::write(producer.join("lib.rs"), "let x = 1;").unwrap();
    fs::write(consumer.join("lib.rs"), "let x = 1;").unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert!(findings.is_empty());
}

#[test]
fn test_comments_not_flagged() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();
    // Only a comment mentioning the field — no actual write
    fs::write(
        producer.join("lib.rs"),
        "// TODO: set borrow_check_passed = true later\nlet x = 1;\n",
    ).unwrap();

    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(consumer.join("lib.rs"), "let x = 1;").unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert!(findings.is_empty());
}

#[test]
fn test_multiple_fields_partial_read() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
    bool type_check_passed = 2;
    bool signature_checked = 3;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    // Producer writes all three
    fs::write(
        producer.join("builder.rs"),
        "VerifiedProgram {\n    borrow_check_passed: true,\n    type_check_passed: true,\n    signature_checked: true,\n}",
    ).unwrap();

    // Consumer only reads borrow_check_passed
    fs::write(
        consumer.join("verifier.cpp"),
        "result.borrow_check_passed()",
    ).unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    // type_check_passed and signature_checked should be findings
    assert_eq!(findings.len(), 2);
    let names: Vec<_> = findings.iter().map(|f| f.field.field_name.as_str()).collect();
    assert!(names.contains(&"type_check_passed"));
    assert!(names.contains(&"signature_checked"));
    assert!(!names.contains(&"borrow_check_passed"));
}

#[test]
fn test_confidence_scoring() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();

    // Hardcoded write
    fs::write(
        producer.join("hardcoded.rs"),
        "VerifiedProgram { borrow_check_passed: true }",
    ).unwrap();

    // Conditional write
    fs::write(
        producer.join("conditional.rs"),
        "result.borrow_check_passed = is_valid();",
    ).unwrap();

    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(consumer.join("lib.rs"), "let x = 1;").unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.write_sites.len(), 2);

    // One should be HIGH, one MEDIUM or LOW
    let confidences: Vec<_> = finding.write_sites.iter().map(|w| &w.confidence).collect();
    assert!(confidences.contains(&&tbsg_detect::Confidence::High));
}

#[test]
fn test_producer_c_and_cc_files() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    fs::create_dir_all(&producer).unwrap();

    // C file
    fs::write(
        producer.join("builder.c"),
        "result.borrow_check_passed = 1;",
    ).unwrap();

    // C++ file
    fs::write(
        producer.join("builder.cpp"),
        "result.set_borrow_check_passed(true);",
    ).unwrap();

    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(consumer.join("lib.rs"), "let x = 1;").unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert_eq!(findings.len(), 1);
    // Should find writes in both .c and .cpp files
    assert_eq!(findings[0].write_sites.len(), 2);
    let files: Vec<_> = findings[0].write_sites.iter()
        .map(|w| w.file_path.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(files.contains(&"builder.c"));
    assert!(files.contains(&"builder.cpp"));
}

#[test]
fn test_full_proto_parsing() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";

message Function {
    string name = 1;
    bool is_mutable = 2;
}

enum VerifyStatus {
    UNKNOWN = 0;
    PASSED = 1;
    FAILED = 2;
}

message VerifiedProgram {
    Function func = 1;
    bool borrow_check_passed = 2;
    VerifyStatus verify_status = 3;
    bool type_check_passed = 4;
}

message CompilationResult {
    VerifiedProgram program = 1;
    bool is_valid = 2;
    string error_message = 3;
}
"#);
    let fields = parse_proto(&schema);
    assert_eq!(fields.len(), 4);
    let names: Vec<_> = fields.iter().map(|f| f.field_name.as_str()).collect();
    assert!(names.contains(&"borrow_check_passed"));
    assert!(names.contains(&"type_check_passed"));
    assert!(names.contains(&"verify_status"));
    assert!(names.contains(&"is_valid"));
    assert!(!names.contains(&"is_mutable"));

    // Check enum fields have correct type
    let enum_fields: Vec<_> = fields.iter().filter(|f| f.field_type == "VerifyStatus").collect();
    assert_eq!(enum_fields.len(), 1);
    assert_eq!(enum_fields[0].field_name, "verify_status");
}

#[test]
fn test_report_format() {
    use tbsg_detect::{Finding, IdlField, WriteSite, Confidence, format_report};

    let findings = vec![Finding {
        field: IdlField {
            message_name: "VerifiedProgram".into(),
            field_name: "borrow_check_passed".into(),
            field_type: "bool".into(),
            line_number: 3,
        },
        write_sites: vec![WriteSite {
            file_path: "/tmp/test.rs".into(),
            line_number: 5,
            line_content: "borrow_check_passed: true".into(),
            confidence: Confidence::High,
        }],
        read_sites: vec![],
        verdict: VerificationVerdict::UnreadAttestation,
    }];

    let report = format_report(&findings);
    assert!(report.contains("Trust Boundary Semantic Gap"));
    assert!(report.contains("borrow_check_passed"));
    assert!(report.contains("HIGH"));
    assert!(report.contains("UNREAD_ATTESTATION"));
}

// ── v0.3.0: read-classification tests ─────────────────────────────────

/// Reviewer #608A regression: a consumer whose ONLY read of the safety
/// boolean is inside computeHmac() — a pass-through/serialization read —
/// must still be flagged, even though v0.2 counted it as a "read".
#[test]
fn test_passthrough_read_flagged() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    fs::write(
        producer.join("builder.rs"),
        "VerifiedProgram { borrow_check_passed: true }",
    ).unwrap();

    // Consumer reads it only to feed the HMAC input buffer — NOT a guard.
    fs::write(
        consumer.join("emitter.cpp"),
        "std::string expected_sig = computeHmac(prog, verified.borrow_check_passed(), verified.type_check_passed(), key);",
    ).unwrap();

    let read_sites = find_read_sites(&consumer, &[IdlField {
        message_name: "VerifiedProgram".into(),
        field_name: "borrow_check_passed".into(),
        field_type: "bool".into(),
        line_number: 1,
    }]);
    let reads = &read_sites["borrow_check_passed"];
    assert_eq!(reads.len(), 1);
    assert_eq!(reads[0].kind, ReadKind::PassThrough);

    let findings = analyze(&schema, &producer, &consumer);
    assert_eq!(findings.len(), 1, "Pass-through-only read must be flagged");
    assert_eq!(findings[0].verdict, VerificationVerdict::UnenforcedSerializationRead);
    assert!(findings[0].verdict.is_finding());
}

/// Full fix: the consumer gates control flow on the flag inside an if —
/// clean, authenticated when crypto markers are present.
#[test]
fn test_control_flow_guard_clean() {
    let tmp = TempDir::new().unwrap();
    let schema = create_proto_file(tmp.path(), r#"
syntax = "proto3";
message VerifiedProgram {
    bool borrow_check_passed = 1;
}
"#);
    let producer = tmp.path().join("producer");
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&producer).unwrap();
    fs::create_dir_all(&consumer).unwrap();

    fs::write(
        producer.join("builder.rs"),
        "VerifiedProgram { borrow_check_passed: true }",
    ).unwrap();

    fs::write(
        consumer.join("emitter.cpp"),
        "if (!verified.borrow_check_passed() || !verified.type_check_passed()) {\n error = \"bad\";\n return false;\n}\nCRYPTO_memcmp(verified.signature().data(), expected_sig.data(), 32);",
    ).unwrap();

    let findings = analyze(&schema, &producer, &consumer);
    assert!(
        findings.is_empty(),
        "Control-flow guard read must be clean; got: {:?}",
        findings.iter().map(|f| f.verdict.tag()).collect::<Vec<_>>()
    );
}

/// Direct verdict-aggregation unit checks.
#[test]
fn test_verdict_aggregation() {
    let mk = |kind: ReadKind| ReadSite {
        file_path: "x.cpp".into(),
        line_number: 1,
        line_content: "x".into(),
        kind,
    };

    assert_eq!(classify_verdict(&[], false), VerificationVerdict::UnreadAttestation);
    assert_eq!(
        classify_verdict(&[mk(ReadKind::PassThrough)], false),
        VerificationVerdict::UnenforcedSerializationRead
    );
    assert_eq!(
        classify_verdict(&[mk(ReadKind::ControlFlowGuard)], false),
        VerificationVerdict::ControlFlowGuard
    );
    assert_eq!(
        classify_verdict(&[mk(ReadKind::ControlFlowGuard)], true),
        VerificationVerdict::AuthenticatedControlFlowGuard
    );
    assert_eq!(
        classify_verdict(&[mk(ReadKind::PassThrough), mk(ReadKind::ControlFlowGuard)], true),
        VerificationVerdict::AuthenticatedControlFlowGuard
    );
    assert_eq!(
        classify_verdict(&[mk(ReadKind::Other)], false),
        VerificationVerdict::UnverifiedRead
    );
    assert!(!VerificationVerdict::AuthenticatedControlFlowGuard.is_finding());
}

/// Multi-line conditions: the read is on a line *after* the `if (` opener.
#[test]
fn test_multiline_condition_guard() {
    let lines = [
        "if (!verified.borrow_check_passed() ||",
        "    !verified.type_check_passed()) {",
        "    error_ = \"Semantic attestation flags false\";",
    ];
    assert_eq!(classify_read_kind(&lines, 0, "borrow_check_passed"), ReadKind::ControlFlowGuard);
    assert_eq!(classify_read_kind(&lines, 1, "type_check_passed"), ReadKind::ControlFlowGuard);
}

/// Comments must never be classified as reads (regression for v0.3.0).
#[test]
fn test_comment_reads_not_flagged() {
    let tmp = TempDir::new().unwrap();
    let consumer = tmp.path().join("consumer");
    fs::create_dir_all(&consumer).unwrap();
    fs::write(
        consumer.join("c.cpp"),
        "// if (!verified.borrow_check_passed()) { ... }",
    ).unwrap();
    let reads = find_read_sites(
        &consumer,
        &[IdlField {
            message_name: "VerifiedProgram".into(),
            field_name: "borrow_check_passed".into(),
            field_type: "bool".into(),
            line_number: 1,
        }],
    );
    assert!(reads.get("borrow_check_passed").map_or(true, |v| v.is_empty()));
}
