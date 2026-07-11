//! Boundary tests: demonstrate the unauthenticated provenance boundary
//! between the Rust analyzer and the C++ backend.
//!
//! Stage 1 (Baseline):      Confirm fail-closed rejection works as expected.
//! Stage 2 (Bypass):        Reproduce the unauthenticated provenance boundary finding.
//! Stage 3 (Field Drift):   Test behavior with unset/malformed protobuf fields.
//! Stage 4 (Fix 2 - HMAC):  Verify that HMAC signing prevents the bypass.

use analyzer::audit_ast;
use hmac::{Hmac, Mac};
use shared_ast::expression::ExprKind;
use shared_ast::statement::StmtKind;
use shared_ast::*;
use prost::Message;
use sha2::Sha256;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const TEST_SIGNING_KEY: &str = "boundary-test-hmac-key-2026";

// ─── Helpers (adapted from borrow_check_tests.rs and backend_integration_test.rs) ──

fn project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("results")
}

fn make_type(kind: i32) -> Option<Type> {
    Some(Type {
        kind,
        custom_name: String::new(),
    })
}

fn make_param(name: &str, kind: i32) -> Parameter {
    Parameter {
        name: name.to_string(),
        r#type: make_type(kind),
    }
}

fn make_lit_int(val: i64) -> Expression {
    Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(ExprKind::Literal(Literal {
            value: Some(literal::Value::IntVal(val)),
        })),
    }
}

fn make_var(name: &str) -> Expression {
    Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(ExprKind::Variable(VariableReference {
            name: name.to_string(),
        })),
    }
}

fn make_decl_mut(name: &str, kind: i32, init: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(StmtKind::Declaration(VariableDeclaration {
            name: name.to_string(),
            r#type: make_type(kind),
            initializer: init,
            is_mutable: true,
            resolved_type: None,
        })),
    }
}

fn make_fn(
    name: &str,
    params: Vec<Parameter>,
    ret_kind: i32,
    body: Vec<Statement>,
) -> Function {
    Function {
        name: name.to_string(),
        parameters: params,
        return_type: make_type(ret_kind),
        body,
        location: None,
    }
}

fn make_return(val: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(StmtKind::ReturnStmt(ReturnStatement { value: val })),
    }
}

/// Sign a VerifiedProgram using the same HMAC-SHA256 scheme as the analyzer.
/// Returns the serialized bytes with the signature field filled in.
fn sign_verified_program(verified: &VerifiedProgram, key: &str) -> Vec<u8> {
    let program = verified.program.as_ref().expect("VerifiedProgram must have a program");
    let borrow_check_passed = verified.borrow_check_passed;
    let type_check_passed = verified.type_check_passed;

    // Match the deterministic per-field encoding used by analyzer/src/lib.rs::compute_hmac
    let mut data = Vec::new();
    let program_bytes = program.encode_to_vec();
    data.extend_from_slice(&(program_bytes.len() as u64).to_le_bytes());
    data.extend_from_slice(&program_bytes);
    data.push(borrow_check_passed as u8);
    data.push(type_check_passed as u8);

    type H = Hmac<Sha256>;
    let mut mac = H::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(&data);
    let tag = mac.finalize().into_bytes().to_vec();

    let mut signed = verified.clone();
    signed.signature = tag;
    signed.encode_to_vec()
}

fn make_call(fn_name: &str, args: Vec<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(StmtKind::ExpressionStmt(Expression {
            location: None,
            resolved_type: None,
            expr_kind: Some(ExprKind::Call(FunctionCall {
                function_name: fn_name.to_string(),
                arguments: args,
            })),
        })),
    }
}

/// Wrap user functions with the "foo"/"bar" helpers that borrow_check_tests
/// rely on for move-into-call patterns.
fn program_with_helpers(functions: Vec<Function>) -> Program {
    let mut funcs = functions;
    funcs.push(make_fn(
        "foo",
        vec![make_param("arg", r#type::Kind::Int as i32)],
        r#type::Kind::Void as i32,
        vec![],
    ));
    funcs.push(make_fn(
        "bar",
        vec![make_param("arg", r#type::Kind::Bool as i32)],
        r#type::Kind::Void as i32,
        vec![],
    ));
    Program {
        name: "test".to_string(),
        functions: funcs,
    }
}

/// Build the exact same use-after-move program from
/// borrow_check_tests::move_variable_then_read_rejected.
fn use_after_move_program() -> Program {
    program_with_helpers(vec![make_fn(
        "main",
        vec![],
        r#type::Kind::Void as i32,
        vec![
            make_decl_mut("x", r#type::Kind::Int as i32, Some(make_lit_int(42))),
            make_call("foo", vec![make_var("x")]),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::ExpressionStmt(make_var("x"))),
            },
        ],
    )])
}

/// Build a trivially valid program (no move/borrow violations).
fn valid_program() -> Program {
    program_with_helpers(vec![make_fn(
        "main",
        vec![],
        r#type::Kind::Int as i32,
        vec![
            make_decl_mut("x", r#type::Kind::Int as i32, Some(make_lit_int(42))),
            make_return(Some(make_var("x"))),
        ],
    )])
}

/// Serialize a VerifiedProgram, write to a temp file, invoke the backend,
/// return (exit_code, stdout, stderr, emitted_ir_or_none).
fn run_backend(verified_bytes: &[u8], test_label: &str) -> (i32, String, String, Option<String>) {
    let input_path = std::env::temp_dir().join(format!("boundary_{}.verified.ast", test_label));
    let output_path = std::env::temp_dir().join(format!("boundary_{}.output.ll", test_label));

    fs::write(&input_path, verified_bytes).expect("write verified ast to temp file");

    let backend_bin = project_root().join("backend/build/neuro_backend");
    assert!(
        backend_bin.exists(),
        "Backend binary not found at {:?}",
        backend_bin
    );

    let output = Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to invoke backend");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let ir = if output_path.exists() {
        fs::read_to_string(&output_path).ok()
    } else {
        None
    };

    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();

    (exit_code, stdout, stderr, ir)
}

/// Like run_backend, but explicitly sets NEURO_SIGNING_KEY for the backend process.
fn run_backend_with_key(verified_bytes: &[u8], test_label: &str, key: &str) -> (i32, String, String, Option<String>) {
    let input_path = std::env::temp_dir().join(format!("boundary_{}.verified.ast", test_label));
    let output_path = std::env::temp_dir().join(format!("boundary_{}.output.ll", test_label));

    fs::write(&input_path, verified_bytes).expect("write verified ast to temp file");

    let backend_bin = project_root().join("backend/build/neuro_backend");
    assert!(
        backend_bin.exists(),
        "Backend binary not found at {:?}",
        backend_bin
    );

    let output = Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .env("NEURO_SIGNING_KEY", key)
        .output()
        .expect("failed to invoke backend");

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let ir = if output_path.exists() {
        fs::read_to_string(&output_path).ok()
    } else {
        None
    };

    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();

    (exit_code, stdout, stderr, ir)
}

/// Write a human-readable artifact log for the paper's evaluation section.
fn write_artifact(
    test_name: &str,
    input: &[u8],
    exit_code: i32,
    stdout: &str,
    stderr: &str,
    ir: Option<&str>,
) {
    let dir = artifacts_dir();
    fs::create_dir_all(&dir).expect("create artifacts directory");
    let path = dir.join(format!("{}.log", test_name));

    let hex: String = input.iter().map(|b| format!("{:02x}", b)).collect();
    let mut content = String::new();
    content.push_str(&format!("=== {} ===\n\n", test_name));
    content.push_str(&format!(
        "INPUT (hex-encoded protobuf, {} bytes):\n{}\n\n",
        input.len(),
        hex
    ));
    content.push_str(&format!("EXIT CODE: {}\n\n", exit_code));
    content.push_str(&format!("BACKEND STDOUT:\n{}\n", stdout));
    content.push_str(&format!("BACKEND STDERR:\n{}\n", stderr));
    match ir {
        Some(ll) => content.push_str(&format!("\nOUTPUT IR:\n{}\n", ll)),
        None => content.push_str("\nOUTPUT IR: (none — file not produced)\n"),
    }
    fs::write(&path, &content).expect("write artifact log");
    eprintln!("  Artifact written to: {}", path.display());
}

// ─── Stage 1: Baseline (fail-closed rejection) ───────────────────────

#[test]
fn stage_1a_valid_program_survives_full_pipeline() {
    let program = valid_program();
    let encoded = program.encode_to_vec();

    std::env::set_var("NEURO_SIGNING_KEY", TEST_SIGNING_KEY);
    let verified_bytes = audit_ast(&encoded).expect("audit_ast should succeed for a valid program");

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage1a_valid", TEST_SIGNING_KEY);

    write_artifact(
        "stage1a_valid",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    assert_eq!(
        exit_code,
        0,
        "Backend should exit 0 for a valid program.\nstderr: {}",
        stderr
    );
    let ir_content = ir.expect("output .ll file should exist");
    assert!(
        ir_content.contains("define"),
        "Emitted IR should contain function definitions.\nIR:\n{}",
        ir_content
    );

    eprintln!("PASS: Stage 1a — valid program survives the full pipeline.\n");
}

#[test]
fn stage_1b_use_after_move_rejected_by_analyzer() {
    let program = use_after_move_program();
    let encoded = program.encode_to_vec();

    std::env::set_var("NEURO_SIGNING_KEY", TEST_SIGNING_KEY);
    let result = audit_ast(&encoded);
    assert!(
        result.is_err(),
        "audit_ast MUST reject a use-after-move program. \
         This is the fail-closed path — if this assertion fails, \
         the borrow checker is broken."
    );

    let err = result.unwrap_err();
    eprintln!("PASS: Stage 1b — audit_ast correctly rejected the use-after-move program.");
    eprintln!("  Error: {:?}", err);
    eprintln!("  No VerifiedProgram bytes were produced; the backend was never invoked.\n");
}

// ─── Stage 2: Bypass / Exploit Reproduction ──────────────────────────

/// Before Fix 2: construct a VerifiedProgram by hand (skipping audit_ast
/// entirely) with flags=true, and the backend would emit LLVM IR — the
/// core vulnerability.
///
/// After Fix 2: the same hand-crafted payload is now REJECTED because it
/// carries no valid HMAC signature. This test documents that the bypass
/// is now blocked.
#[test]
fn stage_2_bypass_use_after_move_flags_true() {
    let program = use_after_move_program();

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage2_flags_true", TEST_SIGNING_KEY);
    write_artifact(
        "stage2_flags_true",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    eprintln!("=== Stage 2: Bypass with flags=true (post-fix) ===");
    eprintln!("  Constructed a VerifiedProgram by hand (skipping audit_ast entirely)");
    eprintln!("  containing a use-after-move program with borrow_check_passed=true, type_check_passed=true.");
    eprintln!("  This payload carries NO valid HMAC signature.");

    assert_ne!(
        exit_code,
        0,
        "Backend SHOULD have REJECTED the unsigned payload (Fix 2 blocks the bypass).\n\
         If it exited 0, the HMAC check is not working.\n\
         stderr: {}",
        stderr
    );

    assert!(
        ir.is_none() || !ir.as_ref().map_or(false, |s| s.contains("define")),
        "Backend should NOT have emitted LLVM IR for the unsigned program.\n\
         stderr: {}",
        stderr
    );

    eprintln!("  RESULT: Backend correctly REJECTED the unsigned VerifiedProgram.");
    eprintln!("  The bypass that was the core finding is now blocked by HMAC signing.\n");
}

/// Before Fix 2: flags=false still produced IR — proving flags are unused.
/// After Fix 2: same rejection as flags=true, because the real check is
/// the HMAC signature, not the flags.
#[test]
fn stage_2_bypass_use_after_move_flags_false() {
    let program = use_after_move_program();

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: false,
        type_check_passed: false,
        signature: vec![],
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage2_flags_false", TEST_SIGNING_KEY);
    write_artifact(
        "stage2_flags_false",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    eprintln!("=== Stage 2: Bypass with flags=false (post-fix) ===");
    eprintln!("  Same use-after-move program, but now borrow_check_passed=false AND type_check_passed=false.");

    assert_ne!(
        exit_code,
        0,
        "Backend SHOULD have REJECTED the unsigned payload.\n\
         stderr: {}",
        stderr
    );

    eprintln!("  RESULT: Backend correctly REJECTED the unsigned VerifiedProgram.");
    eprintln!("  The HMAC signature check is the gate — not the boolean flags.\n");
}

// ─── Stage 3: Field Drift on resolved_type and unset oneofs ──────────

/// When resolved_type is left unset (proto3 default), the backend's
/// typeToLLVM(int kind) receives kind=0 (Type_Kind_INT) and silently
/// returns "i32". This is a silent-default, NOT a rejection.
///
/// Source: LLVMEmitter.cpp:53-62 — `default: return "i32";`
///         LLVMEmitter.cpp:243,270 — `has_resolved_type() ? ... : "i32"`
#[test]
fn stage_3a_unset_resolved_type_defaults_to_i32() {
    let program = Program {
        name: "drift_test".to_string(),
        functions: vec![make_fn(
            "main",
            vec![],
            r#type::Kind::Int as i32,
            vec![
                make_decl_mut("x", r#type::Kind::Int as i32, Some(make_lit_int(7))),
                make_return(Some(make_var("x"))),
            ],
        )],
    };

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let verified_bytes = sign_verified_program(&verified, TEST_SIGNING_KEY);

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage3a_unset_type", TEST_SIGNING_KEY);
    write_artifact(
        "stage3a_unset_type",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    assert_eq!(
        exit_code,
        0,
        "Backend should succeed (it silently defaults, not rejects).\nstderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should exist");
    assert!(
        ir_content.contains("i32"),
        "IR should contain i32 operations (silent default from unset resolved_type).\nIR:\n{}",
        ir_content
    );

    eprintln!("=== Stage 3a: Unset resolved_type ===");
    eprintln!("  Expressions had resolved_type=None. Backend silently defaulted to i32.");
    eprintln!("  This is a silent-default, NOT a rejection — the backend trusts whatever types it receives.\n");
}

/// When a Statement has stmt_kind entirely unset (no oneof variant),
/// emitStatement hits `default: break;` (LLVMEmitter.cpp:234-236) and
/// the statement is silently skipped. The function body is emitted
/// with the missing statement omitted.
#[test]
fn stage_3b_unset_stmt_kind_silently_skipped() {
    let program = Program {
        name: "drift_test".to_string(),
        functions: vec![make_fn(
            "main",
            vec![],
            r#type::Kind::Int as i32,
            vec![
                make_decl_mut("x", r#type::Kind::Int as i32, Some(make_lit_int(42))),
                Statement {
                    location: None,
                    stmt_kind: None,
                },
                make_return(Some(make_var("x"))),
            ],
        )],
    };

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let verified_bytes = sign_verified_program(&verified, TEST_SIGNING_KEY);

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage3b_unset_stmt", TEST_SIGNING_KEY);
    write_artifact(
        "stage3b_unset_stmt",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    assert_eq!(
        exit_code,
        0,
        "Backend should succeed (it silently skips, not rejects).\nstderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should exist");
    assert!(
        ir_content.contains("define"),
        "IR should contain the function definition (empty statement was skipped, not fatal).\nIR:\n{}",
        ir_content
    );

    eprintln!("=== Stage 3b: Unset stmt_kind ===");
    eprintln!("  A Statement with no stmt_kind variant was silently skipped by emitStatement.");
    eprintln!("  The function body was emitted with the missing statement omitted — no rejection.\n");
}

/// When an Expression has expr_kind entirely unset (no oneof variant),
/// emitExpression hits `default:` (LLVMEmitter.cpp:446-448) and emits
/// `%rN = add i32 0, 0` — a no-op instruction that is valid LLVM IR
/// but represents a silent data-corruption fallback.
#[test]
fn stage_3b_unset_expr_kind_emits_default() {
    let program = Program {
        name: "drift_test".to_string(),
        functions: vec![make_fn(
            "main",
            vec![],
            r#type::Kind::Int as i32,
            vec![
                make_decl_mut("x", r#type::Kind::Int as i32, Some(make_lit_int(42))),
                Statement {
                    location: None,
                    stmt_kind: Some(StmtKind::ExpressionStmt(Expression {
                        location: None,
                        resolved_type: None,
                        expr_kind: None,
                    })),
                },
                make_return(Some(make_var("x"))),
            ],
        )],
    };

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let verified_bytes = sign_verified_program(&verified, TEST_SIGNING_KEY);

    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(&verified_bytes, "stage3b_unset_expr", TEST_SIGNING_KEY);
    write_artifact(
        "stage3b_unset_expr",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    assert_eq!(
        exit_code,
        0,
        "Backend should succeed (it emits a default, not rejects).\nstderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should exist");
    assert!(
        ir_content.contains("add i32 0, 0"),
        "IR should contain the default `add i32 0, 0` for the unset expression kind.\nIR:\n{}",
        ir_content
    );

    eprintln!("=== Stage 3b: Unset expr_kind ===");
    eprintln!("  An Expression with no expr_kind was handled by the default case.");
    eprintln!("  Backend emitted `add i32 0, 0` as a fallback — silent corruption, not rejection.\n");
}

// ─── Stage 4: Fix 2 — HMAC Signature Verification ────────────────────

/// Test 4a: The exact Stage 2 bypass exploit (hand-crafted VerifiedProgram,
/// no valid signature) should now be REJECTED by the patched backend.
/// This is the paper's "and here's the fix working" evidence.
#[test]
fn stage_4a_bypass_without_signature_rejected() {
    let program = use_after_move_program();

    // Construct VerifiedProgram by hand WITHOUT a signature (mimics the exploit)
    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![], // Empty signature — should be rejected
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage4a_no_signature");

    eprintln!("=== Stage 4a: Bypass without signature ===");
    eprintln!("  Constructed the same hand-crafted VerifiedProgram as Stage 2,");
    eprintln!("  but now the backend requires an HMAC signature.");

    assert_ne!(
        exit_code,
        0,
        "Backend SHOULD have REJECTED the unsigned VerifiedProgram.\n\
         If it exited 0, the HMAC check is not working.\n\
         stderr: {}",
        stderr
    );

    assert!(
        ir.is_none() || !ir.as_ref().map_or(false, |s| s.contains("define")),
        "Backend should NOT have emitted LLVM IR for the unsigned program.\n\
         If IR was produced, the HMAC check is bypassed.\n\
         stderr: {}",
        stderr
    );

    assert!(
        stderr.contains("signature") || stderr.contains("NEURO_SIGNING_KEY"),
        "Error message should mention signature or signing key.\n\
         stderr: {}",
        stderr
    );

    eprintln!("  RESULT: Backend correctly REJECTED the unsigned VerifiedProgram.");
    eprintln!("  The HMAC signature check prevented the Stage 2 bypass.\n");
}

/// Test 4b: A VerifiedProgram with an INVALID signature should also be rejected.
/// This tests that the HMAC comparison is actually performed, not just a length check.
#[test]
fn stage_4b_bypass_with_invalid_signature_rejected() {
    let program = use_after_move_program();

    // Construct VerifiedProgram with a garbage signature (32 bytes, but wrong)
    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![0xAB; 32], // Invalid signature
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage4b_bad_signature");

    eprintln!("=== Stage 4b: Bypass with invalid signature ===");
    eprintln!("  Constructed VerifiedProgram with a 32-byte garbage signature.");

    assert_ne!(
        exit_code,
        0,
        "Backend SHOULD have REJECTED the VerifiedProgram with invalid signature.\n\
         stderr: {}",
        stderr
    );

    assert!(
        ir.is_none() || !ir.as_ref().map_or(false, |s| s.contains("define")),
        "Backend should NOT have emitted LLVM IR for the invalid signature.\n\
         stderr: {}",
        stderr
    );

    eprintln!("  RESULT: Backend correctly REJECTED the invalid signature.");
    eprintln!("  The HMAC comparison is actually performed, not just a length check.\n");
}

/// Test 4c: The legitimate pipeline (audit_ast → backend) should still work
/// end-to-end with the HMAC signature in place. This confirms that Fix 2
/// doesn't break the honest path.
#[test]
fn stage_4c_legitimate_pipeline_still_works() {
    let program = valid_program();
    let encoded = program.encode_to_vec();

    // Set the signing key for the analyzer
    std::env::set_var("NEURO_SIGNING_KEY", "test-secret-key-for-stage4c");

    let verified_bytes = audit_ast(&encoded).expect("audit_ast should succeed for a valid program");

    // Set the same key for the backend
    let (exit_code, _stdout, stderr, ir) = run_backend_with_key(
        &verified_bytes,
        "stage4c_legitimate",
        "test-secret-key-for-stage4c",
    );

    eprintln!("=== Stage 4c: Legitimate pipeline with HMAC ===");
    eprintln!("  Ran the full audit_ast → backend pipeline with signing enabled.");

    assert_eq!(
        exit_code,
        0,
        "Backend should exit 0 for a legitimately signed program.\n\
         stderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should exist");
    assert!(
        ir_content.contains("define"),
        "Emitted IR should contain function definitions.\nIR:\n{}",
        ir_content
    );

    eprintln!("  RESULT: Legitimate pipeline works correctly with HMAC signing.");
    eprintln!("  The fix doesn't break the honest path.\n");
}

/// Test 4d: Missing NEURO_SIGNING_KEY on the backend side should also fail closed.
/// This confirms that the backend refuses to operate without the signing key.
#[test]
fn stage_4d_missing_key_on_backend_fails_closed() {
    let program = valid_program();
    let encoded = program.encode_to_vec();

    // Set a key for the analyzer so it can sign
    std::env::set_var("NEURO_SIGNING_KEY", "test-key");

    let verified_bytes = audit_ast(&encoded).expect("audit_ast should succeed");

    // Run backend WITHOUT the key (use env_remove on the child process)
    let input_path = std::env::temp_dir().join("boundary_stage4d_no_backend_key.verified.ast");
    let output_path = std::env::temp_dir().join("boundary_stage4d_no_backend_key.output.ll");
    fs::write(&input_path, &verified_bytes).expect("write verified ast");

    let backend_bin = project_root().join("backend/build/neuro_backend");
    let output = Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .env_remove("NEURO_SIGNING_KEY")
        .output()
        .expect("failed to invoke backend");

    let exit_code = output.status.code().unwrap_or(-1);
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();

    eprintln!("=== Stage 4d: Missing key on backend ===");
    eprintln!("  Signed with key, but backend has no NEURO_SIGNING_KEY set.");

    assert_ne!(
        exit_code,
        0,
        "Backend SHOULD fail closed when NEURO_SIGNING_KEY is not set.\n\
         stderr: {}",
        stderr
    );

    eprintln!("  RESULT: Backend correctly fails closed without NEURO_SIGNING_KEY.\n");
}

// ─── Overhead Measurement ────────────────────────────────────────────

#[test]
fn measure_signing_overhead() {
    let program = valid_program();
    let key = "overhead-measurement-key";

    // --- Byte count ---
    let unsigned = VerifiedProgram {
        program: Some(program.clone()),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let unsigned_bytes = unsigned.encode_to_vec();

    std::env::set_var("NEURO_SIGNING_KEY", key);
    let signed_bytes = sign_verified_program(&unsigned, key);

    eprintln!("=== Overhead Measurement ===");
    eprintln!("  Unsigned VerifiedProgram: {} bytes", unsigned_bytes.len());
    eprintln!("  Signed VerifiedProgram:   {} bytes", signed_bytes.len());
    eprintln!("  Signature overhead:       {} bytes (HMAC-SHA256)", signed_bytes.len() - unsigned_bytes.len());

    // --- Sign timing (Rust analyzer side) ---
    let iterations = 200;
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = sign_verified_program(&unsigned, key);
    }
    let sign_elapsed = start.elapsed();
    let sign_per_op = sign_elapsed / iterations;
    eprintln!("  Sign (Rust, {} iters):  {:?} total, {:?}/op", iterations, sign_elapsed, sign_per_op);

    // --- Verify timing (simulated — same HMAC computation as C++ backend) ---
    let signed_vp = VerifiedProgram::decode(signed_bytes.as_slice()).unwrap();
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        let program_bytes = signed_vp.program.as_ref().unwrap().encode_to_vec();
        let mut data = Vec::new();
        data.extend_from_slice(&(program_bytes.len() as u64).to_le_bytes());
        data.extend_from_slice(&program_bytes);
        data.push(signed_vp.borrow_check_passed as u8);
        data.push(signed_vp.type_check_passed as u8);
        type H = Hmac<Sha256>;
        let mut mac = H::new_from_slice(key.as_bytes()).unwrap();
        mac.update(&data);
        let tag = mac.finalize().into_bytes().to_vec();
        let _ = tag == signed_vp.signature;
    }
    let verify_elapsed = start.elapsed();
    let verify_per_op = verify_elapsed / iterations;
    eprintln!("  Verify (Rust, {} iters): {:?} total, {:?}/op", iterations, verify_elapsed, verify_per_op);
    eprintln!("  Combined sign+verify:    {:?}/op", sign_per_op + verify_per_op);

    // --- End-to-end: sign (Rust) → write file → C++ backend verify+emit → cleanup ---
    std::env::set_var("NEURO_SIGNING_KEY", key);
    let backend_bin = project_root().join("backend/build/neuro_backend");
    assert!(backend_bin.exists(), "Backend binary not found");

    let iterations_e2e = 50;
    let start = std::time::Instant::now();
    for _ in 0..iterations_e2e {
        let signed = sign_verified_program(&unsigned, key);
        let input_path = std::env::temp_dir().join("overhead_e2e.verified.ast");
        let output_path = std::env::temp_dir().join("overhead_e2e.output.ll");
        fs::write(&input_path, &signed).expect("write");
        let _ = Command::new(&backend_bin)
            .arg(input_path.to_str().unwrap())
            .arg(output_path.to_str().unwrap())
            .env("NEURO_SIGNING_KEY", key)
            .output();
        fs::remove_file(&input_path).ok();
        fs::remove_file(&output_path).ok();
    }
    let e2e_total = start.elapsed();
    let e2e_per_op = e2e_total / iterations_e2e;
    eprintln!("  E2E (sign+write+C++verify+emit, {} iters): {:?}/op", iterations_e2e, e2e_per_op);
    eprintln!();
}

// ─── Timing Side-Channel Micro-Benchmark ──────────────────────────────

#[test]
fn measure_timing_sidechannel() {
    use std::time::Instant;

    let backend_bin = project_root().join("backend/build/neuro_backend");
    assert!(backend_bin.exists(), "Backend binary not found");

    let key = "timing-test-key";
    let program = valid_program();

    // Signed VP with correct key (baseline — should succeed)
    let signed = sign_verified_program(
        &VerifiedProgram {
            program: Some(program.clone()),
            borrow_check_passed: true,
            type_check_passed: true,
            signature: vec![],
        },
        key,
    );

    // Signed VP with WRONG signature (key present, sig wrong)
    let mut wrong_sig = signed.clone();
    let last = wrong_sig.len() - 1;
    wrong_sig[last] ^= 0xFF; // flip last byte

    // Unsigned VP (no signature field — key present, no sig)
    let unsigned = VerifiedProgram {
        program: Some(program.clone()),
        borrow_check_passed: true,
        type_check_passed: true,
        signature: vec![],
    };
    let unsigned_bytes = unsigned.encode_to_vec();

    let iterations = 1000;
    let input_dir = std::env::temp_dir();

    // --- Case A: correct key, wrong signature ---
    std::env::set_var("NEURO_SIGNING_KEY", key);
    let start = Instant::now();
    for _ in 0..iterations {
        let ip = input_dir.join("timing_a.verified.ast");
        let op = input_dir.join("timing_a.output.ll");
        fs::write(&ip, &wrong_sig).unwrap();
        let _ = Command::new(&backend_bin)
            .arg(ip.to_str().unwrap())
            .arg(op.to_str().unwrap())
            .env("NEURO_SIGNING_KEY", key)
            .output();
        fs::remove_file(&ip).ok();
        fs::remove_file(&op).ok();
    }
    let wrong_sig_elapsed = start.elapsed();
    let wrong_sig_per_op = wrong_sig_elapsed / iterations;

    // --- Case B: key present, no signature ---
    let start = Instant::now();
    for _ in 0..iterations {
        let ip = input_dir.join("timing_b.verified.ast");
        let op = input_dir.join("timing_b.output.ll");
        fs::write(&ip, &unsigned_bytes).unwrap();
        let _ = Command::new(&backend_bin)
            .arg(ip.to_str().unwrap())
            .arg(op.to_str().unwrap())
            .env("NEURO_SIGNING_KEY", key)
            .output();
        fs::remove_file(&ip).ok();
        fs::remove_file(&op).ok();
    }
    let no_sig_elapsed = start.elapsed();
    let no_sig_per_op = no_sig_elapsed / iterations;

    eprintln!("=== Timing Side-Channel Micro-Benchmark ({} iterations) ===", iterations);
    eprintln!("  Case A (key set, wrong signature):  {:?}/op", wrong_sig_per_op);
    eprintln!("  Case B (key set, no signature):     {:?}/op", no_sig_per_op);
    let delta = if wrong_sig_per_op > no_sig_per_op {
        wrong_sig_per_op - no_sig_per_op
    } else {
        no_sig_per_op - wrong_sig_per_op
    };
    eprintln!("  Delta:                              {:?}/op", delta);
    eprintln!("  Conclusion: {}", if delta.as_nanos() < 1000 {
        "NOT EXPLOITABLE — delta < 1µs (within measurement noise)"
    } else {
        "INCONCLUSIVE — delta > 1µs, may warrant constant-time comparison"
    });
    eprintln!();
}
