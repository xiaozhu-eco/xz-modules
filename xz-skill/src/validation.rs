/// WASM binary validation utilities.
///
/// This module provides functions for validating WASM binaries
/// by checking their header bytes, ensuring they have a valid
/// magic number and supported version.

use crate::error::SkillError;

/// Validate a WASM binary by checking its header bytes.
///
/// A valid WASM binary starts with the magic bytes `\0asm` followed by a
/// 32-bit little-endian version number. Currently only version 1 is supported.
///
/// # Errors
/// Returns `SkillError::InvalidWasm` if:
/// - The byte slice is less than 8 bytes
/// - The magic bytes are not `\0asm`
/// - The version is not 1
pub fn validate_wasm(bytes: &[u8]) -> Result<(), SkillError> {
    if bytes.len() < 8 {
        return Err(SkillError::InvalidWasm("too short".into()));
    }
    if &bytes[0..4] != b"\0asm" {
        return Err(SkillError::InvalidWasm("invalid magic bytes".into()));
    }
    if &bytes[4..8] != &[1, 0, 0, 0] {
        return Err(SkillError::InvalidWasm("unsupported version".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_wasm_passes() {
        // A valid WASM binary: magic bytes "\0asm" + version 1 (little-endian)
        let wasm_bytes: &[u8] = b"\0asm\x01\x00\x00\x00";
        assert!(validate_wasm(wasm_bytes).is_ok());
    }

    #[test]
    fn empty_bytes_fails() {
        let result = validate_wasm(b"");
        assert!(result.is_err());
    }

    #[test]
    fn too_short_fails() {
        let result = validate_wasm(b"\0asm");
        // Only 4 bytes, should fail with "too short"
        assert!(result.is_err());
        if let Err(SkillError::InvalidWasm(msg)) = &result {
            assert_eq!(msg.as_str(), "too short");
        } else {
            panic!("Expected InvalidWasm error");
        }
    }

    #[test]
    fn bad_magic_fails() {
        // Invalid magic bytes, but valid length
        let result = validate_wasm(b"NOTasm\x01\x00\x00\x00");
        assert!(result.is_err());
        if let Err(SkillError::InvalidWasm(msg)) = &result {
            assert_eq!(msg.as_str(), "invalid magic bytes");
        } else {
            panic!("Expected InvalidWasm error");
        }
    }

    #[test]
    fn bad_version_fails() {
        // Valid magic but unsupported version (2)
        let result = validate_wasm(b"\0asm\x02\x00\x00\x00");
        assert!(result.is_err());
        if let Err(SkillError::InvalidWasm(msg)) = &result {
            assert_eq!(msg.as_str(), "unsupported version");
        } else {
            panic!("Expected InvalidWasm error");
        }
    }
}
