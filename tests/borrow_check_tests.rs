use analyzer::semantic_analysis::analyze_ast;
use shared_ast::*;
use shared_ast::r#type::Kind;
use shared_ast::expression::ExprKind;
use shared_ast::statement::StmtKind;

fn build_program(name: &str, mut functions: Vec<Function>) -> Program {
    functions.push(make_fn("foo", vec![make_param("arg", Kind::Int as i32)], Kind::Void as i32, vec![]));
    functions.push(make_fn("bar", vec![make_param("arg", Kind::Bool as i32)], Kind::Void as i32, vec![]));
    
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
        expr_kind: Some(ExprKind::Literal(Literal {
            value: Some(literal::Value::IntVal(val)),
        })),
    }
}

fn make_lit_bool(val: bool) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(ExprKind::Literal(Literal {
            value: Some(literal::Value::BoolVal(val)),
        })),
    }
}

fn make_var(name: &str) -> Expression {
    Expression {
        location: None,
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
        })),
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
            expr_kind: Some(ExprKind::Call(FunctionCall {
                function_name: fn_name.to_string(),
                arguments: args,
            })),
        })),
    }
}

#[test]
fn read_after_declaration_is_ok() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(42))),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::ExpressionStmt(make_var("x"))),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn move_variable_then_read_rejected() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(42))),
            make_call("foo", vec![make_var("x")]),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::ExpressionStmt(make_var("x"))),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn move_then_reassign_is_ok() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(42))),
            make_call("foo", vec![make_var("x")]),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::Assignment(Assignment {
                    target_name: "x".to_string(),
                    value: Some(make_lit_int(99)),
                })),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn move_variable_twice_rejected() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(42))),
            make_call("foo", vec![make_var("x")]),
            make_call("foo", vec![make_var("x")]),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn move_literal_arg_is_ok() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_call("foo", vec![make_lit_int(42)]),
            make_call("bar", vec![make_lit_bool(true)]),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn reassigned_variable_becomes_readable_again() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(42))),
            make_call("foo", vec![make_var("x")]),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::Assignment(Assignment {
                    target_name: "x".to_string(),
                    value: Some(make_lit_int(99)),
                })),
            },
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn scope_shadowing_does_not_affect_outer_variable() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(10))),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::IfStmt(IfStatement {
                    condition: Some(make_lit_bool(true)),
                    true_branch: vec![
                        make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(20))),
                        make_call("foo", vec![make_var("x")]),
                    ],
                    false_branch: vec![],
                })),
            },
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn unmoved_variable_still_readable() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(10))),
            make_call("foo", vec![make_lit_int(1)]),
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn move_in_inner_scope_affects_outer_scope_read() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(10))),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::IfStmt(IfStatement {
                    condition: Some(make_lit_bool(true)),
                    true_branch: vec![
                        make_call("foo", vec![make_var("x")]),
                    ],
                    false_branch: vec![],
                })),
            },
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn move_in_inner_scope_then_reassign_in_inner_scope_restores_outer() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, Some(make_lit_int(10))),
            Statement {
                location: None,
                stmt_kind: Some(StmtKind::IfStmt(IfStatement {
                    condition: Some(make_lit_bool(true)),
                    true_branch: vec![
                        make_call("foo", vec![make_var("x")]),
                        Statement {
                            location: None,
                            stmt_kind: Some(StmtKind::Assignment(Assignment {
                                target_name: "x".to_string(),
                                value: Some(make_lit_int(99)),
                            })),
                        },
                    ],
                    false_branch: vec![],
                })),
            },
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}
