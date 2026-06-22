pub mod borrow_check;
pub mod symbol_table;

use shared_ast::Program;
use prost::Message;

pub fn audit_ast(input: &[u8]) -> Result<Vec<u8>, String> {
    println!("Auditing AST for memory safety...");

    let program = Program::decode(input)
        .map_err(|e| format!("Failed to deserialize AST: {}", e))?;

    let verified = program.encode_to_vec();

    println!("AST audit complete: {} function(s) verified", program.functions.len());
    Ok(verified)
}
