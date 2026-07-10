pub mod borrow_check;
pub mod error;
pub mod symbol_table;
pub mod semantic_analysis;

use shared_ast::{Program, VerifiedProgram};
use prost::Message;
use error::NeuroError;
use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Compute HMAC-SHA256 over the VerifiedProgram fields 1-3 (program, borrow_check_passed, type_check_passed).
/// The signature field (field 4) is excluded from the HMAC input.
///
/// SECURITY-RELEVANT BYTE LAYOUT (must match backend/LLVMEmitter.cpp::computeHmac exactly):
///   [0..8]   u64 LE  — byte length of serialized Program protobuf
///   [8..8+N] [u8; N] — serialized Program protobuf (prost encode_to_vec)
///   [8+N]    u8      — borrow_check_passed as 0 or 1
///   [8+N+1]  u8      — type_check_passed as 0 or 1
///
/// Total HMAC input: 8 + N + 2 bytes where N = program.encode_to_vec().len().
/// This layout is intentionally NOT a protobuf encoding — it uses fixed-width
/// length prefix + raw bytes + single-byte booleans to ensure deterministic,
/// platform-independent serialization that both Rust and C++ can compute
/// identically without shared schema dependencies.
fn compute_hmac(program: &Program, borrow_check_passed: bool, type_check_passed: bool, key: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(key)
        .expect("HMAC can take key of any size");
    
    // Encode fields 1-3 in a deterministic order for HMAC
    // We encode each field separately to ensure deterministic ordering
    let program_bytes = program.encode_to_vec();
    
    mac.update(&(program_bytes.len() as u64).to_le_bytes());
    mac.update(&program_bytes);
    mac.update(&[borrow_check_passed as u8]);
    mac.update(&[type_check_passed as u8]);
    
    mac.finalize().into_bytes().to_vec()
}

pub fn audit_ast(input: &[u8]) -> Result<Vec<u8>, NeuroError> {
    let mut program = Program::decode(input)
        .map_err(|e| NeuroError::DeserializationError(e.to_string()))?;

    semantic_analysis::analyze_ast(&mut program)?;

    let borrow_check_passed = true;
    let type_check_passed = true;

    // Get signing key from environment variable
    let key = std::env::var("NEURO_SIGNING_KEY")
        .map_err(|_| NeuroError::SigningError("NEURO_SIGNING_KEY environment variable not set".to_string()))?;
    
    if key.is_empty() {
        return Err(NeuroError::SigningError("NEURO_SIGNING_KEY is empty".to_string()));
    }

    let signature = compute_hmac(&program, borrow_check_passed, type_check_passed, key.as_bytes());

    let verified = VerifiedProgram {
        program: Some(program),
        borrow_check_passed,
        type_check_passed,
        signature,
    };

    Ok(verified.encode_to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hmac_deterministic() {
        let program = Program {
            name: "test".to_string(),
            functions: vec![],
        };
        let key = b"test-secret-key";
        
        let sig1 = compute_hmac(&program, true, true, key);
        let sig2 = compute_hmac(&program, true, true, key);
        let sig3 = compute_hmac(&program, false, true, key);
        
        assert_eq!(sig1, sig2, "HMAC should be deterministic");
        assert_ne!(sig1, sig3, "Different inputs should produce different HMACs");
        assert_eq!(sig1.len(), 32, "HMAC-SHA256 should be 32 bytes");
    }
}
