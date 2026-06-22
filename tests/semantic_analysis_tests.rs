use analyzer::semantic_analysis::analyze_ast;
use shared_ast::*;

fn build_program(name: &str, functions: Vec<Function>) -> Program {
    Program {
        name: name.to_string(),
        functions,
    }
}

fn make_type(kind: i32) -> Option<Type> {
    Some(Type { kind, custom_name: String::new() })
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
        expr_kind: Some(expression::ExprKind::Literal(Literal {
            value: Some(literal::Value::IntVal(val)),
        })),
    }
}

fn make_lit_float(val: f64) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(expression::ExprKind::Literal(Literal {
            value: Some(literal::Value::FloatVal(val)),
        })),
    }
}

fn make_lit_bool(val: bool) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(expression::ExprKind::Literal(Literal {
            value: Some(literal::Value::BoolVal(val)),
        })),
    }
}

fn make_var(name: &str) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(expression::ExprKind::Variable(VariableReference {
            name: name.to_string(),
        })),
    }
}

fn make_decl(name: &str, kind: i32, init: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::Declaration(VariableDeclaration {
            name: name.to_string(),
            r#type: make_type(kind),
            initializer: init,
        })),
    }
}

fn make_return(val: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::ReturnStmt(ReturnStatement { value: val })),
    }
}

fn make_fn(name: &str, params: Vec<Parameter>, ret_kind: i32, body: Vec<Statement>) -> Function {
    Function {
        name: name.to_string(),
        parameters: params,
        return_type: make_type(ret_kind),
        body,
        location: None,
    }
}

#[test]
fn empty_program() {
    let prog = build_program("test", vec![]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn function_no_params_void_body() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn int_declaration_with_init() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![
            make_decl("x", 0, Some(make_lit_int(42))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn type_mismatch_declaration() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![
            make_decl("x", 1, Some(make_lit_int(42))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn return_int_matches() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 0, vec![
            make_return(Some(make_lit_int(0))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn return_type_mismatch() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 0, vec![
            make_return(Some(make_lit_float(1.0))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn param_inserted_and_usable() {
    let prog = build_program("test", vec![
        make_fn("add", vec![make_param("x", 0), make_param("y", 0)], 0, vec![
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn undefined_variable() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 0, vec![
            make_return(Some(make_var("undefined"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn if_bool_condition() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![
            Statement {
                location: None,
                stmt_kind: Some(statement::StmtKind::IfStmt(IfStatement {
                    condition: Some(make_lit_bool(true)),
                    true_branch: vec![],
                    false_branch: vec![],
                })),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn void_returns_nothing() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![
            make_return(None),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn if_non_bool_condition() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], 4, vec![
            Statement {
                location: None,
                stmt_kind: Some(statement::StmtKind::IfStmt(IfStatement {
                    condition: Some(make_lit_int(1)),
                    true_branch: vec![],
                    false_branch: vec![],
                })),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}
