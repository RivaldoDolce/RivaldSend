use subtle::ConstantTimeEq;
use unicode_normalization::UnicodeNormalization;
use crate::error::ProtoError;
use crate::limits::{MAX_MANIFEST_BYTES, MAX_MANIFEST_FILES, MAX_RELATIVE_PATH_DEPTH, MAX_SINGLE_FILE_NAME_LEN, MAX_TOTAL_TRANSFER_BYTES};
use crate::manifest::TransferManifest;
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).unwrap_u8() == 1
}
pub fn is_valid_blake3_hex(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn is_safe_relative_path(p: &str) -> bool {
    if p.is_empty() || p.starts_with('/') || p.starts_with('\\') {
        return false;
    }
    if p.nfc().collect::<String>() != p {
        return false;
    }
    let mut depth = 0usize;
    for comp in p.split('/') {
        if comp.is_empty() || comp == "." {
            return false;
        }
        if comp == ".." {
            return false;
        }
        if comp.len() > MAX_SINGLE_FILE_NAME_LEN {
            return false;
        }
        if comp.contains('\\') || comp.contains('\0') {
            return false;
        }
        depth += 1;
        if depth > MAX_RELATIVE_PATH_DEPTH {
            return false;
        }
    }
    if p.contains("//") {
        return false;
    }
    true
}
pub fn validate_manifest(m: &TransferManifest) -> Result<(), ProtoError> {
    if m.files.len() > MAX_MANIFEST_FILES {
        return Err(ProtoError::Limits(format!("too many files: {}", m.files.len())));
    }
    if m.total_bytes > MAX_TOTAL_TRANSFER_BYTES {
        return Err(ProtoError::Limits(format!("total_bytes exceeds limit: {}", m.total_bytes)));
    }
    let mut sum = 0u64;
    for f in &m.files {
        if !is_safe_relative_path(&f.relative_path) {
            return Err(ProtoError::Validation(format!("unsafe path: {}", f.relative_path)));
        }
        if !is_valid_blake3_hex(&f.blake3) {
            return Err(ProtoError::Validation(format!("invalid blake3: {}", f.blake3)));
        }
        sum = sum.checked_add(f.size).ok_or_else(|| ProtoError::Limits("total overflow".into()))?;
    }
    if sum != m.total_bytes {
        return Err(ProtoError::Validation(format!("total_bytes mismatch: sum {sum} != {}", m.total_bytes)));
    }
    let json_len = serde_json::to_vec(m).map_err(|e| ProtoError::Validation(e.to_string()))?.len();
    if json_len > MAX_MANIFEST_BYTES {
        return Err(ProtoError::Limits(format!("manifest too large: {json_len}")));
    }
    Ok(())
}
