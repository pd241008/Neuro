use analyzer::audit_ast;
use shared_ast;
use prost::Message;
use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn unique_path(suffix: &str) -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(format!("/tmp/neuro_test_{}_{}", n, suffix))
}

fn make_type(kind: i32) -> Option<shared_ast::Type> {
    Some(shared_ast::Type { kind, custom_name: String::new() })
}

fn make_lit_int(val: i64) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::IntVal(val)),
        })),
    }
}

fn make_lit_float(val: f64) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::FloatVal(val)),
        })),
    }
}

fn make_lit_bool(val: bool) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::BoolVal(val)),
        })),
    }
}

fn make_lit_string(s: &str) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::StringVal(s.to_string())),
        })),
    }
}

fn make_var(name: &str) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Variable(shared_ast::VariableReference {
            name: name.to_string(),
        })),
    }
}

fn make_binary_op(op: i32, left: shared_ast::Expression, right: shared_ast::Expression) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::BinaryOp(Box::new(shared_ast::BinaryOperation {
            op, left: Some(Box::new(left)), right: Some(Box::new(right)),
        }))),
    }
}

fn make_unary_op(op: i32, operand: shared_ast::Expression) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::UnaryOp(Box::new(shared_ast::UnaryOperation {
            op, operand: Some(Box::new(operand)),
        }))),
    }
}

fn make_call(name: &str, args: Vec<shared_ast::Expression>) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Call(shared_ast::FunctionCall {
            function_name: name.to_string(), arguments: args,
        })),
    }
}

fn make_fn(name: &str, params: Vec<shared_ast::Parameter>, ret_kind: i32, body: Vec<shared_ast::Statement>) -> shared_ast::Function {
    shared_ast::Function {
        name: name.to_string(), parameters: params,
        return_type: make_type(ret_kind), body, location: None,
    }
}

fn make_decl(name: &str, kind: i32, init: Option<shared_ast::Expression>, is_mut: bool) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::Declaration(shared_ast::VariableDeclaration {
            name: name.to_string(), r#type: make_type(kind),
            initializer: init, is_mutable: is_mut, resolved_type: None,
        })),
    }
}

fn make_assign(name: &str, value: shared_ast::Expression) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::Assignment(shared_ast::Assignment {
            target_name: name.to_string(), value: Some(value),
        })),
    }
}

fn make_return(val: Option<shared_ast::Expression>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::ReturnStmt(shared_ast::ReturnStatement { value: val })),
    }
}

fn make_expression_stmt(expr: shared_ast::Expression) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::ExpressionStmt(expr)),
    }
}

fn make_if(cond: shared_ast::Expression, tb: Vec<shared_ast::Statement>, fb: Vec<shared_ast::Statement>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::IfStmt(shared_ast::IfStatement {
            condition: Some(cond), true_branch: tb, false_branch: fb,
        })),
    }
}

fn make_while(cond: shared_ast::Expression, body: Vec<shared_ast::Statement>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::WhileStmt(shared_ast::WhileStatement {
            condition: Some(cond), body,
        })),
    }
}

fn make_param(name: &str, kind: i32) -> shared_ast::Parameter {
    shared_ast::Parameter { name: name.to_string(), r#type: make_type(kind) }
}

fn run_backend(prog: shared_ast::Program) -> (String, PathBuf, PathBuf) {
    let key = "lowering-test-hmac-key";
    std::env::set_var("NEURO_SIGNING_KEY", key);

    let encoded = prog.encode_to_vec();
    let verified = audit_ast(&encoded).expect("audit_ast should succeed");

    let input_path = unique_path("verified.ast");
    let output_path = unique_path("output.ll");
    fs::write(&input_path, &verified).expect("write test verified ast");

    let backend_bin = project_root().join("backend/build/neuro_backend");
    assert!(backend_bin.exists(), "Backend binary not found at {:?}", backend_bin);

    let output = Command::new(&backend_bin)
        .arg(input_path.to_str().unwrap())
        .arg(output_path.to_str().unwrap())
        .env("NEURO_SIGNING_KEY", key)
        .output()
        .expect("failed to invoke backend");

    assert!(output.status.success(),
        "Backend exited with code {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr));

    let ll_content = fs::read_to_string(&output_path).expect("read output ll");
    (ll_content, input_path, output_path)
}

fn cleanup(paths: &[PathBuf]) {
    for p in paths {
        fs::remove_file(p).ok();
    }
}

#[test]
fn test_lowering_int_literal() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_decl("x", 0, Some(make_lit_int(42)), false),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("add i32 0, 42"), "int literal should produce add: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_float_literal() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 1, vec![
                make_decl("x", 1, Some(make_lit_float(3.14)), false),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("fadd double 0.0, 3.14"), "float literal: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_bool_literal() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 2, vec![
                make_return(Some(make_lit_bool(true))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("add i1 0, 1"), "bool true literal: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_string_literal() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 4, vec![
                make_expression_stmt(make_call("print", vec![make_lit_string("hi")])),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("@.str."), "string literal should produce global: {}", ll);
    assert!(ll.contains("getelementptr inbounds"), "string literal needs GEP: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_unary_neg() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_unary_op(0, make_lit_int(5)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("sub i32 0,"), "negate should use sub from 0: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_unary_not() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 2, vec![
                make_return(Some(make_unary_op(1, make_lit_bool(false)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("xor i1 1,"), "not should use xor: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_binary_add() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_binary_op(0, make_lit_int(3), make_lit_int(4)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("add i32"), "add: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_binary_sub() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_binary_op(1, make_lit_int(10), make_lit_int(3)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("sub i32"), "sub: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_binary_mul() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_binary_op(2, make_lit_int(6), make_lit_int(7)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("mul i32"), "mul: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_binary_div() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_binary_op(3, make_lit_int(10), make_lit_int(2)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("sdiv i32"), "div: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_float_add() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 1, vec![
                make_return(Some(make_binary_op(0, make_lit_float(1.5), make_lit_float(2.5)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("fadd double"), "float add: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_comparison_eq() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 2, vec![
                make_return(Some(make_binary_op(4, make_lit_int(1), make_lit_int(2)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("icmp eq"), "eq: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_comparison_lt() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 2, vec![
                make_return(Some(make_binary_op(6, make_lit_int(1), make_lit_int(2)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("icmp slt"), "lt: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_comparison_gt() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 2, vec![
                make_return(Some(make_binary_op(7, make_lit_int(2), make_lit_int(1)))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("icmp sgt"), "gt: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_if_else() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 4, vec![
                make_if(make_lit_bool(true), vec![
                    make_decl("x", 0, Some(make_lit_int(1)), false),
                ], vec![
                    make_decl("y", 0, Some(make_lit_int(2)), false),
                ]),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("br i1"), "if needs branch: {}", ll);
    assert!(ll.contains("if_true_"), "if_true label: {}", ll);
    assert!(ll.contains("if_false_"), "if_false label: {}", ll);
    assert!(ll.contains("if_merge_"), "if_merge label: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_while() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_decl("x", 0, Some(make_lit_int(0)), true),
                make_while(make_binary_op(6, make_var("x"), make_lit_int(10)), vec![
                    make_assign("x", make_binary_op(0, make_var("x"), make_lit_int(1))),
                ]),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("while_cond_"), "while_cond label: {}", ll);
    assert!(ll.contains("while_body_"), "while_body label: {}", ll);
    assert!(ll.contains("while_end_"), "while_end label: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_assignment() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_decl("x", 0, Some(make_lit_int(0)), true),
                make_assign("x", make_lit_int(42)),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("alloca i32"), "declaration needs alloca: {}", ll);
    assert!(ll.contains("store i32"), "assignment needs store: {}", ll);
    assert!(ll.contains("load i32"), "return needs load: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_function_call() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("add", vec![make_param("a", 0), make_param("b", 0)], 0, vec![
                make_return(Some(make_binary_op(0, make_var("a"), make_var("b")))),
            ]),
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_call("add", vec![make_lit_int(3), make_lit_int(4)]))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("define i32 @_add"), "function definition: {}", ll);
    assert!(ll.contains("define i32 @main"), "main definition: {}", ll);
    assert!(ll.contains("call i32 @_add"), "function call: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_print_int() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 4, vec![
                make_expression_stmt(make_call("print", vec![make_lit_int(42)])),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("@printf"), "print maps to printf: {}", ll);
    assert!(ll.contains("%d"), "int print uses %d format: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_println() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 4, vec![
                make_expression_stmt(make_call("println", vec![make_lit_string("hello")])),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("@printf"), "println maps to printf: {}", ll);
    assert!(ll.contains("\\0A"), "println adds newline: {}", ll);
    cleanup(&[ip, op]);
}

#[test]
fn test_lowering_read() {
    let prog = shared_ast::Program {
        name: "test".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_decl("x", 0, Some(make_call("read", vec![])), false),
                make_return(Some(make_var("x"))),
            ]),
        ],
    };
    let (ll, ip, op) = run_backend(prog);
    assert!(ll.contains("@scanf"), "read maps to scanf: {}", ll);
    assert!(ll.contains("alloca"), "scanf needs alloca target: {}", ll);
    cleanup(&[ip, op]);
}
