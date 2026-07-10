use analyzer::audit_ast;
use shared_ast;
use prost::Message;
use std::fs;
use std::path::PathBuf;

fn make_type(kind: i32) -> Option<shared_ast::Type> {
    Some(shared_ast::Type { kind, custom_name: String::new() })
}

fn make_lit_string(s: &str) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::StringVal(s.to_string())),
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

fn make_call(name: &str, args: Vec<shared_ast::Expression>) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Call(shared_ast::FunctionCall {
            function_name: name.to_string(),
            arguments: args,
        })),
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

fn make_decl(name: &str, kind: i32, init: Option<shared_ast::Expression>, is_mut: bool) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::Declaration(shared_ast::VariableDeclaration {
            name: name.to_string(),
            r#type: make_type(kind),
            initializer: init,
            is_mutable: is_mut,
            resolved_type: None,
        })),
    }
}

fn make_expression_stmt(expr: shared_ast::Expression) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::ExpressionStmt(expr)),
    }
}

#[test]
fn test_print_read_builtins() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 4, vec![
                make_decl("msg", 3, Some(make_lit_string("hello world")), false),
                make_expression_stmt(make_call("print", vec![make_lit_string("hello")])),
                make_expression_stmt(make_call("println", vec![make_var("msg")])),
                make_expression_stmt(make_call("read", vec![])),
            ]),
        ],
    };

    let key = "print-read-test-hmac-key";
    std::env::set_var("NEURO_SIGNING_KEY", key);

    let encoded = prog.encode_to_vec();
    let verified = audit_ast(&encoded).expect("audit_ast should succeed for built-in I/O calls");

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap().to_path_buf();

    let input_path = PathBuf::from("/tmp/test_io_verified.ast");
    let output_path = PathBuf::from("/tmp/test_io_output.ll");
    fs::write(&input_path, &verified).expect("write test verified ast");

    let backend_bin = project_root.join("backend/build/neuro_backend");
    if !backend_bin.exists() {
        panic!("Backend not built at {:?}", backend_bin);
    }

    let output = std::process::Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .env("NEURO_SIGNING_KEY", key)
        .output()
        .expect("failed to invoke backend");

    assert!(output.status.success(),
        "Backend exited with code {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr));

    let ll = fs::read_to_string(&output_path).expect("read output ll");

    // Verify printf/scanf declarations
    assert!(ll.contains("@printf"));
    assert!(ll.contains("@scanf"));

    // Verify print call emits a format string constant and printf call
    assert!(ll.contains("@.str.0"));
    assert!(ll.contains("@printf(i8*"));

    // Verify read call emits alloca, scanf, and load
    assert!(ll.contains("alloca"));
    assert!(ll.contains("@scanf(i8*"));
    assert!(ll.contains("load i32"));

    fs::remove_file(&input_path).ok();
    fs::remove_file(&output_path).ok();
}
