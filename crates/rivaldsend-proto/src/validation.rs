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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{FileMode, ManifestEntry, TransferManifest};

    fn entry(path: &str, size: u64) -> ManifestEntry {
        ManifestEntry {
            relative_path: path.into(),
            size,
            blake3: "a".repeat(64),
            mode: FileMode::File,
        }
    }

    fn manifest(files: Vec<ManifestEntry>, total_bytes: u64) -> TransferManifest {
        TransferManifest {
            transfer_id: uuid::Uuid::new_v4(),
            protocol_version: crate::messages::SemVer { major: 2, minor: 0, patch: 0 },
            files,
            total_bytes,
            created_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn blake3_hex_accepts_64_hex_chars() {
        assert!(is_valid_blake3_hex(&"ab".repeat(32)));
        assert!(is_valid_blake3_hex(&"AB".repeat(32)));
        assert!(!is_valid_blake3_hex(&"ab".repeat(31)));
        assert!(!is_valid_blake3_hex(&"ab".repeat(33)));
        assert!(!is_valid_blake3_hex(&"zz".repeat(32)));
        assert!(!is_valid_blake3_hex(""));
    }

    #[test]
    fn safe_paths() {
        assert!(is_safe_relative_path("photo.jpg"));
        assert!(is_safe_relative_path("docs/2024/photo.jpg"));
        assert!(!is_safe_relative_path(""));
        assert!(!is_safe_relative_path("/absolu/fichier"));
        assert!(!is_safe_relative_path("../echappe"));
        assert!(!is_safe_relative_path("a/../../b"));
        assert!(!is_safe_relative_path("a//b"));
        assert!(!is_safe_relative_path("a/./b"));
        assert!(!is_safe_relative_path("a\\b"));
        assert!(!is_safe_relative_path("a/b\0c"));
    }

    #[test]
    fn path_depth_and_name_limits() {
        let deep = (0..MAX_RELATIVE_PATH_DEPTH).map(|i| format!("d{i}")).collect::<Vec<_>>().join("/");
        assert!(is_safe_relative_path(&deep));
        assert!(!is_safe_relative_path(&format!("{deep}/trop-profond")));
        assert!(is_safe_relative_path(&"x".repeat(MAX_SINGLE_FILE_NAME_LEN)));
        assert!(!is_safe_relative_path(&"x".repeat(MAX_SINGLE_FILE_NAME_LEN + 1)));
    }

    #[test]
    fn constant_time_eq_cases() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[test]
    fn valid_manifest_passes() {
        let m = manifest(vec![entry("a.bin", 10), entry("sub/b.bin", 20)], 30);
        assert!(validate_manifest(&m).is_ok());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn manifest_rejects_total_mismatch() {
        let m = manifest(vec![entry("a.bin", 10)], 11);
        assert!(matches!(validate_manifest(&m), Err(ProtoError::Validation(_))));
    }

    #[test]
    fn manifest_rejects_unsafe_path_and_bad_hash() {
        let m = manifest(vec![entry("../x.bin", 10)], 10);
        assert!(matches!(validate_manifest(&m), Err(ProtoError::Validation(_))));
        let mut bad = entry("ok.bin", 10);
        bad.blake3 = "pas-un-hash".into();
        let m = manifest(vec![bad], 10);
        assert!(matches!(validate_manifest(&m), Err(ProtoError::Validation(_))));
    }

    #[test]
    fn manifest_rejects_oversize_total() {
        let huge = (MAX_TOTAL_TRANSFER_BYTES / 2) + 1;
        let m = manifest(vec![entry("a.bin", huge), entry("b.bin", huge)], huge * 2);
        assert!(matches!(validate_manifest(&m), Err(ProtoError::Limits(_))));
    }
}
