use analyzer::audit_ast;
use shared_ast;
use prost::Message;
use std::process::Command;
use std::path::PathBuf;
use std::fs;

/// Project root relative to this test file (tests/../)
fn project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn make_type(kind: i32) -> Option<shared_ast::Type> {
    Some(shared_ast::Type { kind, custom_name: String::new() })
}

fn make_lit_int(val: i64) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::IntVal(val)),
        })),
    }
}

fn make_var(name: &str) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Variable(shared_ast::VariableReference {
            name: name.to_string(),
        })),
    }
}

fn make_binary_op(op: i32, left: shared_ast::Expression, right: shared_ast::Expression) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::BinaryOp(Box::new(shared_ast::BinaryOperation {
            op,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }))),
    }
}

fn make_fn(name: &str, params: Vec<shared_ast::Parameter>, ret_kind: i32, body: Vec<shared_ast::Statement>) -> shared_ast::Function {
    shared_ast::Function {
        name: name.to_string(),
        parameters: params,
        return_type: make_type(ret_kind),
        body,
        location: None,
    }
}

fn make_decl(name: &str, kind: i32, init: Option<shared_ast::Expression>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::Declaration(shared_ast::VariableDeclaration {
            name: name.to_string(),
            r#type: make_type(kind),
            initializer: init,
            is_mutable: false,
            resolved_type: None,
        })),
    }
}

fn make_return(val: Option<shared_ast::Expression>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::ReturnStmt(shared_ast::ReturnStatement { value: val })),
    }
}

fn make_param(name: &str, kind: i32) -> shared_ast::Parameter {
    shared_ast::Parameter {
        name: name.to_string(),
        r#type: make_type(kind),
    }
}

#[test]
fn test_backend_generates_llvm_ir() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("add", vec![make_param("a", 0), make_param("b", 0)], 0, vec![
                make_return(Some(make_binary_op(0, make_var("a"), make_var("b")))),
            ]),
            make_fn("main", vec![], 0, vec![
                make_decl("x", 0, Some(make_lit_int(42))),
                make_decl("y", 0, Some(make_lit_int(10))),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };

    let encoded = prog.encode_to_vec();
    let verified = audit_ast(&encoded).expect("audit_ast should succeed");

    let input_path = PathBuf::from(std::env::temp_dir()).join("test_verified.ast");
    let output_path = PathBuf::from(std::env::temp_dir()).join("test_output.ll");
    fs::write(&input_path, &verified).expect("write test verified ast");

    let backend_bin = project_root().join("backend/build/neuro_backend");
    if !backend_bin.exists() {
        panic!("Backend binary not found at {:?}", backend_bin);
    }

    let output = Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .output()
        .expect("failed to invoke backend");

    assert!(output.status.success(),
        "Backend exited with code {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr));

    let ll_content = fs::read_to_string(&output_path).expect("read output ll");
    assert!(ll_content.contains("define"), "LLVM IR should contain function definitions");
    assert!(ll_content.contains("@printf"), "LLVM IR should declare libc functions");

    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();
}
