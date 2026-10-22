//! CI/CD demo payload generator (Task 4 of the paper evaluation).
//!
//! Mode `normal`:  signs a valid program with NEURO_SIGNING_KEY (what the
//!                 honest analyzer container would write to the shared volume).
//! Mode `exploit`: hand-crafts the unsigned use-after-move VerifiedProgram
//!                 with flags=true (the 145-byte cache-poisoner payload the
//!                 attacker writes WITHOUT ever seeing the signing key).
//!
//! Usage: ci_payload normal <output-file>   (requires NEURO_SIGNING_KEY)
//!        ci_payload exploit <output-file>

use hmac::{Hmac, Mac};
use prost::Message;
use sha2::Sha256;
use shared_ast::expression::ExprKind;
use shared_ast::statement::StmtKind;
use shared_ast::*;

fn make_type(kind: i32) -> Option<Type> {
    Some(Type { kind, custom_name: String::new() })
}

fn make_param(name: &str, kind: i32) -> Parameter {
    Parameter { name: name.to_string(), r#type: make_type(kind) }
}

fn make_lit_int(val: i64) -> Expression {
    Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(ExprKind::Literal(Literal { value: Some(literal::Value::IntVal(val)) })),
    }
}

fn make_var(name: &str) -> Expression {
    Expression {
        location: None,
        resolved_type: None,
        expr_kind: Some(ExprKind::Variable(VariableReference { name: name.to_string() })),
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
            resolved_type: None,
            expr_kind: Some(ExprKind::Call(FunctionCall {
                function_name: fn_name.to_string(),
                arguments: args,
            })),
        })),
    }
}

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
    Program { name: "ci_demo".to_string(), functions: funcs }
}

/// The exact use-after-move program from tests/boundary_tests.rs.
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

fn sign(verified: &VerifiedProgram, key: &str) -> Vec<u8> {
    let program_bytes = verified.program.as_ref().unwrap().encode_to_vec();
    let mut data = Vec::new();
    data.extend_from_slice(&(program_bytes.len() as u64).to_le_bytes());
    data.extend_from_slice(&program_bytes);
    data.push(verified.borrow_check_passed as u8);
    data.push(verified.type_check_passed as u8);

    type H = Hmac<Sha256>;
    let mut mac = H::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(&data);
    let tag = mac.finalize().into_bytes().to_vec();

    let mut signed = verified.clone();
    signed.signature = tag;
    signed.encode_to_vec()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (mode, out_path) = match args.get(1).map(|s| s.as_str()) {
        Some("normal") => ("normal", args.get(2).expect("need output path")),
        Some("exploit") => ("exploit", args.get(2).expect("need output path")),
        _ => {
            eprintln!("usage: ci_payload normal|exploit <output-file>");
            std::process::exit(2);
        }
    };

    match mode {
        "normal" => {
            let key = std::env::var("NEURO_SIGNING_KEY")
                .expect("analyzer container must have NEURO_SIGNING_KEY");
            let verified = VerifiedProgram {
                program: Some(valid_program()),
                borrow_check_passed: true,
                type_check_passed: true,
                signature: vec![],
            };
            let signed = sign(&verified, &key);
            let n = signed.len();
            std::fs::write(out_path, signed).expect("write signed output.verified.ast");
            eprintln!("[analyzer] wrote {} (signed, {} bytes)", out_path, n);
        }
        "exploit" => {
            // The cache poisoner builds the payload WITHOUT a signing key.
            // A 32-byte garbage signature lets the payload slip past trivial
            // presence checks so it reaches the actual HMAC comparison.
            let exploit = VerifiedProgram {
                program: Some(use_after_move_program()),
                borrow_check_passed: true,
                type_check_passed: true,
                signature: vec![0xAB; 32],
            };
            let bytes = exploit.encode_to_vec();
            let n = bytes.len();
            std::fs::write(out_path, bytes).expect("write exploit payload");
            eprintln!(
                "[poisoner] overwrote {} with exploit payload ({} bytes, unsigned, flags=true)",
                out_path,
                n
            );
        }
        _ => unreachable!(),
    }
}
