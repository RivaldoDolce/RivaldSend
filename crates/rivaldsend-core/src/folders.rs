use std::path::{Path, PathBuf};
use unicode_normalization::UnicodeNormalization;
use crate::error::CoreError;
use rivaldsend_proto::{FileMode, ManifestEntry};

pub fn normalize_nfc(s: &str) -> String {
    s.nfc().collect()
}

pub fn is_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).map(|m| m.file_type().is_symlink()).unwrap_or(false)
}

pub fn collect_manifest_entries(root: &Path, follow_symlinks: bool) -> Result<Vec<ManifestEntry>, CoreError> {
    let mut entries = Vec::new();
    let mut case_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    collect_recursive(root, root, &mut entries, &mut case_map, follow_symlinks, 0)?;
    Ok(entries)
}

fn collect_recursive(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
    case_map: &mut std::collections::HashMap<String, String>,
    follow: bool,
    depth: usize,
) -> Result<(), CoreError> {
    if depth > rivaldsend_proto::limits::MAX_RELATIVE_PATH_DEPTH {
        return Err(CoreError::Validation("profondeur de dossier dépassée".into()));
    }
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if !follow && is_symlink(&path) {
            continue;
        }
        let rel = path.strip_prefix(root).unwrap().to_string_lossy().to_string();
        let rel_nfc = normalize_nfc(&rel);
        if rel_nfc != rel {
            // normalisation NFC implicite
        }
        let lower = rel_nfc.to_lowercase();
        if let Some(prev) = case_map.get(&lower) {
            if prev != &rel_nfc {
                eprintln!("collision de casse détectée: {} vs {}", prev, rel_nfc);
            }
            continue;
        }
        case_map.insert(lower, rel_nfc.clone());
        let meta = std::fs::metadata(&path)?;
        if meta.is_dir() {
            entries.push(ManifestEntry { relative_path: rel_nfc.clone(), size: 0, blake3: "0".repeat(64), mode: FileMode::Directory });
            collect_recursive(root, &path, entries, case_map, follow, depth + 1)?;
        } else if meta.is_file() {
            let size = meta.len();
            let hash = blake3::hash(b"").to_hex().to_string();
            entries.push(ManifestEntry { relative_path: rel_nfc, size, blake3: hash, mode: FileMode::File });
        }
    }
    Ok(())
}

pub fn ensure_no_traversal(dest: &Path, rel: &str) -> Result<PathBuf, CoreError> {
    let candidate = dest.join(rel);
    let canon_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
    let canon_candidate = candidate.canonicalize().unwrap_or(candidate.clone());
    if !canon_candidate.starts_with(&canon_dest) {
        return Err(CoreError::Validation(format!("traversal détecté: {}", rel)));
    }
    Ok(candidate)
}
