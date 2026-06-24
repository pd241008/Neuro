pub mod borrow_check;
pub mod error;
pub mod symbol_table;
pub mod semantic_analysis;

use shared_ast::{Program, VerifiedProgram};
use prost::Message;
use error::NeuroError;

pub fn audit_ast(input: &[u8]) -> Result<Vec<u8>, NeuroError> {
    let mut program = Program::decode(input)
        .map_err(|e| NeuroError::DeserializationError(e.to_string()))?;

    semantic_analysis::analyze_ast(&mut program)?;

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed: true,
        type_check_passed: true,
    };

    Ok(verified.encode_to_vec())
}
