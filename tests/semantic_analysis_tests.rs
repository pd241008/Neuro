use analyzer::semantic_analysis::analyze_ast;
use shared_ast::*;
use shared_ast::r#type::Kind;

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
            is_mutable: false,
        })),
    }
}

fn make_decl_mut(name: &str, kind: i32, init: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::Declaration(VariableDeclaration {
            name: name.to_string(),
            r#type: make_type(kind),
            initializer: init,
            is_mutable: true,
        })),
    }
}

fn make_return(val: Option<Expression>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::ReturnStmt(ReturnStatement { value: val })),
    }
}

fn make_assign(name: &str, value: Expression) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::Assignment(Assignment {
            target_name: name.to_string(),
            value: Some(value),
        })),
    }
}

fn make_while(cond: Expression, body: Vec<Statement>) -> Statement {
    Statement {
        location: None,
        stmt_kind: Some(statement::StmtKind::WhileStmt(WhileStatement {
            condition: Some(cond),
            body,
        })),
    }
}

fn make_binary_op(op: i32, left: Expression, right: Expression) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(expression::ExprKind::BinaryOp(Box::new(BinaryOperation {
            op,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        }))),
    }
}

fn make_unary_op(op: i32, operand: Expression) -> Expression {
    Expression {
        location: None,
        expr_kind: Some(expression::ExprKind::UnaryOp(Box::new(UnaryOperation {
            op,
            operand: Some(Box::new(operand)),
        }))),
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

#[test]
fn while_bool_condition() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_while(make_lit_bool(true), vec![]),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn while_non_bool_condition() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_while(make_lit_int(1), vec![]),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn binary_arithmetic_int() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_return(Some(make_binary_op(0, make_lit_int(1), make_lit_int(2)))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn binary_arithmetic_type_mismatch() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl("x", Kind::Int as i32, Some(
                make_binary_op(0, make_lit_int(1), make_lit_float(2.0)),
            )),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn binary_comparison_returns_bool() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Bool as i32, vec![
            make_return(Some(make_binary_op(6, make_lit_int(1), make_lit_int(2)))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn binary_logical_and() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Bool as i32, vec![
            make_return(Some(make_binary_op(10, make_lit_bool(true), make_lit_bool(false)))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn binary_logical_non_bool() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl("x", Kind::Bool as i32, Some(
                make_binary_op(10, make_lit_bool(true), make_lit_int(0)),
            )),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn unary_negate_int() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_return(Some(make_unary_op(0, make_lit_int(5)))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn unary_not_bool() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Bool as i32, vec![
            make_return(Some(make_unary_op(1, make_lit_bool(true)))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn duplicate_variable_declaration() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl("x", Kind::Int as i32, Some(make_lit_int(1))),
            make_decl("x", Kind::Int as i32, Some(make_lit_int(2))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn duplicate_parameter() {
    let prog = build_program("test", vec![
        make_fn("dup", vec![make_param("x", Kind::Int as i32), make_param("x", Kind::Int as i32)], Kind::Int as i32, vec![
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn assign_to_immutable_rejected() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            make_decl("x", Kind::Int as i32, None),
            make_assign("x", make_lit_int(42)),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn mutable_declaration_and_assign() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl_mut("x", Kind::Int as i32, None),
            make_assign("x", make_lit_int(42)),
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}

#[test]
fn uninitialized_variable_read_rejected() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Int as i32, vec![
            make_decl("x", Kind::Int as i32, None),
            make_return(Some(make_var("x"))),
        ]),
    ]);
    assert!(analyze_ast(&prog).is_err());
}

#[test]
fn expression_statement() {
    let prog = build_program("test", vec![
        make_fn("main", vec![], Kind::Void as i32, vec![
            Statement {
                location: None,
                stmt_kind: Some(statement::StmtKind::ExpressionStmt(make_lit_int(42))),
            },
        ]),
    ]);
    assert!(analyze_ast(&prog).is_ok());
}
