pub mod borrow_check;
pub mod error;
pub mod symbol_table;
pub mod semantic_analysis;

use shared_ast::Program;
use prost::Message;
use error::NeuroError;

pub fn audit_ast(input: &[u8]) -> Result<Vec<u8>, NeuroError> {
    let program = Program::decode(input)
        .map_err(|e| NeuroError::DeserializationError(e.to_string()))?;

    semantic_analysis::analyze_ast(&program)?;

    let verified = program.encode_to_vec();

    Ok(verified)
}
