//! Boundary tests: demonstrate the unauthenticated provenance boundary
//! between the Rust analyzer and the C++ backend.
//!
//! Stage 1 (Baseline):      Confirm fail-closed rejection works as expected.
//! Stage 2 (Bypass):        Reproduce the unauthenticated provenance boundary finding.
//! Stage 3 (Field Drift):   Test behavior with unset/malformed protobuf fields.

use analyzer::audit_ast;
use shared_ast::expression::ExprKind;
use shared_ast::statement::StmtKind;
use shared_ast::*;
use prost::Message;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    let verified_bytes = audit_ast(&encoded).expect("audit_ast should succeed for a valid program");

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage1a_valid");
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

/// Core finding: construct a VerifiedProgram by hand (skipping audit_ast
/// entirely) containing a use-after-move program, with both safety flags
/// set to true. The backend should emit LLVM IR for this program,
/// demonstrating that the provenance boundary is unauthenticated.
#[test]
fn stage_2_bypass_use_after_move_flags_true() {
    let program = use_after_move_program();

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage2_flags_true");
    write_artifact(
        "stage2_flags_true",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    eprintln!("=== Stage 2: Bypass with flags=true ===");
    eprintln!("  Constructed a VerifiedProgram by hand (skipping audit_ast entirely)");
    eprintln!("  containing a use-after-move program with borrow_check_passed=true, type_check_passed=true.");

    assert_eq!(
        exit_code,
        0,
        "Backend SHOULD have exited successfully (this is the vulnerability). \
         If it rejected the input, some unknown check exists.\n\
         stderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should have been produced");
    assert!(
        ir_content.contains("define"),
        "Emitted IR should contain function definitions.\n\
         If missing, the backend silently failed.\nIR:\n{}",
        ir_content
    );

    eprintln!("  RESULT: Backend emitted valid LLVM IR for a use-after-move program");
    eprintln!("  without going through the analyzer, and with borrow_check_passed=true.");
    eprintln!("  The safety property (no use-after-move) was NOT enforced at the boundary.\n");
}

/// Second variant: same malformed program, but now both safety flags are
/// false. The backend should STILL emit LLVM IR, proving that the flags
/// are read by nothing — not just that true values are trusted.
#[test]
fn stage_2_bypass_use_after_move_flags_false() {
    let program = use_after_move_program();

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: false,
        type_check_passed: false,
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage2_flags_false");
    write_artifact(
        "stage2_flags_false",
        &verified_bytes,
        exit_code,
        "",
        &stderr,
        ir.as_deref(),
    );

    eprintln!("=== Stage 2: Bypass with flags=false ===");
    eprintln!("  Same use-after-move program, but now borrow_check_passed=false AND type_check_passed=false.");

    assert_eq!(
        exit_code,
        0,
        "Backend SHOULD have exited successfully (proving flags are not checked). \
         If it rejected the input, some unknown check exists.\n\
         stderr: {}",
        stderr
    );

    let ir_content = ir.expect("output .ll file should have been produced");
    assert!(
        ir_content.contains("define"),
        "Emitted IR should contain function definitions.\nIR:\n{}",
        ir_content
    );

    eprintln!("  RESULT: Backend emitted valid LLVM IR even with borrow_check_passed=false.");
    eprintln!("  This proves the flags are read by NOTHING — not just that true values are trusted.\n");
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
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage3a_unset_type");
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
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage3b_unset_stmt");
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
    };
    let verified_bytes = verified.encode_to_vec();

    let (exit_code, _stdout, stderr, ir) = run_backend(&verified_bytes, "stage3b_unset_expr");
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
